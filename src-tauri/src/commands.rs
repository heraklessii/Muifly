//! Arayüze açılan komutlar.
//!
//! Her komut kilidi alıp bırakıyor; motor kilidi çağrı süresince tutulmuyor
//! çünkü bazı işlemler (DNS ölçümü, yol testi) saniyeler sürüyor ve o sırada
//! arayüzün durum sorgusu bloke olmamalı. Uzun işlemler kilidi hiç almıyor:
//! saf ağ işleri motorun durumuna dokunmuyor.

use tauri::{Emitter, Manager, State};

use crate::error::Result;
use crate::ledger::Kayit;
use crate::library;
use crate::monitor::olcum::{self, KareOlcumDurumu};
use crate::monitor::{GecmisOzeti, Karsilastirma, Ornek, OturumKaydi, Ozet, Satir};
use crate::network_boost::{self, DnsSonucu, TcpDurumu, YolSonucu};
use crate::profile_engine::{self, Profil};
use crate::settings::Ayarlar;
use crate::state::{Durum, Motor, UygulamaSonucu};
use crate::surum::{self, Kisitlar};
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
/// Yeni bir çeviri sonucu hazır (ya da istek hata verdi).
pub const OLAY_CEVIRI: &str = "muifly://ceviri";
/// Model indirmesinin ilerlemesi.
pub const OLAY_CEVIRI_INDIRME: &str = "muifly://ceviri-indirme";

#[tauri::command]
pub fn surum() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Bu ikilide hangi özelliklerin açık olduğu (demo / tam sürüm).
///
/// Arayüz açılışta bir kez okuyor; kapalı özelliklerin sekmesini hiç
/// göstermiyor. Komutlar ayrıca kendi kontrollerini yapıyor: gizlemek bir
/// sınır değil.
#[tauri::command]
pub fn kisitlar() -> Kisitlar {
    surum::kisitlar()
}

/// Üçüncü taraf bileşenlerin listesi (EULA madde 8).
///
/// Lisans METİNLERİ burada yok: liste 258 bileşen ve metinlerin tamamı bir
/// megabaytın üstünde. Kullanıcı bir bileşeni açtığında `ucuncu_taraf_metin`
/// çağrılıyor.
///
/// Demoda da açık: hangi sürümü kullandığından bağımsız olarak, dağıtılan her
/// ikilinin taşıdığı bileşenlerin lisansları görünür olmak zorunda.
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

    // Demo sınırı profil SAYISINDA: var olanı düzenlemek serbest, yenisini
    // eklemek sınırlı (`docs/DISTRIBUTION.md`).
    let yeni = !m
        .profiller
        .iter()
        .any(|p| p.profile_id == profil.profile_id);
    if yeni && !surum::kisitlar().profil_eklenebilir(m.profiller.len()) {
        return Err(crate::error::Error::DemoKisiti("Birden fazla profil"));
    }

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
    surum::profil_aktarimi_gerekli()?;
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
/// gerekiyor. Demoda da açık — bu bir okuma işlemi ve şeffaflık ilkesi
/// sürümden bağımsız.
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
    surum::profil_aktarimi_gerekli()?;
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
    surum::profil_aktarimi_gerekli()?;
    let mut m = motor.lock();

    // Üzerine yazma var olan profili değiştiriyor, sayıyı artırmıyor:
    // demo sınırı yalnızca yeni profil eklerken bakılıyor.
    let mevcut_mu = m
        .profiller
        .iter()
        .any(|p| p.profile_id == profil.profile_id);
    if !(mevcut_mu && uzerine_yaz) && !surum::kisitlar().profil_eklenebilir(m.profiller.len()) {
        return Err(crate::error::Error::DemoKisiti("Birden fazla profil"));
    }

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
    surum::ag_gerekli()?;
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
    surum::ag_gerekli()?;
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
    surum::ag_gerekli()?;
    network_boost::tcp::durum()
}

/// TCP ayarlarını uygular (kalıcı, geri alınabilir).
#[tauri::command]
pub fn tcp_uygula(motor: MotorState<'_>) -> Result<UygulamaSonucu> {
    surum::ag_gerekli()?;
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
    surum::ag_gerekli()?;
    network_boost::qos::listele()
}

