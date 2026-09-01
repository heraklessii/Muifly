//! Ölçüm: CPU/bellek örnekleme, gecikme istatistiği, öncesi/sonrası karşılaştırma.
//!
//! Buradaki istatistik fonksiyonları **saf**: girdi bir dizi ölçüm, çıktı bir
//! sayı. Windows API'sine dokunmuyorlar ve doğrudan test ediliyorlar. Sistem
//! okuması yapan tek yer `Ornekleyici`, o da dosyanın sonunda ayrı duruyor.
//!
//! Tasarım ilkesi 4 hatırlatması: buradaki sayılar kullanıcının KENDİ
//! ölçümüdür. "Şu kadar iyileşir" bir iddia; "bu oturumda şu ölçüldü" bir
//! veri. Arayüz metinleri bu ayrımı korumak zorunda.

use serde::{Deserialize, Serialize};

/// Tek bir ölçüm anı.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ornek {
    /// Unix milisaniye.
    pub zaman: i64,
    /// Sistem geneli CPU kullanımı, yüzde.
    pub cpu: f32,
    /// Kullanımdaki fiziksel bellek, yüzde.
    pub bellek: f32,
    /// Son ICMP turu (ms). Ölçüm yoksa `None` — sıfır DEĞİL.
    /// Sıfır yazmak "0 ms gecikme" gibi okunur ve grafiği yalan söyletir.
    pub gecikme_ms: Option<f32>,
}

/// Bir zaman aralığının özeti.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ozet {
    pub ornek_sayisi: usize,
    pub cpu_ort: f32,
    pub bellek_ort: f32,
    /// Gecikme ölçümü olan örnekler üzerinden.
    pub gecikme_ort_ms: Option<f32>,
    /// Ardışık gecikme ölçümlerinin ortalama mutlak farkı (RFC 3550 mantığı).
    pub jitter_ms: Option<f32>,
    /// Cevapsız kalan ölçüm oranı, yüzde.
    pub kayip_yuzde: f32,
}

/// Ortalama. Boş dizide `0.0` değil `None` dönmüyor çünkü çağıran zaten
/// örnek sayısını görüyor; iki ayrı boşluk temsili karışıklık yaratırdı.
fn ortalama(degerler: impl Iterator<Item = f32>) -> f32 {
    let mut toplam = 0.0f64;
    let mut adet = 0usize;
    for d in degerler {
        toplam += d as f64;
        adet += 1;
    }
    if adet == 0 {
        0.0
    } else {
        (toplam / adet as f64) as f32
    }
}

/// Ardışık gecikme ölçümleri arasındaki ortalama mutlak fark.
///
/// Jitter için standart sapma değil bu ölçü kullanılıyor: oyuncunun hissettiği
/// şey ölçümlerin ortalamadan sapması değil, **ardışık paketler arasındaki
/// oynama**. Sabit 80 ms, 40 ile 60 arasında zıplayan bir bağlantıdan daha
/// oynanabilir; standart sapma bu farkı yeterince göstermiyor.
///
/// Cevapsız ölçümler (`None`) atlanıyor, komşuluk bozulmuş sayılmıyor: bir
/// kayıp paket yüzünden jitter'ı şişirmek yanıltıcı olurdu — kayıp ayrı bir
/// metrik olarak zaten raporlanıyor.
pub fn jitter(gecikmeler: &[Option<f32>]) -> Option<f32> {
    let olculen: Vec<f32> = gecikmeler.iter().filter_map(|g| *g).collect();
    if olculen.len() < 2 {
        return None;
    }
    let farklar = olculen.windows(2).map(|p| (p[1] - p[0]).abs());
    Some(ortalama(farklar))
}

/// Cevapsız ölçüm oranı, yüzde.
pub fn kayip_yuzde(gecikmeler: &[Option<f32>]) -> f32 {
    if gecikmeler.is_empty() {
        return 0.0;
    }
    let kayip = gecikmeler.iter().filter(|g| g.is_none()).count();
    (kayip as f32 / gecikmeler.len() as f32) * 100.0
}

pub fn ozetle(ornekler: &[Ornek]) -> Ozet {
    let gecikmeler: Vec<Option<f32>> = ornekler.iter().map(|o| o.gecikme_ms).collect();
    let olculen: Vec<f32> = gecikmeler.iter().filter_map(|g| *g).collect();

    Ozet {
        ornek_sayisi: ornekler.len(),
        cpu_ort: ortalama(ornekler.iter().map(|o| o.cpu)),
        bellek_ort: ortalama(ornekler.iter().map(|o| o.bellek)),
        gecikme_ort_ms: if olculen.is_empty() {
            None
        } else {
            Some(ortalama(olculen.iter().copied()))
        },
        jitter_ms: jitter(&gecikmeler),
        kayip_yuzde: kayip_yuzde(&gecikmeler),
    }
}

