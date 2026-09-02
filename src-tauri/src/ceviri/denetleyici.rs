//! Ekran çevirisinin denetleyicisi: kısayol, yakalama, OCR, model, sonuç.
//!
//! ## Neden ayrı bir iş parçacığı
//!
//! Bir çeviri isteği yakalama (~100 ms), OCR (karar #28: 14-74 ms) ve model
//! (karar #29: cümle başına ~140 ms, ilk yüklemede ~2,3 s) demek. Bunu
//! arayüzün ya da arka plan döngüsünün iş parçacığında yapmak, o süre boyunca
//! programın geri kalanını durdururdu. Kısayolun kendi iş parçacığı ayrıca
//! var (`super::kisayol`): mesaj kuyruğunda çeviri yapılmaz.
//!
//! ## Model boştayken bellekten düşüyor
//!
//! Karar #22'nin sözü: "Model isteğe bağlı indirilir, **boştayken bellekten
//! düşer**." Yüklü bir seq2seq modeli yüzlerce MB tutuyor ve bu program
//! saatlerce arka planda duruyor. [`Yapilandirma::bosta_dusur_sn`] kadar
//! istek gelmezse oturumlar bırakılıyor; sonraki istek yeniden yüklüyor ve
//! kullanıcı bunu bir kerelik bir gecikme olarak görüyor.
//!
//! ## Çeviri belleği her istekte diskten okunuyor
//!
//! Bellek burada tutulmuyor. Sebebi eşzamanlılık: kullanıcı arayüzden bir
//! çeviriyi düzeltirken (`commands::ceviri_*`) buradaki bir kopya elde
//! kalsaydı, ikisinden biri diğerini ezerdi — üstelik ezilen taraf çoğu
//! zaman kullanıcının kendi düzeltmesi olurdu. Dosya küçük ve okuma bir
//! tuş basışında bir kez oluyor.
//!
//! ## Sistemde bir şey değiştirmiyor
//!
//! `monitor` ve `scaling` gibi bu modül de yalnızca okuyor ve kendi
//! dosyasına yazıyor. Geri alınacak bir sistem değişikliği üretmediği için
//! deftere yazacak bir şeyi yok; günlüğe **yazıyor** (`state::Motor`).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::Serialize;

use super::akis::{self, Sonuc};
use super::alan::Alan;
use crate::error::{Error, Result};

/// Modelin çevirdiği dil çifti. Model tek yönlü ve bu bir sınır, seçenek
/// değil; arayüz de böyle yazıyor (tasarım ilkesi 4).
pub const KAYNAK_DIL: &str = "en";
pub const HEDEF_DIL: &str = "tr";

/// Kısayol kaydına ve ilk kuruluma verilen süre.
const ACILIS_BEKLEME: Duration = Duration::from_secs(5);

/// Döngünün boş turu. Model düşürme kontrolü bu sıklıkta yapılıyor.
const TUR: Duration = Duration::from_secs(1);

/// Yakalama denemesi sayısı ve tur başına zaman aşımı.
///
/// Masaüstü çoğaltması **değişiklik** verdiği için duran bir ekranda kare
/// gelmiyor. Oyunlar sürekli çizdiği için bu pratikte sorun değil; yine de
/// tek denemeyle vazgeçmek, ilk turda kare gelmeyen makinelerde özelliği
/// çalışmaz gösterirdi.
const YAKALAMA_DENEMESI: u32 = 20;
const YAKALAMA_ZAMAN_ASIMI_MS: u32 = 50;

/// Çevirinin hangi adımda olduğu — arayüz bekleyen kullanıcıya ne olduğunu
/// söyleyebilsin diye.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Asama {
    Bosta,
    Yakalaniyor,
    Okunuyor,
    ModelYukleniyor,
    Cevriliyor,
}

/// Arayüze giden durum.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CeviriDurumu {
    pub acik: bool,
    /// Kayıtlı kısayol, örn. `Ctrl+Alt+T`. Açıkken hep dolu.
    pub kisayol: Option<String>,
    pub asama: Asama,
    /// Model şu an bellekte mi? Kullanıcı ilk isteğin neden yavaş olduğunu
    /// görebilmeli.
    pub model_bellekte: bool,
    /// Son isteğin süreleri. `None` = henüz istek olmadı.
    pub yakalama_ms: Option<u64>,
    pub ocr_ms: Option<u64>,
    pub ceviri_ms: Option<u64>,
    /// Son isteğin hatası. Bir sonraki başarılı istekte siliniyor.
    pub son_hata: Option<String>,
    /// Belleğin bağlı olduğu oyun (dosya adı).
    pub oyun: String,
    pub kaynak_dil: String,
    pub hedef_dil: String,
    /// Alanın tamamı ekran mı?
    pub tum_ekran: bool,
}

