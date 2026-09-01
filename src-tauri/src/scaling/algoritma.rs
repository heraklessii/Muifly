//! Ölçekleme algoritmaları — saf referans uygulaması.
//!
//! Bu dosyada Windows API'si YOK. Girdisi bir piksel tamponu, çıktısı bir
//! piksel tamponu. `frames.rs`'in `etw.rs`'ten ayrılmasıyla aynı gerekçe:
//! algoritmanın **doğru** olup olmadığı, gerçek bir oyun açmadan, gerçek bir
//! ekran kartı olmadan sınanabilmeli.
//!
//! # Neden burada CPU, çalışırken GPU
//!
//! Gerçek zamanlı yol bu dosya DEĞİL. 1080p→1440p bir kareyi Lanczos ile
//! CPU'da ölçeklemek, oyunun kendi kare bütçesinin katbekat üstünde bir iş;
//! ölçüldü, varsayılmadı (`decisions.md` #32). Çalışma zamanında ölçekleme
//! `sunum.rs`'teki piksel gölgelendiricilerinde, yakalanan dokunun hiç
//! CPU'ya inmediği bir boru hattında yapılıyor.
//!
//! Buradaki uygulamalar iki işe yarıyor:
//!
//! 1. **Referans**: gölgelendiricinin ne üretmesi gerektiğinin tanımı.
//! 2. **Kalite karşılaştırması**: `ROADMAP.md` Faz 3'ün birinci kabul
//!    kriteri "görüntü kalitesi görsel karşılaştırma testleriyle
//!    doğrulanmış olsun" diyor. O testler bu dosyanın altında.
//!
//! Aynı matematiğin iki yerde durması bir borç ve `decisions.md` #32'de
//! öyle yazıyor; `sunum::testler::golgelendirici_sabitleri_referansla_ayni`
//! ikisinin sabitlerini bağlıyor ama satır satır eşitliği kanıtlamıyor.
//!
//! # Piksel düzeni
//!
//! Her yerde BGRA8: Desktop Duplication'ın verdiği düzen
//! (`DXGI_FORMAT_B8G8R8A8_UNORM`). Kanal sırası kodun içinde bir yerde
//! sessizce çevrilmiyor; yakalamadan sunuma kadar tek düzen var.

use serde::{Deserialize, Serialize};

/// Lanczos çekirdeğinin yarıçapı (yaygın adıyla `a`).
///
/// 3, klasik seçim: 2 fazla yumuşak, 4 halkalanmayı (ringing) görünür
/// kılıyor. Gölgelendiricideki değerle aynı olmak zorunda — testle bağlı.
pub const LANCZOS_A: i32 = 3;

/// xBR'de kenarın "güçlü" sayılması için gereken kat.
///
/// `e * KAT < i` ise kenar güçlü kabul ediliyor ve köşe pikseli tamamen
/// değişiyor; sadece `e < i` ise yarı yarıya karışıyor. Güçlü kenarın tam
/// değişmesi xBR'nin varlık sebebi: köşegen bir kenarı bulanıklaştırmadan
/// düzleştirmek.
pub const XBR_GUCLU_KAT: f32 = 2.0;

/// Zayıf kenarda karışım oranı.
pub const XBR_ZAYIF_KARISIM: f32 = 0.5;

/// Kullanıcının seçebileceği ölçekleme algoritmaları.
///
/// `serde` adları profil dosyasına yazılıyor (`profile_engine::schema`
/// → `scaling.algorithm`) ve kullanıcı o dosyayı elle düzenleyebiliyor;
/// bu yüzden adlar keyfi değiştirilemez.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Algoritma {
    /// Tam sayı katı, en yakın komşu. **Varsayılan.**
    ///
    /// Varsayılan olması bir kalite iddiası değil, bir müdahale ölçüsü: tek
    /// piksel bile yeni bir renk üretmiyor, kaynakta olmayan hiçbir ton
    /// ekranda belirmiyor. Diğerlerini kullanıcı seçiyor.
    #[default]
    TamSayi,
    /// İki doğrusal aradeğerleme. Ucuz, yumuşak.
    Bilinear,
    /// Lanczos (a=3). Ayrılabilir, keskin, halkalanmaya açık.
    Lanczos,
    /// xBR — kenar yönlü, iki katına çıkarır.
    Xbr,
}

