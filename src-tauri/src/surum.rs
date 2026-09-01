//! Demo ve tam sürüm ayrımı.
//!
//! Ayrım **derleme zamanında** yapılıyor: `cargo build --features demo` demo
//! ikilisini üretiyor. Çalışma zamanında lisans anahtarı doğrulama, sunucuya
//! sorma ya da "kilidi aç" akışı yok — demo ayrı bir ikili
//! (`docs/DISTRIBUTION.md`).
//!
//! ## Sınır özellik seviyesinde, zaman seviyesinde değil
//!
//! Demoda zaman sınırı, deneme sayacı, kapanma sayacı ya da nag ekranı **yok
//! ve olmayacak**. Bu bir ürün kararı: zaman baskısı, ürünün "istediğin an
//! geri al" vaadiyle çelişir. Karar aşağıdaki testlerle korunuyor.
//!
//! Demoda kapalı olan bir şey, arayüzde gizlenmekle kalmıyor; komut da hata
//! dönüyor. Tek yerde tutulması bunun içindir.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Bu ikili demo mu?
pub const DEMO: bool = cfg!(feature = "demo");

/// Demoda oluşturulabilecek profil sayısı.
///
/// Bir profil bilinçli: kullanıcı ürünün kendi oyununda gerçekten çalıştığını
/// görebilmeli — "kısıtlanmış" değil "eksiksiz ama dar".
pub const DEMO_PROFIL_SINIRI: usize = 1;

/// Bu ikilide neyin açık olduğu. Arayüz bunu açılışta bir kez okuyor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Kisitlar {
    pub demo: bool,
    /// `None` = sınırsız.
    pub profil_siniri: Option<usize>,
    /// Ağ sekmesi ve ağ komutları.
    pub ag_modulu: bool,
    /// Windows ile başlatma.
    pub otomatik_baslatma: bool,
    /// Profil dosyası içe/dışa aktarma (`docs/DISTRIBUTION.md` → Demo Kapsamı).
    pub profil_aktarimi: bool,
}

/// Bu ikilinin kısıtları.
pub const fn kisitlar() -> Kisitlar {
    if DEMO {
        Kisitlar {
            demo: true,
            profil_siniri: Some(DEMO_PROFIL_SINIRI),
            ag_modulu: false,
            otomatik_baslatma: false,
            profil_aktarimi: false,
        }
    } else {
        Kisitlar {
            demo: false,
            profil_siniri: None,
            ag_modulu: true,
            otomatik_baslatma: true,
            profil_aktarimi: true,
        }
    }
}

impl Kisitlar {
    /// Elde `mevcut` kadar profil varken yenisi eklenebilir mi?
    ///
    /// Var olan bir profilin güncellenmesi sayılmıyor: sınır profil
    /// **sayısında**, düzenleme hakkında değil.
    pub fn profil_eklenebilir(&self, mevcut: usize) -> bool {
        match self.profil_siniri {
            Some(sinir) => mevcut < sinir,
            None => true,
        }
    }
}

/// Ağ modülü kapalıysa hata döner.
///
/// Ağ komutlarının hepsi — ölçenler dahil — bundan geçiyor: demoda Network
/// Boost yok demek, "sekme gizli" değil "modül kapalı" demek.
pub fn ag_gerekli() -> Result<()> {
    if kisitlar().ag_modulu {
        Ok(())
    } else {
        Err(Error::DemoKisiti("Network Boost"))
    }
}

/// Profil aktarımı kapalıysa hata döner.
///
/// Dışa aktarma da kapsamda: demo ikilisi profil dosyası ÜRETMİYOR, çünkü
/// üretilen dosya tam sürümde içe aktarılabilir bir çıktı olurdu ve sınır
/// "eksiksiz ama dar" olma iddiasını bulanıklaştırırdı (`docs/DISTRIBUTION.md`).
pub fn profil_aktarimi_gerekli() -> Result<()> {
    if kisitlar().profil_aktarimi {
        Ok(())
    } else {
        Err(Error::DemoKisiti("Profil içe/dışa aktarma"))
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn kisitlar_bayrakla_tutarli() {
        let k = kisitlar();
        assert_eq!(k.demo, DEMO);
        // Demoda kapalı olması gereken dört şey; tam sürümde dördü de açık.
        assert_eq!(k.ag_modulu, !DEMO);
        assert_eq!(k.otomatik_baslatma, !DEMO);
        assert_eq!(k.profil_aktarimi, !DEMO);
        assert_eq!(k.profil_siniri.is_some(), DEMO);
    }

    #[test]
    fn sinirsiz_surumde_profil_hep_eklenebilir() {
        let k = Kisitlar {
            demo: false,
            profil_siniri: None,
            ag_modulu: true,
            otomatik_baslatma: true,
            profil_aktarimi: true,
        };
        assert!(k.profil_eklenebilir(0));
        assert!(k.profil_eklenebilir(999));
    }

    #[test]
    fn demoda_sinir_asilinca_eklenemiyor() {
        let k = Kisitlar {
            demo: true,
            profil_siniri: Some(1),
            ag_modulu: false,
            otomatik_baslatma: false,
            profil_aktarimi: false,
        };
        assert!(k.profil_eklenebilir(0));
        assert!(!k.profil_eklenebilir(1));
        assert!(!k.profil_eklenebilir(5));
    }

    #[test]
    fn ag_gerekli_bayrakla_tutarli() {
        assert_eq!(ag_gerekli().is_ok(), !DEMO);
    }

    #[test]
    fn profil_aktarimi_bayrakla_tutarli() {
        assert_eq!(profil_aktarimi_gerekli().is_ok(), !DEMO);
    }

    /// Ürün duruşu: demoda zaman sınırı yok.
    ///
    /// Kod seviyesinde korunması gereken şey, bu dosyaya bir "kalan gün",
    /// "ilk çalıştırma tarihi" ya da "açılış sayacı" alanının girmemesi.
    /// Kısıtlar yapısı yalnızca özellik bayrağı taşıyor.
    #[test]
    fn kisitlarda_zaman_alani_yok() {
        let json = serde_json::to_string(&kisitlar()).unwrap();
        for yasak in ["gun", "sure", "tarih", "sayac", "deneme", "kalan"] {
            assert!(
                !json.contains(yasak),
                "kısıtlara zaman/sayaç alanı girmiş: {yasak} — bkz. DISTRIBUTION.md"
            );
        }
    }
}
