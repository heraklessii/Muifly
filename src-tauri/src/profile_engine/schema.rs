//! Profil şeması ve doğrulama.
//!
//! Şema `docs/PROFILES.md` ile birebir; JSON alan adları oradaki taslakla aynı
//! tutuldu çünkü profil dosyaları **kullanıcının okuyup düzenleyebileceği**
//! dosyalar (`docs/DISTRIBUTION.md`: kaynak kapalı ama profil formatı açık).
//!
//! Doğrulama burada iki iş yapıyor:
//!
//! 1. Bozuk bir dosyanın programı çökertmesini engellemek.
//! 2. **Güvenlik kurallarını dosya seviyesinde uygulamak.** Bir profil,
//!    arayüzde engellenen bir şeyi dosyayı elle düzenleyerek isteyemez:
//!    gerçek zamanlı öncelik reddediliyor, rekabetçi profilde kare üretimi
//!    zorla kapatılıyor, sistem süreçleri dondurma listesinden çıkarılıyor.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::system_boost::detect::{ad_normalize, dondurulabilir};

/// CPU affinite tercihi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AffiniteTercihi {
    /// Dokunma. **Varsayılan** — `docs/RISKS.md`: yanlış affinite ataması
    /// hibrit CPU'larda performansı düşürüyor.
    #[default]
    Dokunma,
    /// Yalnızca performans çekirdekleri. Hibrit olmayan CPU'da yok sayılıyor.
    SadecePCore,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SistemBolumu {
    /// `"high"`, `"normal"`… `"realtime"` reddediliyor.
    #[serde(default = "varsayilan_oncelik")]
    pub priority_class: String,
    #[serde(default)]
    pub cpu_affinity: AffiniteTercihi,
    /// Dondurulacak süreçler. Varsayılan BOŞ (`docs/PROFILES.md`).
    #[serde(default)]
    pub suspend_process_list: Vec<String>,
    /// Listede olsa bile dondurulmayacaklar. Kullanıcının kendi istisnası.
    #[serde(default)]
    pub suspend_whitelist_exempt: Vec<String>,
    /// `"balanced"`, `"high_performance"`, `"ultimate_performance"` ya da
    /// `null` (dokunma).
    #[serde(default)]
    pub power_plan: Option<String>,
}

fn varsayilan_oncelik() -> String {
    "high".to_string()
}

