//! Arayüze açılan komutlar.
//!
//! Her komut kilidi alıp bırakıyor; motor kilidi çağrı süresince tutulmuyor
//! çünkü bazı işlemler (DNS ölçümü, yol testi) saniyeler sürüyor ve o sırada
//! arayüzün durum sorgusu bloke olmamalı. Uzun işlemler kilidi hiç almıyor:
//! saf ağ işleri motorun durumuna dokunmuyor.

use tauri::{Emitter, State};

use crate::error::Result;
use crate::ledger::Kayit;
use crate::library;
use crate::monitor::olcum::{self, KareOlcumDurumu};
use crate::monitor::{GecmisOzeti, Karsilastirma, Ornek, OturumKaydi, Ozet, Satir};
use crate::network_boost::{self, DnsSonucu, TcpDurumu, YolSonucu};
use crate::profile_engine::{self, Profil};
use crate::settings::Ayarlar;
use crate::state::{Durum, Motor, UygulamaSonucu};
use crate::system_boost::{self, Surec};
use crate::ucuncu_taraf::{self, Liste as UcuncuTarafListesi};

/// Motor kilidi. `parking_lot` seçildi: standart `Mutex`'in zehirlenme
/// (poisoning) davranışı burada işe yaramıyor — bir panik sonrası motoru
/// erişilemez kılmak, geri alma arayüzünü de kapatırdı.
pub type MotorState<'a> = State<'a, parking_lot::Mutex<Motor>>;

/// Arayüze yayınlanan olaylar. Adlar `src/lib/api.ts` ile birebir aynı.
pub const OLAY_DURUM: &str = "muifly://durum";
pub const OLAY_GUNLUK: &str = "muifly://gunluk";
pub const OLAY_ORNEK: &str = "muifly://ornek";

#[tauri::command]
pub fn surum() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Üçüncü taraf bileşenlerin listesi (LICENSE → üçüncü taraf bildirimleri).
///
/// Lisans METİNLERİ burada yok: liste 258 bileşen ve metinlerin tamamı bir
/// megabaytın üstünde. Kullanıcı bir bileşeni açtığında `ucuncu_taraf_metin`
/// çağrılıyor.
#[tauri::command]
pub fn ucuncu_taraf_listesi() -> Result<UcuncuTarafListesi> {
    ucuncu_taraf::liste()
}

/// Tek bir lisans metni. `no`, listedeki `metinNo` alanından geliyor.
#[tauri::command]
pub fn ucuncu_taraf_metni(no: usize) -> Result<String> {
    ucuncu_taraf::metin(no)
}

#[tauri::command]
pub fn durum(motor: MotorState<'_>) -> Durum {
    motor.lock().durum()
}

#[tauri::command]
pub fn gunluk(motor: MotorState<'_>, adet: Option<usize>) -> Vec<Satir> {
    motor.lock().gunluk.son(adet.unwrap_or(200))
}

#[tauri::command]
pub fn gunlugu_temizle(motor: MotorState<'_>) {
    motor.lock().gunluk.temizle();
}

// ---------------------------------------------------------------------------
// Oturum geçmişi
// ---------------------------------------------------------------------------

/// Diskteki oturum kayıtları, yeniden eskiye.
#[tauri::command]
pub fn gecmis(motor: MotorState<'_>) -> Vec<OturumKaydi> {
    motor.lock().gecmis.liste().to_vec()
}

#[tauri::command]
pub fn gecmis_ozeti(motor: MotorState<'_>) -> GecmisOzeti {
    motor.lock().gecmis_ozeti()
}

/// Geçmişi siler — dosya dahil.
#[tauri::command]
pub fn gecmisi_temizle(motor: MotorState<'_>) {
    motor.lock().gecmisi_temizle();
}

