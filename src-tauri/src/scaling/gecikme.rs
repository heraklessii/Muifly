//! Ölçekleme boru hattının **eklediği** gecikme.
//!
//! `ROADMAP.md` Faz 3'ün ikinci kabul kriteri: "gecikme artışı ölçülmüş ve
//! kullanıcıya gösterilebilir durumda". Bu dosya o ölçümün saf tarafı —
//! Windows API'si yok, girdisi süreler, çıktısı sayılar.
//!
//! # Neden bu ölçüm ürünün kendisi kadar önemli
//!
//! Ölçekleme, oyunun karesini yakalayıp işleyip yeniden gösteriyor. Bu, her
//! karede araya giren gerçek bir gecikmedir; "belki azıcık" değil, ölçülüp
//! söylenmesi gereken bir bedel. Rakip araçların çoğu bu sayıyı hiç
//! göstermiyor. Şeffaflık ilkesi (tasarım ilkesi 2) burada bir özellik
//! değil, modülün var olma şartı: ölçmediğimiz bir bedeli kullanıcıya
//! ödetemeyiz.
//!
//! Bu yüzden özet **hiçbir yerde** "şu kadar hızlandı" demiyor ve bir
//! iyileşme oranı üretmiyor (`decisions.md` #15). Söylediği tek şey: bu
//! pencerede boru hattı kare başına şu kadar sürdü.

use serde::{Deserialize, Serialize};

/// Tampon kapasitesi.
///
/// 60 FPS'te ~10 saniye. Daha uzun bir pencere, kullanıcı algoritmayı
/// değiştirdikten sonra eski algoritmanın sayılarını dakikalarca taşırdı.
pub const KAPASITE: usize = 600;

/// Tek bir karenin boru hattı süreleri (mikrosaniye).
///
/// Üçü ayrı tutuluyor çünkü toplam süre tek başına ne yapılacağını
/// söylemiyor: yakalama baskınsa sorun kaynağın kendisinde, ölçekleme
/// baskınsa algoritma seçiminde, sunum baskınsa dikey eşitlemede.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KareOlcumu {
    pub yakalama_us: u32,
    pub olcekleme_us: u32,
    /// Kare üretiminin (Faz 4) bu turda harcadığı CPU süresi.
    ///
    /// Ayrı bir alan, çünkü kullanıcının sorduğu soru "kare üretimi
    /// bana ne kadara mal oluyor" — ölçeklemeyle toplanmış bir sayı o
    /// soruyu cevaplamıyor. Üretim kapalıyken sıfır.
    pub uretim_us: u32,
    pub sunum_us: u32,
}

impl KareOlcumu {
    pub fn toplam_us(&self) -> u32 {
        self.yakalama_us
            .saturating_add(self.olcekleme_us)
            .saturating_add(self.uretim_us)
            .saturating_add(self.sunum_us)
    }
}

/// Kullanıcıya gösterilen özet.
///
/// Alanların hepsi ölçülen değer. Hedef, tahmin ya da vaat yok.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GecikmeOzeti {
    /// Özete giren kare sayısı.
    pub kare_sayisi: usize,
    /// Boru hattının kare başına ortalama süresi.
    pub ort_ms: f32,
    /// En kötü %1 karenin ortalama süresi.
    ///
    /// `monitor::frames`'teki gerekçenin aynısı: ortalama, arada gelen
    /// takılmayı gizliyor ve oyuncunun hissettiği şey ortalama değil.
    pub p1_kotu_ms: f32,
    /// Penceredeki en kötü tek kare.
    pub en_kotu_ms: f32,
    pub yakalama_ort_ms: f32,
    pub olcekleme_ort_ms: f32,
    /// Kare üretiminin kare başına ortalama CPU süresi (Faz 4).
    pub uretim_ort_ms: f32,
    pub sunum_ort_ms: f32,
    /// Bu pencerede üretilen (gerçek olmayan) kare sayısı.
    ///
    /// Kullanıcının gördüğü karelerin kaçının üretildiğini söylüyor.
    /// Bir "kaç kare/s kazandın" iddiası **değil**: kaç karenin
    /// hesaplandığı, o karelerin ne kadar iyi olduğundan ayrı bir bilgi
    /// ve ikincisini bu modül ölçemiyor (karar #35).
    pub uretilen_kare: u64,
    /// Kare üretiminin bu makinede anlamlı olup olmadığı.
    ///
    /// Ekran yenileme hızı kaynaktan belirgin olarak yüksek değilse
    /// üretilen kare, gerçek karelerin sırasını bekletmekten başka bir işe
    /// yaramıyor. `None` ölçülemedi demek.
    pub yenileme_hz: Option<u32>,
    /// Yakalamanın yeni kare veremediği tur sayısı.
    ///
    /// Hata değil: oyun o anda yeni kare üretmediyse Desktop Duplication
    /// zaman aşımıyla dönüyor. Sayının kendisi bilgi — sürekli artıyorsa
    /// yakalanan kaynak beklenen kaynak değildir.
    pub bos_tur: u64,
}

