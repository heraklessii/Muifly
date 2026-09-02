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

    /// Argümanların **tek bir komut satırı** hâli.
    ///
    /// `ShellExecuteEx` argüman dizisi değil tek bir dize alıyor; yardımcı
    /// ise onu `std::env::args()` ile, yani Windows'un kendi ayrıştırmasıyla
    /// geri çözüyor. Boşlukla birleştirmek bu yüzden yetmiyor: `%TEMP%`
    /// yolunda bir boşluk varsa — `C:\Users\Ada Lovelace\...`, Windows'ta
    /// kural dışı değil — `--cikti` yarıda kesiliyor, yardımcı argüman
    /// hatasıyla kapanıyor ve ölçüm hiç çalışmıyor. Belirtisi de sinsi:
    /// kullanıcı "özet okunamadı" görüyor ve sebebin kendi kullanıcı adında
    /// olduğu hiçbir yerde yazmıyor.
    ///
    /// Bu yüzden her argüman [`tirnakla`] ile kaçırılıyor.
    pub fn komut_satiri(&self) -> String {
        self.argumanlar()
            .iter()
            .map(|a| tirnakla(a))
            .collect::<Vec<_>>()
            .join(" ")
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

/// Bir argümanı Windows komut satırı kuralına göre kaçırır.
///
/// Kural `CommandLineToArgvW`'nin (ve dolayısıyla `std::env::args()`'ın)
/// tersi: ters bölü dizileri yalnızca bir tırnaktan hemen önce ikileniyor,
/// başka yerde olduğu gibi kalıyor. Ayrıntı gerçek, çünkü Windows yolları
/// `\` ile bitebiliyor (`...\Temp\`): ikilenmezse kapanış tırnağı kaçırılmış
/// sayılır ve argüman bir sonrakiyle birleşir.
fn tirnakla(arg: &str) -> String {
    if !arg.is_empty() && !arg.contains([' ', '\t', '"']) {
        return arg.to_string();
    }
    let mut cikti = String::with_capacity(arg.len() + 2);
    cikti.push('"');
    let mut ters_bolu = 0usize;
    for c in arg.chars() {
        match c {
            '\\' => {
                ters_bolu += 1;
                cikti.push(c);
            }
            '"' => {
                // Tırnaktan önceki ters bölüler ikileniyor, sonra tırnağın
                // kendisi kaçırılıyor.
                for _ in 0..ters_bolu {
                    cikti.push('\\');
                }
                ters_bolu = 0;
                cikti.push('\\');
                cikti.push('"');
            }
            _ => {
                ters_bolu = 0;
                cikti.push(c);
            }
        }
    }
    // Kapanış tırnağı da bir tırnak: öncesindeki ters bölüler ikilenmeli.
    for _ in 0..ters_bolu {
        cikti.push('\\');
    }
    cikti.push('"');
    cikti
}

/// Bir komut satırını `CommandLineToArgvW` kurallarıyla argümanlara böler.
///
/// Yalnızca testte kullanılıyor ve bilerek [`tirnakla`]'nın yanında duruyor:
/// kaçırmanın doğruluğu ancak "Windows bunu nasıl geri okuyor" sorusunun
/// cevabıyla ölçülebilir. İkisi ayrı dosyalarda olsaydı biri diğerinden
/// habersiz değişebilirdi.
#[cfg(test)]
fn argv_coz(satir: &str) -> Vec<String> {
    let mut cikti: Vec<String> = Vec::new();
    let mut simdiki = String::new();
    let mut tirnakta = false;
    let mut basladi = false;
    let mut ters_bolu = 0usize;

    for c in satir.chars() {
        match c {
            '\\' => {
                ters_bolu += 1;
                basladi = true;
            }
            '"' => {
                simdiki.push_str(&"\\".repeat(ters_bolu / 2));
                if ters_bolu % 2 == 1 {
                    simdiki.push('"');
                } else {
                    tirnakta = !tirnakta;
                }
                ters_bolu = 0;
                basladi = true;
            }
            ' ' | '\t' if !tirnakta => {
                simdiki.push_str(&"\\".repeat(ters_bolu));
                ters_bolu = 0;
                if basladi {
                    cikti.push(std::mem::take(&mut simdiki));
                    basladi = false;
                }
            }
            _ => {
                simdiki.push_str(&"\\".repeat(ters_bolu));
                ters_bolu = 0;
                simdiki.push(c);
                basladi = true;
            }
        }
    }
    simdiki.push_str(&"\\".repeat(ters_bolu));
    if basladi {
        cikti.push(simdiki);
    }
    cikti
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

/// Özet dosyasının klasör içindeki sabit adı.
const OZET_ADI: &str = "ozet.json";

/// Adın tahmin edilemez parçası.
///
/// `RandomState` anahtarını işletim sisteminden alıyor; burada bir karma
/// fonksiyonu olarak değil, yalnızca **öngörülemez bir sayı** kaynağı
/// olarak kullanılıyor. Kriptografik bir iddiası yok ve olması da
/// gerekmiyor — tek işi aşağıdaki klasör adının önceden bilinememesi.
fn rastgele() -> u64 {
    use std::hash::{BuildHasher, Hasher};
    let mut h = std::collections::hash_map::RandomState::new().build_hasher();
    h.write_u32(std::process::id());
    h.write_i64(chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
    h.finish()
}

/// Geçici özet dosyasını taşıyan, bize ait klasör. Düşerken siliniyor.
///
/// ## Neden ayrı bir klasör ve neden rastgele bir ad
///
/// Özeti **yükseltilmiş** yardımcı yazıyor, dosya ise kullanıcının kendi
/// `%TEMP%` klasöründe duruyor. Adı öngörülebilir olsaydı (eski hâli
/// `muifly-kare-<pid>-<zaman>.json` idi), aynı kullanıcı olarak çalışan
/// kötü niyetli bir süreç o adı önceden bir bağlantı noktası (junction)
/// olarak yaratıp yükseltilmiş yazmayı başka bir yere yönlendirebilirdi —
/// yani yönetici yetkisiyle dosya yazma. Buradaki iki önlem bunu kapatıyor:
///
/// 1. Ad tahmin edilemiyor.
/// 2. Klasör `create_dir` ile açılıyor: aynı adda bir şey **varsa** çağrı
///    hata veriyor, var olanın içine yazılmıyor.
///
/// Kalan sınır dürüstçe yazılsın: aynı kullanıcı olarak çalışan bir süreç
/// `%TEMP%` üzerinde tam yetkili. Bu yüzden dosya, yardımcı başlatılmadan
/// **önce** burada `create_new` ile açılıyor; yardımcının yaptığı tek şey
/// var olan bir dosyanın üstüne yazmak.
struct GeciciKlasor(PathBuf);

impl GeciciKlasor {
    fn ac() -> std::io::Result<Self> {
        for _ in 0..8 {
            let yol = std::env::temp_dir().join(format!(
                "muifly-kare-{:016x}{:016x}",
                rastgele(),
                rastgele()
            ));
            match std::fs::create_dir(&yol) {
                Ok(()) => return Ok(Self(yol)),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(e),
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "geçici klasör açılamadı",
        ))
    }

    fn ozet_yolu(&self) -> PathBuf {
        self.0.join(OZET_ADI)
    }
}

impl Drop for GeciciKlasor {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
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
    // Klasör bu kapsamda yaşıyor; hangi yoldan dönersek dönelim düşerken
    // kendini siliyor. Eski hâlde her erken dönüşte ayrı bir `remove_file`
    // vardı ve birini unutmak geride dosya bırakmak demekti.
    let klasor = GeciciKlasor::ac()
        .map_err(|e| OlcumHatasi::OzetOkunamadi(format!("geçici klasör açılamadı: {e}")))?;
    let cikti = klasor.ozet_yolu();
    // Dosyayı yardımcı değil biz açıyoruz (bkz. `GeciciKlasor`): yükseltilmiş
    // sürecin yaptığı tek şey var olan bir dosyanın üstüne yazmak.
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&cikti)
        .map_err(|e| OlcumHatasi::OzetOkunamadi(format!("geçici dosya açılamadı: {e}")))?;
    let istek = Istek::yeni(pid, saniye, cikti.clone());

    let dosya = utf16(yardimci.as_os_str());
    let fiil = utf16(std::ffi::OsStr::new("runas"));
    // Boşlukla birleştirmek değil, kaçırmak: `%TEMP%` yolunda bir boşluk
    // varsa `--cikti` yarıda kesilirdi (bkz. `Istek::komut_satiri`).
    let parametre = utf16(std::ffi::OsStr::new(&istek.komut_satiri()));

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

    /// Asıl sınav: yardımcıya giden şey argüman DİZİSİ değil, tek bir komut
    /// satırı. Üstteki test bunu göremiyordu ve gerçek hata tam oradaydı —
    /// `%TEMP%` yolunda bir boşluk varsa ölçüm hiç çalışmıyordu.
    #[test]
    fn bosluklu_yol_komut_satirindan_da_gidip_geliyor() {
        let i = Istek::yeni(
            9,
            10,
            PathBuf::from(r"C:\Users\Ada Lovelace\AppData\Local\Temp\muifly\ozet.json"),
        );
        let geri = Istek::ayristir(argv_coz(&i.komut_satiri())).expect("ayrıştırılmalı");
        assert_eq!(i, geri);
    }

    /// Windows yolları `\` ile bitebiliyor; kapanış tırnağından hemen önceki
    /// ters bölü ikilenmezse tırnak kaçırılmış sayılır ve argüman bir
    /// sonrakiyle birleşir.
    #[test]
    fn ters_bolu_ile_biten_yol_bozulmuyor() {
        let cozulen = argv_coz(&format!("{} {}", tirnakla(r"C:\a b\"), tirnakla("--sonraki")));
        assert_eq!(cozulen, vec![r"C:\a b\".to_string(), "--sonraki".into()]);
    }

    #[test]
    fn tirnaksiz_arguman_tirnaklanmiyor() {
        // Gereksiz tırnak bir hata değil ama komut satırını okunmaz yapıyor;
        // UAC istemi kullanıcıya bu satırı gösteriyor.
        assert_eq!(tirnakla("--pid"), "--pid");
        assert_eq!(tirnakla("1234"), "1234");
    }

    #[test]
    fn tirnakli_arguman_da_gidip_geliyor() {
        for ornek in [r#"a "b" c"#, r"C:\yol\", "boşluklu ad", r#"\"#, r#""""#] {
            assert_eq!(
                argv_coz(&tirnakla(ornek)),
                vec![ornek.to_string()],
                "kaçırma bozuk: {ornek}"
            );
        }
    }

    /// Yükseltilmiş yardımcının yazacağı dosyanın adı tahmin edilebilir
    /// olmamalı (bkz. `GeciciKlasor`): öngörülebilir bir ad, aynı kullanıcı
    /// olarak çalışan bir sürece yönetici yetkisiyle yazma imkânı verirdi.
    #[test]
    fn gecici_klasor_adi_ongorulemez_ve_tekil() {
        let a = GeciciKlasor::ac().expect("klasör açılmalı");
        let b = GeciciKlasor::ac().expect("klasör açılmalı");
        assert_ne!(a.0, b.0, "iki çağrı aynı adı verdi");
        assert!(a.0.is_dir() && b.0.is_dir());
        // Klasör adı süreç kimliğinden ya da saatten türetilebilir olmamalı.
        let ad = a.0.file_name().unwrap().to_string_lossy().into_owned();
        assert!(!ad.contains(&std::process::id().to_string()), "ad pid taşıyor: {ad}");

        let yol = a.0.clone();
        drop(a);
        assert!(!yol.exists(), "geçici klasör düşerken silinmedi");
    }

    /// Aynı adda bir şey varsa klasör açılmıyor — var olanın içine yazmak,
    /// önceden yerleştirilmiş bir bağlantı noktasını takip etmek olurdu.
    #[test]
    fn var_olan_klasorun_icine_yazilmiyor() {
        let k = GeciciKlasor::ac().expect("klasör açılmalı");
        assert!(
            std::fs::create_dir(&k.0).is_err(),
            "aynı ada ikinci kez create_dir başarılı oldu"
        );
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
