//! Ölçekleme (Faz 3): ekranı oku, ölçekle, üstte göster.
//!
//! ```text
//! yakalama.rs   Desktop Duplication → D3D11 dokusu   (oyuna dokunmadan)
//! sunum.rs      gölgelendirici + üstte duran pencere  (gerçek zamanlı yol)
//! algoritma.rs  aynı matematiğin CPU referansı        (test edilebilir yol)
//! gecikme.rs    boru hattının EKLEDİĞİ gecikme        (kabul kriteri 2)
//! ```
//!
//! # Bu modül sistemde kalıcı bir şey değiştirmiyor
//!
//! Değişmez kural (`CLAUDE.md`): sistemde bir şey değiştiren her yol
//! `state::Motor`dan geçer ve **hem deftere hem günlüğe** yazar. Ölçekleme
//! deftere yazmıyor ve bu bir istisna değil, kuralın kendisinden çıkan
//! sonuç: burada geri alınacak bir şey yok. Açılan tek şey bir pencere ve o
//! pencere sürecin ömrüyle sınırlı — program çökerse ekranda kalmıyor,
//! registry'de bir iz bırakmıyor, bir sonraki açılışta temizlenecek bir
//! kalıntı üretmiyor. Günlüğe ise **yazıyor**: kullanıcı ne zaman
//! başladığını, hangi algoritmayla çalıştığını ve neden durduğunu görüyor.
//!
//! # Rekabetçi modda kapalı
//!
//! Ölçekleme her karede ölçülebilir bir gecikme ekliyor (`gecikme.rs`).
//! Rekabetçi mod tam olarak bu gecikmeyi en aza indirmek için var. İkisini
//! aynı anda açık tutmak, kullanıcının seçtiği şeyin tersini yapmak olurdu;
//! bu yüzden kısıtlı değil, **kapalı**. `testler::rekabetci_modda_kapali`
//! bu duruşu tutuyor.

pub mod algoritma;
pub mod gecikme;
pub mod sunum;
pub mod yakalama;

pub use algoritma::{Algoritma, Goruntu};
pub use gecikme::{GecikmeOzeti, GecikmeTamponu, KareOlcumu};
pub use yakalama::{Ekran, Engel};

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::profile_engine::Mod;

/// Yakalama turunun zaman aşımı.
///
/// 16 ms ≈ 60 Hz'de bir kare. Daha uzun olsaydı oyun kare üretmediğinde
/// döngü uzun süre uyur ve durdurma isteğine geç cevap verirdi; daha kısa
/// olsaydı boş turlarda CPU'yu boşuna döndürürdük.
const KARE_ZAMAN_ASIMI_MS: u32 = 16;

/// Özetin paylaşılan duruma yazılma sıklığı (kare).
const OZET_ARALIGI: u32 = 30;

/// Ölçekleme başlarken beklenen açılış cevabı.
///
/// Yakalama açılamıyorsa kullanıcı bunu **hemen** görmeli; arka planda
/// sessizce çalışmayan bir özellik, çalışmadığını söylemeyen bir özellikten
/// iyidir.
const ACILIS_BEKLEME: std::time::Duration = std::time::Duration::from_secs(5);

/// Arayüze giden durum.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OlceklemeDurumu {
    pub calisiyor: bool,
    pub algoritma: Option<Algoritma>,
    pub ekran: Option<usize>,
    /// Yakalanan görüntünün boyutu.
    pub kaynak_genislik: u32,
    pub kaynak_yukseklik: u32,
    /// Sunum penceresinin boyutu.
    pub hedef_genislik: u32,
    pub hedef_yukseklik: u32,
    pub gecikme: Option<GecikmeOzeti>,
    /// Neden durduğu. Kullanıcıya gösterilecek cümle.
    pub son_engel: Option<String>,
    /// Çalışıyor ama bir şey beklendiği gibi değil.
    ///
    /// Hatadan ayrı bir alan: ölçekleme sürüyor, kullanıcı görüntüyü
    /// görüyor, ama bir kısıt var (örneğin pencere yakalamanın dışına
    /// çıkarılamadı). Bunu hata gibi göstermek "durdu" diye okunurdu;
    /// hiç göstermemek ise gördüğü bozukluğu açıklamasız bırakırdı.
    pub uyari: Option<String>,
    /// Ölçeklemeyi klavyeden durduran kısayolun etiketi (karar #34).
    ///
    /// Arayüz bunu ölçekleme **başlamadan önce de** gösteriyor: kaçış
    /// yolunu ancak ekran kaplandıktan sonra öğrenen kullanıcı için o yol
    /// yok demektir.
    pub durdurma_kisayoli: Option<String>,
    /// Ölçekleme açık ama ekranda bir şey yok: ölçeklenecek pencere
    /// bulunamadı.
    ///
    /// Hata değil, bekleme durumu — kullanıcı Muifly'ın kendi penceresine
    /// bakarken normal olan hâl. Arayüzün "çalışıyor ama neden bir şey
    /// görmüyorum" sorusuna cevabı bu alan.
    pub hedef_bekleniyor: bool,
    /// Döngü kaçış kısayoluyla durdu mu?
    ///
    /// Arka plan döngüsü bunu görüp günlüğe yazıyor: ölçeklemeyi
    /// durduran her yol günlükte görünmeli, kısayolla durdurulan da
    /// (şeffaflık ilkesi — `lib.rs::arka_plan_dongusu`).
    pub kacisla_durduruldu: bool,
}

