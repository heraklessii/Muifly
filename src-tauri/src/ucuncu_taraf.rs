//! Üçüncü taraf bildirimleri — EULA madde 8'in karşılığı.
//!
//! Madde 8 "uygulama içindeki 'Üçüncü taraf lisanslar' bölümü"nden söz
//! ediyor. Bir lisans metninde verilen söz, ürünün tutması gereken bir
//! sözdür; bu modül o bölümün verisini sağlıyor.
//!
//! ## Veri nereden geliyor
//!
//! `../ucuncu-taraf.json` `arac/ucuncu-taraf-uret.mjs` tarafından üretiliyor
//! ve buraya `include_str!` ile GÖMÜLÜYOR. Kurulum dizinine ayrı bir dosya
//! konmuyor: kullanıcı dosyayı silse ya da bozsa uygulama sözünü tutamaz
//! hâle gelirdi. Gömülü olan silinemez.
//!
//! Ayrıştırma **tembel**: dosya bir megabaytın üstünde ve kullanıcıların
//! çoğu bu ekranı hiç açmıyor. Açılışta ödenecek bir bedel değil.
//!
//! ## Metinler neden ayrı
//!
//! Yüzlerce crate aynı MIT/Apache metnini taşıyor. Metinler bir kez
//! saklanıp bileşenlerden indeksle gösteriliyor (`metin_no`). Arayüz önce
//! yalnızca listeyi çekiyor (~40 KB), metni ancak kullanıcı bir bileşeni
//! açtığında istiyor.
//!
//! ## Metni olmayan bileşenler
//!
//! Bazı paketler yayımlanan sürümlerinde LICENSE dosyası taşımıyor
//! (`webview2-com`, `unic-*`, `selectors`, `alloc-stdlib`). Orada `metin_no`
//! `None` kalıyor; SPDX kimliği ve kaynak adresi duruyor. Uydurma bir metin
//! göstermektense eksiği söylemek doğru — arayüz de bunu böyle yazıyor.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

const GOMULU: &str = include_str!("../ucuncu-taraf.json");

/// Gömülü dosyanın tamamı.
#[derive(Debug, Clone, Deserialize)]
struct Bildirimler {
    uretildi: String,
    hedef: String,
    bilesenler: Vec<Bilesen>,
    metinler: Vec<String>,
}

/// Uygulamayla birlikte dağıtılan tek bir üçüncü taraf bileşen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bilesen {
    pub ad: String,
    pub surum: String,
    pub tur: Tur,
    /// SPDX ifadesi ("MIT OR Apache-2.0"). Paket bildirmediyse `None`.
    pub lisans: Option<String>,
    pub kaynak: Option<String>,
    /// `metin(no)` için indeks. `None` = paket lisans metni yayımlamamış.
    pub metin_no: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tur {
    /// İkiliye linklenen crate.
    Rust,
    /// `dist/` içine giren npm paketi.
    Npm,
    /// Depoya elle konmuş, paket yöneticisinden gelmeyen varlık (yazı tipi).
    Varlik,
}

/// Arayüze giden liste — metinler hariç.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Liste {
    /// Verinin üretildiği tarih (YYYY-AA-GG).
    pub uretildi: String,
    /// Listenin hangi derleme hedefi için çıkarıldığı.
    pub hedef: String,
    pub bilesenler: Vec<Bilesen>,
}

fn bildirimler() -> Result<&'static Bildirimler> {
    static ONBELLEK: OnceLock<std::result::Result<Bildirimler, String>> = OnceLock::new();

    // `serde_json::Error` klonlanamıyor, `OnceLock` ise değeri bir kez
    // saklıyor; hatayı metin olarak tutup her çağrıda yeniden üretiyoruz.
    match ONBELLEK.get_or_init(|| serde_json::from_str(GOMULU).map_err(|e| e.to_string())) {
        Ok(b) => Ok(b),
        Err(mesaj) => Err(Error::UcuncuTaraf(format!(
            "gömülü bildirim dosyası okunamadı: {mesaj}"
        ))),
    }
}

/// Bileşen listesi. Metinler dahil değil.
pub fn liste() -> Result<Liste> {
    let b = bildirimler()?;
    Ok(Liste {
        uretildi: b.uretildi.clone(),
        hedef: b.hedef.clone(),
        bilesenler: b.bilesenler.clone(),
    })
}

/// Tek bir lisans metni.
///
/// `no`, `Bilesen::metin_no` alanından geliyor. Aralık dışı bir değer
/// arayüzün listeyle tutarsız kalması demek — sessizce boş dönmek yerine
/// hata veriliyor.
pub fn metin(no: usize) -> Result<String> {
    let b = bildirimler()?;
    b.metinler
        .get(no)
        .cloned()
        .ok_or_else(|| Error::UcuncuTaraf(format!("lisans metni bulunamadı (no {no})")))
}

#[cfg(test)]
mod testler {
    use super::*;

    /// Cargo.lock da gömülüyor: aşağıdaki tazelik testi onu okuyor.
    const CARGO_LOCK: &str = include_str!("../Cargo.lock");

    fn oku() -> Bildirimler {
        serde_json::from_str(GOMULU).expect("gömülü ucuncu-taraf.json ayrıştırılamadı")
    }

