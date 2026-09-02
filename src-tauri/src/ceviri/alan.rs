//! Çevrilecek ekran alanı.
//!
//! ## Neden bir alan var
//!
//! Karar #22: sürekli çeviri yok, **tuşa basınca seçili alan** var. Gerekçe
//! performans değil kalite — hareket eden ekranda yazı yarı render edilmiş,
//! fade animasyonu içinde ya da hareket bulanıklığı altında yakalanıyor ve
//! OCR çöp üretiyor. Kullanıcı diyalog kutusunun durduğu yeri bir kez
//! gösteriyor, her seferinde orası okunuyor.
//!
//! ## Neden oran, piksel değil
//!
//! Alan profile yazılıyor (karar #22) ve profiller paylaşılabilir dosyalar
//! (karar #9). Piksel olarak yazılsaydı 1080p'de seçilen bir alan 1440p'lik
//! bir makinede yanlış yere düşerdi — üstelik sessizce, çünkü hâlâ geçerli
//! bir dikdörtgen olurdu. Oran, ekranın kendisiyle birlikte ölçekleniyor.
//!
//! ## Alan seçmemek de bir seçenek
//!
//! Alan yoksa ekranın tamamı okunuyor. Karar #28 tam 1080p kare için 74 ms
//! ölçtü; yani "alan seçmeden hiç çalışmayan" bir özellik yapmak için sebep
//! yok. Alan seçimi kaliteyi artırıyor (etraftaki yazılar girmiyor), şart
//! değil.

use serde::{Deserialize, Serialize};

/// OCR'a gönderilebilecek en küçük kenar (piksel).
///
/// Bunun altında kalan bir seçim kazayla yapılmış bir tıklamadır: birkaç
/// piksellik bir şeritte okunacak yazı yok. Sessizce boş sonuç döndürmek
/// yerine seçimi geçersiz saymak, kullanıcıya ne olduğunu söylüyor.
pub const EN_KUCUK_KENAR: u32 = 16;

/// Ekranın çevrilecek parçası — kenar uzunluklarının **oranı** olarak.
///
/// Alan adları İngilizce, çünkü bu yapı profil dosyasına yazılıyor ve
/// profil şemasının tamamı öyle (`profile_engine::schema`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Alan {
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

impl Default for Alan {
    fn default() -> Self {
        Self::tam_ekran()
    }
}

impl Alan {
    pub const fn tam_ekran() -> Self {
        Self {
            left: 0.0,
            top: 0.0,
            width: 1.0,
            height: 1.0,
        }
    }

    /// Piksel dikdörtgeninden oran üretir.
    ///
    /// Seçim penceresi piksel veriyor, profil oran saklıyor; dönüşümün tek
    /// yeri burası.
    pub fn pikselden(x: u32, y: u32, g: u32, b: u32, ekran_g: u32, ekran_y: u32) -> Self {
        if ekran_g == 0 || ekran_y == 0 {
            return Self::tam_ekran();
        }
        Self {
            left: x as f32 / ekran_g as f32,
            top: y as f32 / ekran_y as f32,
            width: g as f32 / ekran_g as f32,
            height: b as f32 / ekran_y as f32,
        }
    }