/// Arayüzdeki algoritma listesi.
///
/// Açıklama metinleri Rust tarafında duruyor, arayüzde kopyalanmıyor: aynı
/// cümlenin iki dosyada bakımı, birinin sessizce eskimesi demek olurdu ve
/// eskiyen cümle tasarım ilkesi 4'ün korunduğu yer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlgoritmaBilgisi {
    pub anahtar: String,
    pub ad: String,
    pub aciklama: String,
}

pub fn algoritmalar() -> Vec<AlgoritmaBilgisi> {
    Algoritma::hepsi()
        .into_iter()
        .map(|a| AlgoritmaBilgisi {
            anahtar: a.anahtar().to_string(),
            ad: a.ad().to_string(),
            aciklama: a.aciklama().to_string(),
        })
        .collect()
}

/// Tek karelik yakalama denemesinin sonucu.
///
/// Kullanıcı ölçeklemeyi açmadan önce yakalamanın çalışıp çalışmadığını
/// görebiliyor. Sorunun yakalamada mı sunumda mı olduğunu ayırt etmek, hata
/// mesajını tahminden çıkarıyor: "ölçekleme açılmadı" cümlesi tek başına
/// kullanıcıya yapacak bir şey bırakmıyor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YakalamaDenemesi {
    pub genislik: u32,
    pub yukseklik: u32,
    /// Deneme süresi içinde ekranda değişiklik oldu mu?
    ///
    /// `false` bir hata değil: hareketsiz bir masaüstünde yeni kare
    /// üretilmiyor. Arayüz bunu böyle yazıyor.
    pub yeni_kare: bool,
}

/// Yakalamanın çalışıp çalışmadığını bir kez dener.
///
/// Sunum penceresi açılmıyor: bu deneme ekranda hiçbir şey göstermiyor,
/// yalnızca çoğaltmanın açılıp açılmadığını söylüyor.
pub fn deneme(ekran: usize) -> Result<YakalamaDenemesi, Engel> {
    #[cfg(windows)]
    {
        use yakalama::{KareDurumu, Yakalayici};
        let mut y = Yakalayici::ac(ekran)?;
        let mut yeni_kare = false;
        // Yarım saniye: hareketsiz bir masaüstünde de en az bir tur dönsün,
        // ama kullanıcı düğmeye bastıktan sonra beklemekte kalmasın.
        let bitis = std::time::Instant::now() + std::time::Duration::from_millis(500);
        while std::time::Instant::now() < bitis {
            match y.kare(KARE_ZAMAN_ASIMI_MS)? {
                KareDurumu::Yeni => {
                    yeni_kare = true;
                    break;
                }
                KareDurumu::Bos => continue,
            }
        }
        // Bir kare CPU'ya indiriliyor: dokunun gerçekten okunabildiğini
        // doğrulayan tek adım bu.
        let g = y.oku()?;
        Ok(YakalamaDenemesi {
            genislik: g.genislik,
            yukseklik: g.yukseklik,
            yeni_kare,
        })
    }
    #[cfg(not(windows))]
    {
        let _ = ekran;
        Err(Engel::Platform)
    }
}

/// Ölçeklemenin bu modda çalışmasına izin var mı?
///
/// Saf fonksiyon: sistemi okumuyor, karar veriyor.
pub fn moda_uygun(mod_: &Mod) -> bool {
    !matches!(mod_, Mod::Rekabetci { .. })
}