impl Default for CeviriDurumu {
    fn default() -> Self {
        Self {
            acik: false,
            kisayol: None,
            asama: Asama::Bosta,
            model_bellekte: false,
            yakalama_ms: None,
            ocr_ms: None,
            ceviri_ms: None,
            son_hata: None,
            oyun: String::new(),
            kaynak_dil: KAYNAK_DIL.into(),
            hedef_dil: HEDEF_DIL.into(),
            tum_ekran: true,
        }
    }
}

/// Çevirinin çalışma ayarları.
#[derive(Debug, Clone, PartialEq)]
pub struct Yapilandirma {
    /// Yakalanacak ekranın sırası (`scaling::yakalama::Ekran`).
    pub ekran: usize,
    pub alan: Alan,
    /// Çeviri belleğinin bağlanacağı oyun — profil kimliği ya da exe adı.
    pub oyun: String,
    /// OCR'ın kaynak dili (BCP-47). Model tek yönlü, bu yalnızca okuma dili.
    pub kaynak_dil: String,
    /// Model kaç saniye boşta kalırsa bellekten düşsün.
    pub bosta_dusur_sn: u64,
}

impl Default for Yapilandirma {
    fn default() -> Self {
        Self {
            ekran: 0,
            alan: Alan::tam_ekran(),
            oyun: "genel".into(),
            kaynak_dil: KAYNAK_DIL.into(),
            // Beş dakika: bir diyaloğu çevirip devam eden oyuncu, sıradaki
            // diyalogda yeniden beklemesin; ama araç gece boyu açık kalırsa
            // yüzlerce MB'ı tutmasın.
            bosta_dusur_sn: 300,
        }
    }
}

/// Ekran çevirisi denetleyicisi.
pub struct Ceviri {
    durum: Arc<Mutex<CeviriDurumu>>,
    son: Arc<Mutex<Option<Sonuc>>>,
    /// Arka plan döngüsünün devralacağı "yeni sonuç var" bayrağı.
    yeni: Arc<AtomicBool>,
    yapilandirma: Arc<Mutex<Yapilandirma>>,
    dur: Arc<AtomicBool>,
    /// Elle çeviri isteği için. Kısayol da aynı kanala yazıyor.
    tetik: Option<Sender<()>>,
    is_parcasi: Option<std::thread::JoinHandle<()>>,
}

impl Default for Ceviri {
    fn default() -> Self {
        Self::yeni()
    }
}

impl Ceviri {
    pub fn yeni() -> Self {
        Self {
            durum: Arc::new(Mutex::new(CeviriDurumu::default())),
            son: Arc::new(Mutex::new(None)),
            yeni: Arc::new(AtomicBool::new(false)),
            yapilandirma: Arc::new(Mutex::new(Yapilandirma::default())),
            dur: Arc::new(AtomicBool::new(false)),
            tetik: None,
            is_parcasi: None,
        }
    }

    pub fn acik(&self) -> bool {
        self.is_parcasi.is_some()
    }

    pub fn durum(&self) -> CeviriDurumu {
        self.durum.lock().clone()
    }

    pub fn son_sonuc(&self) -> Option<Sonuc> {
        self.son.lock().clone()
    }

    /// Yeni bir sonuç geldiyse bayrağı **alıp siler**.
    ///
    /// Alıp silmek şart: bayrak kalsaydı arka plan döngüsü her turda aynı
    /// olayı yayınlar ve aynı satırı günlüğe yazardı (ölçeklemenin kaçış
    /// bayrağıyla aynı gerekçe, karar #34).
    pub fn sonucu_devral(&self) -> bool {
        self.yeni.swap(false, Ordering::Relaxed)
    }

