//! Kare süresi istatistiği.
//!
//! Bu dosyada Windows API'si YOK. Girdisi bir dizi sunum (present) zaman
//! damgası, çıktısı sayılar. Ölçümün nereden geldiği (ETW) ayrı duruyor; bu
//! ayrım sayesinde istatistik tarafı yükseltilmiş yetki olmadan, gerçek bir
//! oyun açmadan test edilebiliyor.
//!
//! **Neden kare süresi, neden ortalama FPS değil**: `metrics.rs`'deki jitter
//! gerekçesinin aynısı burada da geçerli. Oyuncunun hissettiği şey saniyedeki
//! kare sayısının ortalaması değil, kareler arasındaki **düzensizlik**. 120
//! FPS ortalaması, arada 40 ms'lik takılmalar varsa 90 FPS'lik düzgün bir
//! akıştan kötü hissettirir. Bu yüzden özet üç ayrı şey söylüyor: ortalama,
//! en kötü %1, ve ardışık kareler arasındaki oynama.
//!
//! Tasarım ilkesi 4 hatırlatması: buradaki sayılar kullanıcının KENDİ
//! ölçümüdür. "Şu kadar FPS kazanırsın" bir vaat; "bu oturumda şu ölçüldü"
//! bir veri. `decisions.md` #15 gereği burada da öncesi/sonrası tek bir
//! iyileşme oranına indirgenmiyor.

use serde::{Deserialize, Serialize};

/// Bir ölçüm penceresinin kare özeti.
///
/// Alanların hepsi ölçülen değer; hiçbiri tahmin ya da hedef değil. Özet
/// üretilemiyorsa (yeterli kare yok) `ozetle` `None` dönüyor — sıfırla
/// doldurulmuş bir özet "0 FPS ölçüldü" gibi okunurdu.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KareOzeti {
    /// Pencereye düşen sunum sayısı.
    pub kare_sayisi: usize,
    /// İlk ve son sunum arasındaki süre.
    pub sure_s: f32,
    /// Kare sayısı / süre.
    pub ort_fps: f32,
    /// Ortalama kare süresi.
    pub ort_ms: f32,
    /// En kötü %1 karenin ortalama süresi.
    pub p1_kotu_ms: f32,
    /// O sürenin FPS karşılığı. Yaygın adıyla "%1 low".
    pub p1_kotu_fps: f32,
    /// Ardışık kare süreleri arasındaki ortalama mutlak fark.
    ///
    /// `metrics::jitter` ile aynı ölçü, aynı gerekçeyle: sapma değil oynama.
    pub kare_jitter_ms: f32,
}

/// Sunum zaman damgalarından kare sürelerini (ms) çıkarır.
///
/// `damgalar` ETW'den geldiği haliyle QPC sayacı, `frekans` da
/// `QueryPerformanceFrequency` değeri. Dönüşüm burada yapılıyor ki ETW
/// tarafı ham veriyi taşımaktan başka bir iş yapmasın.
///
/// Damgalar sıralı varsayılmıyor: ETW olayları çok çekirdekli bir oturumda
/// tampon sırasına göre gelebiliyor, sıralamayı çağıran unutursa negatif
/// kare süresi çıkardı.
pub fn kare_sureleri(damgalar: &mut [i64], frekans: i64) -> Vec<f32> {
    if frekans <= 0 || damgalar.len() < 2 {
        return Vec::new();
    }
    damgalar.sort_unstable();
    damgalar
        .windows(2)
        .map(|p| ((p[1] - p[0]) as f64 * 1000.0 / frekans as f64) as f32)
        .collect()
}

/// Artan sıralı dizide, en kötü (en büyük) `yuzde` kadarının ortalaması.
///
/// **Neden yüzdelik değil de ortalama**: "en kötü %1" ifadesi kullanıcıya
/// birebir ne diyorsa onu hesaplaması gerekiyor — en kötü %1 karenin ne
/// kadar sürdüğünü. Klasik nearest-rank 99. yüzdelik, tam 100 karelik bir
/// pencerede en kötü kareyi **ıskalıyor** (99. sıra, sondan ikinci kare) ve
/// tek bir takılmayı görünmez kılıyordu. Bu, aracın en çok işe yarayacağı
/// durumda sessiz kalması demekti.
///
/// En az bir kare alınıyor: `yuzde` kaça denk gelirse gelsin, "en kötü %1"
/// boş küme dönemez.
fn en_kotu_yuzde(sirali_artan: &[f32], yuzde: f32) -> f32 {
    if sirali_artan.is_empty() {
        return 0.0;
    }
    let adet =
        ((sirali_artan.len() as f32 * yuzde / 100.0).ceil() as usize).clamp(1, sirali_artan.len());
    let kotu = &sirali_artan[sirali_artan.len() - adet..];
    (kotu.iter().map(|m| *m as f64).sum::<f64>() / adet as f64) as f32
}