/// Ölçekleme iş parçacığının denetleyicisi.
///
/// D3D11 nesneleri iş parçacığının içinde kalıyor ve dışarı çıkmıyor:
/// cihaz, çoğaltma ve pencere aynı iş parçacığında oluşturuluyor,
/// kullanılıyor ve yok ediliyor. Pencere mesaj kuyruğu zaten onu yaratan
/// iş parçacığına bağlı; nesneleri paylaşmaya çalışmak, kazanılacak bir şey
/// olmadan bir sürü kilit demek olurdu.
pub struct Olcekleyici {
    durum: Arc<Mutex<OlceklemeDurumu>>,
    dur: Arc<AtomicBool>,
    /// Çalışırken algoritma değiştirilebiliyor: yeniden başlatmak, ekranın
    /// bir anlığına kararması demek olurdu.
    algoritma: Arc<AtomicU8>,
    is_parcasi: Option<std::thread::JoinHandle<()>>,
}

impl Default for Olcekleyici {
    fn default() -> Self {
        Self::yeni()
    }
}

impl Olcekleyici {
    pub fn yeni() -> Self {
        Self {
            durum: Arc::new(Mutex::new(OlceklemeDurumu::default())),
            dur: Arc::new(AtomicBool::new(false)),
            algoritma: Arc::new(AtomicU8::new(0)),
            is_parcasi: None,
        }
    }

    pub fn calisiyor(&self) -> bool {
        self.is_parcasi.is_some() && self.durum.lock().calisiyor
    }

    pub fn durum(&self) -> OlceklemeDurumu {
        self.durum.lock().clone()
    }

    /// Çalışan ölçeklemenin algoritmasını değiştirir.
    ///
    /// Ölçüm tamponu iş parçacığında sıfırlanıyor: eski algoritmanın
    /// süreleri yenisinin ortalamasına karışsaydı, kullanıcı iki
    /// algoritmayı karşılaştıramazdı.
    pub fn algoritma_ata(&self, a: Algoritma) {
        self.algoritma.store(indeks(a) as u8, Ordering::Relaxed);
    }

    /// Ölçeklemeyi başlatır.
    ///
    /// Açılış hatası **çağırana** dönüyor, arka planda bir günlük satırına
    /// gömülmüyor: kullanıcı düğmeye bastıysa cevabı ekranda görmeli.
    pub fn baslat(&mut self, ekran: usize, algo: Algoritma) -> Result<(), Engel> {
        self.durdur();
        // Önceki turdan devralınmamış bir kaçış bayrağı, yeni başlayan
        // ölçeklemeyi "kısayolla durduruldu" diye günlüğe yazdırırdı.
        self.durum.lock().kacisla_durduruldu = false;
        self.algoritma_ata(algo);
        self.dur.store(false, Ordering::Relaxed);

        let (gonderici, alici) = std::sync::mpsc::channel::<Result<(), Engel>>();
        let durum = Arc::clone(&self.durum);
        let dur = Arc::clone(&self.dur);
        let secili = Arc::clone(&self.algoritma);

        let is = std::thread::Builder::new()
            .name("muifly-olcekleme".into())
            .spawn(move || dongu(ekran, durum, dur, secili, gonderici))
            .map_err(|_| Engel::Desteklenmeyen)?;

        match alici.recv_timeout(ACILIS_BEKLEME) {
            Ok(Ok(())) => {
                self.is_parcasi = Some(is);
                Ok(())
            }
            Ok(Err(e)) => {
                let _ = is.join();
                Err(e)
            }
            Err(_) => {
                // Açılış cevabı gelmedi: iş parçacığını bırakmıyoruz,
                // durduruyoruz. Aksi halde arkada sahipsiz bir pencere
                // kalabilirdi.
                self.dur.store(true, Ordering::Relaxed);
                let _ = is.join();
                Err(Engel::Sistem(0))
            }
        }
    }

    /// Durdurur ve iş parçacığının bitmesini bekler.
    ///
    /// Beklemek şart: pencere iş parçacığının içinde yok ediliyor ve
    /// beklenmezse "durdurdum" dedikten sonra ekranda bir süre daha duran
    /// bir pencere kalırdı.
    pub fn durdur(&mut self) {
        self.dur.store(true, Ordering::Relaxed);
        if let Some(is) = self.is_parcasi.take() {
            let _ = is.join();
        }
        let mut d = self.durum.lock();
        d.calisiyor = false;
        d.gecikme = None;
        d.hedef_bekleniyor = false;
        d.durdurma_kisayoli = None;
    }

