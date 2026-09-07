//! Profil içe/dışa aktarma.
//!
//! Profil dosyaları zaten okunabilir JSON ve elle kopyalanabiliyordu
//! (`store` modül belgesi); eksik olan, arayüzden bunu yapabilmekti.
//!
//! ## İçe aktarmanın kuralı: önce göster, sonra yaz
//!
//! `docs/PROFILES.md` güvenlik notu: paylaşılan bir profil
//! `suspend_process_list` gibi alanlar taşıyor, yani başka birinin dosyası
//! senin makinende uygulama donduruyor. Bu yüzden akış iki adım:
//! **`onizle` diskteki hiçbir şeye dokunmuyor**, yalnızca dosyayı okuyup
//! "bu profil uygulanınca ne olacak" listesini üretiyor; kaydetme ayrı bir
//! komut ve ancak kullanıcı onayladıktan sonra çağrılıyor.
//!
//! ## İçe aktarma var olanı sessizce ezmiyor
//!
//! Aynı kimlikte bir profil varsa iki seçenek de kullanıcının: üzerine yazmak
//! ya da yeni bir kimlikle eklemek (`bos_kimlik`). Varsayılan ikincisi —
//! bir arkadaşından gelen dosyanın senin profilini yok etmesi, geri alınamayan
//! tek işlem olurdu (tasarım ilkesi 1).

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::profile_engine::schema::{AffiniteTercihi, Profil};
use crate::profile_engine::store;
use crate::system_boost::{GucPlani, Oncelik};

/// İçe aktarma önizlemesi. Diskte hiçbir değişiklik yapılmadan üretiliyor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Onizleme {
    /// Doğrulanmış profil. Onaylanırsa kaydedilecek olan tam olarak bu.
    pub profil: Profil,
    /// Seçilen dosyanın adı — kullanıcı hangi dosyaya baktığını görsün.
    pub dosya: String,
    /// Doğrulamanın dosyada değiştirdikleri (`Profil::dogrula`).
    pub duzeltmeler: Vec<String>,
    /// Bu profil uygulanınca ne olacağı, düz cümlelerle.
    pub etkiler: Vec<String>,
    /// Kullanıcının okumadan geçmemesi gereken maddeler.
    pub uyarilar: Vec<String>,
    /// Aynı kimlikte bir profil zaten var mı?
    pub kimlik_cakismasi: bool,
    /// Aynı `.exe`'yi hedefleyen mevcut profillerin görünen adları.
    pub cakisan_profiller: Vec<String>,
    /// Üzerine yazılmazsa kullanılacak kimlik.
    pub bos_kimlik: String,
}

/// Profili verilen dosyaya yazar.
///
/// Kaydetmeden önce doğruluyor: diskteki profil geçerliyken dışa aktarılan
/// kopyanın geçersiz olması, dosyayı alan kişide açıklanamaz bir hataya
/// dönüşürdü.
pub fn disa_aktar(profil: &Profil, hedef: &Path) -> Result<()> {
    let (dogrulanmis, _) = profil.clone().dogrula()?;
    if let Some(ust) = hedef.parent() {
        if !ust.as_os_str().is_empty() {
            std::fs::create_dir_all(ust)?;
        }
    }
    std::fs::write(hedef, serde_json::to_string_pretty(&dogrulanmis)?)?;
    Ok(())
}

/// Dosyayı okur, doğrular ve ne yapacağını anlatır. Diske yazmaz.
pub fn onizle(yol: &Path, mevcut: &[Profil]) -> Result<Onizleme> {
    let (profil, duzeltmeler) = store::yukle(yol)?;

    let kimlik_cakismasi = mevcut.iter().any(|p| p.profile_id == profil.profile_id);
    let cakisan_profiller: Vec<String> = mevcut
        .iter()
        .filter(|p| {
            p.profile_id != profil.profile_id
                && p.executable_names
                    .iter()
                    .any(|e| profil.executable_names.contains(e))
        })
        .map(|p| p.display_name.clone())
        .collect();

    let bos = bos_kimlik(mevcut, &profil.profile_id);

    Ok(Onizleme {
        dosya: yol
            .file_name()
            .map(|a| a.to_string_lossy().to_string())
            .unwrap_or_default(),
        etkiler: etkiler(&profil),
        uyarilar: uyarilar(&profil, kimlik_cakismasi, &cakisan_profiller, &bos),
        bos_kimlik: bos,
        kimlik_cakismasi,
        cakisan_profiller,
        duzeltmeler,
        profil,
    })
}