/// Geçmişi düz metin rapor olarak kullanıcının seçtiği dosyaya yazar.
///
/// Yol arayüzden geliyor ve **kullanıcının kendi seçtiği** kaydetme
/// penceresinden çıkıyor; program kendi başına bir yere dosya bırakmıyor.
/// Rapor hiçbir yere gönderilmiyor: bu bir dosya yazma işlemi, bir paylaşım
/// değil.
#[tauri::command]
pub fn gecmis_disa_aktar(motor: MotorState<'_>, yol: String) -> Result<()> {
    let m = motor.lock();
    let metin = crate::monitor::gecmis::rapor_metni(m.gecmis.liste());
    drop(m);
    std::fs::write(&yol, metin)?;
    motor.lock().gunluk.bilgi(
        crate::monitor::Kategori::Uygulama,
        format!("oturum geçmişi dosyaya yazıldı: {yol}"),
    );
    Ok(())
}

#[tauri::command]
pub fn ornekler(motor: MotorState<'_>) -> Vec<Ornek> {
    motor.lock().ornekler()
}

#[tauri::command]
pub fn ozet(motor: MotorState<'_>) -> Ozet {
    motor.lock().ozet()
}

#[tauri::command]
pub fn karsilastirma(motor: MotorState<'_>) -> Option<Karsilastirma> {
    motor.lock().karsilastirma()
}

// ---------------------------------------------------------------------------
// Geri alma
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn bekleyen_geri_almalar(motor: MotorState<'_>) -> Vec<Kayit> {
    motor.lock().defter.liste().to_vec()
}

/// Tek bir kaydı geri alır.
#[tauri::command]
pub fn geri_al(motor: MotorState<'_>, id: u64) -> Result<()> {
    // Kilidi bir kez çözüyoruz: `MutexGuard` üzerinden iki alanı ayrı ayrı
    // ödünç almak, ödünç denetleyicisi için tek bir `DerefMut` çağrısı
    // olduğundan çakışıyor.
    let mut kilit = motor.lock();
    let m = &mut *kilit;
    let Some(kayit) = m.defter.dus(id) else {
        return Ok(());
    };
    let sonuc = crate::revert::kayitlari_isle(vec![kayit], &mut m.defter, &mut m.gunluk);
    if let Some((ozet, sebep)) = sonuc.basarisiz.into_iter().next() {
        return Err(crate::error::Error::Network(format!("{ozet}: {sebep}")));
    }
    Ok(())
}

