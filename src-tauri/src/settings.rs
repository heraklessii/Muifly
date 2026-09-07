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
    /// kontrolünde tutuyor.
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

    /// Biten oyun oturumlarını diske kaydet (`monitor::gecmis`).
    ///
    /// **Varsayılan açık** — ve bu, "varsayılanların hepsi en az müdahale"
    /// kuralının istisnası değil, uzantısı. Geçmiş sistemde hiçbir şey
    /// değiştirmiyor; şeffaflık günlüğünün program kapandığında kaybolan
    /// yarısını tutuyor (tasarım ilkesi 2). Kapalı gelseydi, kullanıcı dün
    /// gece ne olduğunu ancak önceden açmayı akıl etmişse görebilirdi.
    ///
    /// Dosya hiçbir yere gönderilmiyor ve tek düğmeyle siliniyor.
    #[serde(default = "varsayilan_true")]
    pub gecmis_tut: bool,
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
            gecmis_tut: varsayilan_true(),
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
        atomik_yaz(yol, &serde_json::to_string_pretty(self)?)
    }
}

/// Bir metin dosyasını **ya tamamen ya hiç** yazar.
///
/// ## Neden gerekli
///
/// `std::fs::write` önce dosyayı sıfırlıyor, sonra dolduruyor. Arada program
/// ölürse (çökme, elektrik, Görev Yöneticisi'nden sonlandırma) diskte yarım
/// bir JSON kalıyor. Bunun en pahalı hâli geri alma defteri: karar #3'ün
/// tamamı "program çökerse bekleyen değişiklikler bir sonraki açılışta geri
/// alınsın" üzerine kurulu, ama bozuk bir defter `.bozuk` uzantısıyla
/// kenara konup boş defterle devam ediliyor — yani dondurulmuş süreçler
/// dondurulmuş, güç planı değişmiş kalıyor ve bunu geri alacak kayıt
/// kayboluyor. Çökme sonrası temizliğin en çok gerektiği an, tam da yazma
/// anında ölen bir program.
///
/// Aynı gerekçe ayarlar, oturum geçmişi ve profiller için de
/// geçerli; hepsi buradan geçiyor.
///
/// ## Nasıl
///
/// Geçici bir dosyaya yazılıyor, diske indiriliyor (`sync_all`), sonra
/// hedefin üstüne taşınıyor. `std::fs::rename` Windows'ta var olan dosyanın
/// üstüne yazıyor ve bu taşıma dosya sistemi seviyesinde atomik: okuyan
/// taraf ya eski ya yeni içeriği görüyor, yarısını asla görmüyor.
///
/// Geçici ad hedefin **yanında** duruyor: `%TEMP%`e yazıp taşımak, iki ayrı
/// birim arasında kopyalama olurdu ve orada atomiklik iddiası düşerdi.
pub fn atomik_yaz(yol: &Path, icerik: &str) -> Result<()> {
    use std::io::Write;

    if let Some(ust) = yol.parent() {
        if !ust.as_os_str().is_empty() {
            std::fs::create_dir_all(ust)?;
        }
    }

    let mut gecici = yol.as_os_str().to_os_string();
    // Süreç kimliği ekleniyor: iki Muifly aynı anda yazmaya kalkarsa
    // (tek örnek kuralı var ama çökme sonrası bir an çakışabilirler)
    // birbirinin yarım dosyasını taşımasınlar.
    gecici.push(format!(".{}.yeni", std::process::id()));
    let gecici = PathBuf::from(gecici);

    let sonuc = (|| -> std::io::Result<()> {
        let mut dosya = std::fs::File::create(&gecici)?;
        dosya.write_all(icerik.as_bytes())?;
        // Taşımadan önce diske inmesi şart: aksi halde taşıma tamamlanmış
        // ama içerik hâlâ önbellekte olabilir ve elektrik kesintisi boş bir
        // dosya bırakabilirdi.
        dosya.sync_all()?;
        drop(dosya);
        std::fs::rename(&gecici, yol)
    })();

    if sonuc.is_err() {
        let _ = std::fs::remove_file(&gecici);
    }
    sonuc?;
    Ok(())
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

pub fn gecmis_yolu() -> PathBuf {
    veri_dizini().join("oturum-gecmisi.json")
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn atomik_yaz_ustune_yaziyor_ve_iz_birakmiyor() {
        let dizin = tempfile::tempdir().unwrap();
        let yol = dizin.path().join("alt").join("a.json");

        atomik_yaz(&yol, "birinci").unwrap();
        assert_eq!(std::fs::read_to_string(&yol).unwrap(), "birinci");

        atomik_yaz(&yol, "ikinci").unwrap();
        assert_eq!(std::fs::read_to_string(&yol).unwrap(), "ikinci");

        // Geçici dosya taşındı, geride kalmadı: aksi halde her yazma bir
        // çöp dosya bırakırdı ve profil klasöründe bunlar `.json` olmasa da
        // kullanıcının gözüne çarpardı.
        let kalanlar: Vec<_> = std::fs::read_dir(yol.parent().unwrap())
            .unwrap()
            .flatten()
            .map(|g| g.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(kalanlar, vec!["a.json".to_string()]);
    }

    /// Yazma sırasında ölen bir program yarım dosya bırakmamalı: hedef
    /// dosyaya ancak tamamlanmış içerik taşınıyor. Ölümü taklit edemiyoruz,
    /// ama taşınmadan önce hedefin **eski içeriğinin durduğunu** ölçebiliriz.
    #[test]
    fn yazma_bitene_kadar_eski_icerik_duruyor() {
        let dizin = tempfile::tempdir().unwrap();
        let yol = dizin.path().join("a.json");
        atomik_yaz(&yol, "eski").unwrap();

        // Geçici dosya elle oluşturuluyor: `atomik_yaz`ın yarıda kaldığı an.
        let gecici = dizin.path().join(format!("a.json.{}.yeni", std::process::id()));
        std::fs::write(&gecici, "yarim").unwrap();
        assert_eq!(
            std::fs::read_to_string(&yol).unwrap(),
            "eski",
            "hedef dosya yarım yazma sırasında bozulmamalı"
        );
        std::fs::remove_file(&gecici).unwrap();
    }

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
        assert!(gecmis_yolu().starts_with(&kok));
    }

    #[test]
    fn varsayilan_gecmis_acik() {
        // Şeffaflık günlüğü program kapanınca kayboluyor; geçmiş onun
        // kalıcı yarısı (`monitor::gecmis`). Kapalı gelen bir geçmiş,
        // kullanıcıya ancak önceden açmayı akıl ettiyse bir şey söylerdi.
        assert!(Ayarlar::default().gecmis_tut);
    }
}
