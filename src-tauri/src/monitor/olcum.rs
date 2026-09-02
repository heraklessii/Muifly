//! Kare ölçümünün başlatılması: yükseltilmiş yardımcı süreç.
//!
//! Karar #27 ölçtü: gerçek zamanlı ETW oturumu yükseltilmiş yetki istiyor,
//! tasarım ilkesi 5 ise arka plan izlemesinin yükseltilmiş çalışmasını
//! yasaklıyor. İkisini uzlaştıran tek yol, ölçümü **ana uygulamanın dışında**
//! yapmak: kısa ömürlü, tek işi olan, işi bitince kapanan ayrı bir süreç.
//!
//! ```text
//!   Muifly (yükseltilmemiş)          muifly-olcum.exe (yükseltilmiş)
//!        │                                    │
//!        │  ShellExecuteEx "runas" ─────────► başlar (UAC)
//!        │                                    │  ETW oturumu, N saniye
//!        │  ◄──── JSON dosyası ───────────────┤  özeti yazar
//!        │                                    └─ kapanır, oturumu kapatır
//!        └─ okur, günlüğe yazar, gösterir
//! ```
//!
//! **Neden dosya, neden boru değil**: yükseltilmiş bir sürece boru bağlamak
//! bütünlük seviyesi (integrity level) farkı yüzünden ek izin işi getiriyor.
//! Tek yönlü ve tek seferlik bir özet için geçici dosya hem basit hem
//! denetlenebilir — kullanıcı isterse dosyayı açıp okuyabiliyor, ki bu
//! tasarım ilkesi 2 ile aynı yöne bakıyor.
//!
//! **Ana uygulama asla yükselmiyor.** Bu modül UAC'ı yalnızca kullanıcı
//! ölçümü başlattığında ve nedenini gördükten sonra tetikliyor.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::monitor::etw::{self, Engel};
use crate::monitor::frames::KareOzeti;

/// Yardımcı sürecin adı. Ana ikilinin yanında duruyor.
pub const YARDIMCI: &str = "muifly-olcum.exe";

/// Ölçümün üst sınırı.
///
/// Sınırsız bir ölçüm, yükseltilmiş bir sürecin süresiz açık kalması
/// demekti. Kullanıcı daha uzun ölçmek isterse yeniden başlatır; süresiz
/// çalışan yükseltilmiş bir süreç bırakmak, geri alma defterinin çözdüğü
/// "arkada iz kalmasın" ilkesiyle çelişirdi.
pub const EN_UZUN_SANIYE: u64 = 120;
pub const EN_KISA_SANIYE: u64 = 5;

/// Ölçüm neden yapılamadı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OlcumHatasi {
    /// Kullanıcı UAC istemini reddetti. Hata değil, tercih.
    KullaniciReddetti,
    /// Yardımcı ikili bulunamadı.
    YardimciYok(String),
    /// Yardımcı çalıştı ama başaramadı.
    Engel(Engel),
    /// Yardımcı beklenen sürede bitmedi.
    ZamanAsimi,
    /// Özet dosyası okunamadı ya da bozuk.
    OzetOkunamadi(String),
    /// Yeterli kare toplanamadı — oyun sunum yapmıyor olabilir.
    KareYok,
}

impl std::fmt::Display for OlcumHatasi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OlcumHatasi::KullaniciReddetti => {
                write!(
                    f,
                    "ölçüm için istenen yetki verilmedi, hiçbir şey değişmedi"
                )
            }
            OlcumHatasi::YardimciYok(y) => write!(f, "ölçüm yardımcısı bulunamadı: {y}"),
            OlcumHatasi::Engel(e) => write!(f, "{e}"),
            OlcumHatasi::ZamanAsimi => write!(f, "ölçüm beklenen sürede bitmedi"),
            OlcumHatasi::OzetOkunamadi(s) => write!(f, "ölçüm sonucu okunamadı: {s}"),
            OlcumHatasi::KareYok => write!(
                f,
                "ölçüm süresince bu oyundan kare gelmedi; oyun küçültülmüş ya da \
                 sunum yapmıyor olabilir"
            ),
        }
    }
}

