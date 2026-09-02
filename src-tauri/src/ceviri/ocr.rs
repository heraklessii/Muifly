//! Yakalanan kareden yazının okunması — `Windows.Media.Ocr`.
//!
//! ## Neden Windows'un kendi motoru
//!
//! Karar #28 bu soruyu ölçtü: sentetik oyun metni külliyatında karakter
//! benzerliği %98,3, bölge boyutundaki bir şeritte 14-19 ms. Karar #1 ile de
//! tutarlı — OCR tarafında indirilecek ya da gömülecek model **yok**,
//! motor Windows 10+'ta hazır ve `windows` kasası zaten bağımlılığımız.
//! İkiliye eklediği bayt sıfır.
//!
//! ## Sınırları adlı adınca
//!
//! Karar #28 iki ciddi hata sınıfı buldu ve ikisi de burada kapatılmıyor,
//! **işaretleniyor**: harf aralıklı büyük harf menü metni kelime sınırını
//! kaybediyor (`QUIT TO DESKTOP` → `QUITTODESKTOP`), kalabalık zemin
//! üstünde konturlu metinde noktalama karışıyor (`collapsed.` →
//! `collapsed]`). İşaretleme `super::onisleme`nin işi; burası ham metni
//! olduğu gibi veriyor.
//!
//! Külliyat sentetikti ve bu bilinen bir sınır: gerçek oyun ekran
//! görüntüleriyle tekrar ölçülmesi `docs/tasks.md`de duruyor.
//!
//! ## Satırlar korunuyor
//!
//! `OcrResult::Text()` satırları boşlukla birleştiriyor. Burada
//! `Lines()` okunup satır sonları korunuyor, çünkü iki ayrı arayüz öğesi
//! (başlık ve gövde, iki menü satırı) ayrı satırlar hâlinde geliyor ve
//! `super::cumle` satır sonunu bir sınır olarak kullanıyor. Birleştirilmiş
//! bir metin, modele hiç var olmamış bir cümle verirdi.

use serde::Serialize;

use crate::error::Result;

/// Bir okumanın sonucu.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Okuma {
    /// Satır sonları korunmuş ham metin.
    pub metin: String,
    pub satir_sayisi: usize,
    /// Okumanın süresi. Arayüz gösteriyor: karar #22'nin "çeviri isteği
    /// oyunun akışını kesmiyor" kabul kriteri ancak ölçülürse doğrulanır.
    pub sure_ms: u64,
}

#[cfg(windows)]
pub use win::{apartman_baslat, oku};

#[cfg(windows)]
mod win {
    use super::*;
    use crate::error::Error;
    use crate::scaling::algoritma::Goruntu;
    use windows::core::HSTRING;
    use windows::Globalization::Language;
    use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::DataWriter;
    use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

    /// WinRT apartmanını bu iş parçacığı için başlatır.
    ///
    /// Çeviri kendi iş parçacığında koşuyor ve WinRT çağrıları apartman
    /// başlatılmamış bir iş parçacığında `CO_E_NOTINITIALIZED` ile
    /// dönüyor. Zaten başlatılmışsa dönen hata **yutuluyor**: apartmanın
    /// başka bir kipte açılmış olması bizim için bir sorun değil.
    pub fn apartman_baslat() {
        // SAFETY: iş parçacığının ömrü boyunca bir kez, başka apartman
        // açılmadan çağrılıyor.
        unsafe {
            let _ = RoInitialize(RO_INIT_MULTITHREADED);
        }
    }

    fn winrt(ne: &'static str, e: windows::core::Error) -> Error {
        Error::Ceviri(format!("{ne}: {e}"))
    }