    /// Ekranın içine sığdırılmış hali.
    ///
    /// Elle düzenlenmiş bir profil dosyasından `left: -3.0` gelebilir.
    /// Böyle bir değeri hata saymak yerine kırpmak, `Ayarlar::normalize`
    /// ile aynı tercih: kullanıcının dosyası yüzünden özellik ölmüyor.
    pub fn kirp(self) -> Self {
        let sol = if self.left.is_finite() {
            self.left.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let ust = if self.top.is_finite() {
            self.top.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let g = if self.width.is_finite() {
            self.width.clamp(0.0, 1.0 - sol)
        } else {
            1.0 - sol
        };
        let b = if self.height.is_finite() {
            self.height.clamp(0.0, 1.0 - ust)
        } else {
            1.0 - ust
        };
        Self {
            left: sol,
            top: ust,
            width: g,
            height: b,
        }
    }

    /// Verilen ekran boyutunda piksel dikdörtgeni: `(x, y, genişlik, yükseklik)`.
    pub fn piksel(self, ekran_g: u32, ekran_y: u32) -> (u32, u32, u32, u32) {
        let a = self.kirp();
        let x = (a.left * ekran_g as f32).round() as u32;
        let y = (a.top * ekran_y as f32).round() as u32;
        let g = (a.width * ekran_g as f32).round() as u32;
        let b = (a.height * ekran_y as f32).round() as u32;
        // Yuvarlama bir piksel dışarı taşırabiliyor; kesme burada patlamasın.
        let g = g.min(ekran_g.saturating_sub(x));
        let b = b.min(ekran_y.saturating_sub(y));
        (x, y, g, b)
    }

    /// Bu ekran boyutunda okunabilir bir alan mı?
    pub fn okunabilir(self, ekran_g: u32, ekran_y: u32) -> bool {
        let (_, _, g, b) = self.piksel(ekran_g, ekran_y);
        g >= EN_KUCUK_KENAR && b >= EN_KUCUK_KENAR
    }

    /// Alanın tamamı ekransa `true` — arayüz "tüm ekran" yazabilsin diye.
    pub fn tam_mi(self) -> bool {
        let a = self.kirp();
        a.left <= f32::EPSILON
            && a.top <= f32::EPSILON
            && a.width >= 1.0 - f32::EPSILON
            && a.height >= 1.0 - f32::EPSILON
    }
}

/// Görüntünün alan içinde kalan parçasını kopyalar.
///
/// Kopya şart: OCR'a giden tampon bitişik olmak zorunda, oysa kaynağın satır
/// uzunluğu alanın genişliğinden büyük. Kopyalanan miktar alan kadar, kare
/// kadar değil — alanın asıl kazancı da bu.
pub fn kes(
    goruntu: &crate::scaling::algoritma::Goruntu,
    alan: Alan,
) -> Option<crate::scaling::algoritma::Goruntu> {
    if !goruntu.gecerli() {
        return None;
    }
    let (x, y, g, b) = alan.piksel(goruntu.genislik, goruntu.yukseklik);
    if g < EN_KUCUK_KENAR || b < EN_KUCUK_KENAR {
        return None;
    }

    let mut kesit = crate::scaling::algoritma::Goruntu::yeni(g, b);
    let kaynak_satir = goruntu.genislik as usize * 4;
    let hedef_satir = g as usize * 4;
    for satir in 0..b as usize {
        let kaynak_bas = (y as usize + satir) * kaynak_satir + x as usize * 4;
        let hedef_bas = satir * hedef_satir;
        kesit.pikseller[hedef_bas..hedef_bas + hedef_satir]
            .copy_from_slice(&goruntu.pikseller[kaynak_bas..kaynak_bas + hedef_satir]);
    }
    Some(kesit)
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::scaling::algoritma::Goruntu;

    #[test]
    fn varsayilan_tum_ekran() {
        assert!(Alan::default().tam_mi());
        assert_eq!(Alan::default().piksel(1920, 1080), (0, 0, 1920, 1080));
    }

    // --- Oranın asıl sebebi: başka çözünürlükte doğru yere düşmesi ---

    #[test]
    fn oran_cozunurlukten_bagimsiz() {
        // 1080p'de ekranın alt ortasındaki altyazı şeridi.
        let a = Alan::pikselden(480, 810, 960, 135, 1920, 1080);
        // Aynı profil 1440p bir makinede açıldığında aynı yere düşmeli.
        assert_eq!(a.piksel(2560, 1440), (640, 1080, 1280, 180));
    }

    #[test]
    fn piksel_gidis_donus() {
        let a = Alan::pikselden(100, 200, 300, 400, 1920, 1080);
        assert_eq!(a.piksel(1920, 1080), (100, 200, 300, 400));
    }

    // --- Elle düzenlenmiş dosyadan gelen değerler ---

    #[test]
    fn tasan_alan_kirpiliyor() {
        let a = Alan {
            left: -0.5,
            top: 0.9,
            width: 3.0,
            height: 3.0,
        };
        let (x, y, g, b) = a.piksel(1000, 1000);
        assert_eq!(x, 0);
        assert_eq!(y, 900);
        assert!(x + g <= 1000, "genislik ekrani asiyor");
        assert!(y + b <= 1000, "yukseklik ekrani asiyor");
    }

    #[test]
    fn nan_alan_ekrani_bozmuyor() {
        // Elle düzenlenmiş bir dosyada `NaN` mümkün; `clamp` NaN'da panikler.
        let a = Alan {
            left: f32::NAN,
            top: f32::NAN,
            width: f32::NAN,
            height: f32::NAN,
        };
        assert_eq!(a.piksel(800, 600), (0, 0, 800, 600));
    }

    #[test]
    fn kucucuk_alan_okunabilir_degil() {
        // Kazara yapılmış bir tıklama; birkaç pikselde okunacak yazı yok.
        let a = Alan::pikselden(10, 10, 4, 4, 1920, 1080);
        assert!(!a.okunabilir(1920, 1080));
    }

    // --- Kesme ---

    #[test]
    fn kesit_boyutu_ve_icerigi() {
        let mut g = Goruntu::yeni(64, 64);
        for y in 0..64usize {
            for x in 0..64usize {
                g.pikseller[(y * 64 + x) * 4] = (x + y) as u8;
            }
        }
        let alan = Alan::pikselden(16, 16, 32, 32, 64, 64);
        let k = kes(&g, alan).expect("kesit alinamadi");
        assert_eq!((k.genislik, k.yukseklik), (32, 32));
        assert!(k.gecerli());
        // Kesitin sol üstü kaynağın (16,16) pikseli.
        assert_eq!(k.pikseller[0], 32);
        // Son satırın son pikseli kaynağın (47,47)'si.
        let son = ((31 * 32) + 31) * 4;
        assert_eq!(k.pikseller[son], 94);
    }

    #[test]
    fn cok_kucuk_kesit_reddediliyor() {
        let g = Goruntu::yeni(64, 64);
        let alan = Alan::pikselden(0, 0, 4, 4, 64, 64);
        assert!(kes(&g, alan).is_none());
    }

    #[test]
    fn bos_goruntuden_kesit_alinmiyor() {
        let g = Goruntu {
            genislik: 0,
            yukseklik: 0,
            pikseller: Vec::new(),
        };
        assert!(kes(&g, Alan::tam_ekran()).is_none());
    }
}
