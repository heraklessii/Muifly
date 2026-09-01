//! Epic Games kütüphanesini diskten okur.
//!
//! Epic, kurulu her ürün için `ProgramData\Epic\EpicGamesLauncher\Data\
//! Manifests\` altına bir `.item` dosyası bırakıyor. Bu dosya JSON ve
//! aradığımız üç şeyi doğrudan veriyor:
//!
//! ```text
//! "DisplayName":      "NTE: Neverness to Everness"
//! "InstallLocation":  "C:\\Games\\NTENevernesstoEvernezYbAx"
//! "LaunchExecutable": "NTEGlobal/NTEGlobalLauncher.exe"
//! ```
//!
//! `LaunchExecutable` burada Steam'den bile iyi: exe'yi tahmin etmiyoruz,
//! Epic'in kendisi söylüyor. Yine de kurulum klasörü ayrıca taranıyor
//! (`super::exe`), çünkü bazı oyunlarda bu alan başlatıcıyı gösteriyor ve
//! kullanıcının profili gerçek oyun sürecine bağlaması gerekiyor.
//!
//! Epic'in yerel bir kapak görseli önbelleği yok; görsel tarafı exe ikonuna
//! düşüyor (`super::ikon`).

use std::path::{Path, PathBuf};

/// Epic kütüphanesinde bulunmuş bir oyun.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpicOyunu {
    /// Epic'in kendi kimliği. Kütüphane kimliğinde kullanılıyor.
    pub app_name: String,
    pub ad: String,
    pub kurulum: PathBuf,
    /// Epic'in bildirdiği başlatma exe'si, küçük harfe indirgenmiş dosya adı.
    /// Alan boşsa `None`.
    pub baslatma_exe: Option<String>,
}

/// Manifest klasörü. Epic kurulu değilse klasör de yok.
fn manifest_klasoru() -> PathBuf {
    // %ProgramData% ortam değişkeni her Windows kurulumunda tanımlı; yoksa
    // standart yola düşülüyor.
    let kok = std::env::var("ProgramData").unwrap_or_else(|_| "C:\\ProgramData".to_string());
    Path::new(&kok)
        .join("Epic")
        .join("EpicGamesLauncher")
        .join("Data")
        .join("Manifests")
}

/// Tek bir `.item` dosyasının içeriğini oyuna çevirir.
///
/// Dosya sisteminden bağımsız: testler bu fonksiyonu çağırıyor.
pub fn manifesti_coz(icerik: &str) -> Option<EpicOyunu> {
    let json: serde_json::Value = serde_json::from_str(icerik).ok()?;

    // Eklentiler, motorlar ve DLC'ler de bu klasörde manifest bırakıyor.
    // Kategori alanı varsa oyun olmayanları eliyoruz; alan yoksa (eski
    // sürümler) elemiyoruz — eksik listelemek yanlış listelemekten kötü.
    if let Some(kategoriler) = json.get("AppCategories").and_then(|v| v.as_array()) {
        let oyun_mu = kategoriler
            .iter()
            .filter_map(|v| v.as_str())
            .any(|k| k.eq_ignore_ascii_case("games"));
        if !oyun_mu {
            return None;
        }
    }

    let ad = metin(&json, "DisplayName")?;
    let kurulum = metin(&json, "InstallLocation")?;
    // Kimlik yoksa ada düşülüyor: liste kimliksiz kalmasın.
    let app_name = metin(&json, "AppName").unwrap_or_else(|| ad.clone());

    let baslatma_exe = metin(&json, "LaunchExecutable").and_then(|yol| {
        // "NTEGlobal/NTEGlobalLauncher.exe" gibi alt yol içerebiliyor.
        let ad = crate::system_boost::detect::ad_normalize(&yol);
        (ad.ends_with(".exe")).then_some(ad)
    });

    Some(EpicOyunu {
        app_name,
        ad,
        kurulum: PathBuf::from(kurulum),
        baslatma_exe,
    })
}

/// Boş olmayan bir metin alanı okur.
fn metin(json: &serde_json::Value, alan: &str) -> Option<String> {
    let s = json.get(alan)?.as_str()?.trim();
    (!s.is_empty()).then(|| s.to_string())
}

/// Kurulu bütün Epic oyunları. Epic yoksa boş liste.
pub fn oyunlar() -> Vec<EpicOyunu> {
    let klasor = manifest_klasoru();
    let Ok(girdiler) = std::fs::read_dir(&klasor) else {
        return Vec::new();
    };

    let mut bulunanlar = Vec::new();
    for girdi in girdiler.flatten() {
        let ad = girdi.file_name().to_string_lossy().to_lowercase();
        if !ad.ends_with(".item") {
            continue;
        }
        let Ok(icerik) = std::fs::read_to_string(girdi.path()) else {
            continue;
        };
        if let Some(oyun) = manifesti_coz(&icerik) {
            if !bulunanlar
                .iter()
                .any(|o: &EpicOyunu| o.app_name == oyun.app_name)
            {
                bulunanlar.push(oyun);
            }
        }
    }
    bulunanlar
}

#[cfg(test)]
mod testler {
    use super::*;

    /// Bu makinedeki gerçek bir manifestten kısaltılmış örnek.
    const MANIFEST: &str = r#"{
        "AppName": "b675fd2fd3354d48960f4c1eaa6af466",
        "DisplayName": "NTE: Neverness to Everness",
        "InstallLocation": "C:\\Games\\NTENevernesstoEvernezYbAx",
        "LaunchExecutable": "NTEGlobal/NTEGlobalLauncher.exe",
        "AppCategories": ["public", "games", "applications"]
    }"#;

    #[test]
    fn manifest_okunuyor() {
        let oyun = manifesti_coz(MANIFEST).unwrap();
        assert_eq!(oyun.ad, "NTE: Neverness to Everness");
        assert_eq!(
            oyun.kurulum,
            PathBuf::from("C:\\Games\\NTENevernesstoEvernezYbAx")
        );
        assert_eq!(oyun.baslatma_exe.as_deref(), Some("ntegloballauncher.exe"));
    }

    #[test]
    fn oyun_olmayan_kategori_eleniyor() {
        let m = r#"{
            "DisplayName": "Unreal Engine",
            "InstallLocation": "C:\\UE",
            "AppCategories": ["engines"]
        }"#;
        assert!(manifesti_coz(m).is_none());
    }

    #[test]
    fn kategori_alani_yoksa_elenmiyor() {
        // Eski Epic sürümleri bu alanı yazmıyordu. Eksik listelemektense
        // fazladan listelemeyi tercih ediyoruz.
        let m = r#"{ "DisplayName": "Eski Oyun", "InstallLocation": "C:\\Eski" }"#;
        assert_eq!(manifesti_coz(m).unwrap().ad, "Eski Oyun");
    }

    #[test]
    fn eksik_alan_ve_bozuk_json_atlaniyor() {
        for m in [
            r#"{ "DisplayName": "Yalniz Ad" }"#,
            r#"{ "InstallLocation": "C:\\X" }"#,
            r#"{ "DisplayName": "", "InstallLocation": "C:\\X" }"#,
            "{ bozuk json",
            "",
        ] {
            assert!(manifesti_coz(m).is_none(), "{m} atlanmalıydı");
        }
    }

    #[test]
    fn exe_olmayan_baslatma_alani_yok_sayiliyor() {
        let m = r#"{
            "DisplayName": "Oyun",
            "InstallLocation": "C:\\X",
            "LaunchExecutable": "run.bat"
        }"#;
        assert_eq!(manifesti_coz(m).unwrap().baslatma_exe, None);
    }
}
