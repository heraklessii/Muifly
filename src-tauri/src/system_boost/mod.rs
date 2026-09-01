//! Sistem optimizasyonu (Faz 1).
//!
//! Modülün tamamı iki kurala uyuyor:
//!
//! 1. Sistemde bir şey değiştiren her fonksiyon bir `Undo` döndürür.
//! 2. Oyun sürecine hiçbir şey enjekte edilmez; yalnızca resmi Windows
//!    API'leri üzerinden dışarıdan ayar yapılır.
//!
//! Orkestrasyon burada değil `state.rs`'te: bu dosyadaki alt modüller tek tek
//! işleri yapıyor, "oyun algılandı, şu profili uygula" akışını motor kuruyor.

pub mod detect;
pub mod power;
pub mod priority;
pub mod startup;
pub mod suspend;

pub use detect::{OndekiPencere, Surec};
pub use power::GucPlani;
pub use priority::{CpuTopolojisi, Oncelik};

/// Bellek/standby list temizleme — bilinçli olarak yok.
///
/// `docs/RISKS.md`: agresif standby list temizleme, temizleme anında bir
/// performans düşüşü (yeniden yükleme maliyeti) yaratıyor ve ölçülebilir bir
/// kazanç sağladığı gösterilemiyor. "Bellek temizlendi, X MB boşaldı" ekranı
/// rakiplerin en çok kullandığı placebo göstergesi; Muifly'ın konumlandırması
/// tam olarak buna karşı (`docs/PRODUCT_VISION.md`).
///
/// Ölçülebilir bir fayda gösterilirse eklenir, o zamana kadar yok.
pub fn bellek_temizleme_destegi() -> bool {
    false
}

/// MMCSS (`AvSetMmThreadCharacteristics`) — Faz 2'ye ertelendi.
///
/// MMCSS bir sürecin KENDİ thread'ini kaydettiği bir mekanizma; dışarıdan
/// başka bir sürecin thread'ini "Games" görevine bağlamanın desteklenen bir
/// yolu yok. Oyunun içine kod koymadan yapılamıyor ve oyunun içine kod
/// koymak tasarım ilkesi 3'e takılıyor.
///
/// Muifly kendi ölçüm thread'i için MMCSS kullanabilir (ölçümün düzenli
/// aralıkla koşması için) — bu, oyunla ilgisi olmayan ayrı bir konu ve
/// Faz 2'de ölçüm hassasiyeti sorun olursa değerlendirilecek.
pub fn mmcss_destegi() -> bool {
    false
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn placebo_ozellikler_kapali() {
        // Bu iki test bir ürün duruşunu koruyor. Biri açılırsa, açan kişi
        // testi de değiştirmek zorunda kalır ve karar görünür olur.
        assert!(!bellek_temizleme_destegi());
        assert!(!mmcss_destegi());
    }
}