/// Önizlenen profili diske yazar.
///
/// `uzerine_yaz` false ise kimlik çakışıyorsa boş bir kimliğe kaydırılıyor:
/// içe aktarma hiçbir koşulda var olan bir profili sessizce yok etmiyor.
/// Dönen değer, gerçekten kaydedilen profil ve doğrulama düzeltmeleri.
pub fn ice_aktar(
    dizin: &Path,
    mevcut: &[Profil],
    mut profil: Profil,
    uzerine_yaz: bool,
) -> Result<(Profil, Vec<String>)> {
    let cakisiyor = mevcut.iter().any(|p| p.profile_id == profil.profile_id);
    if cakisiyor && !uzerine_yaz {
        profil.profile_id = bos_kimlik(mevcut, &profil.profile_id);
    }
    store::kaydet(dizin, profil)
}

/// Kullanılmayan bir profil kimliği türetir: `kimlik`, `kimlik-2`, `kimlik-3`…
///
/// Dosya adı kimlikten üretildiği için (`store::dosya_adi`) benzersizlik
/// dosya adı seviyesinde karşılaştırılıyor: `a/b` ve `a_b` aynı dosyaya
/// yazardı, ikisini farklı kimlik saymak bir profili ezerdi.
pub fn bos_kimlik(mevcut: &[Profil], istenen: &str) -> String {
    let dolu = |aday: &str| {
        mevcut
            .iter()
            .any(|p| store::dosya_adi(&p.profile_id) == store::dosya_adi(aday))
    };
    if !dolu(istenen) {
        return istenen.to_string();
    }
    // Üst sınır: kimlik alanı kullanıcı girdisi ve döngünün sonsuz
    // kalabileceği bir durum (hepsi aynı dosya adına indirgenen kimlikler)
    // teoride mümkün.
    for n in 2..1000 {
        let aday = format!("{istenen}-{n}");
        if !dolu(&aday) {
            return aday;
        }
    }
    format!("{istenen}-{}", chrono::Utc::now().timestamp())
}

/// "Bu profil uygulanınca ne olur" listesi.
///
/// Metinler burada, arayüzde değil: aynı liste hem içe aktarma önizlemesinde
/// hem ileride profil detayında kullanılacak ve `DESIGN_PRINCIPLES.md`
/// madde 4 kontrolü (sayısal vaat yok) tek yerde test edilebilsin.
pub fn etkiler(profil: &Profil) -> Vec<String> {
    let mut v = Vec::new();

    v.push(format!(
        "Eşleşen uygulama: {}",
        profil.executable_names.join(", ")
    ));

    if let Ok(oncelik) = Oncelik::ayristir(&profil.system.priority_class) {
        v.push(format!(
            "Oyunun önceliği: {}",
            crate::ledger::oncelik_adi(oncelik.ham())
        ));
    }

    if profil.system.cpu_affinity == AffiniteTercihi::SadecePCore {
        v.push(
            "Oyun performans çekirdeklerine sabitlenir (CPU hibrit değilse atlanır)".to_string(),
        );
    }

    match &profil.system.power_plan {
        Some(plan) => {
            if let Ok(p) = GucPlani::ayristir(plan) {
                v.push(format!(
                    "Güç planı '{}' yapılır, oyun kapanınca geri döner",
                    p.ad()
                ));
            }
        }
        None => v.push("Güç planına dokunulmaz".to_string()),
    }

    let dondurulacaklar = profil.dondurulacaklar();
    if dondurulacaklar.is_empty() {
        v.push("Hiçbir uygulama dondurulmaz".to_string());
    } else {
        v.push(format!(
            "Oyun boyunca dondurulacak uygulamalar: {}",
            dondurulacaklar.join(", ")
        ));
    }

    if profil.network.qos_priority {
        v.push("Oyun için QoS ilkesi eklenir (kayıt defterinde kalıcı)".to_string());
    }
    if profil.network.tcp_nodelay {
        v.push("Nagle paket birleştirmesi kapatılır (kayıt defterinde kalıcı)".to_string());
    }

    v
}

