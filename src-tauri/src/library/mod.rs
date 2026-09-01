//! Oyun kütüphanesi: kurulu oyunların adı, görseli ve çalıştırılabilirleri.
//!
//! Bu modülün varlık sebebi tek bir kullanıcı sorunu: profil açarken
//! `executable_names` alanına exe adını **elle yazmak**. Kullanıcı çoğu
//! zaman oyunun exe adını bilmiyor (`r5apex.exe`? `TslGame.exe`?), yanlış
//! yazınca da profil sessizce hiç eşleşmiyor.
//!
//! ## Neden bir API değil
//!
//! IGDB, RAWG, SteamGridDB ve benzerleri isim/görsel verebilirdi ama
//! hepsi anahtar istiyor. Kapalı kaynak bir masaüstü ikilisine gömülen
//! anahtar çıkarılabilir bir sırdır: kotayı yakar, servisin kullanım
//! koşullarını çiğnetir ve kullanıcının hangi oyunlara sahip olduğunu
//! bir sunucuya bildirir.
//!
//! Oysa aradığımız üç şey **zaten diskte**:
//!
//! | Kaynak | Ad | Görsel | Exe |
//! |---|---|---|---|
//! | Steam | `appmanifest_*.acf` | `appcache/librarycache/` | kurulum klasörü taraması |
//! | Epic | `Manifests/*.item` | (yok, exe ikonu) | `LaunchExecutable` |
//! | Çalışan süreç | pencere sahibi | exe ikonu | doğrudan |
//! | Elle seçim | dosya adı | exe ikonu | doğrudan |
//!
//! Sonuç: anahtar yok, kota yok, çevrimdışı çalışıyor ve **hiçbir ağ
//! isteği yok**. Karar ve gerekçesi: `docs/decisions.md` #25.
//!
//! ## Ne yapmıyor
//!
//! Bu modül hiçbir şeyi değiştirmiyor — ne sistemde, ne Steam/Epic
//! dosyalarında. Tamamı salt okuma. Bulduğu oyun bir profil **taslağı**
//! üretiyor, kullanıcı onaylamadan hiçbir profil kaydedilmiyor.

pub mod epic;
pub mod exe;
pub mod ikon;
pub mod png;
pub mod steam;
pub mod vdf;

use std::path::PathBuf;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// Oyunun nereden bulunduğu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kaynak {
    Steam,
    Epic,
    /// Şu anda çalışan bir süreçten. Kurulum klasörü bilinmiyor.
    #[default]
    Calisan,
    /// Kullanıcının dosya penceresinden seçtiği exe.
    Elle,
}

impl Kaynak {
    pub fn etiket(self) -> &'static str {
        match self {
            Kaynak::Steam => "Steam",
            Kaynak::Epic => "Epic Games",
            Kaynak::Calisan => "Çalışıyor",
            Kaynak::Elle => "Elle eklendi",
        }
    }
}

/// Kütüphanede bulunmuş bir oyun.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Oyun {
    /// `"steam:431960"`, `"epic:b675fd..."`, `"calisan:cs2.exe"`.
    ///
    /// Görsel ve taslak komutları oyunu bu kimlikle geri buluyor. Arayüzden
    /// dosya yolu gönderilmiyor: komutlar yalnızca **kendi taramamızın**
    /// bulduğu dosyalara hizmet ediyor.
    pub kimlik: String,
    pub ad: String,
    pub kaynak: Kaynak,
    /// Kurulum klasörü. Çalışan süreçlerde boş.
    pub kurulum: String,
    /// Aday exe adları, en olası önde. Küçük harfe indirgenmiş.
    pub exeler: Vec<String>,
    /// Görseli var mı? Görselin kendisi ayrı komutla, kart ekranda
    /// görününce isteniyor — 100 oyunluk bir kütüphanede hepsini birden
    /// üretmek megabaytlarca veriyi boşuna taşırdı.
    pub gorsel_var: bool,

    /// Kapak görselinin diskteki yolu. Arayüze gönderilmiyor.
    #[serde(skip)]
    kapak: Option<PathBuf>,
    /// İkonu çıkarılacak exe'nin tam yolu. Arayüze gönderilmiyor.
    #[serde(skip)]
    ikon_kaynagi: Option<PathBuf>,
}

