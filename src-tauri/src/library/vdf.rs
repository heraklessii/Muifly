//! Steam'in metin biçimi (VDF / ACF) için küçük bir çözümleyici.
//!
//! Steam iki dosyayı bu biçimde tutuyor ve ikisini de okumamız gerekiyor:
//! `steamapps/libraryfolders.vdf` (kütüphane klasörleri) ve
//! `steamapps/appmanifest_<appid>.acf` (kurulu oyunun adı ve klasörü).
//!
//! Neden hazır bir kasa değil: bu dosyalardan **yalnızca üç alan** okuyoruz ve
//! dosyalara hiçbir zaman yazmıyoruz. Bir bağımlılık eklemek `docs/decisions.md`
//! #21 gereği üçüncü taraf bildirimlerinin de yeniden üretilmesi demek. Yüz
//! satır okuyucu, kalıcı bir bağımlılıktan ucuz.
//!
//! Çözümleyici **hoşgörülü**: tanımadığı şeyi atlıyor, hata döndürmüyor. Steam
//! bu dosyaların biçimini sürümler arası değiştirebilir ve tek bozuk satır
//! yüzünden kütüphanenin tamamının kaybolması, eksik bir alandan kötüdür.

/// VDF ağacındaki bir düğüm.
#[derive(Debug, Clone, PartialEq)]
pub enum Dugum {
    Metin(String),
    /// Sıra korunuyor: Steam kütüphaneleri `"0"`, `"1"`… diye numaralıyor.
    /// `HashMap` bu sırayı kaybederdi.
    Nesne(Vec<(String, Dugum)>),
}

/// Kendini çağıran çözümlemede yığını koruyan sınır.
///
/// Gerçek dosyalar 4-5 seviye derin. 32, bozuk bir dosyanın programı
/// çökertmesini engelleyecek kadar dar, hiçbir gerçek dosyayı kesmeyecek
/// kadar geniş.
const AZAMI_DERINLIK: usize = 32;

impl Dugum {
    /// Alt alanı adıyla bulur. Anahtar karşılaştırması **büyük/küçük harf
    /// duyarsız**: Steam bazı sürümlerde `"AppID"`, bazılarında `"appid"`
    /// yazıyor.
    pub fn alan(&self, ad: &str) -> Option<&Dugum> {
        match self {
            Dugum::Metin(_) => None,
            Dugum::Nesne(alanlar) => alanlar
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(ad))
                .map(|(_, v)| v),
        }
    }

    /// Alt alanı metin olarak okur. Alan bir nesneyse `None`.
    pub fn metin(&self, ad: &str) -> Option<&str> {
        match self.alan(ad)? {
            Dugum::Metin(s) => Some(s),
            Dugum::Nesne(_) => None,
        }
    }

    /// Doğrudan alt alanların tamamı, dosyadaki sırayla.
    pub fn alanlar(&self) -> &[(String, Dugum)] {
        match self {
            Dugum::Metin(_) => &[],
            Dugum::Nesne(a) => a,
        }
    }
}

/// Metni çözümler. Girdi ne kadar bozuk olursa olsun panik yok: en kötü
/// ihtimalle boş bir nesne döner.
pub fn coz(girdi: &str) -> Dugum {
    let mut imlec = Imlec {
        b: girdi.as_bytes(),
        i: 0,
    };
    Dugum::Nesne(alanlari_oku(&mut imlec, 0))
}

struct Imlec<'a> {
    b: &'a [u8],
    i: usize,
}

impl Imlec<'_> {
    fn bosluklari_atla(&mut self) {
        loop {
            while self.i < self.b.len() && self.b[self.i].is_ascii_whitespace() {
                self.i += 1;
            }
            // `//` yorum satırı: Steam bunu libraryfolders.vdf'te kullanıyor.
            if self.i + 1 < self.b.len() && self.b[self.i] == b'/' && self.b[self.i + 1] == b'/' {
                while self.i < self.b.len() && self.b[self.i] != b'\n' {
                    self.i += 1;
                }
            } else {
                return;
            }
        }
    }

    /// Tırnaklı ya da tırnaksız bir belirteç okur.
    fn belirtec(&mut self) -> Option<String> {
        self.bosluklari_atla();
        if self.i >= self.b.len() {
            return None;
        }
        match self.b[self.i] {
            b'"' => {
                self.i += 1;
                let mut cikti = Vec::new();
                while self.i < self.b.len() {
                    match self.b[self.i] {
                        b'"' => {
                            self.i += 1;
                            break;
                        }
                        b'\\' if self.i + 1 < self.b.len() => {
                            // VDF kaçışları. Yol alanlarında çok sık:
                            // "C:\\Program Files (x86)\\Steam".
                            let k = self.b[self.i + 1];
                            cikti.push(match k {
                                b'n' => b'\n',
                                b't' => b'\t',
                                diger => diger,
                            });
                            self.i += 2;
                        }
                        diger => {
                            cikti.push(diger);
                            self.i += 1;
                        }
                    }
                }
                Some(String::from_utf8_lossy(&cikti).into_owned())
            }
            b'{' | b'}' => {
                let t = self.b[self.i] as char;
                self.i += 1;
                Some(t.to_string())
            }
            _ => {
                let bas = self.i;
                while self.i < self.b.len()
                    && !self.b[self.i].is_ascii_whitespace()
                    && self.b[self.i] != b'{'
                    && self.b[self.i] != b'}'
                {
                    self.i += 1;
                }
                Some(String::from_utf8_lossy(&self.b[bas..self.i]).into_owned())
            }
        }
    }

    /// Belirteci tüketmeden sıradaki baytı gösterir.
    fn gozetle(&mut self) -> Option<u8> {
        self.bosluklari_atla();
        self.b.get(self.i).copied()
    }
}