/// "Varsayılana dön": kalıcı değişiklikler dahil her şey.
#[tauri::command]
pub fn hepsini_geri_al(motor: MotorState<'_>) -> UygulamaSonucu {
    let sonuc = motor.lock().hepsini_geri_al();
    UygulamaSonucu {
        uygulanan: vec![format!("{} değişiklik geri alındı", sonuc.geri_alinan)],
        atlanan: if sonuc.gereksiz > 0 {
            vec![format!("{} kayıt zaten geçersizdi", sonuc.gereksiz)]
        } else {
            Vec::new()
        },
        hatalar: sonuc
            .basarisiz
            .into_iter()
            .map(|(o, s)| format!("{o}: {s}"))
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// Profiller
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn profiller(motor: MotorState<'_>) -> Vec<Profil> {
    motor.lock().profiller.clone()
}

#[tauri::command]
pub fn profil_kaydet(motor: MotorState<'_>, profil: Profil) -> Result<Vec<String>> {
    let mut m = motor.lock();

    let dizin = m.profil_dizini().clone();
    let (kaydedilen, duzeltmeler) = profile_engine::store::kaydet(&dizin, profil)?;
    m.gunluk.bilgi(
        crate::monitor::Kategori::Profil,
        format!("'{}' profili kaydedildi", kaydedilen.display_name),
    );
    for d in &duzeltmeler {
        m.gunluk.uyari(crate::monitor::Kategori::Profil, d.clone());
    }
    m.profilleri_yenile();
    Ok(duzeltmeler)
}

#[tauri::command]
pub fn profil_sil(motor: MotorState<'_>, kimlik: String) -> Result<()> {
    let mut m = motor.lock();
    let dizin = m.profil_dizini().clone();
    profile_engine::store::sil(&dizin, &kimlik)?;
    m.gunluk.bilgi(
        crate::monitor::Kategori::Profil,
        format!("'{kimlik}' profili silindi"),
    );
    m.profilleri_yenile();
    Ok(())
}

/// Profili verilen dosyaya yazar.
///
/// Yol arayüzden geliyor: dosyayı kullanıcı sistem kaydetme penceresinde
/// seçiyor (`dialog:allow-save`). Program kendi kendine bir yere dosya
/// bırakmıyor.
#[tauri::command]
pub fn profil_disa_aktar(motor: MotorState<'_>, kimlik: String, yol: String) -> Result<()> {
    let mut m = motor.lock();
    let profil = m
        .profiller
        .iter()
        .find(|p| p.profile_id == kimlik)
        .cloned()
        .ok_or(crate::error::Error::ProfileNotFound(kimlik))?;
    profile_engine::aktarim::disa_aktar(&profil, std::path::Path::new(&yol))?;
    m.gunluk.bilgi(
        crate::monitor::Kategori::Profil,
        format!("'{}' profili dışa aktarıldı: {yol}", profil.display_name),
    );
    Ok(())
}

/// Kayıtlı bir profilin ne yapacağı — düz cümlelerle.
///
/// İçe aktarma önizlemesiyle aynı metinler (`profile_engine::aktarim`):
/// kullanıcının kendi profili için de aynı soruya aynı cevabı vermesi
/// gerekiyor.
#[tauri::command]
pub fn profil_etkileri(motor: MotorState<'_>, kimlik: String) -> Result<Vec<String>> {
    let m = motor.lock();
    let profil = m
        .profiller
        .iter()
        .find(|p| p.profile_id == kimlik)
        .ok_or(crate::error::Error::ProfileNotFound(kimlik))?;
    Ok(profile_engine::aktarim::etkiler(profil))
}

/// İçe aktarılacak dosyayı okur ve ne yapacağını anlatır. **Diske yazmaz.**
///
/// İki adımlı akışın ilk adımı: `docs/PROFILES.md` güvenlik notu, başka
/// birinin profilinin içeriği gösterilmeden uygulanmamasını istiyor.
#[tauri::command]
pub fn profil_onizle(motor: MotorState<'_>, yol: String) -> Result<profile_engine::Onizleme> {
    let m = motor.lock();
    profile_engine::aktarim::onizle(std::path::Path::new(&yol), &m.profiller)
}

/// Önizlenen profili kaydeder.
///
/// Profilin kendisi arayüzden geri geliyor, dosya yolu değil: kullanıcı
/// onayladığı şeyi kaydediyor, aradan geçen sürede dosyada değişen bir şeyi
/// değil.
#[tauri::command]
pub fn profil_ice_aktar(
    motor: MotorState<'_>,
    profil: Profil,
    uzerine_yaz: bool,
) -> Result<Vec<String>> {
    let mut m = motor.lock();

    let dizin = m.profil_dizini().clone();
    let mevcut = m.profiller.clone();
    let (kaydedilen, duzeltmeler) =
        profile_engine::aktarim::ice_aktar(&dizin, &mevcut, profil, uzerine_yaz)?;
    m.gunluk.bilgi(
        crate::monitor::Kategori::Profil,
        format!(
            "'{}' profili içe aktarıldı (kimlik: {})",
            kaydedilen.display_name, kaydedilen.profile_id
        ),
    );
    for d in &duzeltmeler {
        m.gunluk.uyari(crate::monitor::Kategori::Profil, d.clone());
    }
    m.profilleri_yenile();
    Ok(duzeltmeler)
}

/// Profili şu anda öndeki oyuna uygular.
///
/// Elle tetikleniyor: otomatik uygulama ayarı kapalıyken (varsayılan) tek
/// uygulama yolu bu.
#[tauri::command]
pub fn profil_uygula(motor: MotorState<'_>, kimlik: String) -> Result<UygulamaSonucu> {
    let mut m = motor.lock();

    let onde = system_boost::detect::ondeki_pencere()?;
    let profil = m
        .profiller
        .iter()
        .find(|p| p.profile_id == kimlik)
        .cloned()
        .ok_or(crate::error::Error::ProfileNotFound(kimlik))?;

    Ok(m.profil_uygula(&profil, onde.pid))
}

/// Öndeki uygulamaya, profili olmasa da hafif varsayılanı uygular.
///
/// Asıl iş `Motor::ondekine_uygula`'da: tepsi menüsü de aynı yolu kullanıyor.
#[tauri::command]
pub fn ondekine_uygula(motor: MotorState<'_>) -> Result<UygulamaSonucu> {
    motor.lock().ondekine_uygula()
}

#[tauri::command]
pub fn oturumu_kapat(motor: MotorState<'_>) -> usize {
    motor.lock().oturumu_kapat().geri_alinan
}

// ---------------------------------------------------------------------------
// Sistem bilgisi
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn ondeki_pencere() -> Result<system_boost::OndekiPencere> {
    system_boost::detect::ondeki_pencere()
}

/// Çalışan süreçler — kullanıcının dondurma listesini kurabilmesi için.
///
/// Dondurulamayacaklar listeden ÇIKARILMIYOR, arayüzde işaretli gösteriliyor:
/// kullanıcı neden ekleyemediğini görmeli (şeffaflık ilkesi).
#[tauri::command]
pub fn surecler() -> Result<Vec<Surec>> {
    let mut liste = system_boost::detect::surec_listesi()?;
    liste.sort_by(|a, b| a.ad.cmp(&b.ad));
    liste.dedup_by(|a, b| a.ad == b.ad);
    Ok(liste)
}

#[tauri::command]
pub fn dondurma_adaylari() -> Vec<String> {
    system_boost::suspend::ONERILEN_ADAYLAR
        .iter()
        .map(|s| s.to_string())
        .collect()
}

// ---------------------------------------------------------------------------
// Oyun kütüphanesi
//
// Hepsi salt okuma ve hepsi yerel: bu bölümde ağa çıkan tek bir satır yok
// (`crate::library` modül belgesi, karar #25). Sistemde de hiçbir şey
// değiştirilmiyor — üretilen tek şey, kullanıcının onaylayacağı bir profil
// taslağı.
// ---------------------------------------------------------------------------

/// Steam ve Epic kütüphanelerini tarar.
#[tauri::command]
pub async fn oyunlari_tara() -> Vec<library::Oyun> {
    // Yüzlerce klasör okunuyor; arayüz iş parçacığında yapılamaz.
    tauri::async_runtime::spawn_blocking(library::tara)
        .await
        .unwrap_or_default()
}

/// Oyunun kapak görseli ya da exe ikonu, `data:` adresi olarak.
///
/// Kart ekranda görününce isteniyor; taramada hepsi birden üretilmiyor.
#[tauri::command]
pub async fn oyun_gorseli(kimlik: String) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || library::gorsel(&kimlik))
        .await
        .unwrap_or_default()
}

