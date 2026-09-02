//! Ekran çevirisi (Faz 5).
//!
//! ```text
//! kisayol.rs     RegisterHotKey — kanca DEĞİL (tasarım ilkesi 3)
//! denetleyici.rs iş parçacığı: yakala → oku → çevir → yayınla
//! alan.rs        çevrilecek ekran parçası, oran olarak (profile yazılıyor)
//! ocr.rs         Windows.Media.Ocr (karar #28)
//! onisleme.rs    BÜYÜK HARF küçültme + OCR şüphelerinin işaretlenmesi
//! cumle.rs       çeviri birimlerine ayırma (karar #29 zaaf 3)
//! sozluk.rs      oyun terimlerinin çeviriden korunması (karar #29 zaaf 2)
//! akis.rs        boru hattının kararları — Windows'suz test edilebilir
//! cevirici.rs    ONNX Runtime üzerinde greedy çözümleme (karar #29)
//! model.rs       modelin dosyaları: nerede, kurulu mu, nasıl iniyor
//! indirme.rs     WinHTTP indirmesi
//! sha256.rs      indirilenin doğrulanması
//! bellek.rs      oyun başına JSON: çeviri belleği + terim sözlüğü (karar #22)
//! ocr_dil.rs     kaynak dilin OCR paketi kurulu mu (karar #28)
//! ```
//!
//! ## Faz sırası: bu modül iki turda yazıldı
//!
//! Karar #30 Faz 5'in **yakalamaya dokunmayan** dört parçasını (`onisleme`,
//! `sozluk`, `bellek`, `ocr_dil`) Faz 3'ten önce yazmıştı; uygulanan ayrım
//! testi tek soruydu: *"Faz 3'ün yakalama katmanı geldiğinde bu kod yeniden
//! yazılır mı?"* Cevabı hayır olanlar yazıldı, evet olanlar bekletildi.
//!
//! Karar #37 kalanını yazdı: yakalama, OCR, kısayol, overlay, model indirme
//! ve çıkarım. Ayrım testi tuttu — dört eski parçanın hiçbiri yeniden
//! yazılmadı, dördü de olduğu gibi kullanılıyor.
//!
//! ## Modelin ölçülmüş üç zaafı, koddaki üç karşılığı
//!
//! Karar #29 çeviri kalitesini ölçtü ve üç zaafı adıyla koydu. Hiçbiri
//! "model daha iyi olsun" diye bırakılmadı; her birinin bir karşılığı var:
//!
//! | Zaaf | Karşılığı |
//! |---|---|
//! | TAMAMI BÜYÜK HARF girdi çöküyor | `onisleme` cümle düzenine indiriyor |
//! | Oyun terimleri yanlış çevriliyor | `sozluk` terimi modelden gizliyor |
//! | Cümle sessizce düşüyor | `cumle` her cümleyi ayrı gönderiyor, `akis` düşeni gösteriyor |
//!
//! ## Sistemde bir şey değiştirmiyor
//!
//! `monitor` ve `scaling` gibi bu modül de yalnızca okuyor ve kendi
//! dosyalarına yazıyor (`%APPDATA%\Muifly\ceviri`). CLAUDE.md'nin değişmez
//! kuralı ("sistemde bir şey değiştiren her yol `state::Motor`dan geçer ve
//! hem deftere hem günlüğe yazar") burada deftere değil yalnızca günlüğe
//! yazmakla karşılanıyor: geri alınacak bir sistem değişikliği üretilmiyor.
//! Kaydedilen tek sistem kaynağı klavye kısayolu ve o da sürecin ömrüyle
//! sınırlı.

pub mod akis;
pub mod alan;
pub mod bellek;
pub mod cevirici;
pub mod cumle;
pub mod denetleyici;
pub mod indirme;
pub mod kisayol;
pub mod model;
pub mod ocr;
pub mod ocr_dil;
pub mod onisleme;
pub mod sha256;
pub mod sozluk;

pub use akis::{Birim, Sonuc};
pub use alan::Alan;
pub use bellek::{CeviriBellegi, Kayit, Koken};
pub use denetleyici::{Asama, Ceviri, CeviriDurumu, Yapilandirma};
pub use model::ModelDurumu;
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