/// Son karelerin halka tamponu.
#[derive(Debug)]
pub struct GecikmeTamponu {
    olcumler: std::collections::VecDeque<KareOlcumu>,
    kapasite: usize,
    bos_tur: u64,
    uretilen_kare: u64,
    yenileme_hz: Option<u32>,
}

impl GecikmeTamponu {
    pub fn yeni(kapasite: usize) -> Self {
        Self {
            olcumler: std::collections::VecDeque::with_capacity(kapasite.min(4096)),
            kapasite: kapasite.max(1),
            bos_tur: 0,
            uretilen_kare: 0,
            yenileme_hz: None,
        }
    }

    pub fn ekle(&mut self, o: KareOlcumu) {
        if self.olcumler.len() == self.kapasite {
            self.olcumler.pop_front();
        }
        self.olcumler.push_back(o);
    }

    /// Yakalama yeni kare vermedi.
    pub fn bos_tur(&mut self) {
        self.bos_tur = self.bos_tur.saturating_add(1);
    }

    pub fn len(&self) -> usize {
        self.olcumler.len()
    }

    pub fn is_empty(&self) -> bool {
        self.olcumler.is_empty()
    }

    /// Ölçümleri siler.
    ///
    /// Yenileme hızı KORUNUYOR: o, ölçülen bir performans değeri değil,
    /// makinenin bir özelliği. Algoritma değişince yeniden okumak
    /// gereksiz bir sistem çağrısı olurdu.
    pub fn temizle(&mut self) {
        self.olcumler.clear();
        self.bos_tur = 0;
        self.uretilen_kare = 0;
    }

    /// Özet.
    ///
    /// Tampon boşsa `None`. Sıfırlarla dolu bir özet "gecikme yok" diye
    /// okunurdu ve bu, henüz ölçülmemiş bir şey hakkında iddia olurdu.
    pub fn ozetle(&self) -> Option<GecikmeOzeti> {
        if self.olcumler.is_empty() {
            return None;
        }
        let n = self.olcumler.len();
        let ort = |f: fn(&KareOlcumu) -> u32| -> f32 {
            self.olcumler.iter().map(|o| f(o) as f64).sum::<f64>() as f32 / n as f32 / 1000.0
        };

        let mut toplamlar: Vec<u32> = self.olcumler.iter().map(|o| o.toplam_us()).collect();
        toplamlar.sort_unstable();

        Some(GecikmeOzeti {
            kare_sayisi: n,
            ort_ms: ort(|o| o.toplam_us()),
            p1_kotu_ms: en_kotu_yuzde(&toplamlar, 1.0),
            en_kotu_ms: *toplamlar.last().unwrap() as f32 / 1000.0,
            yakalama_ort_ms: ort(|o| o.yakalama_us),
            olcekleme_ort_ms: ort(|o| o.olcekleme_us),
            uretim_ort_ms: ort(|o| o.uretim_us),
            sunum_ort_ms: ort(|o| o.sunum_us),
            uretilen_kare: self.uretilen_kare,
            yenileme_hz: self.yenileme_hz,
            bos_tur: self.bos_tur,
        })
    }

    /// Bir kare üretildi.
    pub fn uretildi(&mut self) {
        self.uretilen_kare = self.uretilen_kare.saturating_add(1);
    }

    /// Ekranın yenileme hızını kaydeder (bir kez, açılışta).
    pub fn yenileme_ata(&mut self, hz: Option<u32>) {
        self.yenileme_hz = hz;
    }
}

/// Artan sıralı dizide en kötü `yuzde` kadarının ortalaması (ms).
///
/// `monitor::frames::en_kotu_yuzde` ile aynı hesap, aynı gerekçeyle: klasik
/// nearest-rank yüzdelik, küçük pencerelerde en kötü kareyi ıskalıyor. En az
/// bir kare alınıyor.
fn en_kotu_yuzde(sirali_artan: &[u32], yuzde: f32) -> f32 {
    if sirali_artan.is_empty() {
        return 0.0;
    }
    let adet =
        ((sirali_artan.len() as f32 * yuzde / 100.0).round() as usize).clamp(1, sirali_artan.len());
    let dilim = &sirali_artan[sirali_artan.len() - adet..];
    dilim.iter().map(|v| *v as f64).sum::<f64>() as f32 / adet as f32 / 1000.0
}