impl Default for SistemBolumu {
    fn default() -> Self {
        Self {
            priority_class: varsayilan_oncelik(),
            cpu_affinity: AffiniteTercihi::default(),
            suspend_process_list: Vec::new(),
            suspend_whitelist_exempt: Vec::new(),
            power_plan: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AgBolumu {
    /// `"auto_test"` = ölç ve öner. Muifly DNS'i kendiliğinden
    /// DEĞİŞTİRMİYOR (`network_boost::dns` modül belgesi).
    #[serde(default)]
    pub preferred_dns: Option<String>,
    #[serde(default)]
    pub qos_priority: bool,
    #[serde(default)]
    pub tcp_nodelay: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct OlceklemeBolumu {
    #[serde(default)]
    pub enabled: bool,
    /// `"tam_sayi"` | `"bilinear"` | `"lanczos"` | `"xbr"`, ya da `null`
    /// (varsayılan). Tanınmayan bir ad `dogrula` içinde düşürülüyor.
    #[serde(default)]
    pub algorithm: Option<String>,
    /// Faz 4. `dogrula` şimdilik her koşulda kapatıyor.
    #[serde(default)]
    pub frame_generation: bool,
}

impl OlceklemeBolumu {
    /// Profildeki algoritma; yoksa ya da tanınmıyorsa varsayılan.
    ///
    /// Varsayılan tam sayı katı: kaynakta olmayan renk üretmeyen tek yol.
    /// Profilinde algoritma yazmayan bir kullanıcı, en az müdahale edeni
    /// almalı.
    pub fn algoritma(&self) -> crate::scaling::Algoritma {
        self.algorithm
            .as_deref()
            .and_then(crate::scaling::Algoritma::coz)
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Profil {
    pub profile_id: String,
    pub display_name: String,
    /// Eşleşecek çalıştırılabilir adları. Küçük harfe indirgeniyor.
    pub executable_names: Vec<String>,
    /// Rekabetçi mod: kare üretimi ve agresif ölçekleme zorla kapalı.
    #[serde(default)]
    pub competitive: bool,
    #[serde(default)]
    pub system: SistemBolumu,
    #[serde(default)]
    pub network: AgBolumu,
    #[serde(default)]
    pub scaling: OlceklemeBolumu,
    #[serde(default = "varsayilan_olusturan")]
    pub created_by: String,
    /// İleride topluluk paylaşımı için ayrılmış alan.
    #[serde(default)]
    pub shared: bool,
}

fn varsayilan_olusturan() -> String {
    "user".to_string()
}

impl Profil {
    /// Yeni bir profilin güvenli iskeleti.
    pub fn yeni(id: impl Into<String>, ad: impl Into<String>, exe: impl Into<String>) -> Self {
        let exe = ad_normalize(&exe.into());
        Self {
            profile_id: id.into(),
            display_name: ad.into(),
            executable_names: vec![exe],
            competitive: false,
            system: SistemBolumu::default(),
            network: AgBolumu::default(),
            scaling: OlceklemeBolumu::default(),
            created_by: varsayilan_olusturan(),
            shared: false,
        }
    }

    /// Profili doğrular ve **güvenli hale getirir**.
    ///
    /// Dönen değer, uygulanacak profil ve kullanıcıya gösterilecek düzeltme
    /// listesi. Düzeltmeler sessizce yapılmıyor: her biri arayüzde ve günlükte
    /// görünüyor (şeffaflık ilkesi) — "profilimde şunu yazmıştım ama olmadı"
    /// diye bir soru kalmamalı.
    pub fn dogrula(mut self) -> Result<(Profil, Vec<String>)> {
        let mut duzeltmeler = Vec::new();

        if self.profile_id.trim().is_empty() {
            return Err(Error::ProfileInvalid("profil kimliği boş".into()));
        }
        if self.display_name.trim().is_empty() {
            self.display_name = self.profile_id.clone();
            duzeltmeler.push("görünen ad boştu, kimlik kullanıldı".into());
        }

        // Çalıştırılabilir adlar normalleştiriliyor ve boşlar atılıyor.
        let onceki_adet = self.executable_names.len();
        self.executable_names = self
            .executable_names
            .iter()
            .map(|a| ad_normalize(a))
            .filter(|a| !a.is_empty())
            .collect();
        self.executable_names.sort();
        self.executable_names.dedup();
        if self.executable_names.len() != onceki_adet {
            duzeltmeler.push("çalıştırılabilir ad listesi temizlendi (boş/tekrar)".into());
        }
        if self.executable_names.is_empty() {
            return Err(Error::ProfileInvalid(
                "profilde hiç çalıştırılabilir ad yok".into(),
            ));
        }

        // Öncelik: `realtime` burada hata veriyor.
        crate::system_boost::Oncelik::ayristir(&self.system.priority_class)?;

        // Güç planı adı tanınıyor mu?
        if let Some(plan) = &self.system.power_plan {
            crate::system_boost::GucPlani::ayristir(plan)?;
        }

        // Dondurma listesi: sistem süreçleri çıkarılıyor.
        let mut atilanlar = Vec::new();
        self.system.suspend_process_list = self
            .system
            .suspend_process_list
            .iter()
            .map(|a| ad_normalize(a))
            .filter(|a| {
                if a.is_empty() {
                    return false;
                }
                if !dondurulabilir(a) {
                    atilanlar.push(a.clone());
                    return false;
                }
                true
            })
            .collect();
        self.system.suspend_process_list.sort();
        self.system.suspend_process_list.dedup();
        if !atilanlar.is_empty() {
            duzeltmeler.push(format!(
                "sistem süreçleri dondurma listesinden çıkarıldı: {}",
                atilanlar.join(", ")
            ));
        }

        // Oyunun kendisi dondurma listesinde olamaz.
        let kendi_adlari: Vec<String> = self.executable_names.clone();
        let onceki = self.system.suspend_process_list.len();
        self.system
            .suspend_process_list
            .retain(|a| !kendi_adlari.contains(a));
        if self.system.suspend_process_list.len() != onceki {
            duzeltmeler.push("oyunun kendisi dondurma listesinden çıkarıldı".into());
        }

        self.system.suspend_whitelist_exempt = self
            .system
            .suspend_whitelist_exempt
            .iter()
            .map(|a| ad_normalize(a))
            .filter(|a| !a.is_empty())
            .collect();

        // Kare üretimi (Faz 4) henüz yok: profilde açık yazıyorsa
        // kapatılıyor. Rekabetçi profilde ayrıca Faz 4 geldiğinde de kapalı
        // kalacak (`docs/PROFILES.md`).
        if self.scaling.frame_generation {
            self.scaling.frame_generation = false;
            duzeltmeler.push(if self.competitive {
                "rekabetçi profilde kare üretimi kapatıldı (gecikme riski)".into()
            } else {
                "kare üretimi henüz yok (Faz 4), ayar yok sayıldı".to_string()
            });
        }

        // Rekabetçi mod kuralı: ölçekleme de kapalı.
        //
        // "UI seviyesinde de engellenmeli, sadece config'e güvenilmemeli"
        // (`docs/PROFILES.md`) — burası config tarafındaki kapı. Ölçekleme
        // her karede ölçülebilir bir gecikme ekliyor (`scaling::gecikme`) ve
        // rekabetçi mod tam olarak o gecikmeyi en aza indirmek için var.
        if self.competitive && self.scaling.enabled {
            self.scaling.enabled = false;
            duzeltmeler.push("rekabetçi profilde ölçekleme kapatıldı (gecikme ekliyor)".into());
        }

        // Algoritma adı tanınıyor mu? Elle düzenlenmiş bir dosyada yazım
        // hatası olabilir; sessizce varsayılana düşmek, kullanıcının
        // seçtiğini sandığı şeyden başkasını çalıştırmak olurdu.
        if let Some(ad) = self.scaling.algorithm.clone() {
            if crate::scaling::Algoritma::coz(&ad).is_none() {
                self.scaling.algorithm = None;
                duzeltmeler.push(format!(
                    "'{ad}' diye bir ölçekleme algoritması yok, varsayılana dönüldü"
                ));
            }
        }

        Ok((self, duzeltmeler))
    }

    /// Verilen süreç adı bu profile ait mi?
    pub fn eslesiyor(&self, surec_adi: &str) -> bool {
        let ad = ad_normalize(surec_adi);
        self.executable_names.contains(&ad)
    }

    /// Dondurulacak nihai liste (istisnalar düşülmüş).
    pub fn dondurulacaklar(&self) -> Vec<String> {
        self.system
            .suspend_process_list
            .iter()
            .filter(|a| !self.system.suspend_whitelist_exempt.contains(a))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn temel() -> Profil {
        Profil::yeni("test_v1", "Test Oyunu", "oyun.exe")
    }

    #[test]
    fn temel_profil_dogrulaniyor() {
        let (p, duzeltmeler) = temel().dogrula().unwrap();
        assert_eq!(p.executable_names, vec!["oyun.exe"]);
        assert!(duzeltmeler.is_empty(), "temiz profilde düzeltme olmamalı");
    }

    #[test]
    fn realtime_oncelik_profili_reddediyor() {
        let mut p = temel();
        p.system.priority_class = "realtime".into();
        assert!(p.dogrula().is_err(), "dosyadan realtime istenemez");
    }

    #[test]
    fn rekabetci_profilde_kare_uretimi_zorla_kapaniyor() {
        let mut p = temel();
        p.competitive = true;
        p.scaling.frame_generation = true;
        let (p, duzeltmeler) = p.dogrula().unwrap();
        assert!(!p.scaling.frame_generation);
        assert!(duzeltmeler.iter().any(|d| d.contains("kare üretimi")));
    }

    #[test]
    fn sistem_surecleri_dondurma_listesinden_dusuyor() {
        let mut p = temel();
        p.system.suspend_process_list = vec![
            "explorer.exe".into(),
            "discord.exe".into(),
            "csrss.exe".into(),
        ];
        let (p, duzeltmeler) = p.dogrula().unwrap();
        assert_eq!(p.system.suspend_process_list, vec!["discord.exe"]);
        assert!(duzeltmeler.iter().any(|d| d.contains("explorer.exe")));
    }

    #[test]
    fn oyunun_kendisi_dondurma_listesinden_dusuyor() {
        let mut p = temel();
        p.system.suspend_process_list = vec!["OYUN.EXE".into(), "discord.exe".into()];
        let (p, duzeltmeler) = p.dogrula().unwrap();
        assert_eq!(p.system.suspend_process_list, vec!["discord.exe"]);
        assert!(duzeltmeler.iter().any(|d| d.contains("oyunun kendisi")));
    }

    #[test]
    fn olcekleme_ayari_korunuyor() {
        // Faz 3 geldi: geçerli bir ölçekleme ayarı artık siliniyor değil,
        // olduğu gibi uygulanıyor.
        let mut p = temel();
        p.scaling.enabled = true;
        p.scaling.algorithm = Some("lanczos".into());
        let (p, duzeltmeler) = p.dogrula().unwrap();
        assert!(p.scaling.enabled);
        assert_eq!(p.scaling.algorithm.as_deref(), Some("lanczos"));
        assert!(duzeltmeler.is_empty(), "{duzeltmeler:?}");
        assert_eq!(p.scaling.algoritma(), crate::scaling::Algoritma::Lanczos);
    }

    #[test]
    fn bilinmeyen_algoritma_varsayilana_dusuyor() {
        // Elle düzenlenmiş dosyada yazım hatası: sessizce başka bir şey
        // çalıştırmak yerine söyleniyor.
        let mut p = temel();
        p.scaling.enabled = true;
        p.scaling.algorithm = Some("lanzcos".into());
        let (p, duzeltmeler) = p.dogrula().unwrap();
        assert_eq!(p.scaling.algorithm, None);
        assert_eq!(p.scaling.algoritma(), crate::scaling::Algoritma::default());
        assert!(duzeltmeler.iter().any(|d| d.contains("lanzcos")));
    }

    #[test]
    fn rekabetci_profilde_olcekleme_zorla_kapaniyor() {
        // Ürün duruşu: rekabetçi modda ölçekleme kısıtlı değil, kapalı.
        // Gerekçe `scaling` modül belgesinde; burası dosya tarafındaki kapı.
        let mut p = temel();
        p.competitive = true;
        p.scaling.enabled = true;
        p.scaling.algorithm = Some("xbr".into());
        let (p, duzeltmeler) = p.dogrula().unwrap();
        assert!(!p.scaling.enabled);
        assert!(duzeltmeler
            .iter()
            .any(|d| d.contains("ölçekleme kapatıldı")));
    }

    #[test]
    fn kare_uretimi_faz4e_kadar_kapali() {
        let mut p = temel();
        p.scaling.frame_generation = true;
        let (p, duzeltmeler) = p.dogrula().unwrap();
        assert!(!p.scaling.frame_generation);
        assert!(duzeltmeler.iter().any(|d| d.contains("Faz 4")));
    }

    #[test]
    fn bos_exe_listesi_reddediliyor() {
        let mut p = temel();
        p.executable_names = vec!["".into(), "   ".into()];
        // Boşluklu ad normalize edilince boşalmıyor ("   " → "   "), bu yüzden
        // asıl senaryo tamamen boş liste.
        p.executable_names = vec![];
        assert!(p.dogrula().is_err());
    }

    #[test]
    fn exe_adlari_normalize_ve_tekillestiriliyor() {
        let mut p = temel();
        p.executable_names = vec![
            "C:\\Games\\Oyun.exe".into(),
            "oyun.exe".into(),
            "OYUN.EXE".into(),
        ];
        let (p, _) = p.dogrula().unwrap();
        assert_eq!(p.executable_names, vec!["oyun.exe"]);
    }

    #[test]
    fn istisna_listesi_dondurmayi_engelliyor() {
        let mut p = temel();
        p.system.suspend_process_list = vec!["discord.exe".into(), "spotify.exe".into()];
        p.system.suspend_whitelist_exempt = vec!["Discord.exe".into()];
        let (p, _) = p.dogrula().unwrap();
        assert_eq!(p.dondurulacaklar(), vec!["spotify.exe"]);
    }

    #[test]
    fn eslesme_buyuk_kucuk_harf_duyarsiz() {
        let (p, _) = temel().dogrula().unwrap();
        assert!(p.eslesiyor("OYUN.EXE"));
        assert!(p.eslesiyor("C:\\Yol\\Oyun.exe"));
        assert!(!p.eslesiyor("baskaoyun.exe"));
    }

    #[test]
    fn json_gidis_donusu() {
        let (p, _) = temel().dogrula().unwrap();
        let metin = serde_json::to_string_pretty(&p).unwrap();
        let geri: Profil = serde_json::from_str(&metin).unwrap();
        assert_eq!(p, geri);
    }

    #[test]
    fn eksik_alanlar_varsayilanla_dolduruluyor() {
        // Kullanıcı elle yazdığı bir profilde yalnızca zorunlu alanları
        // vermiş olabilir; program çökmemeli.
        let metin = r#"{
            "profile_id": "elle",
            "display_name": "Elle Yazılmış",
            "executable_names": ["oyun.exe"]
        }"#;
        let p: Profil = serde_json::from_str(metin).unwrap();
        assert_eq!(p.system.priority_class, "high");
        assert!(p.system.suspend_process_list.is_empty());
        assert!(!p.competitive);
        p.dogrula().unwrap();
    }

    #[test]
    fn bilinmeyen_guc_plani_reddediliyor() {
        let mut p = temel();
        p.system.power_plan = Some("turbo_mode".into());
        assert!(p.dogrula().is_err());
    }

    #[test]
    fn varsayilan_affinite_dokunma() {
        // `docs/RISKS.md` gereği affinite varsayılan kapalı.
        assert_eq!(
            SistemBolumu::default().cpu_affinity,
            AffiniteTercihi::Dokunma
        );
    }
}
