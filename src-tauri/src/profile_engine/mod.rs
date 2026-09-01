//! Profil motoru: mod tanımları, profil şeması, disk deposu.
//!
//! Mod geçiş mantığı burada; modu gerçekten uygulayan taraf `state.rs`.

pub mod aktarim;
pub mod katalog;
pub mod schema;
pub mod store;

pub use aktarim::Onizleme;
pub use schema::{AffiniteTercihi, AgBolumu, OlceklemeBolumu, Profil, SistemBolumu};

use serde::{Deserialize, Serialize};

/// Programın beş modu (`docs/PROFILES.md`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mod", rename_all = "camelCase")]
pub enum Mod {
    /// Windows açılışında bir kez koşan mod.
    SistemAcilisi,
    /// Varsayılan durum: oyun yok, agresif optimizasyon yok.
    Bosta,
    /// Tam ekran bir uygulama önde ama profili yok.
    OyunGenel { pid: u32, surec: String },
    /// Bilinen oyun, kullanıcı profiliyle.
    OyunProfili {
        pid: u32,
        surec: String,
        profil_id: String,
    },
    /// Kullanıcının açıkça seçtiği rekabetçi mod.
    Rekabetci {
        pid: u32,
        surec: String,
        profil_id: Option<String>,
    },
}

impl Mod {
    pub fn ad(&self) -> &'static str {
        match self {
            Mod::SistemAcilisi => "Sistem Açılışı",
            Mod::Bosta => "Boşta",
            Mod::OyunGenel { .. } => "Oyun Algılandı",
            Mod::OyunProfili { .. } => "Oyun Profili",
            Mod::Rekabetci { .. } => "Rekabetçi Mod",
        }
    }

    /// Bu modda bir oyun korunuyor mu?
    pub fn oyun_pid(&self) -> Option<u32> {
        match self {
            Mod::SistemAcilisi | Mod::Bosta => None,
            Mod::OyunGenel { pid, .. }
            | Mod::OyunProfili { pid, .. }
            | Mod::Rekabetci { pid, .. } => Some(*pid),
        }
    }

    pub fn surec(&self) -> Option<&str> {
        match self {
            Mod::SistemAcilisi | Mod::Bosta => None,
            Mod::OyunGenel { surec, .. }
            | Mod::OyunProfili { surec, .. }
            | Mod::Rekabetci { surec, .. } => Some(surec),
        }
    }

    /// Ölçekleme ve kare üretimi bu modda kullanılabilir mi?
    ///
    /// Faz 3 gelmeden hepsi `false`; rekabetçi modda Faz 3'ten sonra da
    /// kare üretimi `false` kalacak.
    pub fn kare_uretimi_serbest(&self) -> bool {
        !matches!(self, Mod::Rekabetci { .. })
    }
}

/// Öndeki pencereden ve profil listesinden hangi moda geçilmeli?
///
/// Saf fonksiyon: sistemi okumuyor, karar veriyor. Kararın testi bu yüzden
/// gerçek bir oyuna ihtiyaç duymuyor.
///
/// `rekabetci_secili`: kullanıcı arayüzden rekabetçi modu işaretlediyse.
pub fn mod_sec(
    onde: Option<&crate::system_boost::OndekiPencere>,
    profiller: &[Profil],
    rekabetci_secili: bool,
) -> Mod {
    let Some(onde) = onde else { return Mod::Bosta };

    let (profil, _) = store::eslesen(profiller, &onde.ad);

    match profil {
        Some(p) => {
            // Profilin kendisi rekabetçi işaretliyse kullanıcının ayrıca
            // seçmesine gerek yok.
            if p.competitive || rekabetci_secili {
                Mod::Rekabetci {
                    pid: onde.pid,
                    surec: onde.ad.clone(),
                    profil_id: Some(p.profile_id.clone()),
                }
            } else {
                Mod::OyunProfili {
                    pid: onde.pid,
                    surec: onde.ad.clone(),
                    profil_id: p.profile_id.clone(),
                }
            }
        }
        // Profil yok: yalnızca tam ekransa "oyun olabilir" sayılıyor.
        // Pencereli bir uygulamayı oyun sanıp önceliğini yükseltmek, tarayıcı
        // ya da editör açan kullanıcıya sürpriz olurdu.
        None if onde.tam_ekran => {
            if rekabetci_secili {
                Mod::Rekabetci {
                    pid: onde.pid,
                    surec: onde.ad.clone(),
                    profil_id: None,
                }
            } else {
                Mod::OyunGenel {
                    pid: onde.pid,
                    surec: onde.ad.clone(),
                }
            }
        }
        None => Mod::Bosta,
    }
}

