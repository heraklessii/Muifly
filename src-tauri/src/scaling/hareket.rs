//! Kare üretiminin **CPU referansı**: hareket tahmini ve ara kare warp'ı.
//!
//! Çalışma zamanında kullanılmıyor. Gerçek zamanlı yol `uretim.hlsl`;
//! burası o gölgelendiricinin ne yapması gerektiğini tanımlayan ve
//! **test edilebilen** taraf. `algoritma.rs`'in ölçekleme için oynadığı
//! rolün aynısı (karar #32'nin kurduğu düzen, karar #35 bunu sürdürüyor).
//!
//! # Neden CPU referansı şart
//!
//! Kare üretiminin doğru çalıştığı gözle anlaşılmıyor. Yanlış bir hareket
//! vektörü ekranda "biraz bulanık" görünür, hata gibi durmaz; algoritma
//! tamamen bozuk olsa bile görüntü akmaya devam eder. Bu yüzden doğruluk
//! **sentetik girdiyle** ölçülüyor: bilinen bir kaydırma uygulanmış iki
//! kare veriliyor, çıkan vektörün o kaydırma olması bekleniyor. Bu testler
//! gölgelendiriciyi doğrudan denemiyor ama gölgelendiricinin uyması gereken
//! sözleşmeyi yazıyor.
//!
//! # Ne yapmıyor
//!
//! Burada ML yok. Kare üretimi ML **gerektirmiyor** — referans aldığımız
//! Lossless Scaling'in ilk kare üreteci de klasik bir algoritmaydı. ML
//! yolunun ne getireceği ve neye mal olacağı ayrı bir belgede
//! (`docs/FRAME_GENERATION.md`), çünkü o bir kalite yükseltmesi, bu
//! modülün yerine geçecek bir şey değil.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Sabitler — gölgelendiricideki eşleriyle AYNI olmak zorunda
// ---------------------------------------------------------------------------

/// Bir hareket bloğunun tam çözünürlükteki kenarı (piksel).
///
/// 16, iki şeyin ortası: küçük bloklar gürültüde yanlış eşleşiyor (düz bir
/// duvarda her yer her yere benziyor), büyük bloklar içinde birden fazla
/// hareket eden nesne kalıyor ve ikisinin ortalamasını üretiyor.
pub const BLOK: u32 = 16;

/// Arama piramidinin seviye sayısı: tam, 1/2, 1/4 çözünürlük.
///
/// İki seviye denendi ve **yetmedi**: kaba seviye vektörü 4 pikselin katına
/// yuvarlıyor, ince tur da o yuvarlamanın yanlış tarafına düştüğünde
/// gerçek hareketi bir daha yakalayamıyordu. Ortadaki seviye tam olarak bu
/// yuvarlamayı düzeltmek için var.
pub const SEVIYE: usize = 3;

/// Seviye başına arama yarıçapı — `[tam, yarı, çeyrek]`, kendi seviyesinin
/// piksel biriminde.
///
/// Kaba seviyedeki 6, tam çözünürlükte ±24 piksellik bir hareket sınırı
/// demek. Bu sınır bilinçli: sınırsız arama, kare başına maliyeti hareketin
/// hızına bağlardı — yani en çok ihtiyaç duyulan anda en yavaş çalışırdı.
/// Sınırın dışında kalan hareket için vektör bulunamıyor, güven düşüyor ve
/// o blok karışım yerine en yakın gerçek kareye düşüyor.
///
/// Alt seviyelerdeki yarıçaplar 1 **değil**, çünkü indirgeme kutu
/// ortalamasıyla yapılıyor ve tek sayılı bir kaydırma indirgenmiş
/// görüntüde temiz bir kaydırma olarak görünmüyor. Üst seviye böyle bir
/// harekette bir-iki piksel yanılıyor; dar bir düzeltme penceresi o
/// yanılgıyı bir daha kapatamıyordu. `bilinen_kaydirma_bulunuyor` testi
/// tam olarak bunu yakaladı: 5 piksellik kaydırma 3 olarak ölçülüyordu.
pub const YARICAP: [i32; SEVIYE] = [2, 3, 6];