impl Algoritma {
    /// Profil dosyasındaki anahtar.
    pub fn anahtar(self) -> &'static str {
        match self {
            Algoritma::TamSayi => "tam_sayi",
            Algoritma::Bilinear => "bilinear",
            Algoritma::Lanczos => "lanczos",
            Algoritma::Xbr => "xbr",
        }
    }

    /// Elle düzenlenmiş bir profilden gelen metni çözer.
    ///
    /// Bilinmeyen bir ad `None` dönüyor; çağıran varsayılana düşüyor ve
    /// düzeltmeyi kullanıcıya söylüyor (`schema::Profil::dogrula` örüntüsü).
    pub fn coz(s: &str) -> Option<Self> {
        Self::hepsi().into_iter().find(|a| a.anahtar() == s)
    }

    pub fn ad(self) -> &'static str {
        match self {
            Algoritma::TamSayi => "Tam sayı katı",
            Algoritma::Bilinear => "Bilinear",
            Algoritma::Lanczos => "Lanczos",
            Algoritma::Xbr => "xBR",
        }
    }

    /// Kullanıcıya gösterilen açıklama.
    ///
    /// Tasarım ilkesi 4: hiçbiri "daha iyi görünür" demiyor. Ne yaptığını
    /// söylüyor, kararı kullanıcı veriyor —
    /// `testler::aciklamalarda_sayisal_vaat_yok` bunu tutuyor.
    pub fn aciklama(self) -> &'static str {
        match self {
            Algoritma::TamSayi => {
                "Görüntüyü tam sayı katına çıkarır, yeni renk üretmez. \
                 Ekrana tam oturmayan kısım kenarda siyah kalır."
            }
            Algoritma::Bilinear => {
                "Komşu dört pikseli ortalar. Hesabı en ucuz olan yol; \
                 kenarlar yumuşar."
            }
            Algoritma::Lanczos => {
                "Geniş bir pencereyle aradeğerleme yapar, kenarları \
                 bilinear'a göre daha belirgin bırakır. Keskin geçişlerin \
                 iki yanında hafif halkalanma görülebilir."
            }
            Algoritma::Xbr => {
                "Kenar yönünü tahmin edip köşegenleri basamaklı bırakmadan \
                 iki katına çıkarır. Düz alanlara dokunmaz. Piksel çizimli \
                 ve düşük çözünürlüklü kaynaklarda etkisi en görünür olan yol."
            }
        }
    }

    /// Sabit sıra: arayüzdeki liste ve testler aynı sırayı kullanıyor.
    pub fn hepsi() -> [Algoritma; 4] {
        [
            Algoritma::TamSayi,
            Algoritma::Bilinear,
            Algoritma::Lanczos,
            Algoritma::Xbr,
        ]
    }
}

/// BGRA8 bir görüntü tamponu.
///
/// `pikseller` uzunluğu `genislik * yukseklik * 4` olmak zorunda; `gecerli`
/// bunu kontrol ediyor. Satırlar üstten alta, aralarında dolgu yok — dolgulu
/// (pitch'li) bir kaynak `yakalama.rs`'te burada tüketilmeden önce
/// sıkıştırılıyor.
#[derive(Debug, Clone, PartialEq)]
pub struct Goruntu {
    pub genislik: u32,
    pub yukseklik: u32,
    pub pikseller: Vec<u8>,
}

impl Goruntu {
    /// Siyah, tamamen opak bir görüntü.
    pub fn yeni(genislik: u32, yukseklik: u32) -> Self {
        let mut pikseller = vec![0u8; (genislik as usize) * (yukseklik as usize) * 4];
        for p in pikseller.chunks_exact_mut(4) {
            p[3] = 255;
        }
        Self {
            genislik,
            yukseklik,
            pikseller,
        }
    }

    pub fn gecerli(&self) -> bool {
        self.genislik > 0
            && self.yukseklik > 0
            && self.pikseller.len() == (self.genislik as usize) * (self.yukseklik as usize) * 4
    }

    /// Kenar dışına taşan koordinatı içeri kırpar.
    ///
    /// Kırpma (kenar pikselini tekrarlama), sıfırla doldurmaya tercih
    /// edildi: sıfır siyah demek ve görüntünün dört kenarında olmayan bir
    /// siyah çerçeve üretirdi.
    pub fn piksel(&self, x: i32, y: i32) -> [u8; 4] {
        let x = x.clamp(0, self.genislik as i32 - 1) as usize;
        let y = y.clamp(0, self.yukseklik as i32 - 1) as usize;
        let i = (y * self.genislik as usize + x) * 4;
        [
            self.pikseller[i],
            self.pikseller[i + 1],
            self.pikseller[i + 2],
            self.pikseller[i + 3],
        ]
    }

    fn yaz(&mut self, x: u32, y: u32, p: [u8; 4]) {
        let i = ((y as usize) * self.genislik as usize + x as usize) * 4;
        self.pikseller[i..i + 4].copy_from_slice(&p);
    }
}

/// Yakalanan dokunun içinde ölçeklenecek dikdörtgen.
///
/// Masaüstünün tamamı yakalanıyor ama ölçeklenmesi gereken şey **oyun
/// penceresi**. Kırpma olmasaydı ekranın tamamı kendi boyutunda yeniden
/// çizilirdi: hiçbir şey büyütülmemiş olurdu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Alan {
    pub x: u32,
    pub y: u32,
    pub genislik: u32,
    pub yukseklik: u32,
}

impl Alan {
    /// Dokunun tamamı.
    pub fn tam(genislik: u32, yukseklik: u32) -> Self {
        Self {
            x: 0,
            y: 0,
            genislik,
            yukseklik,
        }
    }
}