/// QoS ilkelerini kaldırmak geri alma sayılıyor ve demoda da açık.
///
/// Kapatılan tek şey yeni değişiklik yapmak; **temizlik her sürümde
/// çalışır** — kullanıcı tam sürümden demoya dönse bile sistemde Muifly'ın
/// bıraktığı bir iz kalmasın (tasarım ilkesi 1).
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
    // Açma demoda kapalı; KAPATMA her sürümde serbest. Kayıttaki girdiyi
    // silmek bir geri alma işlemi ve o hiçbir koşulda kilitlenmiyor.
    if acik && !surum::kisitlar().otomatik_baslatma {
        return Err(crate::error::Error::DemoKisiti("Windows ile başlatma"));
    }
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
/// yapmadıklarını söylemesi, yaptıklarını saymasından daha güven verici —
/// ve mağaza sayfasında da aynı liste kullanılıyor (`docs/DISTRIBUTION.md`).
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
            "Ölçekleme için oyuna dokunmaz".into(),
            "Görüntü, Windows'un masaüstü çoğaltma arayüzünden okunuyor. Oyunun belleğine yazılmıyor, çağrıları yönlendirilmiyor, süreci açılmıyor.".into(),
        ),
        (
            "Ölçeklemenin bedelini gizlemez".into(),
            "Ölçekleme her kareye gecikme ekler. Eklenen süre ölçülüp ekranda gösteriliyor; rekabetçi modda ölçekleme hiç açılmıyor.".into(),
        ),
        (
            "Sayısal vaat vermez".into(),
            "'Ping'i şu kadar düşürür' denmiyor. Gösterilen her sayı senin makinende ölçülmüş veri.".into(),
        ),
    ]
}

// ---------------------------------------------------------------------------
// Ölçekleme (Faz 3)
// ---------------------------------------------------------------------------

/// Yakalanabilecek ekranların listesi.
#[tauri::command]
pub fn olcekleme_ekranlari() -> Result<Vec<crate::scaling::Ekran>> {
    crate::scaling::yakalama::ekranlar().map_err(|e| crate::error::Error::Olcum(e.to_string()))
}

/// Algoritmalar ve açıklamaları.
#[tauri::command]
pub fn olcekleme_algoritmalari() -> Vec<crate::scaling::AlgoritmaBilgisi> {
    crate::scaling::algoritmalar()
}

#[tauri::command]
pub fn olcekleme_durumu(motor: MotorState<'_>) -> crate::scaling::OlceklemeDurumu {
    motor.lock().olcekleme_durumu()
}

/// Ölçeklemeyi başlatır.
///
/// Yakalama açılışı saniyenin altında ama bloke edici; kilit çağrı boyunca
/// tutuluyor çünkü aynı anda ikinci bir başlatma isteği gelirse iki pencere
/// açılırdı.
#[tauri::command]
pub fn olcekleme_baslat(motor: MotorState<'_>, algoritma: String) -> Result<()> {
    let algo = crate::scaling::Algoritma::coz(&algoritma).ok_or_else(|| {
        crate::error::Error::ProfileInvalid(format!("bilinmeyen algoritma: {algoritma}"))
    })?;
    motor.lock().olceklemeyi_baslat(algo)
}

#[tauri::command]
pub fn olcekleme_durdur(motor: MotorState<'_>) {
    motor.lock().olceklemeyi_durdur("kullanıcı durdurdu");
}

/// Çalışırken algoritma değiştirir.
#[tauri::command]
pub fn olcekleme_algoritma(motor: MotorState<'_>, algoritma: String) -> Result<()> {
    let algo = crate::scaling::Algoritma::coz(&algoritma).ok_or_else(|| {
        crate::error::Error::ProfileInvalid(format!("bilinmeyen algoritma: {algoritma}"))
    })?;
    motor.lock().olcekleme_algoritmasi(algo);
    Ok(())
}

/// Kare üretimini (Faz 4) çalışırken açar/kapatır.
///
/// Yeniden başlatma yok: kullanıcı farkı **aynı sahnede** görebilmeli.
#[tauri::command]
pub fn olcekleme_uretimi(motor: MotorState<'_>, acik: bool) -> Result<()> {
    motor.lock().olcekleme_uretimi(acik)
}