/// Kaba seviyenin indirgeme katı (`2^(SEVIYE-1)`).
pub const KABA_KAT: u32 = 1 << (SEVIYE - 1);

/// Karışım yerine tek kareye düşme eşiği.
///
/// İki yönden warp edilen örnekler bu kadar ayrışıyorsa orada bir örtüşme
/// (occlusion) var: bir nesnenin arkasından çıkan piksel önceki karede
/// **yok**. Ortalamasını almak orada hayalet bir görüntü üretir; bunun
/// yerine zaman olarak yakın olan kareden alınıyor.
pub const ORTUSME_ESIGI: f32 = 0.18;

// ---------------------------------------------------------------------------
// Veri tipleri
// ---------------------------------------------------------------------------

/// Tek kanallı (parlaklık) görüntü. Hareket tahmini renk kullanmıyor.
///
/// Renk kullanılsaydı maliyet üçe katlanır, karşılığında kazanılan şey
/// yalnızca eşit parlaklıkta farklı renkli yüzeylerin ayrılması olurdu —
/// oyun görüntüsünde nadir bir durum.
#[derive(Debug, Clone, PartialEq)]
pub struct Gri {
    pub genislik: u32,
    pub yukseklik: u32,
    /// Satır sırasına göre, 0.0–1.0.
    pub piksel: Vec<f32>,
}

impl Gri {
    pub fn yeni(genislik: u32, yukseklik: u32) -> Self {
        Self {
            genislik,
            yukseklik,
            piksel: vec![0.0; (genislik * yukseklik) as usize],
        }
    }

    /// Kenarda kırpan okuma. Sıfır döndürmek, kenarlarda olmayan bir
    /// karanlık kenar uydurup hareket tahminini oraya çekerdi.
    pub fn oku(&self, x: i32, y: i32) -> f32 {
        let x = x.clamp(0, self.genislik as i32 - 1) as u32;
        let y = y.clamp(0, self.yukseklik as i32 - 1) as u32;
        self.piksel[(y * self.genislik + x) as usize]
    }

    pub fn yaz(&mut self, x: u32, y: u32, deger: f32) {
        if x < self.genislik && y < self.yukseklik {
            self.piksel[(y * self.genislik + x) as usize] = deger;
        }
    }

    /// İki doğrusal (bilinear) örnekleme. Warp'ın alt piksel doğruluğu
    /// buradan geliyor; en yakın komşuyla warp, hareketi 1 piksellik
    /// basamaklara yuvarlayıp titreme üretirdi.
    ///
    /// Koordinat sürekli uzayda: `x` piksel **merkezleri** 0.5, 1.5, …
    /// konumlarında. Yarım piksel çıkarılmasının sebebi bu — çıkarılmasaydı
    /// hareketsiz bir görüntü bile her karede yarım piksel kayar, yani
    /// kare üretimi açıldığı an ekran hafifçe bulanırdı. Gölgelendiricideki
    /// donanım örnekleyicisi aynı konvansiyonu kullanıyor.
    pub fn ornekle(&self, x: f32, y: f32) -> f32 {
        let x = x - 0.5;
        let y = y - 0.5;
        let x0 = x.floor();
        let y0 = y.floor();
        let fx = x - x0;
        let fy = y - y0;
        let (x0, y0) = (x0 as i32, y0 as i32);
        let a = self.oku(x0, y0);
        let b = self.oku(x0 + 1, y0);
        let c = self.oku(x0, y0 + 1);
        let d = self.oku(x0 + 1, y0 + 1);
        (a * (1.0 - fx) + b * fx) * (1.0 - fy) + (c * (1.0 - fx) + d * fx) * fy
    }
}