    /// Döngü kendi kendine (kaçış kısayoluyla) durduysa bayrağı **alıp**
    /// siler.
    ///
    /// Alıp silmek şart: bayrak kalsaydı arka plan döngüsü her turda aynı
    /// satırı günlüğe yazardı.
    pub fn kacisi_devral(&mut self) -> bool {
        let mut d = self.durum.lock();
        std::mem::replace(&mut d.kacisla_durduruldu, false)
    }
}

impl Drop for Olcekleyici {
    fn drop(&mut self) {
        self.durdur();
    }
}

fn indeks(a: Algoritma) -> usize {
    Algoritma::hepsi().iter().position(|x| *x == a).unwrap_or(0)
}

fn algoritmadan(i: u8) -> Algoritma {
    *Algoritma::hepsi()
        .get(i as usize)
        .unwrap_or(&Algoritma::TamSayi)
}

// ---------------------------------------------------------------------------
// Döngü
// ---------------------------------------------------------------------------

#[cfg(windows)]
fn dongu(
    ekran: usize,
    durum: Arc<Mutex<OlceklemeDurumu>>,
    dur: Arc<AtomicBool>,
    secili: Arc<AtomicU8>,
    gonderici: std::sync::mpsc::Sender<Result<(), Engel>>,
) {
    use std::time::Instant;
    use yakalama::{KareDurumu, Yakalayici};

    let mut yakalayici = match Yakalayici::ac(ekran) {
        Ok(y) => y,
        Err(e) => {
            let _ = gonderici.send(Err(e));
            return;
        }
    };

    // Sunum penceresi yakalanan ekranın tamamını kaplıyor. Köşe
    // koordinatları yakalayıcının kendi ekran kaydından geliyor: çok ekranlı
    // kurulumda ikinci ekranın kökeni (0,0) değil ve sabit (0,0) yazsaydık
    // pencere her zaman birincil ekranda belirirdi.
    let (x, y) = (yakalayici.ekran().x, yakalayici.ekran().y);

    let mut pencere = match sunum::SunumPenceresi::ac(
        &yakalayici.cihaz,
        &yakalayici.baglam,
        x,
        y,
        yakalayici.genislik,
        yakalayici.yukseklik,
    ) {
        Ok(p) => p,
        Err(e) => {
            let _ = gonderici.send(Err(e));
            return;
        }
    };

    // Kaçış kısayolu pencereden ÖNCE değil, hemen sonra ve **açılış
    // cevabından önce** kaydediliyor: kaydedilemezse ölçekleme hiç
    // başlamıyor. Kaçış yolu olmayan bir tam ekran kaplama, kullanıcıya
    // makineyi yeniden başlatmaktan başka çıkış bırakmıyor (karar #34).
    let Some(kacis) = sunum::KacisKisayolu::kaydet() else {
        let _ = gonderici.send(Err(Engel::KacisKisayoluYok));
        return;
    };

    {
        let mut d = durum.lock();
        d.calisiyor = true;
        d.ekran = Some(ekran);
        d.algoritma = Some(algoritmadan(secili.load(Ordering::Relaxed)));
        d.kaynak_genislik = yakalayici.genislik;
        d.kaynak_yukseklik = yakalayici.yukseklik;
        d.hedef_genislik = pencere.genislik;
        d.hedef_yukseklik = pencere.yukseklik;
        d.son_engel = None;
        d.gecikme = None;
        d.durdurma_kisayoli = Some(kacis.etiket.to_string());
        d.hedef_bekleniyor = true;
        d.uyari = (!pencere.yakalamadan_gizli).then(|| {
            "Bu Windows sürümü pencereyi yakalamanın dışına çıkaramıyor; \
             ölçekleme kendi çıktısını yakalayabilir (Windows 10 sürüm 2004 \
             ve üstü gerekiyor)."
                .to_string()
        });
    }
    let _ = gonderici.send(Ok(()));

    let mut tampon = GecikmeTamponu::yeni(gecikme::KAPASITE);
    let mut onceki_algo = algoritmadan(secili.load(Ordering::Relaxed));
    let mut sayac = 0u32;
    let mut engel_metni: Option<String> = None;
    let mut hedef_pencere: Option<windows::Win32::Foundation::HWND> = None;
    // Durum yalnızca **değişince** yazılıyor: her karede kilit almak
    // döngünün ölçtüğü sürelere kendi gürültüsünü katardı.
    let mut bekliyordu = true;
    let mut kacisla_durduruldu = false;

    while !dur.load(Ordering::Relaxed) {
        match pencere.mesajlari_isle() {
            sunum::TurSonucu::Devam => {}
            sunum::TurSonucu::Kapandi => break,
            sunum::TurSonucu::Kacis => {
                kacisla_durduruldu = true;
                break;
            }
        }

        // Ölçeklenecek pencere önde DEĞİLKEN sunum penceresi gizleniyor.
        //
        // Eski davranış iki yerde ekranı kilitliyordu (karar #34):
        //
        // - Ölçeklenecek pencere hiç yokken masaüstünün tamamı ölçekleniyor,
        //   yani birebir kopyalanıyordu. Ekranı kaplayan, odak almayan,
        //   Alt+Tab'da olmayan bir pencerenin altında canlı masaüstü
        //   görünmez oluyordu; kopya tazelenmeyi kestiği anda kullanıcının
        //   önünde donmuş bir resim kalıyordu.
        // - Kullanıcı ayar değiştirmek için Muifly'a geçtiğinde önceki
        //   hedef korunuyor, yani Muifly'ın kendi penceresinin ÜSTÜNDE
        //   oyunun görüntüsü çiziliyordu. "Durdur" düğmesi ekranda vardı
        //   ama görünmüyordu.
        //
        // Gizlemek "hiçbir şey yapma" değil: hedef öne geldiğinde pencere
        // tekrar açılıyor, arada ekran kullanıcınındır.
        let onplan = onplandaki(hedef_pencere);
        if let Onplan::Hedef(h) = onplan {
            hedef_pencere = Some(h);
        }
        let hedef_alani = match onplan {
            Onplan::Hedef(h) => pencere_dikdortgeni(h).and_then(|dikdortgen| {
                algoritma::pencere_alani(
                    dikdortgen,
                    (yakalayici.ekran().x, yakalayici.ekran().y),
                    (yakalayici.genislik, yakalayici.yukseklik),
                )
            }),
            Onplan::Bizim | Onplan::Yok => None,
        };
        let Some(alan) = hedef_alani else {
            pencere.gorunurluk(false);
            if !bekliyordu {
                bekliyordu = true;
                durum.lock().hedef_bekleniyor = true;
                tampon.temizle();
            }
            // Uyumak şart: yakalama yapılmadığı için turun kendi
            // beklemesi yok, uyunmazsa döngü boşuna dönerdi.
            std::thread::sleep(std::time::Duration::from_millis(
                KARE_ZAMAN_ASIMI_MS as u64,
            ));
            continue;
        };
        if bekliyordu {
            bekliyordu = false;
            durum.lock().hedef_bekleniyor = false;
        }

        let algo = algoritmadan(secili.load(Ordering::Relaxed));
        if algo != onceki_algo {
            // Ölçümler algoritmaya ait: karıştırılırsa karşılaştırma
            // anlamını yitirir.
            tampon.temizle();
            onceki_algo = algo;
            durum.lock().algoritma = Some(algo);
        }

        let t0 = Instant::now();
        match yakalayici.kare(KARE_ZAMAN_ASIMI_MS) {
            Ok(KareDurumu::Bos) => {
                tampon.bos_tur();
                continue;
            }
            Ok(KareDurumu::Yeni) => {}
            Err(Engel::ErisimKesildi) => {
                // Ekran modu değişti (oyun açılırken sık olur). Bir kez
                // yeniden denemek, kullanıcıya hata göstermekten iyi.
                if yakalayici.yeniden_ac().is_err() {
                    engel_metni = Some(Engel::ErisimKesildi.to_string());
                    break;
                }
                continue;
            }
            Err(e) => {
                engel_metni = Some(e.to_string());
                break;
            }
        }
        let t1 = Instant::now();

        let cizim = pencere.ciz(
            &yakalayici.gorunum,
            alan,
            yakalayici.genislik,
            yakalayici.yukseklik,
            algo,
        );
        let t2 = Instant::now();
        if let Err(e) = cizim {
            engel_metni = Some(e.to_string());
            break;
        }
        // Pencere ilk **başarılı** çizimden sonra gösteriliyor. Önce
        // gösterip sonra çizmek, bir kare boyunca ekranı kaplayan boş bir
        // siyah dikdörtgen demek olurdu.
        pencere.gorunurluk(true);

        // Üç aşama ayrı ölçülüyor ama üçü de **CPU tarafındaki** süre.
        // `sunum` içinde dikey eşitleme beklemesi de var; arayüz bunu
        // böyle yazıyor, çünkü "1 ms yakalama, 15 ms sunum" satırını
        // açıklamadan göstermek yanlış okunurdu.
        tampon.ekle(KareOlcumu {
            yakalama_us: t1.duration_since(t0).as_micros().min(u32::MAX as u128) as u32,
            olcekleme_us: 0,
            sunum_us: t2.duration_since(t1).as_micros().min(u32::MAX as u128) as u32,
        });

        sayac += 1;
        if sayac % OZET_ARALIGI == 0 {
            let mut d = durum.lock();
            d.gecikme = tampon.ozetle();
            // Kaynak boyutu ölçeklenen ALAN, yakalanan dokunun tamamı
            // değil: kullanıcı "1280×720 → 2560×1440" görmeli, ekranın
            // kendi çözünürlüğünü değil.
            d.kaynak_genislik = alan.genislik;
            d.kaynak_yukseklik = alan.yukseklik;
        }
    }

    // Kısayol kaydı burada düşüyor: kombinasyonun ölçekleme kapalıyken de
    // bizde kalması, kullanıcının kendi kısayolunu çalmak olurdu.
    drop(kacis);

    let mut d = durum.lock();
    d.calisiyor = false;
    d.gecikme = tampon.ozetle();
    d.hedef_bekleniyor = false;
    d.durdurma_kisayoli = None;
    d.kacisla_durduruldu = kacisla_durduruldu;
    d.son_engel = engel_metni;
}