/// Ölçeklemeyi açmadan yakalamanın çalışıp çalışmadığını dener.
///
/// Ayrı bir iş parçacığında: yarım saniyeye kadar sürüyor ve o sırada
/// arayüzün durum sorgusu bloke olmamalı. Motor kilidi yalnızca ekran
/// numarasını okumak için alınıyor.
#[tauri::command]
pub async fn olcekleme_denemesi(motor: MotorState<'_>) -> Result<crate::scaling::YakalamaDenemesi> {
    let ekran = motor.lock().ayarlar.olcekleme_ekrani;
    tauri::async_runtime::spawn_blocking(move || crate::scaling::deneme(ekran))
        .await
        .map_err(|e| crate::error::Error::Olcum(e.to_string()))?
        .map_err(|e| crate::error::Error::Olcum(e.to_string()))
}

// ---------------------------------------------------------------------------
// Ekran çevirisi (Faz 5, karar #37)
// ---------------------------------------------------------------------------

/// Model indirmesinin iptal bayrağı.
///
/// Süreç geneli bir statik: indirme tek seferde bir tane olabilir ve iptal
/// isteği, indirmeyi başlatan komuttan başka bir komuttan geliyor. Motor'da
/// tutulsaydı iptal etmek için Motor kilidini almak gerekirdi — oysa indirme
/// tam da kilidi almadan, ayrı bir iş parçacığında koşuyor.
static INDIRME_IPTAL: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// Aynı anda ikinci bir indirme başlamasın.
static INDIRME_SURUYOR: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// İndirme ilerlemesi — arayüze giden olay yükü.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndirmeIlerlemesi {
    pub sira: usize,
    pub adet: usize,
    pub ad: String,
    pub inen: u64,
    pub toplam: u64,
    pub bitti: bool,
    /// Dolu ise indirme durdu ve sebebi bu.
    pub hata: Option<String>,
}

#[tauri::command]
pub fn ceviri_durumu(motor: MotorState<'_>) -> crate::ceviri::CeviriDurumu {
    motor.lock().ceviri.durum()
}

/// Son çeviri sonucu. Henüz istek olmadıysa `null`.
#[tauri::command]
pub fn ceviri_sonucu(motor: MotorState<'_>) -> Option<crate::ceviri::Sonuc> {
    motor.lock().ceviri.son_sonuc()
}

/// Çeviriyi açar: sisteme klavye kısayolu kaydeder.
///
/// Ayara da yazılıyor: kullanıcı açık bıraktıysa bir sonraki açılışta da
/// açık gelmeli. Kısayol kaydedilemezse ayar **yazılmıyor** — kapalı
/// kalmış bir özelliği açık göstermek, karşılanmamış bir vaat olurdu.
#[tauri::command]
pub fn ceviri_ac(motor: MotorState<'_>) -> Result<()> {
    let mut m = motor.lock();
    m.ceviriyi_ac()?;
    m.ayarlar.ceviri_acik = true;
    let _ = m.ayarlari_kaydet();
    Ok(())
}

#[tauri::command]
pub fn ceviri_kapat(motor: MotorState<'_>) {
    let mut m = motor.lock();
    m.ceviriyi_kapat("kullanıcı kapattı");
    m.ayarlar.ceviri_acik = false;
    let _ = m.ayarlari_kaydet();
}

/// Kısayola basmakla aynı şey. Kullanıcı özelliği pencereden deneyebilsin.
#[tauri::command]
pub fn ceviri_simdi(motor: MotorState<'_>) -> Result<()> {
    motor.lock().ceviri.simdi_cevir()
}

/// Kaynak dilin OCR paketi kurulu mu (karar #28 ürün gereği).
#[tauri::command]
pub fn ceviri_dil_durumu(motor: MotorState<'_>) -> Result<crate::ceviri::DilDurumu> {
    let istenen = motor.lock().ceviri_yapilandirmasi().kaynak_dil;
    let mevcut = crate::ceviri::ocr_dil::mevcut_diller()?;
    Ok(crate::ceviri::ocr_dil::durumu_belirle(&istenen, mevcut))
}

// --- Model dosyaları -------------------------------------------------------

#[tauri::command]
pub fn ceviri_model_durumu() -> crate::ceviri::ModelDurumu {
    crate::ceviri::model::durum()
}