/// Blok başına bir hareket vektörü (tam çözünürlük piksel cinsinden).
///
/// Vektör, **önceki kareden sonraki kareye** olan hareket. Ara kare
/// üretilirken ikiye bölünüyor: önceki ileri, sonraki geri warp ediliyor.
#[derive(Debug, Clone, PartialEq)]
pub struct HareketAlani {
    pub sutun: u32,
    pub satir: u32,
    pub vektor: Vec<[f32; 2]>,
    /// Blok başına eşleşme güveni (0–1). Düşükse warp yerine karışım.
    pub guven: Vec<f32>,
}

impl HareketAlani {
    pub fn yeni(sutun: u32, satir: u32) -> Self {
        Self {
            sutun,
            satir,
            vektor: vec![[0.0, 0.0]; (sutun * satir) as usize],
            guven: vec![0.0; (sutun * satir) as usize],
        }
    }

    pub fn al(&self, sx: i32, sy: i32) -> [f32; 2] {
        let sx = sx.clamp(0, self.sutun as i32 - 1) as u32;
        let sy = sy.clamp(0, self.satir as i32 - 1) as u32;
        self.vektor[(sy * self.sutun + sx) as usize]
    }

    pub fn guven_al(&self, sx: i32, sy: i32) -> f32 {
        let sx = sx.clamp(0, self.sutun as i32 - 1) as u32;
        let sy = sy.clamp(0, self.satir as i32 - 1) as u32;
        self.guven[(sy * self.sutun + sx) as usize]
    }

    /// Blok ızgarasını piksel konumunda yumuşak okuma.
    ///
    /// Izgara doğrudan okunsaydı blok sınırlarında vektör bir anda
    /// değişir ve warp edilen görüntüde 16 pikselde bir görünür bir
    /// basamak oluşurdu.
    pub fn ornekle(&self, x: f32, y: f32) -> [f32; 2] {
        // Blok merkezleri ızgaranın yarım blok içinde.
        let gx = x / BLOK as f32 - 0.5;
        let gy = y / BLOK as f32 - 0.5;
        let x0 = gx.floor();
        let y0 = gy.floor();
        let fx = gx - x0;
        let fy = gy - y0;
        let (ix, iy) = (x0 as i32, y0 as i32);
        let a = self.al(ix, iy);
        let b = self.al(ix + 1, iy);
        let c = self.al(ix, iy + 1);
        let d = self.al(ix + 1, iy + 1);
        [
            (a[0] * (1.0 - fx) + b[0] * fx) * (1.0 - fy) + (c[0] * (1.0 - fx) + d[0] * fx) * fy,
            (a[1] * (1.0 - fx) + b[1] * fx) * (1.0 - fy) + (c[1] * (1.0 - fx) + d[1] * fx) * fy,
        ]
    }
}

// ---------------------------------------------------------------------------
// Adım 1 — indirgeme
// ---------------------------------------------------------------------------

/// Görüntüyü verilen katta küçültür (kutu ortalaması).
///
/// Kutu ortalaması, tek örnek almaya tercih edildi: tek örnek alsaydık
/// ince desenli bir yüzeyde (çim, tuğla) örtüşme (aliasing) oluşur ve kaba
/// arama gerçek olmayan bir hareket bulurdu.
pub fn indirge(kaynak: &Gri, kat: u32) -> Gri {
    let kat = kat.max(1);
    let g = (kaynak.genislik / kat).max(1);
    let y = (kaynak.yukseklik / kat).max(1);
    let mut cikti = Gri::yeni(g, y);
    for sy in 0..y {
        for sx in 0..g {
            let mut toplam = 0.0;
            for dy in 0..kat {
                for dx in 0..kat {
                    toplam += kaynak.oku((sx * kat + dx) as i32, (sy * kat + dy) as i32);
                }
            }
            cikti.yaz(sx, sy, toplam / (kat * kat) as f32);
        }
    }
    cikti
}