/// Önplanda ne var?
///
/// Üç durumun ayrılması şart, çünkü ikisi "çizme" diyor ama farklı
/// sebeplerle ve farklı sonuçlarla (karar #34).
#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Onplan {
    /// Ölçeklenecek pencere önde.
    Hedef(windows::Win32::Foundation::HWND),
    /// Önde **bizim** penceremiz var: kullanıcı Muifly'a bakıyor.
    ///
    /// Sunum penceresi gizleniyor ama hedef unutulmuyor: kullanıcı ayar
    /// değiştirip oyuna döndüğünde ölçekleme kaldığı yerden sürüyor.
    Bizim,
    /// Geçerli bir önplan penceresi yok (masaüstü, kilit ekranı, geçiş anı).
    Yok,
}

/// Önplandaki pencereyi sınıflandırır.
///
/// Kendi süreçlerimizi ayırmak şart. Kullanıcı "Başlat"a Muifly
/// penceresinden basıyor; o an öndeki pencere Muifly'ın kendisi oluyor ve
/// ayrılmazsa program kendi arayüzünü büyütürdü. Sunum penceresi de bize
/// ait — odak almıyor ama bir sürücü yolunda öne geçerse aynı sonuç
/// çıkardı.
///
/// `onceki` yalnızca **aynı pencere hâlâ geçerli mi** sorusu için duruyor:
/// hedef kapandıysa `Yok` dönüyor, korunmuyor. Kapanmış bir pencerenin son
/// karesini ekranda tutmak, kullanıcının donmuş sandığı görüntünün ta
/// kendisiydi.
#[cfg(windows)]
fn onplandaki(onceki: Option<windows::Win32::Foundation::HWND>) -> Onplan {
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId, IsWindow,
    };
    unsafe {
        let pencere = GetForegroundWindow();
        if pencere.is_invalid() {
            return Onplan::Yok;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(pencere, Some(&mut pid));
        if pid == 0 {
            return Onplan::Yok;
        }
        if pid == GetCurrentProcessId() {
            // Önde biziz. Eski hedef hâlâ yaşıyorsa "Bizim", yoksa "Yok":
            // ikisi de gizliyor, ama "Bizim" hedefi hatırlıyor.
            return match onceki {
                Some(h) if IsWindow(Some(h)).as_bool() => Onplan::Bizim,
                _ => Onplan::Yok,
            };
        }
        Onplan::Hedef(pencere)
    }
}