/// Kullanıcının dosya penceresinden seçtiği exe'yi kütüphaneye ekler.
#[tauri::command]
pub fn oyun_elle_ekle(yol: String) -> Result<library::Oyun> {
    library::elle_ekle(&yol).ok_or_else(|| {
        crate::error::Error::ProfileInvalid("seçilen dosya bir .exe değil ya da okunamıyor".into())
    })
}

/// Bir oyundan profil taslağı üretir. **Hiçbir şey kaydetmez.**
///
/// Taslak arayüzde açılıyor; kaydetme de uygulama da kullanıcının onayıyla.
#[tauri::command]
pub fn profil_taslagi(ad: String, exeler: Vec<String>) -> profile_engine::katalog::Taslak {
    profile_engine::katalog::taslak(&ad, &exeler)
}

/// Bir exe katalogda tanınıyor mu? Süreç listesinde rozet göstermek için.
#[tauri::command]
pub fn katalog_girdisi(exe: String) -> Option<profile_engine::katalog::Girdi> {
    profile_engine::katalog::ara(&exe).cloned()
}

/// Şu an çalışan süreçlerden gömülü katalogda tanınanlar.
///
/// Profil penceresinde "elle yaz" alanının yanına konan öneriler bunlar.
/// Tek çağrıda hesaplanıyor: arayüzün her süreç için ayrı ayrı katalog
/// sorması gereksiz bir gidiş geliş olurdu.
#[tauri::command]
pub fn taninan_surecler() -> Result<Vec<profile_engine::katalog::Girdi>> {
    let mut cikti: Vec<profile_engine::katalog::Girdi> = system_boost::detect::surec_listesi()?
        .iter()
        .filter_map(|s| profile_engine::katalog::ara(&s.ad).cloned())
        .collect();
    cikti.sort_by(|a, b| a.ad.cmp(&b.ad));
    cikti.dedup_by(|a, b| a.exe == b.exe);
    Ok(cikti)
}