// ---------------------------------------------------------------------------
// Adım 2 — blok eşleme
// ---------------------------------------------------------------------------

/// Bir blok konumunda iki görüntü arasındaki mutlak fark toplamı (SAD).
fn fark(onceki: &Gri, sonraki: &Gri, bx: i32, by: i32, blok: i32, dx: i32, dy: i32) -> f32 {
    let mut toplam = 0.0;
    for y in 0..blok {
        for x in 0..blok {
            let a = onceki.oku(bx + x, by + y);
            let b = sonraki.oku(bx + x + dx, by + y + dy);
            toplam += (a - b).abs();
        }
    }
    toplam / (blok * blok) as f32
}

/// İki kare arasındaki hareket alanını çıkarır.
///
/// Piramitli arama: en kaba seviyede (1/4) geniş bir arama yapılıyor,
/// bulunan vektör bir alt seviyeye ikiyle çarpılarak taşınıyor ve orada
/// dar bir aramayla düzeltiliyor. Tam çözünürlükte geniş arama aynı sonucu
/// verirdi ama blok başına yüzlerce kat pahalıya; tek başına kaba arama
/// ise hareketi 4 pikselin katına yuvarlardı.
///
/// Bu fonksiyon CPU referansı: her seviye için görüntünün kendi kopyasını
/// üretiyor. Gölgelendirici tarafında aynı piramit üç ayrı geçişte, doku
/// olarak tutuluyor (`uretim.hlsl`).
pub fn hareket_alani(onceki: &Gri, sonraki: &Gri) -> HareketAlani {
    let sutun = (onceki.genislik.div_ceil(BLOK)).max(1);
    let satir = (onceki.yukseklik.div_ceil(BLOK)).max(1);
    let mut alan = HareketAlani::yeni(sutun, satir);

    // Piramit: indeks 0 tam çözünürlük, 1 yarı, 2 çeyrek.
    let piramit = |g: &Gri| -> Vec<Gri> {
        (0..SEVIYE)
            .map(|s| {
                if s == 0 {
                    g.clone()
                } else {
                    indirge(g, 1 << s)
                }
            })
            .collect()
    };
    let p_onceki = piramit(onceki);
    let p_sonraki = piramit(sonraki);

    for sy in 0..satir {
        for sx in 0..sutun {
            let i = (sy * sutun + sx) as usize;
            let mut vektor = (0i32, 0i32);
            let mut son_fark = f32::MAX;

            // Kabadan inceye.
            for seviye in (0..SEVIYE).rev() {
                let kat = 1u32 << seviye;
                let blok = (BLOK / kat).max(1) as i32;
                let bx = (sx * (BLOK / kat)) as i32;
                let by = (sy * (BLOK / kat)) as i32;
                // Üst seviyenin vektörü bu seviyenin birimine taşınıyor.
                let merkez = if seviye == SEVIYE - 1 {
                    (0, 0)
                } else {
                    (vektor.0 * 2, vektor.1 * 2)
                };

                let yaricap = YARICAP[seviye];
                let mut en_iyi = merkez;
                let mut en_iyi_fark = f32::MAX;
                for dy in -yaricap..=yaricap {
                    for dx in -yaricap..=yaricap {
                        let aday = (merkez.0 + dx, merkez.1 + dy);
                        let f = fark(
                            &p_onceki[seviye],
                            &p_sonraki[seviye],
                            bx,
                            by,
                            blok,
                            aday.0,
                            aday.1,
                        );
                        // Eşitlikte merkeze yakın olan kazanıyor: düz bir
                        // yüzeyde bütün konumlar aynı farkı verir ve
                        // "hareket yok" cevabı, rastgele bir yöne
                        // kaymaktan doğrudur.
                        if f < en_iyi_fark - 1e-6 {
                            en_iyi_fark = f;
                            en_iyi = aday;
                        }
                    }
                }
                vektor = en_iyi;
                son_fark = en_iyi_fark;
            }

            alan.vektor[i] = [vektor.0 as f32, vektor.1 as f32];
            // Güven: eşleşme farkı ne kadar küçükse o kadar yüksek.
            // Doğrusal ve kalibre edilmemiş bir ölçü; bir olasılık değil,
            // yalnızca "bu blok diğerinden daha iyi eşleşti" sıralaması.
            alan.guven[i] = (1.0 - (son_fark / ORTUSME_ESIGI).min(1.0)).clamp(0.0, 1.0);
        }
    }
    alan
}