/// Arayüzün ölçümü başlatmadan önce okuduğu bilgi.
///
/// `aciklama` burada, Rust tarafında duruyor — karar #17: kullanıcıya giden
/// metinler tek yerde. Arayüz bu cümleyi UAC istemi çıkmadan ÖNCE gösteriyor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KareOlcumDurumu {
    /// Yardımcı ikili yanımızda duruyor mu?
    pub kullanilabilir: bool,
    /// Her zaman `true`; ölçüldü, karar #27.
    pub yetki_gerekiyor: bool,
    pub en_kisa_saniye: u64,
    pub en_uzun_saniye: u64,
    pub aciklama: String,
}

/// Yardımcı sürece verilen iş.
///
/// Ayrı bir tip olması, komut satırının iki uçta da **aynı yerden**
/// üretilip çözülmesini sağlıyor: ana uygulama `argumanlar()` ile yazıyor,
/// yardımcı `ayristir()` ile okuyor. İkisi ayrı yazılsaydı sessizce
/// birbirinden kayabilirlerdi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Istek {
    pub pid: u32,
    pub saniye: u64,
    pub cikti: PathBuf,
}

impl Istek {
    pub fn yeni(pid: u32, saniye: u64, cikti: PathBuf) -> Self {
        Self {
            pid,
            saniye: saniye.clamp(EN_KISA_SANIYE, EN_UZUN_SANIYE),
            cikti,
        }
    }

    pub fn argumanlar(&self) -> Vec<String> {
        vec![
            "--pid".into(),
            self.pid.to_string(),
            "--saniye".into(),
            self.saniye.to_string(),
            "--cikti".into(),
            self.cikti.to_string_lossy().into_owned(),
        ]
    }

    pub fn ayristir<I: IntoIterator<Item = String>>(args: I) -> Result<Self, String> {
        let a: Vec<String> = args.into_iter().collect();
        let mut pid = None;
        let mut saniye = None;
        let mut cikti = None;

        let mut i = 0;
        while i < a.len() {
            let deger = a.get(i + 1);
            match a[i].as_str() {
                "--pid" => {
                    pid = deger
                        .ok_or_else(|| "--pid değersiz".to_string())?
                        .parse::<u32>()
                        .map_err(|_| "--pid sayı değil".to_string())
                        .map(Some)?;
                    i += 2;
                }
                "--saniye" => {
                    saniye = deger
                        .ok_or_else(|| "--saniye değersiz".to_string())?
                        .parse::<u64>()
                        .map_err(|_| "--saniye sayı değil".to_string())
                        .map(Some)?;
                    i += 2;
                }
                "--cikti" => {
                    cikti = Some(PathBuf::from(
                        deger.ok_or_else(|| "--cikti değersiz".to_string())?,
                    ));
                    i += 2;
                }
                bilinmeyen => return Err(format!("bilinmeyen argüman: {bilinmeyen}")),
            }
        }

        let pid = pid.ok_or_else(|| "--pid eksik".to_string())?;
        if pid == 0 {
            return Err("--pid sıfır olamaz".into());
        }
        Ok(Istek::yeni(
            pid,
            saniye.ok_or_else(|| "--saniye eksik".to_string())?,
            cikti.ok_or_else(|| "--cikti eksik".to_string())?,
        ))
    }
}

/// Arayüze dönen tam rapor: ne ölçüldü, ne kadar, ne çıktı.
///
/// `surec` ve `pid` burada duruyor ki arayüz "ne ölçtün" sorusunu
/// cevaplayabilsin. Şeffaflık ilkesi: ölçümün hangi sürece ait olduğu
/// gösterilmeden bir sayı göstermek, sayının nereden geldiğini saklamak olur.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OlcumRaporu {
    pub surec: String,
    pub pid: u32,
    pub saniye: u64,
    pub sonuc: Sonuc,
}

