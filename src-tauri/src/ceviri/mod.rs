//! Ekran çevirisi (Faz 5) — yakalamadan bağımsız temel katman.
//!
//! ## Bu modülde ne VAR
//!
//! Faz 5'in ekran yakalamaya ihtiyaç duymayan parçaları:
//!
//! - `onisleme` — çeviriye girmeden önce metnin düzeltilmesi ve OCR'ın
//!   bozduğundan şüphelenilen yerlerin **işaretlenmesi** (karar #28, #29)
//! - `sozluk` — oyuna özel terim sözlüğü; çeviri sırasında terimlerin
//!   korunması ve geri konması (karar #22, #29 zaaf 2)
//! - `bellek` — çeviri belleği: oyun başına bir JSON, okunabilir,
//!   düzenlenebilir, silinebilir (karar #22)
//! - `ocr_dil` — kaynak dilin OCR paketi kurulu mu (karar #28 ürün gereği)
//!
//! ## Bu modülde ne YOK, neden
//!
//! Ekran yakalama, overlay penceresi, kısayol ve çeviri modelinin kendisi
//! **bilerek yok**. `ROADMAP.md` → Faz 5: "Bu faz Faz 3'ten önce başlamaz",
//! gerekçesi karar #22'de — yakalama katmanı Faz 3'te geliyor, daha önce
//! yazılırsa aynı iş ikinci kez yazılır. Buradaki dört parçanın hiçbirinin
//! yakalamayla işi yok; Faz 3 geldiğinde yeniden yazılmaları gerekmez.
//!
//! ## Bu modül sistemde hiçbir şey değiştirmez
//!
//! CLAUDE.md'nin değişmez kuralı ("sistemde bir şey değiştiren her yol
//! `state::Motor` üzerinden geçer") burada uygulanmıyor, çünkü kural burada
//! **tetiklenmiyor**: `monitor` gibi bu modül de yalnızca okur ve kendi
//! dosyasına yazar. Geri alınacak bir sistem değişikliği üretmiyor, o yüzden
//! deftere yazacak bir şeyi de yok.
//!
//! ## Motor'a bilerek bağlanmadı
//!
//! Karar #22'nin açık sorusu — "Muifly modülü mü, ayrı bir Mui ürünü mü" —
//! hâlâ cevapsız ve Faz 3'ten sonra bakılacak. Modülün `state::Motor`'a
//! hiçbir bağı yok; iki cevabın da bedeli düşük kalsın diye. Bu bir tembellik
//! değil, kararı ucuz tutma tercihi.

pub mod bellek;
pub mod ocr_dil;
pub mod onisleme;
pub mod sozluk;

pub use bellek::{CeviriBellegi, Kayit, Koken};
pub use ocr_dil::{Dil, DilDurumu};
pub use onisleme::{Hazirlik, Uyari};
pub use sozluk::Yerlesim;

/// Bir metnin çeviri belleğinde anahtar olarak kullanılacak hali.
///
/// OCR aynı diyalog kutusunu iki kez okuduğunda çıktı birebir aynı olmak
/// zorunda değil: satır sonundaki boşluk, iki kelime arasına giren fazladan
/// bir boşluk, baştaki girinti değişebilir. Bunlar farklı sayılırsa önbellek
/// isabet oranı düşer ve model gereksiz yere tekrar çalışır.
///
/// Küçük harfe indirme kasıtlı: `onisleme` zaten TAMAMI BÜYÜK HARF metni
/// küçültüyor (karar #29 zaaf 1), yani aynı cümle bir menüde büyük, bir
/// diyalogda düz harflerle görülebilir. Kaynak dil İngilizce olduğu için
/// `to_lowercase` burada güvenli.
///
/// Anahtar yalnızca ARAMA için; kaydın kendisinde metnin özgün hali duruyor.
pub fn anahtar(metin: &str) -> String {
    metin
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn anahtar_bosluk_farkini_yutuyor() {
        assert_eq!(
            anahtar("  The  bridge \n collapsed. "),
            anahtar("The bridge collapsed.")
        );
    }

    #[test]
    fn anahtar_buyuk_kucuk_harf_farkini_yutuyor() {
        assert_eq!(anahtar("MISSION FAILED"), anahtar("Mission failed"));
    }

    #[test]
    fn anahtar_farkli_metinleri_ayiriyor() {
        assert_ne!(anahtar("Open the door."), anahtar("Close the door."));
    }
}
