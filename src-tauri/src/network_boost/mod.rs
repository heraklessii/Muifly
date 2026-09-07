//! Ağ optimizasyonu (Faz 2).
//!
//! Modülün duruşu, ürünün en çok abartılan alanında en az abartan olmak:
//!
//! | Rakiplerin yaptığı | Muifly'ın yaptığı |
//! |---|---|
//! | Sabit VPN tüneli üzerinden trafiği yönlendirmek | Yönlendirme yapmıyor; yolu ölçüp gösteriyor |
//! | "Ping'i X ms düşürür" | Ölçümü gösteriyor, iddia etmiyor |
//! | DNS'i sessizce değiştirmek | Ölçüp öneriyor, değiştirmeyi kullanıcıya bırakıyor |
//! | Aylık abonelik | Ücretsiz ve açık kaynak |
//!
//! Bu, özelliği zayıflatmak değil: WTFast/ExitLag'in tünelinin ping'i
//! **kötüleştirdiği** durumlar yaygın ve kullanıcı bunu ölçemediği için fark
//! etmiyor. Ölçümü kullanıcıya vermek, tünel satmaktan dürüst bir teklif.

pub mod dns;
pub mod latency;
pub mod qos;
pub mod tcp;

pub use dns::DnsSonucu;
pub use latency::{YolDugumu, YolSonucu};
pub use tcp::TcpDurumu;

/// Ağ trafiğini tünelleme desteği — hiçbir zaman eklenmeyecek.
///
/// Bir VPN tüneli, kullanıcının tüm trafiğini bizim sunucularımızdan
/// geçirmek demek: sunucu maliyeti (abonelik modeline zorlar), gizlilik yükü
/// (trafiği görebilecek konuma geçeriz) ve çoğu durumda daha kötü bir ping.
/// Üçü de bu ürünün konumlandırmasıyla çelişiyor (`docs/PRODUCT_VISION.md`).
pub fn tunel_destegi() -> bool {
    false
}

#[cfg(test)]
mod testler {
    #[test]
    fn tunel_destegi_yok() {
        // Ürün duruşunu koruyan test: açılırsa, açan kişi bu testi de
        // değiştirmek ve kararı görünür kılmak zorunda.
        assert!(!super::tunel_destegi());
    }
}
