//! ATILACAK SONDA — Muifly Faz 5 fizibilitesi, soru 1.
//!
//! "Windows.Media.Ocr hedeflenen oyunlarin yazi tiplerini gercekten okuyor mu?"
//! (`docs/ROADMAP.md` -> Faz 5, karar #22)
//!
//! Kullanim:  ocr-sonda.exe [ornekler-dizini]   (varsayilan: ./ornekler)
//!
//! Dizindeki her `X.png` icin ayni adli `X.txt` dogru metni tutuyor. Sonda
//! OCR ciktisini dogru metinle karsilastirip karakter ve kelime duzeyinde
//! benzerlik veriyor, ayrica gecen sureyi olcuyor.

use std::path::{Path, PathBuf};
use std::time::Instant;
use windows::core::HSTRING;
use windows::Globalization::Language;
use windows::Graphics::Imaging::BitmapDecoder;
use windows::Media::Ocr::OcrEngine;
use windows::Storage::{FileAccessMode, StorageFile};
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};

/// Levenshtein mesafesi. Kucuk girdiler icin duz iki satirli tablo yeterli.
fn mesafe(a: &[char], b: &[char]) -> usize {
    mesafe_genel(a, b)
}

/// Karakter duzeyinde benzerlik, yuzde.
fn benzerlik(a: &str, b: &str) -> f64 {
    let av: Vec<char> = a.chars().collect();
    let bv: Vec<char> = b.chars().collect();
    let en_uzun = av.len().max(bv.len());
    if en_uzun == 0 {
        return 100.0;
    }
    (1.0 - mesafe(&av, &bv) as f64 / en_uzun as f64) * 100.0
}

/// Bosluklari tekile indirip kirpar. OCR satir sonlarini bosluga cevirebiliyor
/// ve bu, ceviri acisindan bir hata degil.
///
/// BOM (U+FEFF) atiliyor: dogru metin dosyalari BOM'lu UTF-8 yazildiginda
/// gorunmez karakter ilk kelimeye yapisip her ornekte kelime hizasini
/// kaydiriyordu. Olcumun kendi hatasiydi, OCR'in degil.
fn sadelestir(s: &str) -> String {
    s.replace('\u{feff}', "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Kelime duzeyinde duzenleme mesafesi ve dogru kelime sayisi.
///
/// **Neden konumsal karsilastirma degil**: ilk denemede kelimeler sirayla
/// eslestiriliyordu. OCR iki kelimeyi birlestirdiginde ("We need" -> "We.peed")
/// sonraki her kelime bir kayiyor ve hepsi yanlis sayiliyordu; 10 kelimelik
/// bir cumle tek hatayla 2/10 gorunuyordu. Bu OCR'in degil metrigin hatasiydi.
/// Kelime dizileri arasinda duzenleme mesafesi, kaymayi tek hata olarak sayiyor.
fn kelime_hatasi(dogru: &str, okunan: &str) -> (usize, usize) {
    let d: Vec<&str> = dogru.split_whitespace().collect();
    let o: Vec<&str> = okunan.split_whitespace().collect();
    (mesafe_genel(&d, &o), d.len())
}

/// `mesafe`nin dilim tipinden bagimsiz hali.
fn mesafe_genel<T: PartialEq>(a: &[T], b: &[T]) -> usize {
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }
    let mut onceki: Vec<usize> = (0..=b.len()).collect();
    let mut simdi = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        simdi[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let bedel = if ca == cb { 0 } else { 1 };
            simdi[j + 1] = (onceki[j] + bedel).min(onceki[j + 1] + 1).min(simdi[j] + 1);
        }
        std::mem::swap(&mut onceki, &mut simdi);
    }
    onceki[b.len()]
}