// ---------------------------------------------------------------------------
// Ağ
// ---------------------------------------------------------------------------

/// DNS karşılaştırması. Sistemi DEĞİŞTİRMEZ, yalnızca ölçer.
#[tauri::command]
pub async fn dns_karsilastir(deneme: Option<u32>) -> Result<Vec<DnsSonucu>> {
    let deneme = deneme.unwrap_or(3).clamp(1, 10);
    // Ölçüm bloke edici; ayrı bir iş parçacığında koşuyor ki arayüz donmasın.
    Ok(
        tauri::async_runtime::spawn_blocking(move || network_boost::dns::karsilastir(deneme, 1500))
            .await
            .unwrap_or_default(),
    )
}

#[tauri::command]
pub async fn yol_testi(hedef: String) -> Result<YolSonucu> {
    tauri::async_runtime::spawn_blocking(move || {
        network_boost::latency::yol_testi(&hedef, 20, 1500)
    })
    .await
    .map_err(|e| crate::error::Error::Network(e.to_string()))?
}

// ---------------------------------------------------------------------------
// Kare ölçümü
// ---------------------------------------------------------------------------

/// Kare ölçümünün bu makinede yapılıp yapılamayacağı ve neyin gerektiği.
///
/// Arayüz bunu ölçümü başlatmadan ÖNCE okuyor: kullanıcı UAC istemiyle
/// karşılaşmadan önce neden istendiğini görmeli (tasarım ilkesi 5).
#[tauri::command]
pub fn kare_olcum_durumu() -> KareOlcumDurumu {
    KareOlcumDurumu {
        kullanilabilir: olcum::yardimci_yolu().is_ok(),
        yetki_gerekiyor: true,
        en_kisa_saniye: olcum::EN_KISA_SANIYE,
        en_uzun_saniye: olcum::EN_UZUN_SANIYE,
        aciklama: "Kare süresi ölçümü Windows'un olay izleme (ETW) altyapısını \
                   kullanıyor ve bu altyapı yönetici yetkisi istiyor. Muifly'ın \
                   kendisi yükseltilmiş çalışmıyor: ölçüm, yalnızca okuma yapan \
                   ayrı ve kısa ömürlü bir yardımcı süreçte yapılıyor. Ölçüm \
                   boyunca sistemde hiçbir şey değişmiyor."
            .into(),
    }
}