/// Pencerenin **istemci alanının** masaüstü koordinatındaki dikdörtgeni.
///
/// İstemci alanı, pencere çerçevesi değil: kenarlıksız bir oyunda ikisi
/// aynı, pencereli bir oyunda ise çerçeve ve başlık çubuğu ölçeklenecek
/// görüntünün parçası değil. `GetWindowRect` kullanılsaydı büyütülen şeyin
/// içinde başlık çubuğu da olurdu.
#[cfg(windows)]
fn pencere_dikdortgeni(pencere: windows::Win32::Foundation::HWND) -> Option<(i32, i32, i32, i32)> {
    use windows::Win32::Foundation::{POINT, RECT};
    use windows::Win32::Graphics::Gdi::ClientToScreen;
    use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, IsIconic};
    unsafe {
        // Küçültülmüş pencerenin dikdörtgeni anlamsız.
        if IsIconic(pencere).as_bool() {
            return None;
        }
        let mut r = RECT::default();
        GetClientRect(pencere, &mut r).ok()?;
        let mut kose = POINT {
            x: r.left,
            y: r.top,
        };
        if !ClientToScreen(pencere, &mut kose).as_bool() {
            return None;
        }
        Some((
            kose.x,
            kose.y,
            kose.x + (r.right - r.left),
            kose.y + (r.bottom - r.top),
        ))
    }
}