fn oku(motor: &OcrEngine, yol: &Path) -> windows::core::Result<(String, u128)> {
    let mutlak = std::fs::canonicalize(yol)
        .map(|p| p.to_string_lossy().trim_start_matches(r"\\?\").to_string())
        .unwrap_or_else(|_| yol.to_string_lossy().to_string());

    let baslangic = Instant::now();
    let dosya = StorageFile::GetFileFromPathAsync(&HSTRING::from(mutlak))?.join()?;
    let akis = dosya.OpenAsync(FileAccessMode::Read)?.join()?;
    let cozucu = BitmapDecoder::CreateAsync(&akis)?.join()?;
    let bitmap = cozucu.GetSoftwareBitmapAsync()?.join()?;
    let sonuc = motor.RecognizeAsync(&bitmap)?.join()?;
    let metin = sonuc.Text()?.to_string_lossy();
    Ok((metin, baslangic.elapsed().as_millis()))
}

fn main() -> windows::core::Result<()> {
    // SAFETY: surecte baska bir apartman baslatilmadi.
    unsafe { RoInitialize(RO_INIT_MULTITHREADED)? };

    println!("Kurulu OCR dilleri:");
    for d in OcrEngine::AvailableRecognizerLanguages()? {
        println!("  {} ({})", d.LanguageTag()?, d.DisplayName()?);
    }
    println!();

    let dil = Language::CreateLanguage(&HSTRING::from("en-US"))?;
    let motor = match OcrEngine::TryCreateFromLanguage(&dil) {
        Ok(m) => m,
        Err(e) => {
            println!("en-US icin OCR motoru olusturulamadi: {e}");
            return Ok(());
        }
    };

    let dizin: PathBuf = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("ornekler"));

    let mut pngler: Vec<PathBuf> = std::fs::read_dir(&dizin)
        .unwrap_or_else(|e| panic!("dizin okunamadi {}: {e}", dizin.display()))
        .filter_map(|g| g.ok().map(|g| g.path()))
        .filter(|p| p.extension().map(|e| e == "png").unwrap_or(false))
        .collect();
    pngler.sort();

    if pngler.is_empty() {
        println!("{} icinde png yok. Once uret-ornekler.ps1 calistir.", dizin.display());
        return Ok(());
    }

    println!(
        "{:<26} {:>7} {:>9} {:>10} {:>7}",
        "ornek", "sure", "karakter", "kelime hata", "durum"
    );
    println!("{}", "-".repeat(64));

    let mut toplam_karakter = 0.0f64;
    let mut toplam_kelime_hata = 0usize;
    let mut toplam_kelime = 0usize;
    let mut toplam_ms = 0u128;
    let mut ayrinti: Vec<(String, String, String)> = Vec::new();

    for png in &pngler {
        let ad = png.file_stem().unwrap().to_string_lossy().to_string();
        let txt = png.with_extension("txt");
        let dogru = std::fs::read_to_string(&txt).unwrap_or_default();
        let dogru = sadelestir(&dogru);

        let (ham, ms) = oku(&motor, png)?;
        let okunan = sadelestir(&ham);

        let b = benzerlik(&dogru, &okunan);
        let (hata, kelime) = kelime_hatasi(&dogru, &okunan);

        toplam_karakter += b;
        toplam_kelime_hata += hata;
        toplam_kelime += kelime;
        toplam_ms += ms;

        let durum = if b >= 99.0 {
            "tam"
        } else if b >= 90.0 {
            "iyi"
        } else if b >= 70.0 {
            "kirik"
        } else {
            "kotu"
        };

        println!(
            "{:<26} {:>5} ms {:>8.1}% {:>6}/{:<3} {:>7}",
            ad, ms, b, hata, kelime, durum
        );

        if b < 99.0 {
            ayrinti.push((ad, dogru, okunan));
        }
    }

    let n = pngler.len() as f64;
    println!("{}", "-".repeat(64));
    println!(
        "{:<26} {:>5} ms {:>8.1}% {:>6}/{:<3}",
        "ORTALAMA",
        toplam_ms / pngler.len() as u128,
        toplam_karakter / n,
        toplam_kelime_hata,
        toplam_kelime
    );

    if !ayrinti.is_empty() {
        println!();
        println!("Birebir okunmayanlar:");
        for (ad, dogru, okunan) in ayrinti {
            println!();
            println!("  [{ad}]");
            println!("    dogru : {dogru}");
            println!("    okunan: {okunan}");
        }
    }

    Ok(())
}