    #[test]
    fn gomulu_dosya_ayristirilabiliyor() {
        let b = oku();
        assert!(!b.bilesenler.is_empty());
        assert!(!b.metinler.is_empty());
        assert_eq!(b.hedef, "x86_64-pc-windows-msvc");
    }

    /// EULA madde 8'in vaadi: bu ekran boş olamaz.
    ///
    /// Sayı bir alt sınır; bağımlılıklar azalsa bile bu kadarının altına
    /// düşmesi dosyanın kırpıldığı ya da üretecin yarım kaldığı anlamına
    /// gelir.
    #[test]
    fn liste_makul_buyuklukte() {
        assert!(
            oku().bilesenler.len() >= 200,
            "bildirim listesi beklenmedik biçimde kısa"
        );
    }

    #[test]
    fn her_bilesenin_adi_ve_surumu_var() {
        for b in oku().bilesenler {
            assert!(!b.ad.trim().is_empty(), "adsız bileşen");
            assert!(!b.surum.trim().is_empty(), "sürümsüz bileşen: {}", b.ad);
        }
    }

    #[test]
    fn metin_indeksleri_aralik_icinde() {
        let b = oku();
        for bilesen in &b.bilesenler {
            if let Some(no) = bilesen.metin_no {
                assert!(
                    no < b.metinler.len(),
                    "{} aralık dışı metin indeksi gösteriyor: {no}",
                    bilesen.ad
                );
            }
        }
    }

    #[test]
    fn her_metin_en_az_bir_bilesence_kullaniliyor() {
        let b = oku();
        for no in 0..b.metinler.len() {
            assert!(
                b.bilesenler.iter().any(|x| x.metin_no == Some(no)),
                "hiçbir bileşenin göstermediği metin: {no}"
            );
        }
    }

    #[test]
    fn metinler_bos_degil() {
        for (no, m) in oku().metinler.iter().enumerate() {
            assert!(!m.trim().is_empty(), "boş lisans metni: {no}");
        }
    }

    /// Dağıttığımız yazı tipi listede olmak zorunda.
    ///
    /// Outfit uygulamaya gömülü (`styles.css`), yani Muifly onu dağıtıyor.
    /// OFL-1.1 dağıtılan yazı tipinin lisans metninin de eşlik etmesini
    /// istiyor; bu test o metnin gömülü kaldığını koruyor.
    #[test]
    fn yazi_tipi_metniyle_birlikte_listede() {
        let b = oku();
        let font = b
            .bilesenler
            .iter()
            .find(|x| x.ad == "Outfit")
            .expect("Outfit yazı tipi bildirim listesinde yok");

        assert_eq!(font.tur, Tur::Varlik);
        assert_eq!(font.lisans.as_deref(), Some("OFL-1.1"));

        let no = font.metin_no.expect("Outfit'in lisans metni gömülmemiş");
        assert!(b.metinler[no].contains("SIL OPEN FONT LICENSE"));
    }

    /// Üretecin yeniden çalıştırılması unutulduysa yakalar.
    ///
    /// Listedeki her crate, `Cargo.lock`'ta AYNI sürümle durmalı. Bir
    /// bağımlılık yükseltilip `arac/ucuncu-taraf-uret.mjs` çalıştırılmazsa
    /// burası kırmızı olur ve kullanıcıya yanlış sürüm gösterilmemiş olur.
    ///
    /// Ters yön (kilitte olup listede olmayan) kasıtlı olarak
    /// denetlenmiyor: `Cargo.lock` geliştirme ve derleme bağımlılıklarını da
    /// içeriyor, liste ise yalnızca ikiliye gireni.
    #[test]
    fn listedeki_surumler_cargo_lock_ile_ayni() {
        let mut kilit: Vec<(String, String)> = Vec::new();
        let mut ad: Option<String> = None;
        for satir in CARGO_LOCK.lines() {
            let satir = satir.trim();
            if let Some(d) = satir.strip_prefix("name = ") {
                ad = Some(d.trim_matches('"').to_string());
            } else if let Some(d) = satir.strip_prefix("version = ") {
                if let Some(a) = ad.take() {
                    kilit.push((a, d.trim_matches('"').to_string()));
                }
            }
        }
        assert!(!kilit.is_empty(), "Cargo.lock ayrıştırılamadı");

        for b in oku().bilesenler.into_iter().filter(|x| x.tur == Tur::Rust) {
            assert!(
                kilit.contains(&(b.ad.clone(), b.surum.clone())),
                "{} {} Cargo.lock ile uyuşmuyor — `node arac/ucuncu-taraf-uret.mjs` çalıştırılmalı",
                b.ad,
                b.surum
            );
        }
    }

    #[test]
    fn metin_aralik_disinda_hata_veriyor() {
        let b = oku();
        assert!(metin(b.metinler.len()).is_err());
    }

    #[test]
    fn metin_gecerli_indekste_icerik_donduruyor() {
        assert!(!metin(0).expect("0 numaralı metin yok").trim().is_empty());
    }

    #[test]
    fn liste_metinleri_tasimiyor() {
        // Arayüze giden ilk yük küçük kalmalı; metinler ayrı komutla geliyor.
        let json = serde_json::to_string(&liste().expect("liste okunamadı")).expect("serileşmedi");
        assert!(
            json.len() < 200 * 1024,
            "liste beklenenden büyük: {} bayt",
            json.len()
        );
    }
}