/// Ekran koordinatındaki bir pencere dikdörtgenini doku alanına çevirir.
///
/// `pencere` masaüstü koordinatında `(sol, üst, sağ, alt)`; `ekran_kokeni`
/// yakalanan ekranın masaüstündeki sol üst köşesi; `doku` yakalanan
/// görüntünün boyutu. Pencere ekranın dışına taşıyorsa kırpılıyor.
///
/// Sonuç boşsa (pencere küçültülmüş, başka ekranda, ya da hiç yok)
/// `None` dönüyor ve çağıran dokunun tamamına düşüyor. Sıfır boyutlu bir
/// alana ölçeklemek, ekranı karartmak demek olurdu.
pub fn pencere_alani(
    pencere: (i32, i32, i32, i32),
    ekran_kokeni: (i32, i32),
    doku: (u32, u32),
) -> Option<Alan> {
    let (sol, ust, sag, alt) = pencere;
    let (ekran_x, ekran_y) = ekran_kokeni;
    let (doku_g, doku_y) = doku;
    let x0 = (sol - ekran_x).max(0);
    let y0 = (ust - ekran_y).max(0);
    let x1 = (sag - ekran_x).min(doku_g as i32);
    let y1 = (alt - ekran_y).min(doku_y as i32);
    // En az bir piksellik bir alan olmalı; altı pikselin altındaki bir
    // pencere zaten ölçeklenecek bir şey taşımıyor.
    if x1 - x0 < 8 || y1 - y0 < 8 {
        return None;
    }
    Some(Alan {
        x: x0 as u32,
        y: y0 as u32,
        genislik: (x1 - x0) as u32,
        yukseklik: (y1 - y0) as u32,
    })
}

/// Kaynağın hedef ekrandaki yerleşimi.
///
/// En/boy oranı **her zaman** korunuyor; ekrana sığmayan kısım kenarda
/// siyah kalıyor. Oranı bozup ekranı doldurmak bir seçenek olarak
/// sunulmuyor: bir oyun görüntüsünü yatay olarak esnetmek, kullanıcının
/// istemediği ama fark etmesi de zaman alan bir bozulma.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Yerlesim {
    pub genislik: u32,
    pub yukseklik: u32,
    pub x_ofset: u32,
    pub y_ofset: u32,
}

/// En/boy oranını koruyarak hedefe oturtur (letterbox).
pub fn en_boy_koru(kaynak_g: u32, kaynak_y: u32, hedef_g: u32, hedef_y: u32) -> Yerlesim {
    if kaynak_g == 0 || kaynak_y == 0 || hedef_g == 0 || hedef_y == 0 {
        return Yerlesim {
            genislik: 0,
            yukseklik: 0,
            x_ofset: 0,
            y_ofset: 0,
        };
    }
    let oran = (hedef_g as f64 / kaynak_g as f64).min(hedef_y as f64 / kaynak_y as f64);
    let g = ((kaynak_g as f64 * oran).round() as u32).clamp(1, hedef_g);
    let y = ((kaynak_y as f64 * oran).round() as u32).clamp(1, hedef_y);
    Yerlesim {
        genislik: g,
        yukseklik: y,
        x_ofset: (hedef_g - g) / 2,
        y_ofset: (hedef_y - y) / 2,
    }
}

/// Kaynağın hedefe sığan en büyük tam sayı katı.
///
/// En az 1: hedef kaynaktan küçükse tam sayı ölçekleme küçültme yapmıyor,
/// görüntü olduğu boyutta ortalanıyor. Küçültmek, "yeni renk üretmez"
/// vaadini bozardı.
pub fn tam_kat(kaynak_g: u32, kaynak_y: u32, hedef_g: u32, hedef_y: u32) -> u32 {
    if kaynak_g == 0 || kaynak_y == 0 {
        return 1;
    }
    (hedef_g / kaynak_g).min(hedef_y / kaynak_y).max(1)
}

/// Seçilen algoritmayla ölçekler.
///
/// Dönen görüntünün boyutu **istenen boyut olmayabilir**: tam sayı
/// ölçeklemede kat aşağı yuvarlanıyor. Ekrana yerleştirmek çağıranın işi
/// (`en_boy_koru`).
pub fn olcekle(kaynak: &Goruntu, algo: Algoritma, hedef_g: u32, hedef_y: u32) -> Goruntu {
    if !kaynak.gecerli() || hedef_g == 0 || hedef_y == 0 {
        return Goruntu::yeni(hedef_g.max(1), hedef_y.max(1));
    }
    match algo {
        Algoritma::TamSayi => {
            let kat = tam_kat(kaynak.genislik, kaynak.yukseklik, hedef_g, hedef_y);
            en_yakin(kaynak, kaynak.genislik * kat, kaynak.yukseklik * kat)
        }
        Algoritma::Bilinear => bilinear(kaynak, hedef_g, hedef_y),
        Algoritma::Lanczos => lanczos(kaynak, hedef_g, hedef_y),
        Algoritma::Xbr => {
            let iki_kat = xbr2x(kaynak);
            // xBR sabit iki kat. Hedef başka bir boyutsa aradaki farkı
            // Lanczos kapatıyor: xBR'nin ürettiği keskin köşegenleri en az
            // bozan ikinci adım o. Hedef zaten iki katsa ikinci adım hiç
            // koşmuyor.
            if iki_kat.genislik == hedef_g && iki_kat.yukseklik == hedef_y {
                iki_kat
            } else {
                lanczos(&iki_kat, hedef_g, hedef_y)
            }
        }
    }
}