/// Yardımcının diske yazdığı sonuç.
///
/// Özet `Option`: ölçüm oturumu açıldı ama hiç kare gelmediyse `None`.
/// Sıfırlarla dolu bir özet "0 FPS ölçüldü" gibi okunurdu.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sonuc {
    pub ozet: Option<KareOzeti>,
    /// Toplanan ham sunum sayısı — özet çıkmasa bile bilgi veriyor.
    pub kare_sayisi: usize,
}

// ---------------------------------------------------------------------------
// Yardımcı süreç tarafı
// ---------------------------------------------------------------------------

/// Yükseltilmiş yardımcının yaptığı iş: ölç, yaz, çık.
///
/// Bu fonksiyon **ana uygulamada çağrılmıyor**; `muifly-olcum` ikilisinin
/// gövdesi. Burada durması, ölçüm mantığının kütüphanenin test edilen
/// kısmında kalmasını sağlıyor.
pub fn yardimci_calistir(istek: &Istek) -> Result<Sonuc, Engel> {
    let olcum = etw::Olcum::baslat(istek.pid)?;
    std::thread::sleep(std::time::Duration::from_secs(istek.saniye));

    let sonuc = Sonuc {
        ozet: olcum.ozet(),
        kare_sayisi: olcum.kare_sayisi(),
    };
    // `olcum` burada düşüyor ve ETW oturumunu kapatıyor.
    Ok(sonuc)
}

pub fn sonucu_yaz(yol: &Path, sonuc: &Sonuc) -> std::io::Result<()> {
    let metin = serde_json::to_string_pretty(sonuc)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(yol, metin)
}

// ---------------------------------------------------------------------------
// Ana uygulama tarafı
// ---------------------------------------------------------------------------

/// Yardımcı ikilinin yolu: ana ikilinin yanında.
pub fn yardimci_yolu() -> Result<PathBuf, OlcumHatasi> {
    let ben = std::env::current_exe()
        .map_err(|e| OlcumHatasi::YardimciYok(format!("kendi yolum okunamadı: {e}")))?;
    let dizin = ben
        .parent()
        .ok_or_else(|| OlcumHatasi::YardimciYok("üst dizin yok".into()))?;
    let yol = dizin.join(YARDIMCI);
    if !yol.exists() {
        return Err(OlcumHatasi::YardimciYok(yol.to_string_lossy().into_owned()));
    }
    Ok(yol)
}

/// Geçici özet dosyasının yolu.
pub fn gecici_cikti() -> PathBuf {
    std::env::temp_dir().join(format!(
        "muifly-kare-{}-{}.json",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ))
}

#[cfg(windows)]
pub fn yukselterek_olc(pid: u32, saniye: u64) -> Result<Sonuc, OlcumHatasi> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, ERROR_CANCELLED, HANDLE, WAIT_OBJECT_0};
    use windows::Win32::System::Threading::WaitForSingleObject;
    use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
    use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;

    fn utf16(s: &std::ffi::OsStr) -> Vec<u16> {
        s.encode_wide().chain(std::iter::once(0)).collect()
    }

    let yardimci = yardimci_yolu()?;
    let cikti = gecici_cikti();
    let istek = Istek::yeni(pid, saniye, cikti.clone());

    let dosya = utf16(yardimci.as_os_str());
    let fiil = utf16(std::ffi::OsStr::new("runas"));
    let parametre = utf16(std::ffi::OsStr::new(&istek.argumanlar().join(" ")));

    let mut bilgi = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: PCWSTR(fiil.as_ptr()),
        lpFile: PCWSTR(dosya.as_ptr()),
        lpParameters: PCWSTR(parametre.as_ptr()),
        nShow: SW_HIDE.0,
        ..Default::default()
    };

    // SAFETY: yapı dolduruldu, işaret ettiği tamponlar bu çağrı boyunca yaşıyor.
    let baslatildi = unsafe { ShellExecuteExW(&mut bilgi) };
    if let Err(e) = baslatildi {
        let _ = std::fs::remove_file(&cikti);
        // UAC reddi hata değil, kullanıcının kararı.
        if e.code().0 as u32 & 0xFFFF == ERROR_CANCELLED.0 {
            return Err(OlcumHatasi::KullaniciReddetti);
        }
        return Err(OlcumHatasi::YardimciYok(e.to_string()));
    }

    let sure_ms = (istek.saniye as u32 + 30) * 1000;
    // SAFETY: `hProcess` `SEE_MASK_NOCLOSEPROCESS` sayesinde geçerli.
    let bekleme = unsafe { WaitForSingleObject(bilgi.hProcess, sure_ms) };
    // SAFETY: kolu bir kez kapatıyoruz.
    unsafe {
        let _ = CloseHandle(HANDLE(bilgi.hProcess.0));
    }
    if bekleme != WAIT_OBJECT_0 {
        let _ = std::fs::remove_file(&cikti);
        return Err(OlcumHatasi::ZamanAsimi);
    }

    let sonuc = oku_ve_sil(&cikti)?;
    if sonuc.kare_sayisi == 0 {
        return Err(OlcumHatasi::KareYok);
    }
    Ok(sonuc)
}