// ---------------------------------------------------------------------------
// Adım 3 — alanı düzeltme
// ---------------------------------------------------------------------------

/// Hareket alanındaki tek tük yanlış vektörleri komşularıyla düzeltir.
///
/// Blok eşleme yerel bir karar veriyor ve düz yüzeylerde yanılıyor. Tek
/// başına yanlış duran bir vektör, ara karede o blok kadar bir bölgenin
/// yanlış yere kaymasına — yani gözle görülür bir "sıçrayan kare"ye —
/// yol açıyor. Ortanca (median) seçiliyor, ortalama değil: ortalama, bir
/// yanlış vektörü komşularına bulaştırırdı.
pub fn duzelt(alan: &HareketAlani) -> HareketAlani {
    let mut cikti = alan.clone();
    for sy in 0..alan.satir as i32 {
        for sx in 0..alan.sutun as i32 {
            let mut xs = Vec::with_capacity(9);
            let mut ys = Vec::with_capacity(9);
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let v = alan.al(sx + dx, sy + dy);
                    xs.push(v[0]);
                    ys.push(v[1]);
                }
            }
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            ys.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let i = (sy as u32 * alan.sutun + sx as u32) as usize;
            cikti.vektor[i] = [xs[xs.len() / 2], ys[ys.len() / 2]];
        }
    }
    cikti
}

// ---------------------------------------------------------------------------
// Adım 4 — ara kare
// ---------------------------------------------------------------------------

/// `t` anındaki ara kareyi üretir (`t` = 0 önceki, 1 sonraki).
///
/// Çift yönlü warp: önceki kare ileri, sonraki kare geri taşınıyor ve
/// ikisi karıştırılıyor. Tek yönlü warp (yalnızca önceki kareyi ileri
/// taşımak) daha ucuz olurdu ama nesnenin **arkasında açılan** boşluğu
/// doldurabilecek bir kaynağı olmazdı; o boşluk her karede esneyen bir iz
/// bırakır.
///
/// İki yön birbirini tutmuyorsa (örtüşme) karışım yapılmıyor, zaman olarak
/// yakın olan kare seçiliyor. Görünen sonuç: örtüşen bölgede ara kare bir
/// gerçek kareye eşit oluyor — yani "yanlış" değil, "üretilmemiş".
pub fn ara_kare(onceki: &Gri, sonraki: &Gri, alan: &HareketAlani, t: f32) -> Gri {
    let t = t.clamp(0.0, 1.0);
    let mut cikti = Gri::yeni(onceki.genislik, onceki.yukseklik);
    for y in 0..onceki.yukseklik {
        for x in 0..onceki.genislik {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let v = alan.ornekle(px, py);

            // Önceki kareden ileri, sonraki kareden geri.
            let a = onceki.ornekle(px - v[0] * t, py - v[1] * t);
            let b = sonraki.ornekle(px + v[0] * (1.0 - t), py + v[1] * (1.0 - t));

            let ayrisma = (a - b).abs();
            let deger = if ayrisma > ORTUSME_ESIGI {
                // Örtüşme: karıştırma, yakın olanı al.
                if t < 0.5 {
                    a
                } else {
                    b
                }
            } else {
                a * (1.0 - t) + b * t
            };
            cikti.yaz(x, y, deger);
        }
    }
    cikti
}