/// En yakın komşu. Tam sayı katında piksel-birebir kopyalama demek.
fn en_yakin(kaynak: &Goruntu, hedef_g: u32, hedef_y: u32) -> Goruntu {
    let mut cikti = Goruntu::yeni(hedef_g, hedef_y);
    for y in 0..hedef_y {
        let sy = (y as u64 * kaynak.yukseklik as u64 / hedef_y as u64) as i32;
        for x in 0..hedef_g {
            let sx = (x as u64 * kaynak.genislik as u64 / hedef_g as u64) as i32;
            cikti.yaz(x, y, kaynak.piksel(sx, sy));
        }
    }
    cikti
}

/// Çıktı koordinatının kaynaktaki merkezi.
///
/// `+0.5 / -0.5`, piksel merkezine göre hizalama. Bu düzeltme olmadan
/// görüntü ölçek oranına bağlı olarak yarım piksel kayıyor; küçük bir hata
/// ama ölçekleme oranı büyüdükçe kenarlarda görünür hale geliyor.
fn kaynak_merkezi(hedef: u32, kaynak: u32, i: u32) -> f32 {
    (i as f32 + 0.5) * (kaynak as f32 / hedef as f32) - 0.5
}

fn bilinear(kaynak: &Goruntu, hedef_g: u32, hedef_y: u32) -> Goruntu {
    let mut cikti = Goruntu::yeni(hedef_g, hedef_y);
    for y in 0..hedef_y {
        let fy = kaynak_merkezi(hedef_y, kaynak.yukseklik, y);
        let y0 = fy.floor();
        let ty = fy - y0;
        for x in 0..hedef_g {
            let fx = kaynak_merkezi(hedef_g, kaynak.genislik, x);
            let x0 = fx.floor();
            let tx = fx - x0;
            let (x0, y0) = (x0 as i32, y0 as i32);
            let mut p = [0u8; 4];
            for (k, kanal) in p.iter_mut().enumerate() {
                let ust = kaynak.piksel(x0, y0)[k] as f32 * (1.0 - tx)
                    + kaynak.piksel(x0 + 1, y0)[k] as f32 * tx;
                let alt = kaynak.piksel(x0, y0 + 1)[k] as f32 * (1.0 - tx)
                    + kaynak.piksel(x0 + 1, y0 + 1)[k] as f32 * tx;
                *kanal = kirp(ust * (1.0 - ty) + alt * ty);
            }
            cikti.yaz(x, y, p);
        }
    }
    cikti
}