/// Optimizasyon öncesi ve sonrası iki özet.
///
/// Fark hesabı burada YAPILMIYOR, iki özet olduğu gibi taşınıyor. Sebep:
/// "%12 iyileşme" gibi tek bir sayı üretmek, ölçüm koşullarının aynı
/// olduğunu varsayar — ki oyun içinde asla aynı değildir. Arayüz iki değeri
/// yan yana gösterip yorumu kullanıcıya bırakıyor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Karsilastirma {
    pub onceki: Ozet,
    pub sonraki: Ozet,
    /// İki ölçümün kapsadığı süre (saniye) — kullanıcı kısa bir ölçümün
    /// ne kadar güvenilir olduğunu görebilsin.
    pub onceki_saniye: f32,
    pub sonraki_saniye: f32,
}

/// Sabit uzunluklu örnek tamponu.
#[derive(Debug)]
pub struct Tampon {
    ornekler: std::collections::VecDeque<Ornek>,
    kapasite: usize,
}

impl Tampon {
    pub fn yeni(kapasite: usize) -> Self {
        Self {
            ornekler: std::collections::VecDeque::with_capacity(kapasite),
            kapasite,
        }
    }

    pub fn ekle(&mut self, ornek: Ornek) {
        if self.ornekler.len() == self.kapasite {
            self.ornekler.pop_front();
        }
        self.ornekler.push_back(ornek);
    }

    pub fn hepsi(&self) -> Vec<Ornek> {
        self.ornekler.iter().copied().collect()
    }

    /// Verilen zaman damgasından sonraki örnekler.
    pub fn sonrasi(&self, zaman: i64) -> Vec<Ornek> {
        self.ornekler
            .iter()
            .filter(|o| o.zaman >= zaman)
            .copied()
            .collect()
    }

    pub fn bos_mu(&self) -> bool {
        self.ornekler.is_empty()
    }

    pub fn temizle(&mut self) {
        self.ornekler.clear();
    }
}

// ---------------------------------------------------------------------------
// Sistem okuması
// ---------------------------------------------------------------------------

/// CPU kullanımı, iki `GetSystemTimes` çağrısı arasındaki farktan hesaplanıyor.
///
/// Tek çağrıyla anlık CPU okunamıyor: Windows kümülatif tik sayısı veriyor,
/// yüzde ancak iki ölçüm arasındaki delta ile çıkıyor. Bu yüzden örnekleyici
/// durum tutuyor ve ilk örnek her zaman 0 dönüyor.
#[derive(Debug, Default)]
pub struct Ornekleyici {
    onceki_bos: u64,
    onceki_toplam: u64,
}

impl Ornekleyici {
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Bir örnek alır. `gecikme_ms` dışarıdan geliyor: ağ ölçümü ayrı bir
    /// zamanlamada koşuyor ve CPU örneklemesini bloke etmemeli.
    pub fn ornek_al(&mut self, gecikme_ms: Option<f32>) -> Ornek {
        Ornek {
            zaman: chrono::Utc::now().timestamp_millis(),
            cpu: self.cpu_yuzde(),
            bellek: bellek_yuzde(),
            gecikme_ms,
        }
    }

    #[cfg(windows)]
    fn cpu_yuzde(&mut self) -> f32 {
        use windows::Win32::Foundation::FILETIME;
        use windows::Win32::System::Threading::GetSystemTimes;

        fn tik(f: FILETIME) -> u64 {
            ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64
        }

        let mut bos = FILETIME::default();
        let mut cekirdek = FILETIME::default();
        let mut kullanici = FILETIME::default();

        // SAFETY: üç çıktı işaretçisi de yerel, geçerli ve hizalı.
        if unsafe { GetSystemTimes(Some(&mut bos), Some(&mut cekirdek), Some(&mut kullanici)) }
            .is_err()
        {
            return 0.0;
        }

        // `cekirdek` boşta geçen süreyi de İÇERİYOR (Windows böyle raporluyor),
        // bu yüzden toplam = çekirdek + kullanıcı, boş ayrıca çıkarılıyor.
        let bos = tik(bos);
        let toplam = tik(cekirdek) + tik(kullanici);

        let d_bos = bos.saturating_sub(self.onceki_bos);
        let d_toplam = toplam.saturating_sub(self.onceki_toplam);
        self.onceki_bos = bos;
        self.onceki_toplam = toplam;

        if d_toplam == 0 {
            return 0.0;
        }
        let mesgul = d_toplam.saturating_sub(d_bos);
        ((mesgul as f64 / d_toplam as f64) * 100.0).clamp(0.0, 100.0) as f32
    }

