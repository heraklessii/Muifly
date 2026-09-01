//! Tek hata tipi.
//!
//! Tauri komutları `Result<T, Error>` döndürüyor ve `Error` doğrudan
//! serileştiriliyor: arayüz hatayı metin olarak alıyor. Ayrı bir "komut hatası"
//! tipi yok, çünkü ikinci bir tip her komutta bir dönüştürme adımı demek ve
//! hata mesajının kullanıcıya ulaşana kadar bir yerde yutulma riskini artırır.

use std::fmt;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// PID artık yaşamıyor ya da erişilemiyor. Suspend/resume döngüsünde
    /// beklenen bir durum: kullanıcı oyun açıkken Discord'u kapatabilir.
    #[error("süreç bulunamadı (pid {0})")]
    ProcessNotFound(u32),

    /// PID yeniden kullanılmış: defterdeki ad ile şu anki ad tutmuyor.
    /// Bu durumda geri alma UYGULANMAZ — yanlış sürece dokunmaktansa
    /// defteri düşürmek doğru.
    #[error("pid {pid} artık '{expected}' değil ('{actual}') — geri alma atlandı")]
    ProcessIdentityMismatch {
        pid: u32,
        expected: String,
        actual: String,
    },

    /// Yönetici yetkisi gerekiyor. Arayüz bunu görünce UAC yükseltmesi
    /// öneriyor ve NE İÇİN gerektiğini yazıyor (tasarım ilkesi 5).
    #[error("bu işlem yönetici yetkisi istiyor: {0}")]
    NeedsElevation(&'static str),

    #[error("Windows API hatası ({islem}): {kod}")]
    Windows { islem: &'static str, kod: String },

    #[error("profil bulunamadı: {0}")]
    ProfileNotFound(String),

    #[error("profil geçersiz: {0}")]
    ProfileInvalid(String),

    #[error("ağ ölçümü başarısız: {0}")]
    Network(String),

    #[error("dosya hatası: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON hatası: {0}")]
    Json(#[from] serde_json::Error),

    /// Gömülü üçüncü taraf bildirim dosyasıyla ilgili bir sorun.
    /// Kullanıcının düzeltebileceği bir şey değil — bir derleme hatasıdır —
    /// ama sessizce boş bir ekran göstermektense söylenmesi doğru.
    #[error("üçüncü taraf bildirimleri okunamadı: {0}")]
    UcuncuTaraf(String),

    /// Demo ikilisinde kapalı bir özellik istendi.
    ///
    /// Mesaj kullanıcıya gösteriliyor: neyin kapalı olduğu söyleniyor, baskı
    /// kurulmuyor (`docs/DISTRIBUTION.md` — nag ekranı yok).
    #[error("{0} demo sürümde kapalı")]
    DemoKisiti(&'static str),

    /// Windows dışı bir hedefte derlenen sistem çağrıları buraya düşüyor.
    /// Ürün Windows'a özel; bu varyant yalnızca kodun başka bir platformda
    /// da derlenip saf mantık testlerinin koşabilmesi için var.
    #[error("bu platformda desteklenmiyor: {0}")]
    Unsupported(&'static str),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Arayüze hata **metni** gidiyor, yapı değil.
///
/// Sebep: hata mesajları kullanıcıya gösterilecek Türkçe cümleler; arayüzde
/// ikinci kez cümle kurmak iki yerde metin bakımı demek olurdu.
impl serde::Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

/// `windows` crate'inin hatasını taşınabilir bir dizeye çevirir.
///
/// `windows::core::Error` doğrudan saklanmıyor: tip Windows dışı hedefte yok
/// ve `Error` enum'ının tamamı cfg'lenmek zorunda kalırdı.
#[cfg(windows)]
pub fn win(islem: &'static str, e: windows::core::Error) -> Error {
    // ERROR_ACCESS_DENIED (5) çoğu zaman "yükseltilmiş süreç" demek, "bozuk"
    // değil. Kullanıcıya doğru öneriyi verebilmek için ayrıştırılıyor.
    if e.code().0 as u32 == 0x8007_0005 {
        return Error::NeedsElevation(islem);
    }
    Error::Windows {
        islem,
        kod: format!("{e}"),
    }
}

/// Hata zincirini tek satıra indirger — log satırları için.
pub fn tek_satir(e: &dyn fmt::Display) -> String {
    e.to_string().replace('\n', " ")
}