fn kirp(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

/// Lanczos çekirdeği.
pub fn lanczos_agirlik(x: f32) -> f32 {
    let a = LANCZOS_A as f32;
    let ax = x.abs();
    if ax < 1e-6 {
        return 1.0;
    }
    if ax >= a {
        return 0.0;
    }
    let px = std::f32::consts::PI * x;
    (a * px.sin() * (px / a).sin()) / (px * px)
}

/// Tek eksen için önceden hesaplanmış ağırlıklar.
///
/// Her çıktı koordinatı için ağırlıkları yeniden hesaplamak, aynı sinüsü
/// milyonlarca kez almak demek olurdu. Ayrılabilir (separable) uygulama da
/// bunun için: iki geçiş ve eksen başına `2a` katsayı; tek geçişte `2a * 2a`
/// olurdu.
fn agirliklar(kaynak: u32, hedef: u32) -> Vec<(i32, Vec<f32>)> {
    // Küçültmede çekirdek genişliyor: aksi halde atlanan kaynak pikselleri
    // hiç okunmaz ve örtüşme (aliasing) olurdu.
    let oran = (kaynak as f32 / hedef as f32).max(1.0);
    let destek = LANCZOS_A as f32 * oran;
    (0..hedef)
        .map(|i| {
            let merkez = kaynak_merkezi(hedef, kaynak, i);
            let ilk = (merkez - destek).ceil() as i32;
            let son = (merkez + destek).floor() as i32;
            let mut w: Vec<f32> = (ilk..=son)
                .map(|k| lanczos_agirlik((k as f32 - merkez) / oran))
                .collect();
            // Normalleştirme: kırpılan kuyruklar yüzünden toplam 1 olmuyor
            // ve düzeltilmezse görüntünün kenarları koyulaşırdı.
            let toplam: f32 = w.iter().sum();
            if toplam.abs() > 1e-6 {
                for v in &mut w {
                    *v /= toplam;
                }
            }
            (ilk, w)
        })
        .collect()
}

fn lanczos(kaynak: &Goruntu, hedef_g: u32, hedef_y: u32) -> Goruntu {
    let yatay = agirliklar(kaynak.genislik, hedef_g);
    let dikey = agirliklar(kaynak.yukseklik, hedef_y);

    // Ara tampon f32: iki geçiş arasında u8'e yuvarlamak, her kanalda yarım
    // birimlik bir hatayı ikinci geçişe taşırdı.
    let mut ara = vec![0f32; hedef_g as usize * kaynak.yukseklik as usize * 4];
    for y in 0..kaynak.yukseklik {
        for (x, (ilk, w)) in yatay.iter().enumerate() {
            let mut top = [0f32; 4];
            for (j, agirlik) in w.iter().enumerate() {
                let p = kaynak.piksel(ilk + j as i32, y as i32);
                for (k, t) in top.iter_mut().enumerate() {
                    *t += p[k] as f32 * agirlik;
                }
            }
            let i = (y as usize * hedef_g as usize + x) * 4;
            ara[i..i + 4].copy_from_slice(&top);
        }
    }

    let mut cikti = Goruntu::yeni(hedef_g, hedef_y);
    for (y, (ilk, w)) in dikey.iter().enumerate() {
        for x in 0..hedef_g as usize {
            let mut top = [0f32; 4];
            for (j, agirlik) in w.iter().enumerate() {
                let sy = (ilk + j as i32).clamp(0, kaynak.yukseklik as i32 - 1) as usize;
                let i = (sy * hedef_g as usize + x) * 4;
                for (k, t) in top.iter_mut().enumerate() {
                    *t += ara[i + k] * agirlik;
                }
            }
            cikti.yaz(
                x as u32,
                y as u32,
                [kirp(top[0]), kirp(top[1]), kirp(top[2]), kirp(top[3])],
            );
        }
    }
    cikti
}

/// İki rengin xBR'deki uzaklığı.
///
/// RGB farkı değil, ağırlıklı YUV farkı: kenar tespitinin insan gözünün
/// ayırdığı farkı izlemesi gerekiyor. Parlaklık (Y) baskın ağırlığı alıyor
/// çünkü göz kenarı önce parlaklıktan görüyor.
fn yuv_uzaklik(a: [u8; 4], b: [u8; 4]) -> f32 {
    // BGRA düzeni: [0]=B, [1]=G, [2]=R.
    let dr = a[2] as f32 - b[2] as f32;
    let dg = a[1] as f32 - b[1] as f32;
    let db = a[0] as f32 - b[0] as f32;
    let dy = 0.299 * dr + 0.587 * dg + 0.114 * db;
    let du = -0.169 * dr - 0.331 * dg + 0.5 * db;
    let dv = 0.5 * dr - 0.419 * dg - 0.081 * db;
    48.0 * dy.abs() + 7.0 * du.abs() + 6.0 * dv.abs()
}

fn karistir(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
    let mut c = [0u8; 4];
    for (k, kanal) in c.iter_mut().enumerate() {
        *kanal = kirp(a[k] as f32 * (1.0 - t) + b[k] as f32 * t);
    }
    c
}

/// Bir kaynak pikselin çevresi, dönebilen koordinatlarla.
///
/// Kural tek bir köşe için yazılıyor, dört köşe komşuluğun döndürülmesiyle
/// ele alınıyor. Kuralı dört kez elle yazmak, dördünün birbiriyle tutarlı
/// kaldığını da elle korumak demek olurdu — xBR uygulamalarının bilinen
/// hatalarının çoğu tam olarak orada.
struct Komsuluk<'a> {
    g: &'a Goruntu,
    x: i32,
    y: i32,
    donus: u8,
}

impl Komsuluk<'_> {
    fn p(&self, dx: i32, dy: i32) -> [u8; 4] {
        let (dx, dy) = match self.donus {
            0 => (dx, dy),
            1 => (-dy, dx),
            2 => (-dx, -dy),
            _ => (dy, -dx),
        };
        self.g.piksel(self.x + dx, self.y + dy)
    }
}

/// xBR'nin kenar kuralı: köşe pikseli ne olmalı?
///
/// `None` = kenar yok, köşe merkez pikselin kendisi kalıyor. Düz alanlarda
/// hiçbir şey değiştirmemesi xBR'nin bilinear'dan ayrıldığı yer ve
/// `testler::xbr_duz_alani_bozmuyor` bunu tutuyor.
///
/// Kural, xBR ailesinin kenar yönlü karşılaştırması: köşenin iki
/// köşegeninden hangisinin "daha ucuz" olduğu ağırlıklı uzaklık
/// toplamlarıyla ölçülüyor. Üst kaynakla piksel-birebir aynılık iddia
/// EDİLMİYOR — `decisions.md` #32.
fn kenar_karisimi(k: &Komsuluk) -> Option<[u8; 4]> {
    let (e, f, h, i) = (k.p(0, 0), k.p(1, 0), k.p(0, 1), k.p(1, 1));
    let (b, c, d, g) = (k.p(0, -1), k.p(1, -1), k.p(-1, 0), k.p(-1, 1));
    let (f4, i4, h5, i5) = (k.p(2, 0), k.p(2, 1), k.p(0, 2), k.p(1, 2));

    let u = yuv_uzaklik;
    // Kenarın E-I köşegeninde olduğu varsayımının maliyeti.
    let e_maliyet = u(e, c) + u(e, g) + u(i, h5) + u(i, f4) + 4.0 * u(h, f);
    // Kenarın F-H köşegeninde olduğu varsayımının maliyeti.
    let i_maliyet = u(h, d) + u(h, i5) + u(f, i4) + u(f, b) + 4.0 * u(e, i);

    if e_maliyet >= i_maliyet {
        return None;
    }
    // Köşeyi hangi komşunun rengiyle dolduracağız: E'ye yakın olan.
    let px = if u(e, f) <= u(e, h) { f } else { h };
    if e_maliyet * XBR_GUCLU_KAT < i_maliyet {
        // Güçlü kenar: köşe tamamen değişiyor. Köşegenin kesintisiz
        // görünmesinin tek yolu bu; yarı karışım basamağı geri getirirdi.
        Some(px)
    } else {
        Some(karistir(e, px, XBR_ZAYIF_KARISIM))
    }
}