// ---------------------------------------------------------------------------
// Sunuma giden ayarlar
// ---------------------------------------------------------------------------

/// Kare üretiminin kullanıcıya görünen ayarı.
///
/// Şu an tek bir çarpan var ve o da 2x: iki gerçek kare arasına bir kare.
/// 3x/4x, aynı iki kareden birden fazla ara kare üretmek demek ve her ek
/// kare aynı hareket alanına dayandığı için hata da onunla birlikte
/// çoğalıyor. Ayrıca beklemesi gereken süre değişmiyor, yani gecikme
/// bedeli aynı kalırken görüntü kalitesi düşüyor. Tek çarpanla başlamak,
/// olmayan bir seçeneği menüye koymamak demek (`decisions.md` #35).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Carpan {
    /// İki gerçek kare arasına bir üretilmiş kare.
    Iki,
}

impl Carpan {
    /// Bir gerçek kare çifti başına üretilen kare sayısı.
    pub fn ara_kare_sayisi(&self) -> u32 {
        match self {
            Carpan::Iki => 1,
        }
    }
}

// ---------------------------------------------------------------------------
// Testler
// ---------------------------------------------------------------------------

#[cfg(test)]
mod testler {
    use super::*;

    /// Test deseni: periyodu olmayan, hafifçe yumuşatılmış sözde rastgele
    /// görüntü.
    ///
    /// İlk denemede `(x*7 + y*13) % 37` gibi aritmetik bir desen
    /// kullanılmıştı ve **testi yanlış yere baktırdı**: o desende (15, 9)
    /// kaydırması görüntüyü birebir tekrar ediyor (7·15 + 13·9 = 222 =
    /// 6·37), yani blok eşleme için mükemmel bir sahte eşleşme var. Testler
    /// algoritmayı suçladı, kusur desendeydi. Karıştırıcı (hash) tabanlı
    /// desende böyle bir periyot yok.
    ///
    /// Düz bir gradyan da olmaz: orada blok eşleme her konumda benzer fark
    /// bulur ve test, algoritma bozuk olsa bile geçerdi.
    ///
    /// Yumuşatma şart, çünkü saf gürültüde alt piksel örnekleme anlamını
    /// yitirir — komşu pikseller ilişkisizken ara değerin doğru olup
    /// olmadığı ölçülemez.
    fn desen(genislik: u32, yukseklik: u32) -> Gri {
        let mut ham = Gri::yeni(genislik, yukseklik);
        for y in 0..yukseklik {
            for x in 0..genislik {
                let mut h = x
                    .wrapping_mul(374_761_393)
                    .wrapping_add(y.wrapping_mul(668_265_263));
                h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
                h ^= h >> 16;
                ham.yaz(x, y, (h & 0xffff) as f32 / 65535.0);
            }
        }
        let mut g = Gri::yeni(genislik, yukseklik);
        for y in 0..yukseklik {
            for x in 0..genislik {
                let mut t = 0.0;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        t += ham.oku(x as i32 + dx, y as i32 + dy);
                    }
                }
                g.yaz(x, y, t / 9.0);
            }
        }
        g
    }

    /// Deseni tam sayı kadar kaydırır.
    fn kaydir(kaynak: &Gri, dx: i32, dy: i32) -> Gri {
        let mut c = Gri::yeni(kaynak.genislik, kaynak.yukseklik);
        for y in 0..kaynak.yukseklik {
            for x in 0..kaynak.genislik {
                c.yaz(x, y, kaynak.oku(x as i32 - dx, y as i32 - dy));
            }
        }
        c
    }

    /// Kenardan uzak blokların ortalama vektörü.
    ///
    /// Kenar blokları dışarıda kalan bilgiyi göremiyor ve orada yanlış
    /// vektör bulmak beklenen davranış; test onları dışarıda bırakıyor.
    fn ic_ortalama(alan: &HareketAlani) -> [f32; 2] {
        let mut toplam = [0.0f32; 2];
        let mut adet = 0;
        for sy in 2..alan.satir as i32 - 2 {
            for sx in 2..alan.sutun as i32 - 2 {
                let v = alan.al(sx, sy);
                toplam[0] += v[0];
                toplam[1] += v[1];
                adet += 1;
            }
        }
        assert!(adet > 0, "test görüntüsü çok küçük");
        [toplam[0] / adet as f32, toplam[1] / adet as f32]
    }

    /// Bilinen bir kaydırma, bilinen bir vektör vermeli.
    ///
    /// Bu, modülün ana testi: geçmezse hareket tahmini çalışmıyor
    /// demektir ve ara kareler anlamsız olur.
    #[test]
    fn bilinen_kaydirma_bulunuyor() {
        for (dx, dy) in [(5, 0), (0, -4), (7, 3), (-6, -5), (12, 9)] {
            let a = desen(256, 192);
            let b = kaydir(&a, dx, dy);
            let alan = duzelt(&hareket_alani(&a, &b));
            let ort = ic_ortalama(&alan);
            assert!(
                (ort[0] - dx as f32).abs() < 1.0 && (ort[1] - dy as f32).abs() < 1.0,
                "kaydırma ({dx},{dy}) için bulunan vektör {ort:?}"
            );
        }
    }

    /// Hareket yoksa vektör de yok.
    ///
    /// "Sıfır hareket" durumunda uydurma bir vektör bulmak, duran bir
    /// ekranda titreme üretirdi — kullanıcının göreceği ilk kusur bu olurdu.
    #[test]
    fn hareketsiz_karede_vektor_sifir() {
        let a = desen(192, 128);
        let alan = duzelt(&hareket_alani(&a, &a));
        for v in &alan.vektor {
            assert_eq!(*v, [0.0, 0.0], "hareketsiz karede vektör bulundu: {v:?}");
        }
    }

    /// Hareketsiz karenin ara karesi, karenin kendisi olmalı.
    #[test]
    fn hareketsiz_ara_kare_ayni() {
        let a = desen(128, 96);
        let alan = hareket_alani(&a, &a);
        let ara = ara_kare(&a, &a, &alan, 0.5);
        for (i, (x, y)) in ara.piksel.iter().zip(a.piksel.iter()).enumerate() {
            assert!((x - y).abs() < 1e-4, "piksel {i}: {x} != {y}");
        }
    }

    /// t=0.5'teki ara kare, yarı yolda kaydırılmış görüntüye yakın olmalı.
    ///
    /// Bu testin ölçtüğü şey kare üretiminin **işe yarayıp yaramadığı**:
    /// üretilen kare, hiç üretmeyip önceki kareyi tekrarlamaktan belirgin
    /// olarak daha yakın olmalı. Değilse üretim bedeline değmiyor demektir.
    #[test]
    fn ara_kare_yari_yolda() {
        let a = desen(256, 192);
        let b = kaydir(&a, 8, 4);
        let beklenen = kaydir(&a, 4, 2);
        let alan = duzelt(&hareket_alani(&a, &b));
        let ara = ara_kare(&a, &b, &alan, 0.5);

        let hata = |g: &Gri| -> f32 {
            let mut t = 0.0;
            let mut n = 0;
            // Kenarlar hariç: orada kaydırma dışarıdan bilgi getiriyor.
            for y in 24..g.yukseklik - 24 {
                for x in 24..g.genislik - 24 {
                    t += (g.oku(x as i32, y as i32) - beklenen.oku(x as i32, y as i32)).abs();
                    n += 1;
                }
            }
            t / n as f32
        };

        let uretilen = hata(&ara);
        let tekrar = hata(&a);
        assert!(
            uretilen < tekrar * 0.5,
            "üretilen kare tekrardan yeterince iyi değil: {uretilen} vs {tekrar}"
        );
    }

    /// Arama yarıçapının dışındaki hareket sessizce yanlış cevap vermemeli.
    ///
    /// Sınırın ötesinde doğru vektörü bulmak imkânsız; beklenen davranış
    /// güvenin düşmesi. Güven yüksek kalsaydı ara kare, bulunamamış bir
    /// hareketi bulunmuş gibi kullanırdı.
    #[test]
    fn sinir_disi_hareket_guveni_dusuruyor() {
        let a = desen(256, 192);
        let yakin = kaydir(&a, 6, 0);
        let uzak = kaydir(&a, 60, 0);

        let ic_guven = |alan: &HareketAlani| {
            let mut t = 0.0;
            let mut n = 0;
            for sy in 2..alan.satir as i32 - 2 {
                for sx in 2..alan.sutun as i32 - 2 {
                    t += alan.guven_al(sx, sy);
                    n += 1;
                }
            }
            t / n as f32
        };

        let g_yakin = ic_guven(&hareket_alani(&a, &yakin));
        let g_uzak = ic_guven(&hareket_alani(&a, &uzak));
        assert!(
            g_uzak < g_yakin,
            "sınır dışı hareket güveni düşürmedi: yakın {g_yakin}, uzak {g_uzak}"
        );
    }

    /// Izgara okuması blok sınırında sıçramıyor.
    ///
    /// Sıçrasaydı ara karede 16 pikselde bir görünür bir dikey/yatay çizgi
    /// oluşurdu — kare üretiminin en tipik görsel kusuru.
    #[test]
    fn izgara_ornegi_surekli() {
        let mut alan = HareketAlani::yeni(8, 8);
        for (i, v) in alan.vektor.iter_mut().enumerate() {
            *v = [(i % 8) as f32, (i / 8) as f32];
        }
        let mut onceki = alan.ornekle(0.5, 40.0);
        for x in 1..120 {
            let simdi = alan.ornekle(x as f32 + 0.5, 40.0);
            let atlama = (simdi[0] - onceki[0]).abs();
            assert!(atlama < 0.2, "x={x} konumunda vektör sıçradı: {atlama}");
            onceki = simdi;
        }
    }

    /// İndirgeme boyutu ve ortalaması doğru.
    #[test]
    fn indirgeme_kutu_ortalamasi() {
        let mut g = Gri::yeni(4, 4);
        for y in 0..4 {
            for x in 0..4 {
                g.yaz(x, y, if (x + y) % 2 == 0 { 1.0 } else { 0.0 });
            }
        }
        let k = indirge(&g, 2);
        assert_eq!((k.genislik, k.yukseklik), (2, 2));
        for p in &k.piksel {
            assert!((p - 0.5).abs() < 1e-6, "kutu ortalaması yanlış: {p}");
        }
    }

    /// Ortanca düzeltme tek tük yanlış vektörü temizliyor.
    #[test]
    fn duzeltme_aykiri_vektoru_atiyor() {
        let mut alan = HareketAlani::yeni(5, 5);
        for v in alan.vektor.iter_mut() {
            *v = [3.0, 1.0];
        }
        // Ortadaki blok saçmalıyor.
        alan.vektor[12] = [-40.0, 25.0];
        let temiz = duzelt(&alan);
        assert_eq!(
            temiz.al(2, 2),
            [3.0, 1.0],
            "aykırı vektör komşularıyla düzeltilmedi"
        );
    }

    /// Çarpan tek seçenekli ve bir ara kare üretiyor.
    ///
    /// `CLAUDE.md`'nin kuralı: bir "yapmıyoruz" kararı testle korunuyor.
    /// Burada korunan duruş, 3x/4x çarpanların menüde olmaması.
    #[test]
    fn tek_carpan_var() {
        assert_eq!(Carpan::Iki.ara_kare_sayisi(), 1);
    }
}