/// Alan seçici penceresinin göstereceği ekran görüntüsü.
///
/// **Küçültülmüş**: alan oran olarak saklandığı için (`alan::Alan`) seçimin
/// doğruluğu görüntünün çözünürlüğüne bağlı değil, ama `data:` adresi
/// arayüze bir dize olarak geçiyor. 4K bir kare, sıkıştırmasız PNG ve base64
/// birleşince onlarca megabaytlık bir dize demek olurdu — bir önizleme için
/// ödenecek bir bedel değil.
pub fn ekran_goruntusu(ekran: usize) -> crate::error::Result<String> {
    let kare = denetleyici::yakala(ekran)?;
    let kucuk = kucult(&kare, ONIZLEME_EN_FAZLA_GENISLIK);
    // `library::png` RGBA bekliyor, yakalama BGRA veriyor.
    let mut rgba = kucuk.pikseller.clone();
    for p in rgba.chunks_exact_mut(4) {
        p.swap(0, 2);
    }
    let png = crate::library::png::kodla(kucuk.genislik, kucuk.yukseklik, &rgba)
        .ok_or_else(|| crate::error::Error::Ceviri("ekran görüntüsü kodlanamadı".into()))?;
    Ok(crate::library::veri_adresi("image/png", &png))
}

/// Önizlemenin en fazla genişliği (piksel).
pub const ONIZLEME_EN_FAZLA_GENISLIK: u32 = 1280;

/// Görüntüyü tam sayı katına indirir (kutu ortalaması).
///
/// Ortalama, en yakın komşuya tercih edildi: önizlemede okunacak şey yazı ve
/// en yakın komşu ince yazıyı kırpıp okunmaz hâle getiriyor. Kullanıcı
/// alanı, ekranda gördüğü yazıya bakarak seçiyor.
///
/// Bu, `scaling::algoritma`nın işi DEĞİL ve oraya bağlanmadı: orası ölçüm
/// altındaki bir referans (karar #32) ve önizleme küçültmesi için oraya
/// çalışma zamanı bir çağıran eklemek, o modülün kimliğini değiştirirdi.
fn kucult(
    kaynak: &crate::scaling::algoritma::Goruntu,
    en_fazla: u32,
) -> crate::scaling::algoritma::Goruntu {
    if kaynak.genislik <= en_fazla || en_fazla == 0 {
        return kaynak.clone();
    }
    let kat = (kaynak.genislik as f32 / en_fazla as f32).ceil() as u32;
    let kat = kat.max(1);
    let g = (kaynak.genislik / kat).max(1);
    let y = (kaynak.yukseklik / kat).max(1);

    let mut hedef = crate::scaling::algoritma::Goruntu::yeni(g, y);
    let bolen = kat * kat;
    for hy in 0..y as usize {
        for hx in 0..g as usize {
            let mut toplam = [0u32; 4];
            for ky in 0..kat as usize {
                for kx in 0..kat as usize {
                    let sx = hx * kat as usize + kx;
                    let sy = hy * kat as usize + ky;
                    let i = (sy * kaynak.genislik as usize + sx) * 4;
                    for (k, t) in toplam.iter_mut().enumerate() {
                        *t += kaynak.pikseller[i + k] as u32;
                    }
                }
            }
            let i = (hy * g as usize + hx) * 4;
            for (k, t) in toplam.iter().enumerate() {
                hedef.pikseller[i + k] = (t / bolen) as u8;
            }
        }
    }
    hedef
}

#[cfg(test)]
mod testler {
    use super::*;


    #[test]
    fn kucultme_orani_koruyor() {
        let g = crate::scaling::algoritma::Goruntu::yeni(2560, 1440);
        let k = kucult(&g, ONIZLEME_EN_FAZLA_GENISLIK);
        assert!(k.genislik <= ONIZLEME_EN_FAZLA_GENISLIK);
        assert!(k.gecerli());
        // Kat tam sayı: 2560/1280 = 2 → 1280x720.
        assert_eq!((k.genislik, k.yukseklik), (1280, 720));
    }

    #[test]
    fn kucuk_goruntu_kucultulmuyor() {
        let g = crate::scaling::algoritma::Goruntu::yeni(800, 600);
        let k = kucult(&g, ONIZLEME_EN_FAZLA_GENISLIK);
        assert_eq!((k.genislik, k.yukseklik), (800, 600));
    }

    #[test]
    fn kucultme_ortalama_aliyor() {
        // 2x2'lik bir blok: iki siyah iki beyaz → ortalama gri.
        let mut g = crate::scaling::algoritma::Goruntu::yeni(4, 2);
        for x in 0..4usize {
            let deger = if x % 2 == 0 { 0u8 } else { 255u8 };
            for y in 0..2usize {
                let i = (y * 4 + x) * 4;
                g.pikseller[i] = deger;
                g.pikseller[i + 1] = deger;
                g.pikseller[i + 2] = deger;
            }
        }
        let k = kucult(&g, 2);
        assert_eq!((k.genislik, k.yukseklik), (2, 1));
        assert_eq!(k.pikseller[0], 127);
    }

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