#[cfg(not(windows))]
pub fn yukselterek_olc(_pid: u32, _saniye: u64) -> Result<Sonuc, OlcumHatasi> {
    Err(OlcumHatasi::Engel(Engel::Desteklenmiyor))
}

/// Özeti okur ve geçici dosyayı **her durumda** siler.
///
/// Silme `Result`'a bağlanmıyor: hata yolunda da dosya kalmamalı. Geride
/// bırakılan geçici dosya, programın kendi "arkada iz bırakma" duruşuyla
/// çelişirdi.
fn oku_ve_sil(yol: &Path) -> Result<Sonuc, OlcumHatasi> {
    let ham = std::fs::read_to_string(yol);
    let _ = std::fs::remove_file(yol);
    let ham = ham.map_err(|e| OlcumHatasi::OzetOkunamadi(e.to_string()))?;
    serde_json::from_str(&ham).map_err(|e| OlcumHatasi::OzetOkunamadi(e.to_string()))
}

#[cfg(test)]
mod testler {
    use super::*;

    /// Yardımcı ikili `required-features` arkasında kalmalı.
    ///
    /// Kaldırılırsa normal `cargo build` yardımcıyı da üretiyor, `tauri
    /// build` onu `externalBin` sidecar'ının yanına ikinci kez kuruluma
    /// koyuyor ve **MSI hedefi kırılıyor** (WiX ICE30: aynı dosya iki farklı
    /// bileşenden). NSIS buna katlanıp üstüne yazdığı için sorun yalnızca
    /// MSI üretilirken görünüyordu — 0.3.0'a kadar hiç MSI çıkmamasının
    /// sebebi buydu.
    ///
    /// Bu test o düzeltmenin sessizce geri alınmasını engelliyor: sorun
    /// derleme zamanında değil, yalnızca paketleme gününde ortaya çıkardı.
    #[test]
    fn yardimci_ikili_ozellik_arkasinda() {
        let manifest = include_str!("../../Cargo.toml");
        let bin = manifest
            .split("[[bin]]")
            .nth(1)
            .expect("yardımcı için [[bin]] bölümü yok");
        assert!(
            bin.contains("name = \"muifly-olcum\""),
            "ilk [[bin]] bölümü yardımcıya ait değil"
        );
        assert!(
            bin.contains("required-features = [\"olcum-yardimcisi\"]"),
            "yardımcı ikilisi required-features arkasında değil — \
             bu kaldırılırsa MSI paketlemesi kırılır"
        );
    }

    #[test]
    fn arguman_gidip_geliyor() {
        let i = Istek::yeni(1234, 20, PathBuf::from(r"C:\gecici\ozet.json"));
        let geri = Istek::ayristir(i.argumanlar()).expect("ayrıştırılmalı");
        assert_eq!(i, geri);
    }