/// xBR, sabit iki kat.
pub fn xbr2x(kaynak: &Goruntu) -> Goruntu {
    let mut cikti = Goruntu::yeni(kaynak.genislik * 2, kaynak.yukseklik * 2);
    // donus 0 sağ altı, 1 sol altı, 2 sol üstü, 3 sağ üstü dolduruyor:
    // (sağ, aşağı) yönü her 90 derecelik dönüşte bir sonraki köşeye geçiyor.
    let hedefler = [(1, 1), (0, 1), (0, 0), (1, 0)];
    for y in 0..kaynak.yukseklik as i32 {
        for x in 0..kaynak.genislik as i32 {
            let e = kaynak.piksel(x, y);
            for (donus, (ox, oy)) in hedefler.iter().enumerate() {
                let k = Komsuluk {
                    g: kaynak,
                    x,
                    y,
                    donus: donus as u8,
                };
                let renk = kenar_karisimi(&k).unwrap_or(e);
                cikti.yaz((x * 2 + ox) as u32, (y * 2 + oy) as u32, renk);
            }
        }
    }
    cikti
}

#[cfg(test)]
mod testler {
    use super::*;

    /// Test görüntüsü: köşegen sert kenar.
    fn kosegen(boyut: u32) -> Goruntu {
        let mut g = Goruntu::yeni(boyut, boyut);
        for y in 0..boyut {
            for x in 0..boyut {
                let renk = if x >= y {
                    [255, 255, 255, 255]
                } else {
                    [0, 0, 0, 255]
                };
                g.yaz(x, y, renk);
            }
        }
        g
    }

    /// Test görüntüsü: yatay yumuşak geçiş.
    fn gecis(boyut: u32) -> Goruntu {
        let mut g = Goruntu::yeni(boyut, boyut);
        for y in 0..boyut {
            for x in 0..boyut {
                let v = (x * 255 / (boyut - 1)) as u8;
                g.yaz(x, y, [v, v, v, 255]);
            }
        }
        g
    }

    fn psnr(a: &Goruntu, b: &Goruntu) -> f64 {
        assert_eq!(a.pikseller.len(), b.pikseller.len());
        let kare_hata: f64 = a
            .pikseller
            .iter()
            .zip(&b.pikseller)
            .map(|(x, y)| {
                let d = *x as f64 - *y as f64;
                d * d
            })
            .sum();
        let ort = kare_hata / a.pikseller.len() as f64;
        if ort < 1e-9 {
            return f64::INFINITY;
        }
        10.0 * (255.0f64 * 255.0 / ort).log10()
    }

    /// Kaynakta olmayan bir ton üretilmiş piksellerin oranı.
    ///
    /// Siyah-beyaz bir kaynakta ara ton = bulanıklık. Kenar koruma ölçütü
    /// olarak kullanılıyor.
    fn ara_ton_orani(g: &Goruntu) -> f64 {
        let ara = g
            .pikseller
            .chunks_exact(4)
            .filter(|p| p[0] > 8 && p[0] < 247)
            .count();
        ara as f64 / (g.pikseller.len() / 4) as f64
    }

    #[test]
    fn algoritma_anahtarlari_gidip_geliyor() {
        for a in Algoritma::hepsi() {
            assert_eq!(Algoritma::coz(a.anahtar()), Some(a));
            // Profil dosyasına yazılıp geri okunabilmeli.
            let json = serde_json::to_string(&a).unwrap();
            assert_eq!(json, format!("\"{}\"", a.anahtar()));
        }
        assert_eq!(Algoritma::coz("bilinmeyen"), None);
    }

    #[test]
    fn varsayilan_en_az_mudahale_eden() {
        // Varsayılan, kaynakta olmayan renk üretmeyen tek algoritma olmalı.
        assert_eq!(Algoritma::default(), Algoritma::TamSayi);
    }

    #[test]
    fn aciklamalarda_sayisal_vaat_yok() {
        // Tasarım ilkesi 4'ün otomatik kontrolü — `network_boost::tcp`deki
        // testin bu modüldeki karşılığı.
        let yasakli = [
            "% ",
            "fps kazan",
            "kat hızl",
            "garanti",
            "artırır",
            "daha iyi",
            "en iyi",
        ];
        for a in Algoritma::hepsi() {
            let metin = a.aciklama().to_lowercase();
            for y in yasakli {
                assert!(
                    !metin.contains(y),
                    "'{}' açıklaması vaat içeriyor: {y}",
                    a.ad()
                );
            }
        }
    }