/// Modeli indirir. Yarım gigabayt; ayrı iş parçacığında ve iptal edilebilir.
///
/// İlerleme olayla akıyor, komutun dönüşüyle değil: yarım saatlik bir
/// indirmede tek bir `await`in dönmesini beklemek, kullanıcıya donmuş bir
/// ekran göstermek olurdu.
#[tauri::command]
pub async fn ceviri_model_indir(uygulama: tauri::AppHandle) -> Result<()> {
    use std::sync::atomic::Ordering;

    if INDIRME_SURUYOR.swap(true, Ordering::SeqCst) {
        return Err(crate::error::Error::Indirme("indirme zaten sürüyor".into()));
    }
    INDIRME_IPTAL.store(false, Ordering::SeqCst);

    let kol = uygulama.clone();
    let sonuc = tauri::async_runtime::spawn_blocking(move || {
        crate::ceviri::model::indir(&INDIRME_IPTAL, |adim| {
            let _ = kol.emit(
                OLAY_CEVIRI_INDIRME,
                IndirmeIlerlemesi {
                    sira: adim.sira,
                    adet: adim.adet,
                    ad: adim.ad.to_string(),
                    inen: adim.inen,
                    toplam: adim.toplam,
                    bitti: false,
                    hata: None,
                },
            );
        })
    })
    .await;

    INDIRME_SURUYOR.store(false, Ordering::SeqCst);

    let sonuc = match sonuc {
        Ok(s) => s,
        Err(e) => Err(crate::error::Error::Indirme(e.to_string())),
    };

    let toplam = crate::ceviri::model::toplam_bayt();
    let hata = sonuc.as_ref().err().map(|e| e.to_string());
    let _ = uygulama.emit(
        OLAY_CEVIRI_INDIRME,
        IndirmeIlerlemesi {
            sira: 0,
            adet: 0,
            ad: String::new(),
            inen: if hata.is_none() { toplam } else { 0 },
            toplam,
            bitti: true,
            hata,
        },
    );
    sonuc
}

#[tauri::command]
pub fn ceviri_model_indirmeyi_durdur() {
    INDIRME_IPTAL.store(true, std::sync::atomic::Ordering::SeqCst);
}

/// Model dosyalarını siler. Çeviri belleğine dokunmuyor.
#[tauri::command]
pub fn ceviri_model_sil(motor: MotorState<'_>) -> Result<()> {
    // Model bellekteyken dosyaları silmek, bir sonraki isteği anlaşılmaz bir
    // hataya düşürürdü: önce çeviri kapatılıyor.
    let acikti = {
        let mut m = motor.lock();
        let acikti = m.ceviri.acik();
        if acikti {
            m.ceviriyi_kapat("model siliniyor");
        }
        acikti
    };
    let sonuc = crate::ceviri::model::sil();
    if acikti {
        // Kısayol kullanıcının açık bıraktığı bir şeydi; geri veriliyor.
        let _ = motor.lock().ceviriyi_ac();
    }
    sonuc
}

/// Diskteki dosyaların özetini beklenenle karşılaştırır.
///
/// Yarım gigabayt okunuyor, o yüzden ayrı iş parçacığında ve yalnızca
/// kullanıcı isteyince.
#[tauri::command]
pub async fn ceviri_model_dogrula() -> Result<()> {
    tauri::async_runtime::spawn_blocking(crate::ceviri::model::dogrula)
        .await
        .map_err(|e| crate::error::Error::Indirme(e.to_string()))?
}

// --- Alan seçimi -----------------------------------------------------------

/// Alan seçme penceresi için ekranın o anki görüntüsü (PNG, `data:` adresi).
///
/// Donmuş bir görüntü üzerinde seçim yaptırmak, canlı ekranın üstüne
/// saydam bir pencere açmaktan iyi: kullanıcı tam olarak neyin
/// okunacağını görüyor ve seçerken oyunun görüntüsü değişmiyor.
#[tauri::command]
pub async fn ceviri_ekran_goruntusu(motor: MotorState<'_>) -> Result<String> {
    let ekran = motor.lock().ayarlar.ceviri_ekrani;
    tauri::async_runtime::spawn_blocking(move || crate::ceviri::ekran_goruntusu(ekran))
        .await
        .map_err(|e| crate::error::Error::Ceviri(e.to_string()))?
}