/// Önizlemede vurgulanacak maddeler.
///
/// Uyarı listesi kısa tutuluyor: her satırı uyarı yapan bir ekran, hiçbirinin
/// okunmamasıyla sonuçlanır.
fn uyarilar(
    profil: &Profil,
    kimlik_cakismasi: bool,
    cakisan_profiller: &[String],
    bos_kimlik: &str,
) -> Vec<String> {
    let mut v = Vec::new();

    let dondurulacaklar = profil.dondurulacaklar();
    if !dondurulacaklar.is_empty() {
        v.push(format!(
            "Bu profil senin makinende şu uygulamaları donduracak: {}. Tanımadığın bir ad varsa içe aktarmadan önce bak.",
            dondurulacaklar.join(", ")
        ));
    }

    if profil.network.qos_priority || profil.network.tcp_nodelay {
        v.push(
            "Ağ ayarları kayıt defterine kalıcı yazılır. Günlükten tek tek ya da 'Varsayılana dön' ile geri alınabilir.".to_string(),
        );
    }

    if kimlik_cakismasi {
        v.push(
            "Aynı kimlikte bir profilin zaten var. Üzerine yazmayı seçmezsen yeni bir kimlikle eklenir."
                .to_string(),
        );
    } else if bos_kimlik != profil.profile_id {
        // Kimlikler farklı ama diskte aynı dosyaya düşüyorlar
        // (`store::dosya_adi` tek yönlü). `ice_aktar` kimliği kaydırarak
        // veri kaybını zaten önlüyor; söylenmezse kullanıcı profilini
        // yazdığı kimlikle arar ve bulamaz.
        v.push(format!(
            "'{}' kimliği diskte var olan bir profilin dosyasıyla aynı ada düşüyor; \
             profil '{bos_kimlik}' kimliğiyle eklenecek.",
            profil.profile_id
        ));
    }

    if !cakisan_profiller.is_empty() {
        v.push(format!(
            "Aynı oyunu hedefleyen başka profil(ler) var: {}. Oyun algılandığında hangisinin seçileceği belirsiz olur.",
            cakisan_profiller.join(", ")
        ));
    }

    v
}

#[cfg(test)]
mod testler {
    use super::*;

    fn ornek(kimlik: &str, exe: &str) -> Profil {
        Profil::yeni(kimlik, kimlik, exe)
    }

    fn dogrulanmis(kimlik: &str, exe: &str) -> Profil {
        ornek(kimlik, exe).dogrula().unwrap().0
    }

    #[test]
    fn disa_aktarilan_dosya_geri_okunuyor() {
        let dizin = tempfile::tempdir().unwrap();
        let hedef = dizin.path().join("alt").join("paylas.json");
        let mut p = dogrulanmis("cs2", "cs2.exe");
        p.system.suspend_process_list = vec!["discord.exe".into()];
        let (p, _) = p.dogrula().unwrap();

        disa_aktar(&p, &hedef).unwrap();
        let (okunan, duzeltmeler) = store::yukle(&hedef).unwrap();
        assert_eq!(okunan, p);
        assert!(duzeltmeler.is_empty());
    }

    /// Kimlikler farklı ama diskte aynı dosyaya düşüyorlar. `ice_aktar`
    /// kimliği zaten kaydırıyor; önizleme bunu **söylemezse** kullanıcı
    /// profilini yazdığı kimlikle arar ve bulamaz.
    #[test]
    fn ayni_dosyaya_dusen_kimlik_onizlemede_soyleniyor() {
        let dizin = tempfile::tempdir().unwrap();
        let kaynak = dizin.path().join("gelen.json");
        // `oyun 1` ve `oyun.1`in ikisi de `oyun_1.json`a düşüyor.
        disa_aktar(&dogrulanmis("oyun 1", "a.exe"), &kaynak).unwrap();
        let mevcut = vec![dogrulanmis("oyun.1", "b.exe")];

        let onizleme = onizle(&kaynak, &mevcut).unwrap();
        assert!(
            !onizleme.kimlik_cakismasi,
            "kimlikler farklı; 'üzerine yaz' seçeneği çıkmamalı"
        );
        assert_ne!(onizleme.bos_kimlik, "oyun 1");
        assert!(
            onizleme.uyarilar.iter().any(|u| u.contains(&onizleme.bos_kimlik)),
            "kimliğin değişeceği söylenmedi: {:?}",
            onizleme.uyarilar
        );
    }