    #[test]
    fn tam_sayi_yeni_renk_uretmiyor() {
        // Bu algoritmanın tek vaadi bu, ve vaadin testi bu.
        let k = kosegen(16);
        let c = olcekle(&k, Algoritma::TamSayi, 48, 48);
        assert_eq!((c.genislik, c.yukseklik), (48, 48));
        assert_eq!(ara_ton_orani(&c), 0.0);
    }

    #[test]
    fn tam_kat_asagi_yuvarliyor() {
        assert_eq!(tam_kat(1920, 1080, 2560, 1440), 1);
        assert_eq!(tam_kat(960, 540, 1920, 1080), 2);
        assert_eq!(tam_kat(640, 360, 1920, 1080), 3);
        // Hedef kaynaktan küçükse küçültme yok.
        assert_eq!(tam_kat(1920, 1080, 800, 600), 1);
    }

    #[test]
    fn en_boy_orani_korunuyor() {
        // 16:9 kaynak, 16:10 ekran: üstte ve altta siyah kalmalı.
        let y = en_boy_koru(1920, 1080, 1920, 1200);
        assert_eq!(y.genislik, 1920);
        assert_eq!(y.yukseklik, 1080);
        assert_eq!(y.x_ofset, 0);
        assert_eq!(y.y_ofset, 60);

        // 4:3 kaynak, 16:9 ekran: yanlarda siyah.
        let y = en_boy_koru(1024, 768, 1920, 1080);
        assert_eq!(y.yukseklik, 1080);
        assert_eq!(y.genislik, 1440);
        assert_eq!(y.x_ofset, 240);
        assert_eq!(y.y_ofset, 0);
    }

    #[test]
    fn olcekleme_boyutu_dogru() {
        let k = kosegen(32);
        for a in [Algoritma::Bilinear, Algoritma::Lanczos, Algoritma::Xbr] {
            let c = olcekle(&k, a, 96, 72);
            assert_eq!((c.genislik, c.yukseklik), (96, 72), "{}", a.ad());
            assert!(c.gecerli());
        }
    }

    #[test]
    fn bozuk_girdi_cokmuyor() {
        // Yakalama yarım kalmış bir kare verebilir; program çökmemeli.
        let bozuk = Goruntu {
            genislik: 10,
            yukseklik: 10,
            pikseller: vec![0; 7],
        };
        for a in Algoritma::hepsi() {
            let c = olcekle(&bozuk, a, 40, 40);
            assert!(c.gecerli());
        }
        let bos = Goruntu::yeni(4, 4);
        assert!(olcekle(&bos, Algoritma::Lanczos, 0, 0).gecerli());
    }

    /// Faz 3 kabul kriteri 1: kalite karşılaştırması.
    ///
    /// Yumuşak geçişli bir kaynakta Lanczos, bilinear'dan daha az hata
    /// yapmalı. Ölçüt: yarıya indirilip geri büyütülen görüntünün aslına
    /// olan PSNR'ı.
    #[test]
    fn lanczos_yumusak_gecise_bilineardan_yakin() {
        let asil = gecis(64);
        let kucuk = lanczos(&asil, 32, 32);
        let l = lanczos(&kucuk, 64, 64);
        let b = bilinear(&kucuk, 64, 64);
        assert!(
            psnr(&l, &asil) > psnr(&b, &asil),
            "Lanczos {:.2} dB, bilinear {:.2} dB",
            psnr(&l, &asil),
            psnr(&b, &asil)
        );
    }

    /// Faz 3 kabul kriteri 1: kenar korunuyor mu?
    ///
    /// Sert köşegen bir kenarda xBR, bilinear'dan belirgin olarak daha az
    /// ara ton üretmeli — "kenarı bulanıklaştırmadan büyütme" iddiasının
    /// ölçülebilir hali.
    #[test]
    fn xbr_kenari_bilineardan_az_bulandiriyor() {
        let k = kosegen(32);
        let x = olcekle(&k, Algoritma::Xbr, 64, 64);
        let b = olcekle(&k, Algoritma::Bilinear, 64, 64);
        let (ax, ab) = (ara_ton_orani(&x), ara_ton_orani(&b));
        assert!(ax < ab, "xBR {ax:.4}, bilinear {ab:.4}");
    }

    #[test]
    fn xbr_duz_alani_bozmuyor() {
        // Tek renkli bir alanda kenar yok: çıktı kaynakla aynı olmalı.
        // Değilse algoritma görüntüye kendiliğinden gürültü ekliyor.
        let mut duz = Goruntu::yeni(8, 8);
        for y in 0..8 {
            for x in 0..8 {
                duz.yaz(x, y, [30, 90, 200, 255]);
            }
        }
        let c = xbr2x(&duz);
        assert!(c.pikseller.chunks_exact(4).all(|p| p == [30, 90, 200, 255]));
    }