/// Ardışık kare süreleri arasındaki ortalama mutlak fark.
pub fn kare_jitter(kare_ms: &[f32]) -> f32 {
    if kare_ms.len() < 2 {
        return 0.0;
    }
    let toplam: f64 = kare_ms.windows(2).map(|p| (p[1] - p[0]).abs() as f64).sum();
    (toplam / (kare_ms.len() - 1) as f64) as f32
}

/// Bir pencerenin özeti.
///
/// **En az kaç kare gerekiyor**: `EN_AZ_KARE`. Altında `None` dönüyor. Üç
/// karelik bir örnekten "%1 en kötü" çıkarmak, tek bir kareyi istatistik diye
/// sunmak olurdu.
pub const EN_AZ_KARE: usize = 30;

pub fn ozetle(kare_ms: &[f32]) -> Option<KareOzeti> {
    if kare_ms.len() < EN_AZ_KARE {
        return None;
    }

    let toplam_ms: f64 = kare_ms.iter().map(|m| *m as f64).sum();
    if toplam_ms <= 0.0 {
        return None;
    }
    let sure_s = (toplam_ms / 1000.0) as f32;
    let ort_ms = (toplam_ms / kare_ms.len() as f64) as f32;

    let mut sirali = kare_ms.to_vec();
    sirali.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p1_kotu_ms = en_kotu_yuzde(&sirali, 1.0);

    Some(KareOzeti {
        kare_sayisi: kare_ms.len(),
        sure_s,
        ort_fps: kare_ms.len() as f32 / sure_s,
        ort_ms,
        p1_kotu_ms,
        p1_kotu_fps: if p1_kotu_ms > 0.0 {
            1000.0 / p1_kotu_ms
        } else {
            0.0
        },
        kare_jitter_ms: kare_jitter(kare_ms),
    })
}

/// Sabit uzunluklu sunum damgası tamponu.
///
/// `metrics::Tampon` saniyede bir örnek tutuyor; bu tampon saniyede yüzlerce
/// olay alıyor, o yüzden ayrı ve daha büyük. Kapasite `KARE_KAPASITESI`.
#[derive(Debug)]
pub struct KareTamponu {
    damgalar: std::collections::VecDeque<i64>,
    kapasite: usize,
}

/// 240 FPS'te ~40 saniye, 60 FPS'te ~2,5 dakika. Bir ölçüm penceresi için
/// fazlasıyla yeterli; bellek maliyeti 80 KB (i64 × 10 000).
pub const KARE_KAPASITESI: usize = 10_000;

impl KareTamponu {
    pub fn yeni(kapasite: usize) -> Self {
        Self {
            damgalar: std::collections::VecDeque::with_capacity(kapasite.min(KARE_KAPASITESI)),
            kapasite,
        }
    }

    pub fn ekle(&mut self, damga: i64) {
        if self.damgalar.len() == self.kapasite {
            self.damgalar.pop_front();
        }
        self.damgalar.push_back(damga);
    }

    pub fn hepsi(&self) -> Vec<i64> {
        self.damgalar.iter().copied().collect()
    }

    pub fn len(&self) -> usize {
        self.damgalar.len()
    }

    pub fn is_empty(&self) -> bool {
        self.damgalar.is_empty()
    }

    pub fn temizle(&mut self) {
        self.damgalar.clear();
    }