#[cfg(not(windows))]
fn dongu(
    _ekran: usize,
    _durum: Arc<Mutex<OlceklemeDurumu>>,
    _dur: Arc<AtomicBool>,
    _secili: Arc<AtomicU8>,
    gonderici: std::sync::mpsc::Sender<Result<(), Engel>>,
) {
    let _ = gonderici.send(Err(Engel::Platform));
}

#[cfg(test)]
mod testler {
    use super::*;

    fn rekabetci() -> Mod {
        Mod::Rekabetci {
            pid: 1,
            surec: "oyun.exe".into(),
            profil_id: None,
        }
    }

    /// Kaçış kısayolu gerçekten kaydedilebiliyor mu?
    ///
    /// Bu testin sebebi karar #34: kısayol kaydedilemezse ölçekleme hiç
    /// başlamıyor, yani bu yol kırılırsa özellik tamamen ölür. Kayıt
    /// başarısız olursa test değil ürün kırılmış olur — o yüzden `kaydet`
    /// burada gerçekten çağrılıyor, taklit edilmiyor.
    ///
    /// Kısayol test bittiğinde `Drop` ile geri veriliyor; testi çalıştıran
    /// makinede kombinasyon kalıcı olarak bizde kalmıyor.
    #[cfg(windows)]
    #[test]
    fn kacis_kisayolu_kaydedilebiliyor() {
        let k = sunum::KacisKisayolu::kaydet().expect(
            "hiçbir kaçış adayı kaydedilemedi — ölçekleme bu makinede \
             hiç başlamayacak demektir",
        );
        assert!(!k.etiket.is_empty());
    }

    /// İkinci kayıt **aynı** kombinasyonu almıyor.
    ///
    /// Aday listesinin varlık sebebi bu: birinci kombinasyon meşgulse
    /// ikinciye düşülüyor. Liste tek elemanlı olsaydı kısayolu kullanan
    /// herhangi bir uygulama ölçeklemeyi tamamen engellerdi.
    #[cfg(windows)]
    #[test]
    fn ikinci_kayit_baska_adaya_dusuyor() {
        let ilk = sunum::KacisKisayolu::kaydet().expect("ilk kayıt");
        let ikinci = sunum::KacisKisayolu::kaydet().expect("ikinci kayıt");
        assert_ne!(
            ilk.etiket, ikinci.etiket,
            "iki kayıt aynı kombinasyonu aldı: aday listesi işlemiyor"
        );
    }

    /// Durum, kaçış yolunu arayüze taşıyor mu?
    ///
    /// Kısayol kullanıcıya gösterilmezse var sayılmaz: ekranı kaplayan
    /// pencerenin nasıl kapatılacağını bilmeyen kullanıcı için o pencere
    /// hâlâ çıkışsızdır (karar #34).
    #[test]
    fn durumda_kacis_yolu_alanlari_var() {
        let d = OlceklemeDurumu {
            durdurma_kisayoli: Some("Ctrl+Alt+Shift+S".into()),
            hedef_bekleniyor: true,
            ..Default::default()
        };
        let json = serde_json::to_string(&d).expect("serile");
        assert!(json.contains("durdurmaKisayoli"), "{json}");
        assert!(json.contains("hedefBekleniyor"), "{json}");
        assert!(json.contains("kacislaDurduruldu"), "{json}");
    }

    /// Ürün duruşu: rekabetçi modda ölçekleme kapalı.
    ///
    /// Kısıtlı değil kapalı; gerekçe modül belgesinde. Bu testin varlık
    /// sebebi `CLAUDE.md`'nin kuralı: yeni bir "yapmıyoruz" kararı
    /// verildiğinde onu koruyan bir test yazılır.
    #[test]
    fn rekabetci_modda_kapali() {
        assert!(!moda_uygun(&rekabetci()));
        assert!(!moda_uygun(&Mod::Rekabetci {
            pid: 2,
            surec: "x.exe".into(),
            profil_id: Some("p".into()),
        }));
    }