fn alanlari_oku(imlec: &mut Imlec, derinlik: usize) -> Vec<(String, Dugum)> {
    let mut alanlar = Vec::new();
    loop {
        match imlec.gozetle() {
            None => return alanlar,
            Some(b'}') => {
                imlec.i += 1;
                return alanlar;
            }
            _ => {}
        }
        let Some(anahtar) = imlec.belirtec() else {
            return alanlar;
        };
        // Anahtar yerine gelen parantez bozuk dosya demek; atlanıyor.
        if anahtar == "{" || anahtar == "}" {
            continue;
        }
        match imlec.gozetle() {
            Some(b'{') => {
                imlec.i += 1;
                if derinlik >= AZAMI_DERINLIK {
                    // Sınırı aşan dal okunmadan kapatılıyor ki yığın taşmasın.
                    atla(imlec);
                    alanlar.push((anahtar, Dugum::Nesne(Vec::new())));
                } else {
                    let ic = alanlari_oku(imlec, derinlik + 1);
                    alanlar.push((anahtar, Dugum::Nesne(ic)));
                }
            }
            None => return alanlar,
            _ => {
                let Some(deger) = imlec.belirtec() else {
                    return alanlar;
                };
                alanlar.push((anahtar, Dugum::Metin(deger)));
            }
        }
    }
}

/// Açılmış bir süslü parantezi, içeriğini okumadan kapatır.
fn atla(imlec: &mut Imlec) {
    let mut derinlik = 1usize;
    while derinlik > 0 {
        let Some(t) = imlec.belirtec() else { return };
        match t.as_str() {
            "{" => derinlik += 1,
            "}" => derinlik -= 1,
            _ => {}
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    /// Gerçek bir `libraryfolders.vdf`'ten kısaltılmış örnek.
    const KITAPLIK: &str = r#"
"libraryfolders"
{
	"0"
	{
		"path"		"C:\\Program Files (x86)\\Steam"
		"label"		""
		"totalsize"		"0"
		"apps"
		{
			"431960"		"0"
		}
	}
	"1"
	{
		"path"		"D:\\SteamLibrary"
		"apps"
		{
		}
	}
}
"#;

    #[test]
    fn kitaplik_yollari_okunuyor() {
        let kok = coz(KITAPLIK);
        let kitapliklar = kok.alan("libraryfolders").unwrap();
        let yollar: Vec<&str> = kitapliklar
            .alanlar()
            .iter()
            .filter_map(|(_, v)| v.metin("path"))
            .collect();
        assert_eq!(
            yollar,
            vec![r"C:\Program Files (x86)\Steam", r"D:\SteamLibrary"]
        );
    }

    #[test]
    fn kacislar_cozuluyor() {
        let kok = coz(r#""a" { "yol" "C:\\Oyun\\bin" }"#);
        assert_eq!(kok.alan("a").unwrap().metin("yol"), Some(r"C:\Oyun\bin"));
    }

    #[test]
    fn anahtar_buyuk_kucuk_harf_duyarsiz() {
        let kok = coz(r#""AppState" { "AppID" "431960" }"#);
        assert_eq!(kok.alan("appstate").unwrap().metin("appid"), Some("431960"));
    }

    #[test]
    fn yorum_satiri_atlaniyor() {
        let kok = coz("// aciklama\n\"a\" { \"b\" \"c\" }");
        assert_eq!(kok.alan("a").unwrap().metin("b"), Some("c"));
    }

    #[test]
    fn bozuk_dosya_panige_yol_acmiyor() {
        // Kapanmamış parantez, yarım tırnak, çöp: hiçbiri çökertmemeli.
        for girdi in [
            "\"a\" {",
            "\"a\" { \"b\"",
            "}}}}",
            "\"a\" \"b\" \"c\"",
            "",
            "\"kacisla_biten\" \"deger\\",
        ] {
            let _ = coz(girdi);
        }
    }

    #[test]
    fn asiri_derinlik_yigini_tasirmiyor() {
        let derin = "\"a\" {".repeat(5000);
        let _ = coz(&derin);
    }
}
