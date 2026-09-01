//! Uygulama ayarları.
//!
//! Ayar dosyası `%APPDATA%\Muifly\ayarlar.json`. Profillerle aynı klasörde
//! ama ayrı dosya: profiller paylaşılabilir, ayarlar makineye özel.
//!
//! Varsayılanların hepsi **en az müdahale** yönünde. Bir performans aracı,
//! kurulduğu anda sistemi değiştirmeye başlamamalı; kullanıcı ne istediğini
//! seçmeli (tasarım ilkesi 2 ve 5).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ayarlar {
    /// Oyun algılandığında profili kendiliğinden uygula.
    ///
    /// **Varsayılan kapalı.** Programın kullanıcı bakmadan sistemi
    /// değiştirmesi, ancak kullanıcı bunu açıkça istediğinde olmalı.
    #[serde(default)]
    pub otomatik_uygula: bool,

    /// Oyun kapandıktan sonra optimizasyonun geri alınması için beklenen süre.
    ///
    /// Alt+Tab yapan kullanıcı için tampon: her sekmede optimizasyonun
    /// kalkıp geri gelmesi hem gereksiz hem gürültülü.
    #[serde(default = "varsayilan_gecikme")]
    pub oyun_cikis_gecikmesi_sn: u32,

    /// Ölçüm örneği alma aralığı.
    #[serde(default = "varsayilan_ornek_araligi")]
    pub olcum_araligi_sn: u32,

    /// Gecikme ölçümünün hedefi.
    #[serde(default = "varsayilan_hedef")]
    pub gecikme_hedefi: String,

    /// Gecikme ölçümü açık mı? Kapalıysa hiç ICMP paketi gönderilmiyor.
    ///
    /// Kapatılabilir olması önemli: program ağa çıkan her işlemi kullanıcının
    /// kontrolünde tutuyor (`docs/DISTRIBUTION.md` telemetri bölümü).
    #[serde(default = "varsayilan_true")]
    pub gecikme_olcumu: bool,

    /// Kullanıcının rekabetçi mod işareti.
    #[serde(default)]
    pub rekabetci_mod: bool,

    /// Mod değiştiğinde masaüstü bildirimi göster.
    ///
    /// **Varsayılan kapalı.** Arka planda duran bir aracın her Alt+Tab'da
    /// bildirim atması gürültü; ama pencere kapalıyken ne olduğunu görmenin
    /// tek yolu da bu. Kararı kullanıcı veriyor (`docs/tasks.md`).
    #[serde(default)]
    pub mod_bildirimi: bool,

    /// Kapatınca sistem tepsisine in.
    #[serde(default = "varsayilan_true")]
    pub tepsiye_kucult: bool,

    /// Arayüz teması: `"dark"` | `"light"`.
    #[serde(default = "varsayilan_tema")]
    pub tema: String,
}

fn varsayilan_gecikme() -> u32 {
    10
}
fn varsayilan_ornek_araligi() -> u32 {
    2
}
fn varsayilan_hedef() -> String {
    crate::network_boost::latency::VARSAYILAN_HEDEF.to_string()
}
fn varsayilan_true() -> bool {
    true
}
fn varsayilan_tema() -> String {
    "dark".to_string()
}

impl Default for Ayarlar {
    fn default() -> Self {
        Self {
            otomatik_uygula: false,
            oyun_cikis_gecikmesi_sn: varsayilan_gecikme(),
            olcum_araligi_sn: varsayilan_ornek_araligi(),
            gecikme_hedefi: varsayilan_hedef(),
            gecikme_olcumu: varsayilan_true(),
            rekabetci_mod: false,
            mod_bildirimi: false,
            tepsiye_kucult: varsayilan_true(),
            tema: varsayilan_tema(),
        }
    }
}

impl Ayarlar {
    /// Değerleri güvenli aralığa çeker.
    ///
    /// Elle düzenlenmiş bir dosyadan gelen `olcum_araligi_sn: 0`, saniyede
    /// binlerce ölçüm demek olurdu — aracın kendisi sistemi yavaşlatırdı.
    pub fn normalize(mut self) -> Self {
        self.oyun_cikis_gecikmesi_sn = self.oyun_cikis_gecikmesi_sn.clamp(0, 300);
        self.olcum_araligi_sn = self.olcum_araligi_sn.clamp(1, 60);
        if self.gecikme_hedefi.trim().is_empty() {
            self.gecikme_hedefi = varsayilan_hedef();
        }
        if self.tema != "dark" && self.tema != "light" {
            self.tema = varsayilan_tema();
        }
        self
    }