    /// Tampondaki damgalardan doğrudan özet.
    pub fn ozet(&self, frekans: i64) -> Option<KareOzeti> {
        let mut d = self.hepsi();
        let kareler = kare_sureleri(&mut d, frekans);
        ozetle(&kareler)
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    /// `adet` kare, her biri `ms` süren, QPC frekansı 10 000 000 (100 ns tik).
    fn duzgun(adet: usize, ms: f32) -> Vec<f32> {
        vec![ms; adet]
    }

    #[test]
    fn az_kareyle_ozet_uretilmiyor() {
        assert!(ozetle(&duzgun(EN_AZ_KARE - 1, 16.6)).is_none());
        assert!(ozetle(&[]).is_none());
    }

    #[test]
    fn duzgun_akista_ortalama_ve_en_kotu_ayni() {
        let o = ozetle(&duzgun(100, 10.0)).unwrap();
        assert!((o.ort_ms - 10.0).abs() < 0.01);
        assert!((o.p1_kotu_ms - 10.0).abs() < 0.01);
        assert!((o.ort_fps - 100.0).abs() < 0.1, "10 ms → 100 FPS");
        assert!((o.p1_kotu_fps - 100.0).abs() < 0.1);
        assert!(o.kare_jitter_ms.abs() < f32::EPSILON);
    }

    #[test]
    fn tek_takilma_ortalamayi_degil_en_kotuyu_bozuyor() {
        // 99 kare 10 ms + 1 kare 100 ms. Ürünün iddiasının temeli: ortalama
        // neredeyse aynı kalıyor, "%1 en kötü" gerçeği söylüyor.
        let mut k = duzgun(99, 10.0);
        k.push(100.0);
        let o = ozetle(&k).unwrap();

        assert!(o.ort_ms < 11.0, "ortalama takılmayı saklıyor: {}", o.ort_ms);
        assert!(
            o.p1_kotu_ms > 50.0,
            "en kötü %1 takılmayı göstermeli: {}",
            o.p1_kotu_ms
        );
        assert!(o.p1_kotu_fps < o.ort_fps);
    }

    #[test]
    fn en_kotu_yuzde_tam_yuz_karede_en_kotuyu_isklamiyor() {
        // Regresyon koruması: nearest-rank 99. yüzdelik burada sondan ikinci
        // kareyi seçiyor ve tek takılmayı görünmez kılıyordu.
        let mut d = vec![10.0f32; 99];
        d.push(100.0);
        d.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert!((en_kotu_yuzde(&d, 1.0) - 100.0).abs() < 0.01);
    }

    #[test]
    fn en_kotu_yuzde_en_az_bir_kare_aliyor() {
        let d = vec![1.0f32, 2.0, 5.0];
        // %1'i 0,03 kare eder; yine de en kötü kare dönmeli.
        assert!((en_kotu_yuzde(&d, 1.0) - 5.0).abs() < f32::EPSILON);
        assert_eq!(en_kotu_yuzde(&[], 1.0), 0.0);
    }

    #[test]
    fn ayni_ortalama_farkli_duzgunluk_ayirt_ediliyor() {
        // İkisi de ortalama 20 ms (50 FPS), biri düzgün biri zıplayan.
        let sabit = duzgun(100, 20.0);
        let ziplayan: Vec<f32> = (0..100)
            .map(|i| if i % 2 == 0 { 10.0 } else { 30.0 })
            .collect();

        let a = ozetle(&sabit).unwrap();
        let b = ozetle(&ziplayan).unwrap();

        assert!(
            (a.ort_ms - b.ort_ms).abs() < 0.01,
            "ortalamalar eşit olmalı"
        );
        assert!(
            b.kare_jitter_ms > a.kare_jitter_ms,
            "zıplayan akışın jitter'ı yüksek olmalı"
        );
    }

    #[test]
    fn qpc_damgalari_milisaniyeye_cevriliyor() {
        // 10 MHz frekans → 1 tik = 100 ns. 100 000 tik = 10 ms.
        let mut damgalar = vec![0i64, 100_000, 200_000, 300_000];
        let k = kare_sureleri(&mut damgalar, 10_000_000);
        assert_eq!(k.len(), 3);
        for m in k {
            assert!((m - 10.0).abs() < 0.001, "beklenen 10 ms, gelen {m}");
        }
    }

    #[test]
    fn sirasiz_damgalar_negatif_kare_uretmiyor() {
        // ETW olayları tampon sırasına göre gelebilir.
        let mut damgalar = vec![200_000i64, 0, 100_000];
        let k = kare_sureleri(&mut damgalar, 10_000_000);
        assert!(k.iter().all(|m| *m > 0.0), "negatif kare süresi: {k:?}");
    }

    #[test]
    fn gecersiz_frekans_bos_donuyor() {
        assert!(kare_sureleri(&mut [0, 1, 2], 0).is_empty());
        assert!(kare_sureleri(&mut [0, 1, 2], -1).is_empty());
    }

    #[test]
    fn tek_damgadan_kare_cikmiyor() {
        assert!(kare_sureleri(&mut [42], 10_000_000).is_empty());
        assert!(kare_sureleri(&mut [], 10_000_000).is_empty());
    }

    #[test]
    fn tampon_kapasiteyi_asmiyor() {
        let mut t = KareTamponu::yeni(5);
        for i in 0..20 {
            t.ekle(i);
        }
        assert_eq!(t.len(), 5);
        assert_eq!(t.hepsi()[0], 15);
    }

    #[test]
    fn tampon_ozeti_az_kareyle_none() {
        let mut t = KareTamponu::yeni(100);
        for i in 0..10 {
            t.ekle(i * 100_000);
        }
        assert!(t.ozet(10_000_000).is_none());
    }

    #[test]
    fn tampon_ozeti_yeterli_kareyle_uretiliyor() {
        let mut t = KareTamponu::yeni(1000);
        for i in 0..=EN_AZ_KARE as i64 {
            t.ekle(i * 100_000); // 10 ms aralık
        }
        let o = t.ozet(10_000_000).expect("özet çıkmalı");
        assert_eq!(o.kare_sayisi, EN_AZ_KARE);
        assert!((o.ort_fps - 100.0).abs() < 0.5);
    }
}