    #[test]
    fn bosluklu_yol_da_gidip_geliyor() {
        // Kullanıcı adında boşluk olması Windows'ta kural dışı değil.
        let i = Istek::yeni(9, 10, PathBuf::from(r"C:\Program Files\a b\ozet.json"));
        let geri = Istek::ayristir(i.argumanlar()).expect("ayrıştırılmalı");
        assert_eq!(i.cikti, geri.cikti);
    }

    #[test]
    fn sure_sinirlari_zorlaniyor() {
        let yol = PathBuf::from("x.json");
        assert_eq!(Istek::yeni(1, 0, yol.clone()).saniye, EN_KISA_SANIYE);
        assert_eq!(
            Istek::yeni(1, 9_999, yol.clone()).saniye,
            EN_UZUN_SANIYE,
            "yükseltilmiş süreç süresiz açık kalmamalı"
        );
    }

    #[test]
    fn eksik_arguman_hata() {
        assert!(Istek::ayristir(vec!["--pid".into(), "5".into()]).is_err());
        assert!(Istek::ayristir(Vec::new()).is_err());
        assert!(Istek::ayristir(vec!["--saçma".into(), "5".into()]).is_err());
    }

    #[test]
    fn sifir_pid_reddediliyor() {
        // PID 0 "System Idle Process"; ölçüm hedefi olamaz ve süzgeç olarak
        // verilirse ETW tarafında beklenmedik davranır.
        let args = vec![
            "--pid".into(),
            "0".into(),
            "--saniye".into(),
            "10".into(),
            "--cikti".into(),
            "x.json".into(),
        ];
        assert!(Istek::ayristir(args).is_err());
    }

    #[test]
    fn sonuc_json_gidip_geliyor() {
        let s = Sonuc {
            ozet: None,
            kare_sayisi: 0,
        };
        let m = serde_json::to_string(&s).unwrap();
        let g: Sonuc = serde_json::from_str(&m).unwrap();
        assert_eq!(g.kare_sayisi, 0);
        assert!(g.ozet.is_none());
    }

    #[test]
    fn ozet_dosyasi_okununca_siliniyor() {
        let dizin = tempfile::tempdir().unwrap();
        let yol = dizin.path().join("ozet.json");
        sonucu_yaz(
            &yol,
            &Sonuc {
                ozet: None,
                kare_sayisi: 7,
            },
        )
        .unwrap();
        assert!(yol.exists());

        let s = oku_ve_sil(&yol).unwrap();
        assert_eq!(s.kare_sayisi, 7);
        assert!(!yol.exists(), "geçici dosya arkada kalmamalı");
    }

    #[test]
    fn bozuk_ozet_dosyasi_da_siliniyor() {
        let dizin = tempfile::tempdir().unwrap();
        let yol = dizin.path().join("bozuk.json");
        std::fs::write(&yol, "{ bu json değil").unwrap();

        assert!(oku_ve_sil(&yol).is_err());
        assert!(!yol.exists(), "hata yolunda da dosya kalmamalı");
    }

    #[test]
    fn hata_metinleri_sayisal_vaat_icermiyor() {
        for h in [
            OlcumHatasi::KullaniciReddetti,
            OlcumHatasi::YardimciYok("x".into()),
            OlcumHatasi::Engel(Engel::YetkiYok),
            OlcumHatasi::ZamanAsimi,
            OlcumHatasi::OzetOkunamadi("x".into()),
            OlcumHatasi::KareYok,
        ] {
            let m = h.to_string().to_lowercase();
            for yasak in ["fps", "daha hızlı", "kazan", "%"] {
                assert!(!m.contains(yasak), "yasak ifade '{yasak}': {m}");
            }
        }
    }

    #[test]
    fn uac_reddi_hata_gibi_yazilmiyor() {
        // Kullanıcı vazgeçtiyse ona "başarısız oldu" denmemeli; hiçbir şey
        // değişmediği açıkça söylenmeli.
        let m = OlcumHatasi::KullaniciReddetti.to_string();
        assert!(m.contains("hiçbir şey değişmedi"), "gelen: {m}");
    }
}