#[cfg(test)]
mod testler {
    use super::*;

    fn olcum(y: u32, o: u32, s: u32) -> KareOlcumu {
        KareOlcumu {
            yakalama_us: y,
            olcekleme_us: o,
            sunum_us: s,
            ..Default::default()
        }
    }

    #[test]
    fn bos_tamponda_ozet_yok() {
        // Sıfırlarla dolu bir özet "gecikme yok" diye okunurdu.
        assert!(GecikmeTamponu::yeni(10).ozetle().is_none());
    }

    #[test]
    fn kapasite_donuyor() {
        let mut t = GecikmeTamponu::yeni(3);
        for i in 1..=5u32 {
            t.ekle(olcum(i * 1000, 0, 0));
        }
        assert_eq!(t.len(), 3);
        let o = t.ozetle().unwrap();
        // Son üç kare: 3, 4, 5 ms.
        assert!((o.ort_ms - 4.0).abs() < 1e-3, "{}", o.ort_ms);
    }

    #[test]
    fn asamalar_ayri_toplaniyor() {
        let mut t = GecikmeTamponu::yeni(10);
        t.ekle(olcum(1000, 2000, 3000));
        t.ekle(olcum(3000, 2000, 1000));
        let o = t.ozetle().unwrap();
        assert!((o.yakalama_ort_ms - 2.0).abs() < 1e-3);
        assert!((o.olcekleme_ort_ms - 2.0).abs() < 1e-3);
        assert!((o.sunum_ort_ms - 2.0).abs() < 1e-3);
        assert!((o.ort_ms - 6.0).abs() < 1e-3);
    }

    #[test]
    fn en_kotu_kare_ortalamada_kaybolmuyor() {
        // 99 hızlı kare + 1 takılma. Ortalama sorunu göstermiyor,
        // en kötü %1 gösteriyor: modülün ölçüm vaadi tam olarak bu.
        let mut t = GecikmeTamponu::yeni(200);
        for _ in 0..99 {
            t.ekle(olcum(500, 500, 500));
        }
        t.ekle(olcum(30_000, 0, 0));
        let o = t.ozetle().unwrap();
        assert!(o.ort_ms < 2.0, "ortalama {}", o.ort_ms);
        assert!((o.p1_kotu_ms - 30.0).abs() < 1e-3, "p1 {}", o.p1_kotu_ms);
        assert!((o.en_kotu_ms - 30.0).abs() < 1e-3);
    }

    #[test]
    fn bos_turlar_sayiliyor() {
        let mut t = GecikmeTamponu::yeni(10);
        t.ekle(olcum(100, 100, 100));
        t.bos_tur();
        t.bos_tur();
        assert_eq!(t.ozetle().unwrap().bos_tur, 2);
        // Boş tur bir kare ölçümü değil: kare sayısına girmiyor.
        assert_eq!(t.ozetle().unwrap().kare_sayisi, 1);
    }

    #[test]
    fn temizleme_her_seyi_sifirliyor() {
        // Kullanıcı algoritmayı değiştirdiğinde eski sayılar taşınmamalı.
        let mut t = GecikmeTamponu::yeni(10);
        t.ekle(olcum(1, 2, 3));
        t.bos_tur();
        t.temizle();
        assert!(t.is_empty());
        assert!(t.ozetle().is_none());
    }

    #[test]
    fn tasma_panige_donmuyor() {
        // u32 mikrosaniye ~71 dakika; yine de toplama doygun.
        let o = olcum(u32::MAX, u32::MAX, u32::MAX);
        assert_eq!(o.toplam_us(), u32::MAX);
    }

    /// Özet yapısı bir "iyileşme" alanı taşımıyor.
    ///
    /// Karar #15'in bu modüldeki karşılığı: gecikme ölçümü bir bedeli
    /// gösteriyor, bir kazanç iddiası taşımıyor. Alan adı seviyesinde
    /// korunuyor ki ileride biri "kazanc_ms" eklemeye kalkışırsa test düşsün.
    #[test]
    fn ozette_iyilesme_alani_yok() {
        let mut t = GecikmeTamponu::yeni(4);
        t.ekle(olcum(1000, 1000, 1000));
        let json = serde_json::to_string(&t.ozetle().unwrap()).unwrap();
        for yasak in ["kazan", "iyiles", "hizlan", "fark", "oran"] {
            assert!(!json.to_lowercase().contains(yasak), "yasak alan: {yasak}");
        }
    }
}