    #[test]
    fn onizleme_diske_yazmiyor() {
        let dizin = tempfile::tempdir().unwrap();
        let kaynak = dizin.path().join("gelen.json");
        disa_aktar(&dogrulanmis("valorant", "valorant.exe"), &kaynak).unwrap();

        let profil_dizini = dizin.path().join("profiller");
        let onizleme = onizle(&kaynak, &[]).unwrap();

        assert_eq!(onizleme.dosya, "gelen.json");
        assert!(!onizleme.kimlik_cakismasi);
        assert!(
            !profil_dizini.exists(),
            "önizleme profil klasörüne dokunmamalı"
        );
    }

    #[test]
    fn onizleme_kimlik_cakismasini_bildiriyor() {
        let dizin = tempfile::tempdir().unwrap();
        let kaynak = dizin.path().join("gelen.json");
        let gelen = dogrulanmis("cs2", "cs2.exe");
        disa_aktar(&gelen, &kaynak).unwrap();

        let onizleme = onizle(&kaynak, std::slice::from_ref(&gelen)).unwrap();
        assert!(onizleme.kimlik_cakismasi);
        assert_eq!(onizleme.bos_kimlik, "cs2-2");
        assert!(
            onizleme
                .uyarilar
                .iter()
                .any(|u| u.to_lowercase().contains("aynı kimlikte")),
            "çakışma uyarısı eksik: {:?}",
            onizleme.uyarilar
        );
    }

    #[test]
    fn onizleme_ayni_oyunu_hedefleyen_profili_bildiriyor() {
        let dizin = tempfile::tempdir().unwrap();
        let kaynak = dizin.path().join("gelen.json");
        disa_aktar(&dogrulanmis("cs2-arkadas", "cs2.exe"), &kaynak).unwrap();

        let mut benimki = dogrulanmis("cs2-benim", "cs2.exe");
        benimki.display_name = "CS2 (benim)".into();

        let onizleme = onizle(&kaynak, &[benimki]).unwrap();
        assert_eq!(onizleme.cakisan_profiller, vec!["CS2 (benim)"]);
        assert!(!onizleme.kimlik_cakismasi);
    }

    #[test]
    fn onizleme_dondurma_listesini_uyariya_koyuyor() {
        // `docs/PROFILES.md` güvenlik notu: kör güven olmasın diye liste
        // kullanıcıya AÇIKÇA gösteriliyor.
        let dizin = tempfile::tempdir().unwrap();
        let kaynak = dizin.path().join("gelen.json");
        let mut p = ornek("oyun", "oyun.exe");
        p.system.suspend_process_list = vec!["discord.exe".into(), "spotify.exe".into()];
        disa_aktar(&p, &kaynak).unwrap();

        let onizleme = onizle(&kaynak, &[]).unwrap();
        assert!(
            onizleme
                .uyarilar
                .iter()
                .any(|u| u.contains("discord.exe") && u.contains("spotify.exe")),
            "dondurulacak adların tamamı uyarıda görünmeli: {:?}",
            onizleme.uyarilar
        );
    }

    #[test]
    fn bozuk_dosya_onizlemede_hata() {
        let dizin = tempfile::tempdir().unwrap();
        let kaynak = dizin.path().join("bozuk.json");
        std::fs::write(&kaynak, "{ bu json degil").unwrap();
        assert!(onizle(&kaynak, &[]).is_err());
    }

    #[test]
    fn ice_aktarma_var_olani_ezmiyor() {
        let dizin = tempfile::tempdir().unwrap();
        let mevcut = dogrulanmis("cs2", "cs2.exe");
        store::kaydet(dizin.path(), mevcut.clone()).unwrap();

        let mut gelen = dogrulanmis("cs2", "cs2.exe");
        gelen.display_name = "Arkadaşın profili".into();
        let (kaydedilen, _) =
            ice_aktar(dizin.path(), std::slice::from_ref(&mevcut), gelen, false).unwrap();

        assert_eq!(kaydedilen.profile_id, "cs2-2");
        let (hepsi, _) = store::hepsini_yukle(dizin.path());
        assert_eq!(hepsi.len(), 2, "eski profil yerinde durmalı");
        assert!(hepsi.iter().any(|p| p.display_name == mevcut.display_name));
    }