/// Seçilen alanı profile yazar.
///
/// Alan profile ait (karar #22). Profil yoksa yazılacak yer de yok ve bu
/// bir hata değil bir durum: kullanıcıya önce profil oluşturması söyleniyor.
#[tauri::command]
pub fn ceviri_alani_kaydet(
    motor: MotorState<'_>,
    kimlik: String,
    alan: Option<crate::ceviri::Alan>,
) -> Result<Vec<String>> {
    let profil = {
        let m = motor.lock();
        m.profiller
            .iter()
            .find(|p| p.profile_id == kimlik)
            .cloned()
            .ok_or_else(|| crate::error::Error::ProfileNotFound(kimlik.clone()))?
    };
    let mut profil = profil;
    profil.ceviri.region = alan;
    profil.ceviri.enabled = alan.is_some() || profil.ceviri.enabled;
    let duzeltmeler = profil_kaydet(motor.clone(), profil)?;
    motor.lock().ceviriyi_tazele();
    Ok(duzeltmeler)
}

// --- Çeviri belleği --------------------------------------------------------

/// Öndeki oyunun çeviri belleği.
#[tauri::command]
pub fn ceviri_bellegi(motor: MotorState<'_>) -> Result<crate::ceviri::CeviriBellegi> {
    let oyun = motor.lock().ceviri_oyunu();
    let yol = crate::ceviri::bellek::yolu(&crate::settings::ceviri_dizini(), &oyun);
    crate::ceviri::bellek::yukle(
        &yol,
        &oyun,
        crate::ceviri::denetleyici::KAYNAK_DIL,
        crate::ceviri::denetleyici::HEDEF_DIL,
    )
}

/// Belleği okuyup değiştirip yazar.
///
/// Her komut dosyayı yeniden okuyor: çeviri iş parçacığı da aynı dosyayı
/// kullanıyor ve elde tutulan bir kopya, ikisinden birinin diğerini
/// ezmesi demek olurdu (`ceviri::denetleyici` modül belgesi).
fn bellegi_degistir(
    motor: &MotorState<'_>,
    degistir: impl FnOnce(&mut crate::ceviri::CeviriBellegi),
) -> Result<()> {
    let oyun = motor.lock().ceviri_oyunu();
    let yol = crate::ceviri::bellek::yolu(&crate::settings::ceviri_dizini(), &oyun);
    let mut bellek = crate::ceviri::bellek::yukle(
        &yol,
        &oyun,
        crate::ceviri::denetleyici::KAYNAK_DIL,
        crate::ceviri::denetleyici::HEDEF_DIL,
    )?;
    degistir(&mut bellek);
    bellek.kaydet(&yol)
}

/// Kullanıcının düzelttiği çeviri.
///
/// Karar #22: asıl olan "Düzelt", "Reddet" değil — reddetme yalnızca yanlış
/// olduğunu söyler, doğrusunu söylemez. Kayıt `Kullanici` kökeniyle
/// giriyor ve bir daha makine çevirisiyle ezilmiyor.
#[tauri::command]
pub fn ceviri_duzelt(motor: MotorState<'_>, metin: String, ceviri: String) -> Result<()> {
    bellegi_degistir(&motor, |b| {
        b.ekle(&metin, &ceviri, crate::ceviri::Koken::Kullanici);
    })
}

/// Bir kaydı siler — "Reddet"in karşılığı (karar #22).
#[tauri::command]
pub fn ceviri_kaydi_sil(motor: MotorState<'_>, metin: String) -> Result<()> {
    bellegi_degistir(&motor, |b| {
        b.sil(&metin);
    })
}

#[tauri::command]
pub fn ceviri_terim_ekle(motor: MotorState<'_>, terim: String, karsilik: String) -> Result<()> {
    bellegi_degistir(&motor, |b| b.terim_ekle(&terim, &karsilik))
}

#[tauri::command]
pub fn ceviri_terim_sil(motor: MotorState<'_>, terim: String) -> Result<()> {
    bellegi_degistir(&motor, |b| {
        b.terim_sil(&terim);
    })
}