    /// Görüntüdeki yazıyı okur.
    ///
    /// `dil` BCP-47 etiketi (`en-US`). Paketi kurulu değilse burada değil
    /// `super::ocr_dil`de anlaşılıyor; buradaki hata metni yine de
    /// kullanıcıya ne olduğunu söylüyor.
    pub fn oku(goruntu: &Goruntu, dil: &str) -> Result<Okuma> {
        if !goruntu.gecerli() {
            return Err(Error::Ceviri("okunacak görüntü boş".into()));
        }
        let baslangic = std::time::Instant::now();

        let dil_nesnesi = Language::CreateLanguage(&HSTRING::from(dil))
            .map_err(|e| winrt("dil tanınmadı", e))?;
        let motor = OcrEngine::TryCreateFromLanguage(&dil_nesnesi).map_err(|_| {
            Error::Ceviri(format!(
                "'{dil}' için OCR motoru açılamadı — dil paketi kurulu olmayabilir"
            ))
        })?;

        if let Ok(en_fazla) = OcrEngine::MaxImageDimension() {
            if goruntu.genislik > en_fazla || goruntu.yukseklik > en_fazla {
                return Err(Error::Ceviri(format!(
                    "seçilen alan OCR motorunun sınırından büyük (en fazla {en_fazla} piksel)"
                )));
            }
        }

        let bitmap = bitmap_kur(goruntu)?;
        let sonuc = motor
            .RecognizeAsync(&bitmap)
            .map_err(|e| winrt("okuma başlatılamadı", e))?
            .join()
            .map_err(|e| winrt("okuma tamamlanamadı", e))?;

        // Satır sonları korunuyor; gerekçe modül belgesinde.
        let mut satirlar: Vec<String> = Vec::new();
        if let Ok(liste) = sonuc.Lines() {
            for satir in liste {
                if let Ok(m) = satir.Text() {
                    let m = m.to_string();
                    if !m.trim().is_empty() {
                        satirlar.push(m);
                    }
                }
            }
        }

        Ok(Okuma {
            satir_sayisi: satirlar.len(),
            metin: satirlar.join("\n"),
            sure_ms: baslangic.elapsed().as_millis() as u64,
        })
    }

    /// BGRA piksellerden `SoftwareBitmap` üretir.
    ///
    /// `DataWriter` yolu seçildi: alternatifi `IMemoryBufferByteAccess` COM
    /// arayüzünü elle çağırmak, yani ham işaretçiyle yazmak. Kopyalanan
    /// miktar zaten seçili alan kadar; kazanılacak birkaç milisaniye için
    /// güvenli olmayan bir yol açmanın karşılığı yok.
    fn bitmap_kur(goruntu: &Goruntu) -> Result<SoftwareBitmap> {
        let yazici = DataWriter::new().map_err(|e| winrt("tampon açılamadı", e))?;
        yazici
            .WriteBytes(&goruntu.pikseller)
            .map_err(|e| winrt("tampon yazılamadı", e))?;
        let tampon = yazici
            .DetachBuffer()
            .map_err(|e| winrt("tampon alınamadı", e))?;

        // Yakalama BGRA8 veriyor (`DXGI_FORMAT_B8G8R8A8_UNORM`); biçim
        // burada da o yüzden Bgra8. Yanlış biçim renkleri değil,
        // OCR'ın gördüğü kontrastı bozardı.
        SoftwareBitmap::CreateCopyFromBuffer(
            &tampon,
            BitmapPixelFormat::Bgra8,
            goruntu.genislik as i32,
            goruntu.yukseklik as i32,
        )
        .map_err(|e| winrt("görüntü OCR biçimine çevrilemedi", e))
    }
}

// ---------------------------------------------------------------------------
// Windows dışı: derlensin diye. Ürün Windows'a özel (`decisions.md` #2).
// ---------------------------------------------------------------------------

#[cfg(not(windows))]
pub fn apartman_baslat() {}

#[cfg(not(windows))]
pub fn oku(_goruntu: &crate::scaling::algoritma::Goruntu, _dil: &str) -> Result<Okuma> {
    Err(crate::error::Error::Unsupported("OCR"))
}