    #[test]
    fn xbr_iki_kat_uretiyor() {
        let k = kosegen(9);
        let c = xbr2x(&k);
        assert_eq!((c.genislik, c.yukseklik), (18, 18));
    }

    #[test]
    fn lanczos_agirligi_bilinen_noktalarda() {
        assert!((lanczos_agirlik(0.0) - 1.0).abs() < 1e-6);
        // Tam sayı uzaklıklarda sıfır (sinc'in kökleri).
        for k in 1..LANCZOS_A {
            assert!(lanczos_agirlik(k as f32).abs() < 1e-5, "k={k}");
        }
        // Yarıçapın dışında sıfır.
        assert_eq!(lanczos_agirlik(LANCZOS_A as f32 + 0.5), 0.0);
        // Birinci lobun dışı negatif: keskinliğin geldiği yer.
        assert!(lanczos_agirlik(1.5) < 0.0);
    }

    #[test]
    fn agirliklar_bire_toplaniyor() {
        // Toplam 1 değilse görüntünün parlaklığı ölçekleme sırasında kayar.
        for (kaynak, hedef) in [(64u32, 128u32), (128, 64), (100, 137)] {
            for (_, w) in agirliklar(kaynak, hedef) {
                let t: f32 = w.iter().sum();
                assert!((t - 1.0).abs() < 1e-4, "{kaynak}->{hedef}: {t}");
            }
        }
    }

    /// CPU yolunun gerçek zamanlı olmadığının ölçümü.
    ///
    /// `decisions.md` #32'nin dayandığı sayı buradan geliyor ve iddia
    /// değil ölçüm olsun diye tekrarlanabilir bırakıldı:
    ///
    /// ```text
    /// cargo test --release cpu_yolunun_maliyeti -- --ignored --nocapture
    /// ```
    ///
    /// Varsayılan olarak koşmuyor: sonuç makineye bağlı ve bir eşiğe
    /// bağlanırsa test, ölçtüğü şeyi değil koştuğu makineyi sınar.
    #[test]
    #[ignore = "ölçüm; makineye bağlı"]
    fn cpu_yolunun_maliyeti() {
        let kaynak = gecis(1920);
        let kaynak = Goruntu {
            genislik: 1920,
            yukseklik: 1080,
            pikseller: kaynak.pikseller[..1920 * 1080 * 4].to_vec(),
        };
        for a in Algoritma::hepsi() {
            let t = std::time::Instant::now();
            let _ = olcekle(&kaynak, a, 2560, 1440);
            println!(
                "{:>14}: {:>8.2} ms/kare",
                a.ad(),
                t.elapsed().as_secs_f64() * 1000.0
            );
        }
        println!("60 FPS'in kare bütçesi: 16.67 ms");
    }

    #[test]
    fn pencere_alani_ekrana_kirpiliyor() {
        // Pencere ekranın soluna taşıyor: taşan kısım atılıyor.
        let a = pencere_alani((-100, 50, 700, 450), (0, 0), (1920, 1080)).unwrap();
        assert_eq!(
            a,
            Alan {
                x: 0,
                y: 50,
                genislik: 700,
                yukseklik: 400
            }
        );

        // İkinci ekran: masaüstü kökeni (1920, 0).
        let a = pencere_alani((2000, 100, 2600, 500), (1920, 0), (1920, 1080)).unwrap();
        assert_eq!(
            a,
            Alan {
                x: 80,
                y: 100,
                genislik: 600,
                yukseklik: 400
            }
        );

        // Pencere ekranın sağına taşıyor.
        let a = pencere_alani((1800, 0, 2200, 300), (0, 0), (1920, 1080)).unwrap();
        assert_eq!(a.genislik, 120);
    }

    #[test]
    fn kucultulmus_pencere_alan_vermiyor() {
        // Küçültülmüş pencerenin dikdörtgeni boş/negatif geliyor. Sıfır
        // boyutlu bir alana ölçeklemek ekranı karartırdı.
        assert!(pencere_alani((0, 0, 0, 0), (0, 0), (1920, 1080)).is_none());
        // Başka bir ekranda duran pencere.
        assert!(pencere_alani((3000, 0, 3500, 400), (0, 0), (1920, 1080)).is_none());
        // Bir tutamak kadar pencere.
        assert!(pencere_alani((10, 10, 14, 14), (0, 0), (1920, 1080)).is_none());
    }

    #[test]
    fn tam_alan_dokunun_tamami() {
        assert_eq!(
            Alan::tam(1920, 1080),
            Alan {
                x: 0,
                y: 0,
                genislik: 1920,
                yukseklik: 1080
            }
        );
    }

    #[test]
    fn yuv_uzakligi_simetrik() {
        let a = [10, 20, 30, 255];
        let b = [200, 100, 50, 255];
        assert!((yuv_uzaklik(a, b) - yuv_uzaklik(b, a)).abs() < 1e-3);
        assert_eq!(yuv_uzaklik(a, a), 0.0);
        assert!(yuv_uzaklik(a, b) > 0.0);
    }
}
