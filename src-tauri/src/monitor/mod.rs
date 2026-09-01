//! İzleme: şeffaflık günlüğü + canlı ölçüm.
//!
//! Bu modül sistemde hiçbir şey DEĞİŞTİRMEZ, sadece okur ve kaydeder.
//! `system_boost` ve `network_boost` buraya yazar; arayüz buradan okur.

pub mod log;
pub mod metrics;

pub use log::{Duzey, Gunluk, Kategori, Satir};
pub use metrics::{Karsilastirma, Ornek, Ornekleyici, Ozet, Tampon};

/// Örnek tamponunun kapasitesi.
///
/// Saniyede bir örnekle ~30 dakika: bir oyun oturumunun öncesi/sonrası
/// karşılaştırması için fazlasıyla yeterli, bellek maliyeti birkaç yüz KB.
pub const ORNEK_KAPASITESI: usize = 1800;

/// FPS ölçümü — Faz 2'ye ertelendi.
///
/// Doğru FPS okumanın hook'suz yolu ETW (Event Tracing for Windows) tabanlı,
/// PresentMon'un kullandığı yaklaşım. Hook'lu alternatif `DESIGN_PRINCIPLES.md`
/// madde 3 ile çelişiyor, bu yüzden ETW yolu araştırılmadan bu fonksiyon
/// implemente edilmeyecek. Bkz. `docs/RISKS.md`.
///
/// Şu an CPU/bellek/gecikme ölçülüyor; FPS yerine bunlar gösteriliyor ve
/// arayüz FPS alanını "henüz ölçülmüyor" olarak işaretliyor — sahte bir sayı
/// göstermektense boş bırakmak doğru.
pub fn fps_destegi_var() -> bool {
    false
}