/// Makine kayıtlarını temizler; kullanıcının düzeltmelerine dokunmaz.
#[tauri::command]
pub fn ceviri_bellegini_temizle(motor: MotorState<'_>) -> Result<()> {
    bellegi_degistir(&motor, |b| b.makine_kayitlarini_temizle())
}

// --- Çeviri pencereleri ----------------------------------------------------

/// Sonucu ekranın üstünde gösteren pencerenin etiketi.
pub const PENCERE_CEVIRI_OVERLAY: &str = "ceviri-overlay";
/// Alan seçme penceresinin etiketi.
pub const PENCERE_CEVIRI_ALAN: &str = "ceviri-alan";

/// Overlay'in ekranın altında kapladığı yer.
///
/// Altta ve ekranın dörtte biri kadar: oyun metni çoğunlukla altta olur ve
/// üstüne binen bir çeviri penceresi, çevrilen şeyi kapatırdı. Genişlik tam
/// ekran değil — kenarlarda kalan boşluk, pencerenin nerede bittiğini
/// gösteriyor ve panelin "ekranı ele geçirdiği" hissini kaldırıyor.
const OVERLAY_GENISLIK_ORANI: f64 = 0.72;
const OVERLAY_YUKSEKLIK_ORANI: f64 = 0.26;
/// Ekranın alt kenarından boşluk (fiziksel piksel).
const OVERLAY_ALT_BOSLUK: i32 = 48;

/// Seçili ekranın fiziksel yerleşimi.
fn ceviri_ekrani(indeks: usize) -> Option<crate::scaling::Ekran> {
    let ekranlar = crate::scaling::yakalama::ekranlar().ok()?;
    ekranlar
        .iter()
        .find(|e| e.indeks == indeks)
        .or_else(|| ekranlar.first())
        .cloned()
}