    pub fn yukle(yol: &Path) -> Self {
        match std::fs::read_to_string(yol) {
            Ok(icerik) => serde_json::from_str::<Ayarlar>(&icerik)
                .unwrap_or_default()
                .normalize(),
            Err(_) => Ayarlar::default(),
        }
    }

    pub fn kaydet(&self, yol: &Path) -> Result<()> {
        if let Some(ust) = yol.parent() {
            std::fs::create_dir_all(ust)?;
        }
        std::fs::write(yol, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

/// Uygulama veri klasörü: `%APPDATA%\Muifly`.
///
/// Tauri'nin kendi yol çözücüsü kullanılmıyor: defter, program penceresi hiç
/// açılmadan (çökme sonrası temizlik) da okunabilmeli.
pub fn veri_dizini() -> PathBuf {
    let taban = std::env::var("APPDATA")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    taban.join("Muifly")
}

pub fn ayar_yolu() -> PathBuf {
    veri_dizini().join("ayarlar.json")
}

pub fn defter_yolu() -> PathBuf {
    veri_dizini().join("geri-alma-defteri.json")
}

pub fn profil_dizini() -> PathBuf {
    veri_dizini().join("profiller")
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn varsayilan_otomatik_uygulama_kapali() {
        // Ürün duruşu: program kendiliğinden sistemi değiştirmez.
        assert!(!Ayarlar::default().otomatik_uygula);
    }

    #[test]
    fn varsayilan_bildirim_kapali() {
        // Kurulduğu anda bildirim atan bir araç, ilk izlenimini gürültüyle
        // veriyor. Açmak kullanıcının kararı.
        assert!(!Ayarlar::default().mod_bildirimi);
    }

    #[test]
    fn sifir_olcum_araligi_duzeltiliyor() {
        let a = Ayarlar {
            olcum_araligi_sn: 0,
            ..Default::default()
        }
        .normalize();
        assert!(a.olcum_araligi_sn >= 1);
    }

    #[test]
    fn asiri_gecikme_kirpiliyor() {
        let a = Ayarlar {
            oyun_cikis_gecikmesi_sn: 99_999,
            ..Default::default()
        }
        .normalize();
        assert_eq!(a.oyun_cikis_gecikmesi_sn, 300);
    }

    #[test]
    fn bos_hedef_varsayilana_donuyor() {
        let a = Ayarlar {
            gecikme_hedefi: "   ".into(),
            ..Default::default()
        }
        .normalize();
        assert_eq!(a.gecikme_hedefi, varsayilan_hedef());
    }

    #[test]
    fn bilinmeyen_tema_duzeltiliyor() {
        let a = Ayarlar {
            tema: "neon".into(),
            ..Default::default()
        }
        .normalize();
        assert_eq!(a.tema, "dark");
    }

    #[test]
    fn eksik_alanlar_varsayilanla_okunuyor() {
        let a: Ayarlar = serde_json::from_str("{}").unwrap();
        assert_eq!(a, Ayarlar::default());
    }

    #[test]
    fn disk_gidis_donusu() {
        let dizin = tempfile::tempdir().unwrap();
        let yol = dizin.path().join("ayarlar.json");
        let a = Ayarlar {
            otomatik_uygula: true,
            tema: "light".into(),
            ..Default::default()
        };
        a.kaydet(&yol).unwrap();
        assert_eq!(Ayarlar::yukle(&yol), a);
    }

    #[test]
    fn bozuk_ayar_dosyasi_varsayilana_dusuyor() {
        let dizin = tempfile::tempdir().unwrap();
        let yol = dizin.path().join("ayarlar.json");
        std::fs::write(&yol, "bu json degil").unwrap();
        assert_eq!(Ayarlar::yukle(&yol), Ayarlar::default());
    }

    #[test]
    fn veri_yollari_ayni_koke_bagli() {
        let kok = veri_dizini();
        assert!(ayar_yolu().starts_with(&kok));
        assert!(defter_yolu().starts_with(&kok));
        assert!(profil_dizini().starts_with(&kok));
    }
}
