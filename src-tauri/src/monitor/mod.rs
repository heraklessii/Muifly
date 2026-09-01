//! İzleme: şeffaflık günlüğü + canlı ölçüm.
//!
//! Bu modül sistemde hiçbir şey DEĞİŞTİRMEZ, sadece okur ve kaydeder.
//! `system_boost` ve `network_boost` buraya yazar; arayüz buradan okur.

pub mod etw;
pub mod frames;
pub mod log;
pub mod metrics;
pub mod olcum;

pub use frames::{KareOzeti, KareTamponu};
pub use log::{Duzey, Gunluk, Kategori, Satir};
pub use metrics::{Karsilastirma, Ornek, Ornekleyici, Ozet, Tampon};

/// Örnek tamponunun kapasitesi.
///
/// Saniyede bir örnekle ~30 dakika: bir oyun oturumunun öncesi/sonrası
/// karşılaştırması için fazlasıyla yeterli, bellek maliyeti birkaç yüz KB.
pub const ORNEK_KAPASITESI: usize = 1800;