/// Son taramanın sonucu.
///
/// Görsel ve taslak komutları buradan okuyor. Önbellek olmasaydı arayüzün
/// dosya yolu göndermesi gerekirdi ve bu, webview'e "istediğin dosyayı
/// okut" yüzeyi açardı.
static SON_TARAMA: Mutex<Vec<Oyun>> = Mutex::new(Vec::new());

/// Kurulu oyunları tarar. Bloke edici: çağıran ayrı iş parçacığında koşmalı.
pub fn tara() -> Vec<Oyun> {
    let mut oyunlar = Vec::new();

    for s in steam::oyunlar() {
        let adaylar = exe::adaylar(&s.kurulum, &s.ad);
        let kapak = steam::gorsel_yolu(&s.appid);
        oyunlar.push(Oyun {
            kimlik: format!("steam:{}", s.appid),
            ad: s.ad,
            kaynak: Kaynak::Steam,
            kurulum: s.kurulum.to_string_lossy().into_owned(),
            gorsel_var: kapak.is_some() || !adaylar.is_empty(),
            ikon_kaynagi: adaylar.first().map(|a| PathBuf::from(&a.yol)),
            exeler: adaylar.into_iter().map(|a| a.ad).collect(),
            kapak,
        });
    }

    for e in epic::oyunlar() {
        let adaylar = exe::adaylar(&e.kurulum, &e.ad);
        let ikon_kaynagi = adaylar.first().map(|a| PathBuf::from(&a.yol));
        let mut exeler: Vec<String> = adaylar.into_iter().map(|a| a.ad).collect();
        // Epic'in bildirdiği exe listede yoksa sona ekleniyor. Başa DEĞİL:
        // `LaunchExecutable` çoğu zaman bir başlatıcı ve profil, oyunun
        // penceresini açan sürece bağlanmalı — mod algılaması (`detect.rs`)
        // öndeki tam ekran pencerenin sahibine bakıyor, başlatıcıya değil.
        // Gerçek örnek: bu makinede NTE'nin `LaunchExecutable` alanı
        // `NTEGlobalLauncher.exe`, oyunun kendisi `NTEGlobalGame.exe`.
        if let Some(bildirilen) = e.baslatma_exe {
            if !exeler.contains(&bildirilen) {
                exeler.push(bildirilen);
            }
        }
        oyunlar.push(Oyun {
            kimlik: format!("epic:{}", e.app_name),
            ad: e.ad,
            kaynak: Kaynak::Epic,
            kurulum: e.kurulum.to_string_lossy().into_owned(),
            gorsel_var: ikon_kaynagi.is_some(),
            kapak: None,
            ikon_kaynagi,
            exeler,
        });
    }

    // Kullanıcının elle eklediği exe'ler yeniden taramada kaybolmuyor:
    // onları bulan bir tarayıcı yok, tek kaynakları bu önbellek.
    {
        let onceki = SON_TARAMA.lock();
        for o in onceki.iter().filter(|o| o.kaynak == Kaynak::Elle) {
            if !oyunlar.iter().any(|x| x.kimlik == o.kimlik) {
                oyunlar.push(o.clone());
            }
        }
    }

    // Aynı oyun iki mağazadan kurulu olabilir; kimlikler farklı olduğu için
    // ikisi de listede kalıyor. Ad sırası kullanıcının aradığını bulmasını
    // kolaylaştırıyor.
    oyunlar.sort_by_key(|o| o.ad.to_lowercase());

    *SON_TARAMA.lock() = oyunlar.clone();
    oyunlar
}