    /// Çalışırken ayarları değiştirir.
    ///
    /// Yeniden başlatma yok: alan seçen kullanıcı, seçtikten hemen sonra
    /// kısayola basabilmeli. Kısayolun yeniden kaydedilmesi ise başka bir
    /// uygulamanın onu kapmasına açık bir pencere bırakırdı.
    pub fn yapilandir(&self, y: Yapilandirma) {
        let tum_ekran = y.alan.tam_mi();
        let oyun = y.oyun.clone();
        let kaynak = y.kaynak_dil.clone();
        *self.yapilandirma.lock() = y;
        let mut d = self.durum.lock();
        d.tum_ekran = tum_ekran;
        d.oyun = oyun;
        d.kaynak_dil = kaynak;
    }

    /// Çeviriyi açar: kısayolu kaydeder ve iş parçacığını başlatır.
    ///
    /// Kısayol kaydedilemezse **açılmıyor** ve hata çağırana dönüyor.
    /// Kısayolsuz bir ekran çevirisi, oyunun içindeyken tetiklenemeyeceği
    /// için çalışmayan bir özelliktir; sessizce açık görünmesi, karşılanmamış
    /// bir vaat olurdu (tasarım ilkesi 4).
    pub fn ac(&mut self, y: Yapilandirma) -> Result<()> {
        self.kapat();
        self.yapilandir(y);
        self.dur.store(false, Ordering::Relaxed);

        let (tetik, alici) = std::sync::mpsc::channel::<()>();
        let (acilis, acilis_alici) = std::sync::mpsc::channel::<Result<String>>();

        let durum = Arc::clone(&self.durum);
        let son = Arc::clone(&self.son);
        let yeni = Arc::clone(&self.yeni);
        let yapilandirma = Arc::clone(&self.yapilandirma);
        let dur = Arc::clone(&self.dur);
        let kisayol_tetik = tetik.clone();

        let is = std::thread::Builder::new()
            .name("muifly-ceviri".into())
            .spawn(move || {
                dongu(
                    Baglam {
                        durum,
                        son,
                        yeni,
                        yapilandirma,
                        dur,
                    },
                    alici,
                    kisayol_tetik,
                    acilis,
                )
            })
            .map_err(|_| Error::Ceviri("çeviri iş parçacığı başlatılamadı".into()))?;

        match acilis_alici.recv_timeout(ACILIS_BEKLEME) {
            Ok(Ok(etiket)) => {
                self.tetik = Some(tetik);
                self.is_parcasi = Some(is);
                let mut d = self.durum.lock();
                d.acik = true;
                d.kisayol = Some(etiket);
                d.son_hata = None;
                Ok(())
            }
            Ok(Err(e)) => {
                self.dur.store(true, Ordering::Relaxed);
                let _ = is.join();
                Err(e)
            }
            Err(_) => {
                self.dur.store(true, Ordering::Relaxed);
                drop(tetik);
                let _ = is.join();
                Err(Error::Ceviri("çeviri açılırken cevap gelmedi".into()))
            }
        }
    }

    /// Kapatır ve iş parçacığının bitmesini bekler.
    ///
    /// Beklemek şart: kısayol iş parçacığın içinde kaldırılıyor ve
    /// beklenmezse "kapattım" dedikten sonra kombinasyon bir süre daha
    /// bizde kalırdı.
    pub fn kapat(&mut self) {
        self.dur.store(true, Ordering::Relaxed);
        // Kanalın kapanması döngüyü `recv_timeout`tan uyandırıyor.
        self.tetik = None;
        if let Some(is) = self.is_parcasi.take() {
            let _ = is.join();
        }
        let mut d = self.durum.lock();
        d.acik = false;
        d.kisayol = None;
        d.asama = Asama::Bosta;
        d.model_bellekte = false;
    }

    /// Arayüzden elle çeviri isteği.
    ///
    /// Kısayolun aynısını yapıyor. Var olma sebebi deneme: kullanıcı
    /// özelliğin çalıştığını, oyuna geçmeden, Muifly penceresindeyken
    /// görebilmeli.
    pub fn simdi_cevir(&self) -> Result<()> {
        let Some(t) = &self.tetik else {
            return Err(Error::Ceviri("çeviri açık değil".into()));
        };
        t.send(())
            .map_err(|_| Error::Ceviri("çeviri iş parçacığı yanıt vermiyor".into()))
    }
}

impl Drop for Ceviri {
    fn drop(&mut self) {
        self.kapat();
    }
}

