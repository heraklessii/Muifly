//! Steam kütüphanesini **diskten** okur. Ağa çıkmaz, anahtar istemez.
//!
//! Steam istemcisi kurulu oyunların adını, klasörünü ve kapak görsellerini
//! zaten yerelde tutuyor:
//!
//! - `steamapps/libraryfolders.vdf` — kütüphane klasörlerinin listesi
//! - `steamapps/appmanifest_<appid>.acf` — oyunun adı ve kurulum klasörü
//! - `appcache/librarycache/<appid>/library_600x900.jpg` — kapak görseli
//!
//! Bunları okumak, IGDB/RAWG/SteamGridDB gibi servislere gitmekten her
//! açıdan üstün: anahtar gömmek gerekmiyor (dağıtılan bir ikiliden
//! çıkarılabilirdi), kota yok, çevrimdışı çalışıyor ve kullanıcının
//! kütüphanesi hiçbir sunucuya bildirilmiyor. `docs/decisions.md` #25.
//!
//! Görseller kullanıcının kendi diskinde duran, yayıncılara ait telifli
//! içerik. Muifly bunları **yalnızca görüntülüyor**: kopyalamıyor, ikiliye
//! gömmüyor, hiçbir yere göndermiyor.
//!
//! Yalnızca okuma yapılıyor; bu dosyada Steam'e yazan tek bir satır yok.

use std::path::{Path, PathBuf};

use super::vdf;

/// Steam kütüphanesinde bulunmuş bir oyun.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteamOyunu {
    pub appid: String,
    pub ad: String,
    pub kurulum: PathBuf,
}

/// Oyun olmayan Steam uygulamaları.
///
/// Bunlar kütüphanede "kurulu" görünüyor ama kullanıcının profil açacağı
/// şeyler değil. Liste dar tutuldu: emin olmadığımız bir appid'yi elemek,
/// gerçek bir oyunu gizlemek demek olurdu.
const OYUN_OLMAYANLAR: &[&str] = &[
    "228980",  // Steamworks Common Redistributables
    "1070560", // Steam Linux Runtime 1.0
    "1391110", // Steam Linux Runtime 2.0
    "1493710", // Proton Experimental
    "1628350", // Steam Linux Runtime 3.0
];

/// Kapak görseli için sırayla denenen dosya adları.
///
/// Sıra bilinçli: dikey kapak (600x900) kart düzenine en uygun olanı; yoksa
/// yatay başlık, o da yoksa logo. Steam istemcisi 2023'ten sonra bu
/// dosyaları `librarycache/<appid>/` altına taşıdı, öncesinde tek klasörde
/// `<appid>_library_600x900.jpg` biçiminde duruyorlardı. İkisi de deneniyor
/// çünkü eski önbellek güncellemeden sonra da diskte kalabiliyor.
const GORSEL_ADLARI: &[&str] = &[
    "library_600x900.jpg",
    "library_600x900_2x.jpg",
    "header.jpg",
    "library_hero.jpg",
    "logo.png",
];

/// Steam'in kurulu olduğu klasör. Steam yoksa `None`.
pub fn steam_yolu() -> Option<PathBuf> {
    use crate::registry::{metin_oku, Kok};

    // HKCU önce: kullanıcı başına kurulum ve yönetici yetkisi gerektirmiyor.
    let adaylar = [
        (Kok::Kullanici, "Software\\Valve\\Steam", "SteamPath"),
        (
            Kok::Makine,
            "SOFTWARE\\WOW6432Node\\Valve\\Steam",
            "InstallPath",
        ),
        (Kok::Makine, "SOFTWARE\\Valve\\Steam", "InstallPath"),
    ];

    for (kok, yol, ad) in adaylar {
        if let Ok(Some(deger)) = metin_oku(kok, yol, ad) {
            // HKCU'daki değer eğik bölü kullanıyor ("c:/program files
            // (x86)/steam"); Windows ikisini de kabul ediyor ama karşılaştırma
            // ve birleştirme için tek biçime indiriyoruz.
            let yol = PathBuf::from(deger.replace('/', "\\"));
            if yol.is_dir() {
                return Some(yol);
            }
        }
    }
    None
}

/// Kütüphane klasörleri. Steam'in kendi klasörü her zaman listede.
pub fn kitaplik_yollari(steam: &Path) -> Vec<PathBuf> {
    let mut yollar = vec![steam.to_path_buf()];

    let vdf_yolu = steam.join("steamapps").join("libraryfolders.vdf");
    if let Ok(icerik) = std::fs::read_to_string(&vdf_yolu) {
        for yol in kitapliklari_coz(&icerik) {
            let p = PathBuf::from(yol);
            if !yollar.contains(&p) {
                yollar.push(p);
            }
        }
    }

    yollar.retain(|y| y.join("steamapps").is_dir());
    yollar
}

/// `libraryfolders.vdf` içeriğinden yol listesi çıkarır.
///
/// Dosya sisteminden bağımsız; testi buradan geçiyor.
pub fn kitapliklari_coz(icerik: &str) -> Vec<String> {
    let kok = vdf::coz(icerik);
    let Some(kitapliklar) = kok.alan("libraryfolders") else {
        return Vec::new();
    };
    kitapliklar
        .alanlar()
        .iter()
        .filter_map(|(_, v)| v.metin("path"))
        .map(|s| s.to_string())
        .collect()
}