/// Kullanıcının dosya penceresinden seçtiği bir exe'yi kütüphaneye ekler.
///
/// Kütüphane taramasının kaçırdığı her şeyin çıkış yolu bu: taşınabilir
/// kurulumlar, Xbox/Microsoft Store oyunları, emülatörler, itch.io'dan
/// indirilenler. Yol kullanıcının **kendi seçtiği** dosya penceresinden
/// geliyor; arayüz kendi başına bir yol uyduramıyor.
pub fn elle_ekle(yol: &str) -> Option<Oyun> {
    let p = PathBuf::from(yol);
    if !p.is_file() {
        return None;
    }
    let ad_normal = crate::system_boost::detect::ad_normalize(yol);
    if !ad_normal.ends_with(".exe") {
        return None;
    }

    let gorunen = p
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| ad_normal.clone());

    let oyun = Oyun {
        kimlik: format!("elle:{ad_normal}"),
        ad: gorunen,
        kaynak: Kaynak::Elle,
        kurulum: p
            .parent()
            .map(|d| d.to_string_lossy().into_owned())
            .unwrap_or_default(),
        exeler: vec![ad_normal],
        gorsel_var: true,
        kapak: None,
        ikon_kaynagi: Some(p),
    };

    let mut onbellek = SON_TARAMA.lock();
    onbellek.retain(|o| o.kimlik != oyun.kimlik);
    onbellek.push(oyun.clone());
    Some(oyun)
}

/// Son taramada bulunmuş oyunu kimliğiyle getirir.
pub fn oyun(kimlik: &str) -> Option<Oyun> {
    SON_TARAMA
        .lock()
        .iter()
        .find(|o| o.kimlik == kimlik)
        .cloned()
}

/// Oyunun görselini `data:` adresi olarak üretir.
///
/// Önce Steam'in yerel kapağı, yoksa exe ikonu. İkisi de yoksa `None` —
/// arayüz baş harflerden bir yer tutucu çiziyor.
pub fn gorsel(kimlik: &str) -> Option<String> {
    let o = oyun(kimlik)?;

    if let Some(kapak) = o.kapak {
        if let Ok(bayt) = std::fs::read(&kapak) {
            let tur = if kapak
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("png"))
            {
                "image/png"
            } else {
                "image/jpeg"
            };
            return Some(veri_adresi(tur, &bayt));
        }
    }

    let yol = o.ikon_kaynagi?;
    let png = ikon::exe_ikonu(&yol.to_string_lossy())?;
    Some(veri_adresi("image/png", &png))
}

/// `data:<tur>;base64,<...>` adresi kurar.
fn veri_adresi(tur: &str, bayt: &[u8]) -> String {
    format!("data:{tur};base64,{}", base64_kodla(bayt))
}

/// Standart base64 (RFC 4648, dolgulu).
///
/// Bir kasa eklemek yerine on satır: tek kullanım yeri bu ve yeni bağımlılık
/// üçüncü taraf bildirimlerinin de yeniden üretilmesi demek (karar #21).
fn base64_kodla(veri: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut cikti = String::with_capacity(veri.len().div_ceil(3) * 4);
    for parca in veri.chunks(3) {
        let b = [
            parca[0],
            *parca.get(1).unwrap_or(&0),
            *parca.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        cikti.push(ABC[(n >> 18) as usize & 63] as char);
        cikti.push(ABC[(n >> 12) as usize & 63] as char);
        cikti.push(if parca.len() > 1 {
            ABC[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        cikti.push(if parca.len() > 2 {
            ABC[n as usize & 63] as char
        } else {
            '='
        });
    }
    cikti
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn base64_bilinen_degerler() {
        // RFC 4648 test vektörleri: dolgunun üç halini de kapsıyor.
        assert_eq!(base64_kodla(b""), "");
        assert_eq!(base64_kodla(b"f"), "Zg==");
        assert_eq!(base64_kodla(b"fo"), "Zm8=");
        assert_eq!(base64_kodla(b"foo"), "Zm9v");
        assert_eq!(base64_kodla(b"foob"), "Zm9vYg==");
        assert_eq!(base64_kodla(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_kodla(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn veri_adresi_bicimi() {
        assert_eq!(
            veri_adresi("image/png", b"foo"),
            "data:image/png;base64,Zm9v"
        );
    }

    #[test]
    fn bilinmeyen_kimlik_gorsel_dondurmuyor() {
        assert!(gorsel("steam:boyle-bir-oyun-yok").is_none());
        assert!(oyun("epic:yok").is_none());
    }

    #[test]
    fn kaynak_etiketleri_dolu() {
        for k in [Kaynak::Steam, Kaynak::Epic, Kaynak::Calisan, Kaynak::Elle] {
            assert!(!k.etiket().is_empty());
        }
    }
}