/// Döngünün paylaşılan durumu. Tek yapıda, çünkü altı ayrı `Arc` parametresi
/// imzayı okunmaz hâle getiriyordu.
struct Baglam {
    durum: Arc<Mutex<CeviriDurumu>>,
    son: Arc<Mutex<Option<Sonuc>>>,
    yeni: Arc<AtomicBool>,
    yapilandirma: Arc<Mutex<Yapilandirma>>,
    dur: Arc<AtomicBool>,
}

fn dongu(
    b: Baglam,
    alici: Receiver<()>,
    kisayol_tetik: Sender<()>,
    acilis: Sender<Result<String>>,
) {
    // WinRT çağrıları (OCR) apartman başlatılmamış bir iş parçacığında
    // çalışmıyor.
    super::ocr::apartman_baslat();

    let Some(kisayol) = super::kisayol::Kisayol::kaydet(move || {
        // Geri çağrı kısayolun mesaj kuyruğunda koşuyor: burada yalnızca
        // haber veriliyor, iş bu iş parçacığında yapılıyor.
        let _ = kisayol_tetik.send(());
    }) else {
        let _ = acilis.send(Err(Error::Ceviri(
            "çeviri kısayolu kaydedilemedi — kombinasyonu kullanan uygulamayı kapatıp \
             yeniden deneyin"
                .into(),
        )));
        return;
    };
    if acilis.send(Ok(kisayol.etiket.to_string())).is_err() {
        return;
    }

    let mut cevirici: Option<super::cevirici::Cevirici> = None;
    let mut son_kullanim = Instant::now();

    loop {
        match alici.recv_timeout(TUR) {
            Ok(()) => {
                if b.dur.load(Ordering::Relaxed) {
                    break;
                }
                let y = b.yapilandirma.lock().clone();
                let sonuc = bir_istek(&b, &y, &mut cevirici);
                son_kullanim = Instant::now();

                let mut d = b.durum.lock();
                d.asama = Asama::Bosta;
                d.model_bellekte = cevirici.is_some();
                match sonuc {
                    Ok(()) => d.son_hata = None,
                    Err(e) => d.son_hata = Some(e.to_string()),
                }
                drop(d);
                b.yeni.store(true, Ordering::Relaxed);
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }

        if b.dur.load(Ordering::Relaxed) {
            break;
        }

        // Karar #22: model boştayken bellekten düşer.
        if cevirici.is_some() {
            let bosta = b.yapilandirma.lock().bosta_dusur_sn;
            if bosta > 0 && son_kullanim.elapsed() >= Duration::from_secs(bosta) {
                cevirici = None;
                b.durum.lock().model_bellekte = false;
            }
        }
    }

    drop(kisayol);
}

/// Tek bir çeviri isteği.
fn bir_istek(
    b: &Baglam,
    y: &Yapilandirma,
    cevirici: &mut Option<super::cevirici::Cevirici>,
) -> Result<()> {
    b.durum.lock().asama = Asama::Yakalaniyor;
    let yakalama_basi = Instant::now();
    let goruntu = yakala(y.ekran)?;
    let kesit = super::alan::kes(&goruntu, y.alan).ok_or_else(|| {
        Error::Ceviri(
            "seçilen alan okunamayacak kadar küçük — Çeviri ekranından yeniden seçin".into(),
        )
    })?;
    let yakalama_ms = yakalama_basi.elapsed().as_millis() as u64;

    b.durum.lock().asama = Asama::Okunuyor;
    let okuma = super::ocr::oku(&kesit, &y.kaynak_dil)?;

    {
        let mut d = b.durum.lock();
        d.yakalama_ms = Some(yakalama_ms);
        d.ocr_ms = Some(okuma.sure_ms);
    }

    if okuma.metin.trim().is_empty() {
        *b.son.lock() = Some(Sonuc {
            ham: String::new(),
            uyarilar: vec![
                "Seçilen alanda okunabilir bir yazı bulunamadı. Alanı yeniden seçmeyi ya da \
                 yazının durduğu bir anda denemeyi deneyebilirsiniz."
                    .into(),
            ],
            birimler: Vec::new(),
            bellekten: 0,
            ceviri_ms: 0,
        });
        b.durum.lock().ceviri_ms = Some(0);
        return Ok(());
    }

    if cevirici.is_none() {
        b.durum.lock().asama = Asama::ModelYukleniyor;
        *cevirici = Some(super::cevirici::Cevirici::yukle()?);
        b.durum.lock().model_bellekte = true;
    }

    b.durum.lock().asama = Asama::Cevriliyor;
    let yol = super::bellek::yolu(&crate::settings::ceviri_dizini(), &y.oyun);
    let mut bellek = super::bellek::yukle(&yol, &y.oyun, KAYNAK_DIL, HEDEF_DIL)?;

    let motor = cevirici.as_mut().expect("model yeni yüklendi");
    let sonuc = akis::cevir(&okuma.metin, &mut bellek, motor)?;

    // Bellek yazılamıyorsa çeviri yine de gösteriliyor: diske yazamamak,
    // kullanıcının elindeki çeviriyi geri almak için sebep değil.
    if let Err(e) = bellek.kaydet(&yol) {
        log::warn!("çeviri belleği yazılamadı: {e}");
    }

    b.durum.lock().ceviri_ms = Some(sonuc.ceviri_ms);
    *b.son.lock() = Some(sonuc);
    Ok(())
}

/// Ekrandan tek bir kare alır.
#[cfg(windows)]
pub(crate) fn yakala(ekran: usize) -> Result<crate::scaling::algoritma::Goruntu> {
    use crate::scaling::yakalama::{KareDurumu, Yakalayici};

    let mut y = Yakalayici::ac(ekran).map_err(|e| Error::Ceviri(e.to_string()))?;
    for _ in 0..YAKALAMA_DENEMESI {
        match y.kare(YAKALAMA_ZAMAN_ASIMI_MS) {
            Ok(KareDurumu::Yeni) => {
                return y.oku().map_err(|e| Error::Ceviri(e.to_string()));
            }
            Ok(KareDurumu::Bos) => continue,
            Err(e) => return Err(Error::Ceviri(e.to_string())),
        }
    }
    // Masaüstü çoğaltması yalnızca DEĞİŞİKLİK veriyor; hiç kare gelmemesi
    // "ekran donmuş" değil "ekranda hiçbir şey değişmiyor" demek.
    Err(Error::Ceviri(
        "ekrandan yeni bir kare gelmedi — oyun duraklatılmış ya da münhasır tam ekranda \
         olabilir; kenarlıksız pencere modunda çalışıyor"
            .into(),
    ))
}

#[cfg(not(windows))]
pub(crate) fn yakala(_ekran: usize) -> Result<crate::scaling::algoritma::Goruntu> {
    Err(Error::Unsupported("ekran yakalama"))
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn varsayilan_kapali() {
        // Kısayol kapan, ekran okuyan bir özellik kendiliğinden açılmamalı.
        let c = Ceviri::yeni();
        assert!(!c.acik());
        assert!(!c.durum().acik);
        assert!(c.durum().kisayol.is_none());
    }

    #[test]
    fn kapaliyken_elle_ceviri_hata_veriyor() {
        let c = Ceviri::yeni();
        assert!(c.simdi_cevir().is_err());
    }

    #[test]
    fn yapilandirma_duruma_yansiyor() {
        let c = Ceviri::yeni();
        c.yapilandir(Yapilandirma {
            oyun: "skyrim.exe".into(),
            alan: Alan::pikselden(0, 800, 1920, 200, 1920, 1080),
            ..Default::default()
        });
        let d = c.durum();
        assert_eq!(d.oyun, "skyrim.exe");
        assert!(!d.tum_ekran, "alan seçiliyken 'tüm ekran' yazıyor");
    }

    #[test]
    fn varsayilan_alan_tum_ekran() {
        let c = Ceviri::yeni();
        c.yapilandir(Yapilandirma::default());
        assert!(c.durum().tum_ekran);
    }

    #[test]
    fn sonuc_bayragi_bir_kez_devraliniyor() {
        let c = Ceviri::yeni();
        c.yeni.store(true, Ordering::Relaxed);
        assert!(c.sonucu_devral());
        assert!(!c.sonucu_devral(), "bayrak silinmedi, olay tekrarlanır");
    }

    #[test]
    fn dil_cifti_sabit() {
        // Model tek yönlü; arayüzde seçenekmiş gibi gösterilmemeli.
        let d = CeviriDurumu::default();
        assert_eq!(d.kaynak_dil, "en");
        assert_eq!(d.hedef_dil, "tr");
    }
}