    #[test]
    fn diger_modlarda_serbest() {
        assert!(moda_uygun(&Mod::Bosta));
        assert!(moda_uygun(&Mod::SistemAcilisi));
        assert!(moda_uygun(&Mod::OyunGenel {
            pid: 1,
            surec: "a.exe".into()
        }));
        assert!(moda_uygun(&Mod::OyunProfili {
            pid: 1,
            surec: "a.exe".into(),
            profil_id: "p".into()
        }));
    }

    #[test]
    fn algoritma_indeksi_gidip_geliyor() {
        // Atomik saklama indeksle yapılıyor; sıra bozulursa kullanıcının
        // seçtiğinden başka bir algoritma çalışırdı.
        for a in Algoritma::hepsi() {
            assert_eq!(algoritmadan(indeks(a) as u8), a);
        }
        // Bozuk değer varsayılana düşüyor, panik etmiyor.
        assert_eq!(algoritmadan(200), Algoritma::TamSayi);
    }

    #[test]
    fn baslamamis_olcekleyici_bos_durum() {
        let o = Olcekleyici::yeni();
        assert!(!o.calisiyor());
        let d = o.durum();
        assert!(!d.calisiyor);
        assert!(d.gecikme.is_none());
        assert!(d.son_engel.is_none());
    }

    #[test]
    fn durdurma_baslamamisken_de_guvenli() {
        // Arayüz "durdur" düğmesini iki kez gönderebilir.
        let mut o = Olcekleyici::yeni();
        o.durdur();
        o.durdur();
        assert!(!o.calisiyor());
    }

    /// Boru hattının gerçek bir ekranda uçtan uca koştuğunun kontrolü.
    ///
    /// Varsayılan olarak koşmuyor: birkaç saniye boyunca ekranın üstünde
    /// bir pencere açıyor ve gerçek bir ekran kartı istiyor. Birim testi
    /// değil, elle çalıştırılan bir doğrulama:
    ///
    /// ```text
    /// cargo test gercek_ekranda_bir_tur -- --ignored --nocapture
    /// ```
    ///
    /// Kontrol ettiği şey **çalıştığı**: yakalama açıldı mı, gölgelendirici
    /// derlendi mi, pencere kuruldu mu, kare ölçüldü mü, kapanışta engel
    /// kaldı mı. Görüntünün **doğru** olduğunu söylemiyor — onu ancak göz
    /// söyler (`tasks.md` → Sıradaki 7).
    #[cfg(windows)]
    #[test]
    #[ignore = "gerçek ekran açıyor"]
    fn gercek_ekranda_bir_tur() {
        // Hangi pencere ölçekleniyor: kırpmanın çalıştığı buradan görünüyor.
        // `Bizim`/`Yok` beklenen sonuç değil ama hata da değil: testi
        // konsoldan çalıştıran kullanıcının önünde bir oyun penceresi
        // olmayabilir. O durumda sunum penceresi gizli kalıyor (karar #34)
        // ve aşağıdaki `kaynak_genislik` beklemesi düşer — çıktı bunu
        // söylesin diye yazdırılıyor.
        let onplan = onplandaki(None);
        println!(
            "önplan: {onplan:?} → {:?}",
            match onplan {
                Onplan::Hedef(h) => pencere_dikdortgeni(h),
                _ => None,
            }
        );

        let mut o = Olcekleyici::yeni();
        if let Err(e) = o.baslat(0, Algoritma::Lanczos) {
            // Ekransız/uzak oturumda çalışmaması bir kod hatası değil.
            println!("başlatılamadı: {e}");
            return;
        }
        assert!(o.calisiyor(), "başladı dedi ama çalışmıyor");
        std::thread::sleep(std::time::Duration::from_secs(2));

        let d = o.durum();
        println!(
            "{}x{} → {}x{}",
            d.kaynak_genislik, d.kaynak_yukseklik, d.hedef_genislik, d.hedef_yukseklik
        );
        println!("gecikme: {:?}", d.gecikme);
        println!("engel: {:?}", d.son_engel);
        assert!(d.calisiyor, "iki saniye içinde durdu: {:?}", d.son_engel);
        assert!(d.kaynak_genislik > 0 && d.hedef_genislik > 0);

        o.durdur();
        assert!(!o.calisiyor(), "durdurma iş parçacığını kapatmadı");
    }

    /// Windows dışında ölçekleme açılmıyor ve bunu söylüyor.
    #[cfg(not(windows))]
    #[test]
    fn platform_disinda_engel_donuyor() {
        let mut o = Olcekleyici::yeni();
        assert_eq!(o.baslat(0, Algoritma::TamSayi), Err(Engel::Platform));
    }
}