/// Motorun **algıladığı oyunu** belirtilen süre boyunca ölçer.
///
/// Arayüz PID taşımıyor, taşıyamaz da: hedefi motor seçiyor. İki sebep var.
/// Birincisi karar #25'in gerekçesiyle aynı — webview'e "istediğin süreci
/// ölç" yüzeyi açmamak. İkincisi daha somut: kullanıcı ölçüm düğmesine
/// bastığı anda **öndeki pencere Muifly'ın kendisi olur**, yani öndeki
/// pencereye bakan bir ölçüm her seferinde yanlış süreci ölçerdi. Motorun
/// mod durumu oyunu alt-tab tamponuyla birlikte hatırlıyor.
///
/// UAC istemi burada çıkıyor. Kullanıcı reddederse bu bir hata değil:
/// günlüğe yazılıyor ve hiçbir şey değişmiyor.
#[tauri::command]
pub async fn oyunu_olc(
    motor: MotorState<'_>,
    saniye: Option<u64>,
) -> Result<crate::monitor::olcum::OlcumRaporu> {
    let (pid, surec) = {
        let m = motor.lock();
        match (m.mod_.oyun_pid(), m.mod_.surec()) {
            (Some(p), Some(s)) => (p, s.to_string()),
            _ => {
                return Err(crate::error::Error::Olcum(
                    "ölçüm için algılanmış bir oyun yok — oyunu açıp bir kez öne \
                     getir, sonra buraya dön"
                        .into(),
                ))
            }
        }
    };

    let saniye = saniye
        .unwrap_or(20)
        .clamp(olcum::EN_KISA_SANIYE, olcum::EN_UZUN_SANIYE);

    // Ölçüm bloke edici ve saniyeler sürüyor; motor kilidi bu sırada
    // ALINMIYOR ki arayüzün durum sorgusu donmasın.
    let sonuc = tauri::async_runtime::spawn_blocking(move || olcum::yukselterek_olc(pid, saniye))
        .await
        .map_err(|e| crate::error::Error::Olcum(e.to_string()))?;

    let mut m = motor.lock();
    match &sonuc {
        Ok(s) => {
            let mesaj = match &s.ozet {
                Some(o) => format!(
                    "Kare ölçümü ({surec}, {saniye} sn): {} sunum, ortalama {:.1} kare/sn, \
                     en kötü %1 {:.1} ms. Bu senin makinende bu oturumda ölçülen değer.",
                    o.kare_sayisi, o.ort_fps, o.p1_kotu_ms
                ),
                None => format!(
                    "Kare ölçümü ({surec}): {} sunum toplandı, özet çıkarmaya yetmedi.",
                    s.kare_sayisi
                ),
            };
            m.gunluk.bilgi(crate::monitor::Kategori::Olcum, mesaj);
            // Sürmekte olan bir oturum varsa ölçüm onun geçmiş kaydına da
            // giriyor: "o akşam ne ölçmüştüm" sorusunun cevabı, program
            // kapandıktan sonra da dursun.
            if let Some(o) = s.ozet {
                m.kare_olcumu_kaydet(o);
            }
        }
        Err(h) => {
            m.gunluk.bilgi(
                crate::monitor::Kategori::Olcum,
                format!("Kare ölçümü ({surec}): {h}"),
            );
        }
    }
    drop(m);

    sonuc
        .map(|s| olcum::OlcumRaporu {
            surec,
            pid,
            saniye,
            sonuc: s,
        })
        .map_err(|h| crate::error::Error::Olcum(h.to_string()))
}

#[tauri::command]
pub fn tcp_durumu() -> Result<TcpDurumu> {
    network_boost::tcp::durum()
}

/// TCP ayarlarını uygular (kalıcı, geri alınabilir).
#[tauri::command]
pub fn tcp_uygula(motor: MotorState<'_>) -> Result<UygulamaSonucu> {
    let mut m = motor.lock();
    let mut cikti = UygulamaSonucu::default();

    let (nagle, hatalar) = network_boost::tcp::nagle_kapat()?;
    for undo in nagle {
        let ozet = undo.geri_alma_ozeti();
        m.defter.kaydet(ozet, crate::ledger::Kapsam::Kalici, undo);
    }
    let throttling = network_boost::tcp::throttling_kaldir()?;
    for undo in throttling {
        let ozet = undo.geri_alma_ozeti();
        m.defter.kaydet(ozet, crate::ledger::Kapsam::Kalici, undo);
    }

    m.gunluk.yaz(
        crate::monitor::Duzey::Aksiyon,
        crate::monitor::Kategori::Ag,
        "TCP ve ağ zamanlayıcı ayarları uygulandı",
        None,
    );
    cikti
        .uygulanan
        .push("TCP ve ağ zamanlayıcı ayarları uygulandı".into());
    cikti.hatalar = hatalar;
    Ok(cikti)
}

#[tauri::command]
pub fn qos_ilkeleri() -> Result<Vec<network_boost::qos::Ilke>> {
    network_boost::qos::listele()
}

/// Muifly'ın eklediği QoS ilkelerini kaldırır.
///
/// Geri alma sayılıyor: program kaldırılsa bile sistemde bıraktığı iz
/// temizlenebilmeli (tasarım ilkesi 1).
#[tauri::command]
pub fn qos_kaldir(motor: MotorState<'_>) -> Result<usize> {
    let adet = network_boost::qos::bizim_ilkeleri_kaldir()?;
    motor.lock().gunluk.yaz(
        crate::monitor::Duzey::GeriAlma,
        crate::monitor::Kategori::Ag,
        format!("{adet} QoS ilkesi kaldırıldı"),
        None,
    );
    Ok(adet)
}