/// Overlay penceresini kurar (yoksa) ve sonucu göstermek üzere açar.
///
/// # Kaçış yolu
///
/// Karar #34, ölçekleme penceresinin kapatılacak hiçbir yolu olmadığında
/// makineyi kullanılamaz hâle getirdiğini anlatıyor. Bu pencere o hatayı
/// tekrarlamıyor ve tekrarlamamasının sebebi tek tek sayılabilir: ekranın
/// tamamını kaplamıyor, tıklamaları geçirmiyor (yani üstündeki kapatma
/// düğmesi çalışıyor), görev çubuğunda görünmüyor ama kendi kapatma düğmesi
/// var, ve çeviri kapatıldığında pencere de kapanıyor.
pub fn overlay_goster(uygulama: &tauri::AppHandle, ekran: usize) {
    use tauri::{LogicalSize, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

    if let Some(pencere) = uygulama.get_webview_window(PENCERE_CEVIRI_OVERLAY) {
        let _ = pencere.show();
        return;
    }

    let Some(e) = ceviri_ekrani(ekran) else {
        log::warn!("çeviri overlay'i için ekran bulunamadı");
        return;
    };
    let genislik = (e.genislik as f64 * OVERLAY_GENISLIK_ORANI) as u32;
    let yukseklik = (e.yukseklik as f64 * OVERLAY_YUKSEKLIK_ORANI) as u32;

    let sonuc = WebviewWindowBuilder::new(
        uygulama,
        PENCERE_CEVIRI_OVERLAY,
        WebviewUrl::App("index.html?pencere=ceviri-overlay".into()),
    )
    .title("Muifly — çeviri")
    .inner_size(genislik as f64, yukseklik as f64)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .focused(false)
    .visible(false)
    .build();

    let pencere = match sonuc {
        Ok(p) => p,
        Err(err) => {
            log::warn!("çeviri overlay'i açılamadı: {err}");
            return;
        }
    };

    // Boyut ve konum fiziksel piksel: ekran listesi DXGI'dan geliyor ve orası
    // mantıksal (ölçeklenmiş) koordinat bilmiyor. Mantıksal boyut verilseydi
    // %150 ölçekli bir ekranda pencere ekranın dışına taşardı.
    let _ = pencere.set_size(PhysicalSize::new(genislik, yukseklik));
    let _ = pencere.set_position(PhysicalPosition::new(
        e.x + ((e.genislik - genislik) / 2) as i32,
        e.y + e.yukseklik as i32 - yukseklik as i32 - OVERLAY_ALT_BOSLUK,
    ));
    // Kullanılmayan içe aktarmayı önlemek için değil, okunurluk için:
    // mantıksal boyut yalnızca en küçük ölçüyü söylemekte kullanılıyor.
    let _ = pencere.set_min_size(Some(LogicalSize::new(320.0, 120.0)));
    let _ = pencere.show();
}

/// Overlay'i kapatır. Çeviri kapatıldığında ve kullanıcı istediğinde.
#[tauri::command]
pub fn ceviri_overlay_kapat(uygulama: tauri::AppHandle) {
    if let Some(p) = uygulama.get_webview_window(PENCERE_CEVIRI_OVERLAY) {
        let _ = p.close();
    }
}

/// Alan seçme penceresini açar.
///
/// Tam ekran ve **odaklı**: kullanıcı burada fare ile bir dikdörtgen
/// çiziyor, yani tıklamaları alması gerekiyor. Overlay'den farkı bu ve
/// tehlikeli olmamasının sebebi de aynı: pencere kullanıcının açtığı,
/// Esc ile kapanan ve görev çubuğunda görünen bir pencere.
#[tauri::command]
pub fn ceviri_alan_secici_ac(
    uygulama: tauri::AppHandle,
    motor: MotorState<'_>,
    kimlik: String,
) -> Result<()> {
    use tauri::{PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

    if let Some(p) = uygulama.get_webview_window(PENCERE_CEVIRI_ALAN) {
        let _ = p.set_focus();
        return Ok(());
    }

    let ekran = motor.lock().ayarlar.ceviri_ekrani;
    let e = ceviri_ekrani(ekran)
        .ok_or_else(|| crate::error::Error::Ceviri("alan seçilecek ekran bulunamadı".into()))?;

    let pencere = WebviewWindowBuilder::new(
        &uygulama,
        PENCERE_CEVIRI_ALAN,
        WebviewUrl::App(
            format!("index.html?pencere=ceviri-alan&kimlik={}", urlencode(&kimlik)).into(),
        ),
    )
    .title("Muifly — çevrilecek alanı seç")
    .decorations(false)
    .always_on_top(true)
    .resizable(false)
    .build()
    .map_err(|err| crate::error::Error::Ceviri(format!("alan seçici açılamadı: {err}")))?;

    let _ = pencere.set_size(PhysicalSize::new(e.genislik, e.yukseklik));
    let _ = pencere.set_position(PhysicalPosition::new(e.x, e.y));
    let _ = pencere.set_focus();
    Ok(())
}

#[tauri::command]
pub fn ceviri_alan_secici_kapat(uygulama: tauri::AppHandle) {
    if let Some(p) = uygulama.get_webview_window(PENCERE_CEVIRI_ALAN) {
        let _ = p.close();
    }
}

/// Adres satırına konacak dizeyi kaçırır.
///
/// Profil kimliği kullanıcının yazdığı bir dize ve pencere adresine
/// giriyor. Tam bir URL kodlayıcı değil; alfasayısal olmayan her şeyi
/// yüzdeyle yazan en dar hâli — geçirilen şey bir kimlik, bir adres değil.
fn urlencode(ham: &str) -> String {
    let mut cikti = String::with_capacity(ham.len());
    for b in ham.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            cikti.push(*b as char);
        } else {
            cikti.push_str(&format!("%{b:02X}"));
        }
    }
    cikti
}

#[cfg(test)]
mod ceviri_testleri {
    use super::*;

    #[test]
    fn urlencode_kimligi_bozmuyor() {
        assert_eq!(urlencode("skyrim-se_1.0"), "skyrim-se_1.0");
    }

    #[test]
    fn urlencode_adres_kacisi_birakmiyor() {
        // Kimlik pencere adresine giriyor; sorgu ayracı ya da kod
        // çalıştırabilecek bir karakter olduğu gibi geçmemeli.
        let kotu = urlencode("a&pencere=x #<script>");
        for yasak in ['&', '=', ' ', '#', '<', '>'] {
            assert!(!kotu.contains(yasak), "kaçırılmadı: {yasak} → {kotu}");
        }
    }

    #[test]
    fn urlencode_turkce_harfleri_kacirilyor() {
        let s = urlencode("çğüş");
        assert!(s.starts_with('%'));
        assert!(s.is_ascii());
    }
}