/// Tek bir `appmanifest_*.acf` içeriğini oyuna çevirir.
///
/// `kitaplik`, manifestin bulunduğu kütüphane klasörü — kurulum yolu
/// oradan türetiliyor.
pub fn manifesti_coz(icerik: &str, kitaplik: &Path) -> Option<SteamOyunu> {
    let kok = vdf::coz(icerik);
    let durum = kok.alan("AppState")?;
    let appid = durum.metin("appid")?.trim().to_string();
    let ad = durum.metin("name")?.trim().to_string();
    let kurulum_adi = durum.metin("installdir")?.trim();

    if appid.is_empty() || ad.is_empty() || kurulum_adi.is_empty() {
        return None;
    }
    if OYUN_OLMAYANLAR.contains(&appid.as_str()) {
        return None;
    }

    Some(SteamOyunu {
        appid,
        ad,
        kurulum: kitaplik.join("steamapps").join("common").join(kurulum_adi),
    })
}

/// Kurulu bütün Steam oyunları. Steam yoksa boş liste.
pub fn oyunlar() -> Vec<SteamOyunu> {
    let Some(steam) = steam_yolu() else {
        return Vec::new();
    };

    let mut bulunanlar: Vec<SteamOyunu> = Vec::new();
    for kitaplik in kitaplik_yollari(&steam) {
        let Ok(girdiler) = std::fs::read_dir(kitaplik.join("steamapps")) else {
            continue;
        };
        for girdi in girdiler.flatten() {
            let ad = girdi.file_name().to_string_lossy().to_lowercase();
            if !ad.starts_with("appmanifest_") || !ad.ends_with(".acf") {
                continue;
            }
            let Ok(icerik) = std::fs::read_to_string(girdi.path()) else {
                continue;
            };
            if let Some(oyun) = manifesti_coz(&icerik, &kitaplik) {
                // Aynı oyun iki kütüphanede kayıtlı olabilir (taşıma sonrası
                // artık manifest). Kurulum klasörü duran kazanıyor.
                if let Some(var) = bulunanlar.iter_mut().find(|o| o.appid == oyun.appid) {
                    if !var.kurulum.is_dir() && oyun.kurulum.is_dir() {
                        *var = oyun;
                    }
                } else {
                    bulunanlar.push(oyun);
                }
            }
        }
    }
    bulunanlar
}

/// Oyunun yerel kapak görselinin yolu. Önbellekte yoksa `None`.
pub fn gorsel_yolu(appid: &str) -> Option<PathBuf> {
    let steam = steam_yolu()?;
    let onbellek = steam.join("appcache").join("librarycache");

    for ad in GORSEL_ADLARI {
        // Yeni düzen: librarycache/<appid>/library_600x900.jpg
        let yeni = onbellek.join(appid).join(ad);
        if yeni.is_file() {
            return Some(yeni);
        }
        // Eski düzen: librarycache/<appid>_library_600x900.jpg
        let eski = onbellek.join(format!("{appid}_{ad}"));
        if eski.is_file() {
            return Some(eski);
        }
    }
    None
}

#[cfg(test)]
mod testler {
    use super::*;

    /// Bu makinedeki gerçek `appmanifest_431960.acf`'ten kısaltılmış örnek.
    const MANIFEST: &str = r#"
"AppState"
{
	"appid"		"431960"
	"name"		"Wallpaper Engine"
	"installdir"		"wallpaper_engine"
	"SizeOnDisk"		"826275581"
}
"#;

    #[test]
    fn manifest_okunuyor() {
        let kitaplik = Path::new("C:\\Program Files (x86)\\Steam");
        let oyun = manifesti_coz(MANIFEST, kitaplik).unwrap();
        assert_eq!(oyun.appid, "431960");
        assert_eq!(oyun.ad, "Wallpaper Engine");
        assert_eq!(
            oyun.kurulum,
            Path::new("C:\\Program Files (x86)\\Steam\\steamapps\\common\\wallpaper_engine")
        );
    }

    #[test]
    fn oyun_olmayanlar_eleniyor() {
        let m = r#""AppState" { "appid" "228980" "name" "Steamworks Common Redistributables" "installdir" "Steamworks Shared" }"#;
        assert!(manifesti_coz(m, Path::new("C:\\Steam")).is_none());
    }

    #[test]
    fn eksik_alanli_manifest_atlaniyor() {
        for m in [
            r#""AppState" { "appid" "1" "name" "Oyun" }"#,
            r#""AppState" { "appid" "1" "installdir" "oyun" }"#,
            r#""AppState" { "name" "Oyun" "installdir" "oyun" }"#,
            r#""AppState" { "appid" "" "name" "" "installdir" "" }"#,
            "",
        ] {
            assert!(manifesti_coz(m, Path::new("C:\\Steam")).is_none());
        }
    }

    #[test]
    fn kitaplik_listesi_cozuluyor() {
        let icerik = r#"
"libraryfolders"
{
	"0" { "path" "C:\\Program Files (x86)\\Steam" }
	"1" { "path" "D:\\SteamLibrary" }
}
"#;
        assert_eq!(
            kitapliklari_coz(icerik),
            vec![r"C:\Program Files (x86)\Steam", r"D:\SteamLibrary"]
        );
    }

    #[test]
    fn bozuk_kitaplik_dosyasi_bos_liste() {
        assert!(kitapliklari_coz("cop icerik").is_empty());
        assert!(kitapliklari_coz("").is_empty());
    }

    #[test]
    fn gorsel_adlari_dikey_kapakla_basliyor() {
        // Kart düzeni dikey kapağa göre tasarlandı; sıra değişirse arayüz
        // bozulur. Bu test sırayı sabitliyor.
        assert_eq!(GORSEL_ADLARI[0], "library_600x900.jpg");
    }
}