/// Ağ ayarlarının kullanıcıya nasıl anlatıldığı.
///
/// Metinler Rust tarafında: `DESIGN_PRINCIPLES.md` madde 4 kontrolü tek
/// yerde yapılabilsin ve testle korunabilsin (`network_boost::tcp` testleri).
#[tauri::command]
pub fn ag_aciklamalari() -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = network_boost::tcp::ACIKLAMALAR
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
    v.push((
        "QoS önceliklendirme".to_string(),
        network_boost::qos::ACIKLAMA.to_string(),
    ));
    v
}

// ---------------------------------------------------------------------------
// Ayarlar ve açılış
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn ayarlar(motor: MotorState<'_>) -> Ayarlar {
    motor.lock().ayarlar.clone()
}

#[tauri::command]
pub fn ayarlari_yaz(
    uygulama: tauri::AppHandle,
    motor: MotorState<'_>,
    ayarlar: Ayarlar,
) -> Result<Ayarlar> {
    let mut m = motor.lock();
    m.ayarlar = ayarlar.normalize();
    m.ayarlari_kaydet()?;
    let durum = m.durum();
    drop(m);
    let _ = uygulama.emit(OLAY_DURUM, durum);
    Ok(motor.lock().ayarlar.clone())
}

#[tauri::command]
pub fn otomatik_baslatma_ayarla(motor: MotorState<'_>, acik: bool) -> Result<bool> {
    if acik {
        system_boost::startup::otomatik_baslatmayi_ac()?;
    } else {
        system_boost::startup::otomatik_baslatmayi_kapat()?;
    }
    let durum = system_boost::startup::otomatik_baslatma_acik()?;
    motor.lock().gunluk.bilgi(
        crate::monitor::Kategori::Uygulama,
        if durum {
            "Windows ile başlatma açıldı"
        } else {
            "Windows ile başlatma kapatıldı"
        },
    );
    Ok(durum)
}

/// Açılış komutunun registry'ye ne yazdığı — kullanıcı görebilsin.
#[tauri::command]
pub fn otomatik_baslatma_komutu() -> Result<Option<String>> {
    system_boost::startup::otomatik_baslatma_komutu()
}

/// Programın hangi özellikleri BİLİNÇLİ olarak yapmadığı.
///
/// Arayüzde "Ne yapmaz" bölümü olarak gösteriliyor. Bir performans aracının
/// yapmadıklarını söylemesi, yaptıklarını saymasından daha güven verici.
#[tauri::command]
pub fn yapilmayanlar() -> Vec<(String, String)> {
    vec![
        (
            "Oyun sürecine kod enjekte etmez".into(),
            "DLL injection, bellek hook'lama ya da oyun dosyalarını değiştirme yok. Yalnızca Windows'un kendi API'leri kullanılıyor.".into(),
        ),
        (
            "Ağ trafiğini kendi sunucularına yönlendirmez".into(),
            "VPN tüneli yok. Bağlantın olduğu gibi kalıyor; ölçüm yapılıyor, yönlendirme yapılmıyor.".into(),
        ),
        (
            "DNS ayarını kendiliğinden değiştirmez".into(),
            "En hızlı çözümleyici ölçülüp gösteriliyor; değiştirme kararı ve işlemi sende.".into(),
        ),
        (
            "Süreçleri kapatmaz".into(),
            "Arka plan uygulamaları dondurulur, kapatılmaz. Oyun bitince kaldıkları yerden devam ederler.".into(),
        ),
        (
            "Bellek 'temizlemez'".into(),
            "Standby list temizleme gibi ölçülebilir faydası gösterilemeyen işlemler yapılmıyor.".into(),
        ),
        (
            "Oyun kütüphaneni dışarı bildirmez".into(),
            "Oyun adları ve kapak görselleri Steam ve Epic'in kendi disk dosyalarından okunuyor. Hangi oyunlara sahip olduğun hiçbir servise sorulmuyor ve hiçbir yere gönderilmiyor.".into(),
        ),
        (
            "Telemetri toplamaz".into(),
            "Kullanım istatistiği, çökme raporu ya da analytics gönderilmiyor. Ağa yalnızca senin başlattığın ölçümler için çıkılıyor.".into(),
        ),
        (
            "Sayısal vaat vermez".into(),
            "'Ping'i şu kadar düşürür' denmiyor. Gösterilen her sayı senin makinende ölçülmüş veri.".into(),
        ),
    ]
}