/// Profili olmayan bir oyun için uygulanan hafif varsayılan.
///
/// `docs/PROFILES.md`: "varsayılan hafif profil: process priority + temel
/// suspend + QoS". Dondurma listesi burada da BOŞ — tanımadığımız bir oyun
/// için kullanıcının uygulamalarını dondurmak, izin alınmamış bir müdahale.
pub fn genel_profil(surec: &str) -> Profil {
    let mut p = Profil::yeni("__genel__", "Genel oyun profili", surec);
    p.system.priority_class = "high".into();
    p.system.power_plan = Some("high_performance".into());
    p
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::system_boost::OndekiPencere;

    fn pencere(ad: &str, tam_ekran: bool) -> OndekiPencere {
        OndekiPencere {
            pid: 100,
            ad: ad.into(),
            tam_ekran,
        }
    }

    fn profil(kimlik: &str, exe: &str, rekabetci: bool) -> Profil {
        let mut p = Profil::yeni(kimlik, kimlik, exe);
        p.competitive = rekabetci;
        p.dogrula().unwrap().0
    }

    #[test]
    fn pencere_yoksa_bosta() {
        assert_eq!(mod_sec(None, &[], false), Mod::Bosta);
    }

    #[test]
    fn profilli_oyun_profil_moduna_geciyor() {
        let p = vec![profil("cs2", "cs2.exe", false)];
        let m = mod_sec(Some(&pencere("cs2.exe", true)), &p, false);
        assert!(matches!(m, Mod::OyunProfili { .. }));
    }

    #[test]
    fn rekabetci_profil_rekabetci_moda_geciyor() {
        let p = vec![profil("cs2", "cs2.exe", true)];
        let m = mod_sec(Some(&pencere("cs2.exe", true)), &p, false);
        assert!(matches!(m, Mod::Rekabetci { .. }));
    }

    #[test]
    fn kullanici_secimi_rekabetci_yapiyor() {
        let p = vec![profil("cs2", "cs2.exe", false)];
        let m = mod_sec(Some(&pencere("cs2.exe", true)), &p, true);
        assert!(matches!(m, Mod::Rekabetci { .. }));
    }

    #[test]
    fn tam_ekran_bilinmeyen_uygulama_genel_moda_geciyor() {
        let m = mod_sec(Some(&pencere("bilinmeyen.exe", true)), &[], false);
        assert!(matches!(m, Mod::OyunGenel { .. }));
    }

    #[test]
    fn pencereli_uygulama_oyun_sayilmiyor() {
        // Tarayıcı ya da editör açan kullanıcı optimizasyon görmemeli.
        let m = mod_sec(Some(&pencere("chrome.exe", false)), &[], false);
        assert_eq!(m, Mod::Bosta);
    }

    #[test]
    fn profilli_oyun_pencereliyken_de_taniniyor() {
        // Kenarlıksız değil pencereli oynayan kullanıcı, profilini tanımlamışsa
        // optimizasyonu almalı: tam ekran sezgisi yalnızca profilsizler için.
        let p = vec![profil("cs2", "cs2.exe", false)];
        let m = mod_sec(Some(&pencere("cs2.exe", false)), &p, false);
        assert!(matches!(m, Mod::OyunProfili { .. }));
    }

    #[test]
    fn rekabetci_modda_kare_uretimi_kapali() {
        let m = Mod::Rekabetci {
            pid: 1,
            surec: "cs2.exe".into(),
            profil_id: None,
        };
        assert!(!m.kare_uretimi_serbest());
    }

    #[test]
    fn genel_profil_dondurma_listesi_bos() {
        // Tanımadığımız bir oyun için kullanıcının uygulamalarına dokunmuyoruz.
        let p = genel_profil("bilinmeyen.exe");
        assert!(p.system.suspend_process_list.is_empty());
        assert!(p.dondurulacaklar().is_empty());
    }

    #[test]
    fn genel_profil_dogrulanabiliyor() {
        genel_profil("oyun.exe").dogrula().unwrap();
    }

    #[test]
    fn bosta_modda_oyun_pid_yok() {
        assert_eq!(Mod::Bosta.oyun_pid(), None);
        assert_eq!(
            Mod::OyunGenel {
                pid: 42,
                surec: "a.exe".into()
            }
            .oyun_pid(),
            Some(42)
        );
    }
}