    #[cfg(not(windows))]
    fn cpu_yuzde(&mut self) -> f32 {
        0.0
    }
}

#[cfg(windows)]
fn bellek_yuzde() -> f32 {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    let mut durum = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    // SAFETY: `dwLength` doldurulmuş, yapı yerel ve geçerli.
    if unsafe { GlobalMemoryStatusEx(&mut durum) }.is_err() {
        return 0.0;
    }
    durum.dwMemoryLoad as f32
}

#[cfg(not(windows))]
fn bellek_yuzde() -> f32 {
    0.0
}

#[cfg(test)]
mod testler {
    use super::*;

    fn ornek(cpu: f32, gecikme: Option<f32>) -> Ornek {
        Ornek {
            zaman: 0,
            cpu,
            bellek: 50.0,
            gecikme_ms: gecikme,
        }
    }

    #[test]
    fn jitter_tek_olcumle_hesaplanmiyor() {
        assert_eq!(jitter(&[Some(40.0)]), None);
        assert_eq!(jitter(&[]), None);
    }

    #[test]
    fn sabit_gecikmede_jitter_sifir() {
        let j = jitter(&[Some(40.0), Some(40.0), Some(40.0)]).unwrap();
        assert!(j.abs() < f32::EPSILON, "sabit gecikmede jitter 0 olmalı");
    }

    #[test]
    fn jitter_ardisik_farkin_ortalamasi() {
        // Farklar: |50-40|=10, |45-50|=5  → ortalama 7.5
        let j = jitter(&[Some(40.0), Some(50.0), Some(45.0)]).unwrap();
        assert!((j - 7.5).abs() < 0.001, "beklenen 7.5, gelen {j}");
    }

    #[test]
    fn sabit_yuksek_gecikme_dusuk_jitter_veriyor() {
        // Ürünün iddiasının temeli: 80 ms sabit, 40-60 zıplayandan daha
        // düşük jitter'lı olmalı.
        let sabit = jitter(&[Some(80.0), Some(80.0), Some(80.0), Some(80.0)]).unwrap();
        let ziplayan = jitter(&[Some(40.0), Some(60.0), Some(40.0), Some(60.0)]).unwrap();
        assert!(sabit < ziplayan);
    }

    #[test]
    fn kayip_paket_jitteri_sismiyor() {
        // Ortadaki kayıp, 40→40 komşuluğunu bozmamalı.
        let j = jitter(&[Some(40.0), None, Some(40.0)]).unwrap();
        assert!(j.abs() < f32::EPSILON);
    }

    #[test]
    fn kayip_orani_dogru() {
        let k = kayip_yuzde(&[Some(1.0), None, Some(1.0), None]);
        assert!((k - 50.0).abs() < 0.001);
    }

    #[test]
    fn hic_gecikme_olcumu_yoksa_ortalama_none() {
        let o = ozetle(&[ornek(10.0, None), ornek(20.0, None)]);
        assert_eq!(o.gecikme_ort_ms, None, "ölçüm yoksa 0 değil None");
        assert_eq!(o.kayip_yuzde, 100.0);
        assert!((o.cpu_ort - 15.0).abs() < 0.001);
    }

    #[test]
    fn bos_ozet_cokmuyor() {
        let o = ozetle(&[]);
        assert_eq!(o.ornek_sayisi, 0);
        assert_eq!(o.cpu_ort, 0.0);
        assert_eq!(o.jitter_ms, None);
    }

    #[test]
    fn tampon_kapasiteyi_asmiyor() {
        let mut t = Tampon::yeni(3);
        for i in 0..10 {
            t.ekle(ornek(i as f32, None));
        }
        let hepsi = t.hepsi();
        assert_eq!(hepsi.len(), 3);
        assert_eq!(hepsi[0].cpu, 7.0);
    }

    #[test]
    fn tampon_zamana_gore_suzuluyor() {
        let mut t = Tampon::yeni(10);
        for z in [100i64, 200, 300] {
            t.ekle(Ornek {
                zaman: z,
                cpu: 1.0,
                bellek: 1.0,
                gecikme_ms: None,
            });
        }
        assert_eq!(t.sonrasi(200).len(), 2);
    }
}