    #[test]
    fn uzerine_yazma_istenirse_kimlik_korunuyor() {
        let dizin = tempfile::tempdir().unwrap();
        let mevcut = dogrulanmis("cs2", "cs2.exe");
        store::kaydet(dizin.path(), mevcut.clone()).unwrap();

        let mut gelen = dogrulanmis("cs2", "cs2.exe");
        gelen.display_name = "Yeni sürüm".into();
        let (kaydedilen, _) = ice_aktar(dizin.path(), &[mevcut], gelen, true).unwrap();

        assert_eq!(kaydedilen.profile_id, "cs2");
        let (hepsi, _) = store::hepsini_yukle(dizin.path());
        assert_eq!(hepsi.len(), 1);
        assert_eq!(hepsi[0].display_name, "Yeni sürüm");
    }

    #[test]
    fn bos_kimlik_dosya_adi_seviyesinde_bakiyor() {
        // `a/b` ve `a_b` aynı dosyaya yazardı: farklı kimlik saymak bir
        // profili ezerdi.
        let mevcut = [dogrulanmis("a_b", "oyun.exe")];
        assert_eq!(bos_kimlik(&mevcut, "a/b"), "a/b-2");
        assert_eq!(bos_kimlik(&mevcut, "bambaska"), "bambaska");
    }

    #[test]
    fn ice_aktarma_dogrulamadan_geciyor() {
        // Dosyadan gelen bir profil, arayüzde engellenen bir şeyi isteyemez.
        let dizin = tempfile::tempdir().unwrap();
        let mut gelen = ornek("kotu", "oyun.exe");
        gelen.system.priority_class = "realtime".into();
        assert!(ice_aktar(dizin.path(), &[], gelen, false).is_err());
    }

    #[test]
    fn etkiler_profilin_tamamini_anlatiyor() {
        let mut p = ornek("cs2", "cs2.exe");
        p.system.suspend_process_list = vec!["discord.exe".into()];
        p.system.power_plan = Some("high_performance".into());
        p.network.qos_priority = true;
        let (p, _) = p.dogrula().unwrap();

        let etkiler = etkiler(&p);
        let hepsi = etkiler.join("\n");
        assert!(hepsi.contains("cs2.exe"));
        assert!(hepsi.contains("discord.exe"));
        assert!(hepsi.contains("QoS"));
        assert!(hepsi.to_lowercase().contains("güç planı"));
    }

    #[test]
    fn dokunulmayan_alanlar_da_yaziliyor() {
        // "Ne yapmayacağı" da bilgi: güç planı boşsa bunu söylemek,
        // kullanıcının dosyayı ayrıca açıp bakmasını gereksiz kılıyor.
        let p = dogrulanmis("sade", "sade.exe");
        let hepsi = etkiler(&p).join("\n");
        assert!(hepsi.contains("Güç planına dokunulmaz"));
        assert!(hepsi.contains("Hiçbir uygulama dondurulmaz"));
    }

    #[test]
    fn etkilerde_sayisal_vaat_yok() {
        // DESIGN_PRINCIPLES.md madde 4. Buradaki metinler kullanıcıya
        // gösteriliyor; "şu kadar ms kazandırır" cinsinden bir cümle
        // giremez.
        let mut p = ornek("cs2", "cs2.exe");
        p.system.suspend_process_list = vec!["discord.exe".into()];
        p.network.tcp_nodelay = true;
        let (p, _) = p.dogrula().unwrap();

        for cumle in etkiler(&p) {
            let k = cumle.to_lowercase();
            for yasak in ["ms ", " fps", "kat ", "%", "daha hızlı", "düşürür"] {
                assert!(
                    !k.contains(yasak),
                    "etki metninde sayısal vaat: '{cumle}' ({yasak})"
                );
            }
        }
    }
}
