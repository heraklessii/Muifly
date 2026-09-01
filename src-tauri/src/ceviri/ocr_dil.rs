//! Kaynak dilin OCR paketi kurulu mu?
//!
//! ## Neden ayrı bir kontrol gerekiyor
//!
//! Karar #28'den çıkan ürün gereği: `OcrEngine::AvailableRecognizerLanguages`
//! ölçüm yapılan makinede `en-US` ve `tr` verdi, **ama bu Windows'un kurulu
//! dil paketlerine bağlı ve garanti değil.** Kaynak dilin paketi yoksa
//! özellik sessizce yanlış çalışmamalı; ne eksik olduğunu ve nasıl
//! kurulacağını söylemeli.
//!
//! Sessiz başarısızlığın buradaki hali özellikle kötü olurdu: motor
//! kurulamayınca metin okunmaz, kullanıcı da "OCR bu yazıyı okuyamadı"
//! sanır. Oysa sorun yazıda değil, eksik bir Windows bileşeninde — ve
//! çözümü kullanıcının elinde.
//!
//! ## İkiliye bir bayt eklemiyor
//!
//! Karar #28: OCR tarafında gömülecek ya da indirilecek model **yok**,
//! `Windows.Media.Ocr` Windows 10+'ta hazır. Model sorusu yalnızca çeviri
//! tarafında (karar #29, ~512 MB, isteğe bağlı indirme).

use serde::Serialize;

use crate::error::Result;

/// OCR motorunun tanıdığı bir dil.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dil {
    /// BCP-47 etiketi, örn. `en-US`.
    pub etiket: String,
    /// Windows'un verdiği okunur ad, örn. "English (United States)".
    pub ad: String,
}

/// İstenen dilin durumu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", tag = "durum")]
pub enum DilDurumu {
    Var {
        dil: Dil,
    },
    /// Paket kurulu değil. Mesaj kullanıcıya gösterilmek için hazır.
    Yok {
        etiket: String,
        /// Nasıl kurulacağı. Karar #28 "ne eksik ve nasıl kurulur" diyor;
        /// eksik olanı söyleyip çözümü söylememek yarım bir cevap.
        nasil_kurulur: String,
        /// Motorun tanıdığı diller — istenen yoksa kullanıcı neyin
        /// olduğunu görsün.
        mevcut: Vec<Dil>,
    },
}

/// Eksik dil paketinin nasıl kurulacağı.
///
/// Metin Rust tarafında (karar #17). Sayısal ya da karşılanmamış bir vaat
/// içermiyor: yalnızca Windows'un kendi adımlarını tarif ediyor.
fn kurulum_yolu(etiket: &str) -> String {
    format!(
        "Windows Ayarlar → Saat ve dil → Dil ve bölge yolundan '{etiket}' dilini ekleyin, \
         sonra dilin yanındaki üç noktadan Dil seçenekleri → İsteğe bağlı özellikler \
         altında Metin tanıma (OCR) bileşenini kurun."
    )
}

/// Bir dil etiketi istenen dille eşleşiyor mu?
///
/// Tolerans kasıtlı: kullanıcı `en` derken motor `en-US` tanıyor olabilir.
/// Birincil alt etiket eşleşiyorsa yeterli sayılıyor — `en` isteyene
/// "İngilizce OCR yok" demek yanlış olurdu.
///
/// Ters yön de geçerli: `en-GB` istenip motorda `en` varsa da eşleşiyor.
pub fn etiket_eslesiyor(istenen: &str, motorda: &str) -> bool {
    let birincil = |s: &str| {
        s.split(['-', '_'])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase()
    };
    let i = istenen.trim();
    let m = motorda.trim();
    if i.is_empty() || m.is_empty() {
        return false;
    }
    i.eq_ignore_ascii_case(m) || birincil(i) == birincil(m)
}

/// İstenen dilin durumunu, verilmiş bir dil listesine göre belirler.
///
/// Windows çağrısından ayrı tutuluyor ki karar mantığı her platformda
/// test edilebilsin; WinRT'ye bağlı olan yalnızca listeyi getirmek.
pub fn durumu_belirle(istenen: &str, mevcut: Vec<Dil>) -> DilDurumu {
    match mevcut.iter().find(|d| etiket_eslesiyor(istenen, &d.etiket)) {
        Some(d) => DilDurumu::Var { dil: d.clone() },
        None => DilDurumu::Yok {
            etiket: istenen.to_string(),
            nasil_kurulur: kurulum_yolu(istenen),
            mevcut,
        },
    }
}

/// OCR motorunun tanıdığı diller.
#[cfg(windows)]
pub fn mevcut_diller() -> Result<Vec<Dil>> {
    use windows::Media::Ocr::OcrEngine;

    let liste = OcrEngine::AvailableRecognizerLanguages()
        .map_err(|e| crate::error::win("OCR dilleri okunamadı", e))?;

    let mut diller = Vec::new();
    for d in liste {
        // Tek bir dilin adı okunamazsa listenin tamamı düşmüyor: eksik bir
        // satır, hiç liste olmamasından iyi.
        let Ok(etiket) = d.LanguageTag() else {
            continue;
        };
        let ad = d.DisplayName().map(|a| a.to_string()).unwrap_or_default();
        diller.push(Dil {
            etiket: etiket.to_string(),
            ad,
        });
    }
    Ok(diller)
}

#[cfg(not(windows))]
pub fn mevcut_diller() -> Result<Vec<Dil>> {
    Err(crate::error::Error::Unsupported("OCR dil listesi"))
}

/// İstenen kaynak dilin OCR paketi kurulu mu?
pub fn durum(istenen: &str) -> Result<DilDurumu> {
    Ok(durumu_belirle(istenen, mevcut_diller()?))
}

#[cfg(test)]
mod testler {
    use super::*;

    fn dil(etiket: &str, ad: &str) -> Dil {
        Dil {
            etiket: etiket.to_string(),
            ad: ad.to_string(),
        }
    }

    #[test]
    fn bolge_farki_eslesmeyi_bozmuyor() {
        assert!(etiket_eslesiyor("en", "en-US"));
        assert!(etiket_eslesiyor("en-GB", "en"));
        assert!(etiket_eslesiyor("EN-us", "en-US"));
    }

    #[test]
    fn farkli_dil_eslesmiyor() {
        assert!(!etiket_eslesiyor("en", "tr"));
        assert!(!etiket_eslesiyor("en", ""));
    }

    #[test]
    fn kurulu_dil_bulunuyor() {
        let durum = durumu_belirle("en", vec![dil("en-US", "English (United States)")]);
        assert!(matches!(durum, DilDurumu::Var { .. }));
    }

    // --- Karar #28 ürün gereği: sessiz yanlış çalışma yok ---

    #[test]
    fn eksik_dil_nasil_kurulacagini_soyluyor() {
        let durum = durumu_belirle("ja", vec![dil("en-US", "English (United States)")]);
        match durum {
            DilDurumu::Yok {
                etiket,
                nasil_kurulur,
                mevcut,
            } => {
                assert_eq!(etiket, "ja");
                assert!(
                    !nasil_kurulur.trim().is_empty(),
                    "eksik olanı söyleyip çözümü söylememek yarım cevap"
                );
                assert!(nasil_kurulur.contains("ja"), "hangi dil eksik yazmıyor");
                // Kullanıcı neyin kurulu olduğunu da görmeli.
                assert_eq!(mevcut.len(), 1);
            }
            _ => panic!("kurulu olmayan dil 'var' göründü"),
        }
    }

    #[test]
    fn hic_dil_yokken_de_cevap_veriliyor() {
        // OCR paketi hiç kurulu olmayan bir makine; çökmemeli.
        let durum = durumu_belirle("en", Vec::new());
        assert!(matches!(durum, DilDurumu::Yok { .. }));
    }

    #[test]
    fn kurulum_metninde_sayisal_vaat_yok() {
        // Tasarım ilkesi 4. Metin yalnızca Windows'un adımlarını tarif eder.
        let metin = kurulum_yolu("en");
        for yasak in ["ms", "fps", "%", "kat", "daha hızlı"] {
            assert!(
                !metin.contains(yasak),
                "kurulum metninde sayısal/performans vaadi: {yasak}"
            );
        }
    }
}
