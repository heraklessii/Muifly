//! Ekran yakalama — Desktop Duplication API.
//!
//! # Neden bu API
//!
//! Tasarım ilkesi 3: oyun sürecine hiçbir şey enjekte edilmiyor. Bir
//! overlay kütüphanesinin yaptığı gibi oyunun `Present` çağrısına atlamak
//! ya da adres alanına DLL yazmak yerine, işletim sisteminin masaüstü
//! bileşicisinden (DWM) **dışarıdan** okuma yapılıyor. Oyunun belleğine
//! dokunulmuyor, çağrıları yönlendirilmiyor, süreç açılmıyor bile.
//!
//! Bu, `monitor::etw`nin kare ölçümünde yaptığı tercihin aynısı: ölçmek ya
//! da okumak için resmî bir kanal varken, olmayan bir kanalı zorlamıyoruz.
//!
//! # Yakalayamadığı yer: tam ekran (exclusive)
//!
//! Oyun münhasır tam ekran modundaysa masaüstü bileşicisi devre dışı kalıyor
//! ve `AcquireNextFrame` `DXGI_ERROR_ACCESS_LOST` dönüyor. Bu bir hata değil,
//! API'nin sınırı; çözümü de kullanıcının bileceği bir şey: oyunu kenarlıksız
//! pencere moduna almak. Bu yüzden `Engel::ErisimKesildi` ayrı bir varyant ve
//! metni ne yapılacağını söylüyor — "yakalama başarısız" demek, kullanıcıyı
//! çözümü olan bir sorunla baş başa bırakmak olurdu.
//!
//! # Kopya neden gerekli
//!
//! `AcquireNextFrame`in verdiği doku, `ReleaseFrame` çağrılana kadar
//! geçerli ve bu süre boyunca masaüstü bileşicisi bekliyor. Kareyi kendi
//! dokumuza kopyalayıp bırakmak, sistemin geri kalanını kendi ölçekleme
//! işimiz kadar bekletmemek için. Kopya GPU üzerinde; kare CPU'ya inmiyor.

use serde::{Deserialize, Serialize};

/// Yakalamanın neden yapılamadığı.
///
/// `monitor::etw::Engel` ile aynı gerekçe: hata metni değil tip. Arayüz her
/// durumda ne yazacağına kendisi karar veriyor, kullanıcıya Win32 kodu
/// gösterilmiyor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engel {
    /// Masaüstü çoğaltması alınamadı: oyun münhasır tam ekranda ya da ekran
    /// modu değişti.
    ErisimKesildi,
    /// Sistemde aynı anda açılabilecek çoğaltma sayısı dolmuş.
    /// Genelde başka bir yakalama/ölçekleme aracı açık demek.
    BaskaUygulamaKullaniyor,
    /// Ekran kartı ya da sürücü bu yolu desteklemiyor.
    Desteklenmeyen,
    /// İstenen ekran yok (takılı ekran sayısı değişmiş olabilir).
    EkranYok,
    /// Ölçeklemeyi durduracak klavye kısayolu kaydedilemedi.
    ///
    /// Ölçekleme bu durumda **başlamıyor**: kaçış yolu olmayan bir tam
    /// ekran kaplama, kullanıcıya makineyi yeniden başlatmaktan başka
    /// çıkış bırakmıyor (karar #34).
    KacisKisayoluYok,
    /// Başka bir sistem hatası.
    Sistem(i32),
    /// Windows dışı derleme.
    Platform,
}

impl std::fmt::Display for Engel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Engel::ErisimKesildi => write!(
                f,
                "ekran okunamadı — oyun münhasır tam ekranda olabilir; \
                 kenarlıksız pencere modunda çalışıyor"
            ),
            Engel::BaskaUygulamaKullaniyor => write!(
                f,
                "ekran çoğaltması başka bir uygulamada açık; onu kapatıp \
                 yeniden deneyin"
            ),
            Engel::Desteklenmeyen => {
                write!(f, "bu ekran kartı/sürücü masaüstü çoğaltmasını vermiyor")
            }
            Engel::EkranYok => write!(f, "seçilen ekran bulunamadı"),
            Engel::KacisKisayoluYok => write!(
                f,
                "ölçeklemeyi durduracak klavye kısayolu kaydedilemedi — \
                 kısayolu kullanan uygulamayı kapatıp yeniden deneyin; \
                 kaçış yolu olmadan ölçekleme başlatılmıyor"
            ),
            Engel::Sistem(k) => write!(f, "ekran yakalanamadı (sistem kodu 0x{k:08X})"),
            Engel::Platform => write!(f, "ekran yakalama bu platformda yok"),
        }
    }
}

/// Bir ekranın kullanıcıya gösterilen tanımı.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ekran {
    /// Listedeki sırası. Ayarlarda saklanan değer bu.
    pub indeks: usize,
    /// Sürücünün verdiği ad (`\\.\DISPLAY1` gibi).
    pub ad: String,
    pub genislik: u32,
    pub yukseklik: u32,
    pub birincil: bool,
    /// Ekranın masaüstündeki sol üst köşesi.
    ///
    /// Çok ekranlı kurulumda ikinci ekranın kökeni (0,0) değil; sunum
    /// penceresi buraya açılıyor. Bu iki alan olmasaydı ölçekleme
    /// penceresi her zaman birincil ekranda belirirdi.
    pub x: i32,
    pub y: i32,
    /// Hangi ekran kartına bağlı. Çok kartlı makinelerde çoğaltma, ekranı
    /// süren kartın cihazıyla açılmak zorunda.
    #[serde(skip)]
    pub adaptor: u32,
    #[serde(skip)]
    pub cikis: u32,
}

/// Bir turda ne olduğu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KareDurumu {
    /// Yeni kare alındı ve kendi dokumuza kopyalandı.
    Yeni,
    /// Zaman aşımı: oyun bu süre içinde yeni kare üretmedi. Hata değil.
    Bos,
}

#[cfg(windows)]
pub use win::{ekranlar, Yakalayici};

#[cfg(windows)]
mod win {
    use super::*;
    use windows::core::Interface;
    use windows::Win32::Graphics::Direct3D::{
        D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1,
    };
    use windows::Win32::Graphics::Direct3D11::*;
    use windows::Win32::Graphics::Dxgi::Common::*;
    use windows::Win32::Graphics::Dxgi::*;

    fn engel(e: &windows::core::Error) -> Engel {
        let kod = e.code();
        if kod == DXGI_ERROR_ACCESS_LOST {
            Engel::ErisimKesildi
        } else if kod == DXGI_ERROR_NOT_CURRENTLY_AVAILABLE {
            Engel::BaskaUygulamaKullaniyor
        } else if kod == DXGI_ERROR_UNSUPPORTED {
            Engel::Desteklenmeyen
        } else {
            Engel::Sistem(kod.0)
        }
    }

    /// Takılı ekranların listesi.
    ///
    /// Bütün adaptörler taranıyor, sadece birincisi değil: dizüstülerde ekran
    /// çoğu zaman tümleşik karta bağlı, oyun ise ayrık kartta koşuyor.
    /// Yalnızca adaptör 0'a bakan bir uygulama o makinelerde "ekran yok"
    /// derdi.
    pub fn ekranlar() -> Result<Vec<Ekran>, Engel> {
        unsafe {
            let fabrika: IDXGIFactory1 =
                CreateDXGIFactory1().map_err(|e| Engel::Sistem(e.code().0))?;
            let mut liste = Vec::new();
            let mut a = 0u32;
            while let Ok(adaptor) = fabrika.EnumAdapters1(a) {
                let mut c = 0u32;
                while let Ok(cikis) = adaptor.EnumOutputs(c) {
                    if let Ok(tanim) = cikis.GetDesc() {
                        let r = tanim.DesktopCoordinates;
                        let ad = String::from_utf16_lossy(
                            &tanim.DeviceName[..tanim
                                .DeviceName
                                .iter()
                                .position(|&x| x == 0)
                                .unwrap_or(tanim.DeviceName.len())],
                        );
                        liste.push(Ekran {
                            indeks: liste.len(),
                            ad,
                            genislik: (r.right - r.left).max(0) as u32,
                            yukseklik: (r.bottom - r.top).max(0) as u32,
                            // Birincil ekran, masaüstü kökeninde olan.
                            birincil: r.left == 0 && r.top == 0,
                            x: r.left,
                            y: r.top,
                            adaptor: a,
                            cikis: c,
                        });
                    }
                    c += 1;
                }
                a += 1;
            }
            if liste.is_empty() {
                return Err(Engel::EkranYok);
            }
            Ok(liste)
        }
    }

    /// Açık bir çoğaltma oturumu.
    ///
    /// `cihaz` ve `baglam` dışarıya açık: sunum penceresi **aynı** cihazı
    /// kullanıyor. İki ayrı cihaz olsaydı her kare bir paylaşımlı doku
    /// aktarımı gerektirirdi; ölçekleme boru hattının bütün kazancı orada
    /// giderdi.
    pub struct Yakalayici {
        pub cihaz: ID3D11Device,
        pub baglam: ID3D11DeviceContext,
        /// Çoğaltma oturumu.
        ///
        /// `Option`, çünkü yenilerken eskisinin **önce** bırakılması
        /// gerekiyor: aynı cihaz aynı çıkışı ikinci kez çoğaltamıyor
        /// (karar #36).
        cogaltma: Option<IDXGIOutputDuplication>,
        /// Yakalanan karenin kendi kopyamız. Gölgelendiriciye bu bağlanıyor.
        pub doku: ID3D11Texture2D,
        pub gorunum: ID3D11ShaderResourceView,
        pub genislik: u32,
        pub yukseklik: u32,
        ekran: Ekran,
        /// Son `kare()` çağrısında yeni kare beklenen süre (mikrosaniye).
        bekleme_us: u32,
    }

    impl Yakalayici {
        /// Seçilen ekran için çoğaltmayı açar.
        pub fn ac(indeks: usize) -> Result<Self, Engel> {
            let ekran = ekranlar()?
                .into_iter()
                .find(|e| e.indeks == indeks)
                .ok_or(Engel::EkranYok)?;
            Self::ekranla(ekran)
        }

        fn ekranla(ekran: Ekran) -> Result<Self, Engel> {
            unsafe {
                let fabrika: IDXGIFactory1 =
                    CreateDXGIFactory1().map_err(|e| Engel::Sistem(e.code().0))?;
                let adaptor = fabrika
                    .EnumAdapters1(ekran.adaptor)
                    .map_err(|_| Engel::EkranYok)?;
                let mut cihaz: Option<ID3D11Device> = None;
                let mut baglam: Option<ID3D11DeviceContext> = None;
                let adaptor_iface: IDXGIAdapter = adaptor.cast().map_err(|e| engel(&e))?;
                D3D11CreateDevice(
                    Some(&adaptor_iface),
                    // Adaptör verildiğinde sürücü tipi UNKNOWN olmak
                    // zorunda; HARDWARE yazmak E_INVALIDARG döndürür.
                    D3D_DRIVER_TYPE_UNKNOWN,
                    Default::default(),
                    // BGRA desteği: yakalanan biçim B8G8R8A8 ve sunum da
                    // aynı biçimde yapılıyor.
                    D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                    Some(&[D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0]),
                    D3D11_SDK_VERSION,
                    Some(&mut cihaz),
                    None,
                    Some(&mut baglam),
                )
                .map_err(|e| engel(&e))?;
                let (cihaz, baglam) = match (cihaz, baglam) {
                    (Some(c), Some(b)) => (c, b),
                    _ => return Err(Engel::Desteklenmeyen),
                };

                let cogaltma = cogalt(&cihaz, &ekran)?;

                let tanim = cogaltma.GetDesc();
                let genislik = tanim.ModeDesc.Width;
                let yukseklik = tanim.ModeDesc.Height;
                if genislik == 0 || yukseklik == 0 {
                    return Err(Engel::Desteklenmeyen);
                }

                let (doku, gorunum) = hedef_doku(&cihaz, genislik, yukseklik)?;

                Ok(Self {
                    cihaz,
                    baglam,
                    cogaltma: Some(cogaltma),
                    doku,
                    gorunum,
                    genislik,
                    yukseklik,
                    ekran,
                    bekleme_us: 0,
                })
            }
        }

        /// Hangi ekran yakalanıyor.
        pub fn ekran(&self) -> &Ekran {
            &self.ekran
        }

        /// Ekranın yenileme hızı (Hz), okunabiliyorsa.
        ///
        /// Kare üretimi (Faz 4) bu sayıya bağlı: ekran kaynaktan belirgin
        /// olarak hızlı değilse üretilen kare, gerçek karelerin sırasını
        /// bekletmekten başka bir işe yaramıyor. Sayı ölçülemezse özellik
        /// engellenmiyor ama uyarı da verilmiyor — olmayan bir ölçüme
        /// dayanarak kullanıcıyı uyarmak, tasarım ilkesi 4'ün yasakladığı
        /// şey.
        pub fn yenileme_hz(&self) -> Option<u32> {
            use windows::Win32::Graphics::Gdi::{
                EnumDisplaySettingsW, DEVMODEW, ENUM_CURRENT_SETTINGS,
            };
            unsafe {
                let mut ad: Vec<u16> = self.ekran.ad.encode_utf16().collect();
                ad.push(0);
                let mut mod_ = DEVMODEW {
                    dmSize: std::mem::size_of::<DEVMODEW>() as u16,
                    ..Default::default()
                };
                let ok = EnumDisplaySettingsW(
                    windows::core::PCWSTR(ad.as_ptr()),
                    ENUM_CURRENT_SETTINGS,
                    &mut mod_,
                );
                // 0 ve 1, "varsayılan/bilinmiyor" için ayrılmış değerler.
                (ok.as_bool() && mod_.dmDisplayFrequency > 1)
                    .then_some(mod_.dmDisplayFrequency)
            }
        }

        /// Çoğaltmayı kapatıp yeniden açar.
        ///
        /// `ErisimKesildi` sonrası tek doğru davranış bu: ekran modu
        /// değiştiğinde (oyun açılırken çok olur) eski çoğaltma bir daha
        /// kare vermiyor. Kullanıcıya hata göstermeden önce bir kez
        /// deneniyor.
        ///
        /// **Cihaz korunuyor** (karar #36). Eskiden burada yepyeni bir
        /// `Yakalayici` kuruluyordu; yeni cihazın dokusu, sunum
        /// penceresinin ve kare üreticisinin cihazına ait olmadığı için
        /// D3D11 çizimi sessizce yok sayardı: ölçekleme "çalışıyor"
        /// görünüp siyah kalırdı. Aynı cihazla yalnızca çoğaltma
        /// yenileniyor; cihaz da gitmişse (sürücü sıfırlaması) her şey
        /// yeniden kuruluyor ve çağıran bunu `true` olarak öğreniyor.
        ///
        /// Dönen değer: **aşağı akış yeniden kurulmalı mı?** Boyut, köşe
        /// ya da cihaz değiştiyse `true` — sunum penceresi ve kare
        /// üreticisi o değerlere göre ayrıldı.
        pub fn yeniden_ac(&mut self) -> Result<bool, Engel> {
            let eski_kose = (self.ekran.x, self.ekran.y);
            let eski_boyut = (self.genislik, self.yukseklik);

            // Ekranın güncel geometrisi: çözünürlük değiştiyse köşe de
            // kaymış olabilir (çok ekranlı masaüstünde sık). Ad üzerinden
            // eşleştiriliyor; liste sırası değişebilir ama sürücünün
            // verdiği ad aynı kalıyor.
            if let Ok(liste) = ekranlar() {
                if let Some(g) = liste.into_iter().find(|e| e.ad == self.ekran.ad) {
                    let indeks = self.ekran.indeks;
                    self.ekran = Ekran { indeks, ..g };
                }
            }

            // Eskisi ÖNCE bırakılıyor: aynı cihaz aynı çıkışı ikinci kez
            // çoğaltamaz, elde tutulursa yenisi hiç açılmaz.
            self.cogaltma = None;
            let Ok(cogaltma) = cogalt(&self.cihaz, &self.ekran) else {
                // Cihaz da gitti (sürücü sıfırlaması, kart değişimi).
                // Baştan kurmaktan başka yol yok ve bu, aşağı akıştaki
                // her şeyin yeniden kurulması demek.
                *self = Self::ekranla(self.ekran.clone())?;
                return Ok(true);
            };

            let tanim = unsafe { cogaltma.GetDesc() };
            let (g, y) = (tanim.ModeDesc.Width, tanim.ModeDesc.Height);
            if g == 0 || y == 0 {
                return Err(Engel::Desteklenmeyen);
            }
            self.cogaltma = Some(cogaltma);

            if (g, y) != eski_boyut {
                // Doku yeni çözünürlüğe göre ayrılıyor; eskisine kopyalamak
                // `CopyResource` boyut uyuşmazlığı demek olurdu.
                let (doku, gorunum) = hedef_doku(&self.cihaz, g, y)?;
                self.doku = doku;
                self.gorunum = gorunum;
                self.genislik = g;
                self.yukseklik = y;
                return Ok(true);
            }
            Ok(eski_kose != (self.ekran.x, self.ekran.y))
        }

        /// Bir sonraki kareyi bekler ve kendi dokumuza kopyalar.
        pub fn kare(&mut self, zaman_asimi_ms: u32) -> Result<KareDurumu, Engel> {
            unsafe {
                let mut bilgi = DXGI_OUTDUPL_FRAME_INFO::default();
                let mut kaynak: Option<IDXGIResource> = None;
                // Çoğaltma yenileme sırasında bir an boş kalabiliyor;
                // boşken kare istemek hata değil, "erişim kesildi" hâli.
                let Some(cogaltma) = self.cogaltma.as_ref() else {
                    return Err(Engel::ErisimKesildi);
                };
                // Beklemenin süresi ayrı ölçülüyor: bu sürenin çoğu oyunun
                // bir sonraki karesini üretmesini beklemekle geçiyor ve o,
                // ölçeklemenin EKLEDİĞİ bir gecikme değil (karar #36).
                let bekleme_basi = std::time::Instant::now();
                let sonuc = cogaltma.AcquireNextFrame(zaman_asimi_ms, &mut bilgi, &mut kaynak);
                self.bekleme_us = bekleme_basi
                    .elapsed()
                    .as_micros()
                    .min(u32::MAX as u128) as u32;
                if let Err(e) = sonuc {
                    if e.code() == DXGI_ERROR_WAIT_TIMEOUT {
                        return Ok(KareDurumu::Bos);
                    }
                    return Err(engel(&e));
                }
                let Some(kaynak) = kaynak else {
                    // Kare gelmedi ama hata da yok: bırakma çağrısı yine de
                    // yapılmalı, yoksa çoğaltma bir daha kare vermez.
                    let _ = cogaltma.ReleaseFrame();
                    return Ok(KareDurumu::Bos);
                };

                // Yalnızca fare imleci değiştiyse `LastPresentTime` sıfır
                // kalıyor: yeni görüntü yok demek. Kopyalamak boşa iş olurdu.
                let yeni_goruntu = bilgi.LastPresentTime != 0;
                if yeni_goruntu {
                    if let Ok(doku) = kaynak.cast::<ID3D11Texture2D>() {
                        self.baglam.CopyResource(&self.doku, &doku);
                    }
                }
                // Bırakma her yolda: erken dönen bir `?` burada masaüstü
                // bileşicisini kilitli bırakırdı.
                let _ = cogaltma.ReleaseFrame();
                Ok(if yeni_goruntu {
                    KareDurumu::Yeni
                } else {
                    KareDurumu::Bos
                })
            }
        }

        /// Son `kare()` çağrısında yeni kare BEKLENEN süre (mikrosaniye).
        ///
        /// Çağıran bunu yakalama süresinden düşüyor. Ayrı durması şart:
        /// bekleme, kaynağın kare hızının bir sonucu ve ölçekleme kapalıyken
        /// de olurdu; boru hattının bedeliyle toplanırsa ölçüm, ölçmek
        /// istediği şeyden başka bir şeyi ölçer (karar #36).
        pub fn son_bekleme_us(&self) -> u32 {
            self.bekleme_us
        }

        /// Son kareyi CPU'ya indirir.
        ///
        /// Gerçek zamanlı yolda **kullanılmıyor**; yalnızca kullanıcının
        /// "yakalamayı dene" düğmesi için. Ölçekleme açılmadan önce sorunun
        /// yakalamada mı sunumda mı olduğunu ayırt edebilmek, hata mesajını
        /// tahminden çıkarıyor.
        pub fn oku(&self) -> Result<crate::scaling::algoritma::Goruntu, Engel> {
            unsafe {
                let mut tanim = D3D11_TEXTURE2D_DESC::default();
                self.doku.GetDesc(&mut tanim);
                tanim.Usage = D3D11_USAGE_STAGING;
                tanim.BindFlags = 0;
                tanim.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
                tanim.MiscFlags = 0;

                let mut gecici: Option<ID3D11Texture2D> = None;
                self.cihaz
                    .CreateTexture2D(&tanim, None, Some(&mut gecici))
                    .map_err(|e| engel(&e))?;
                let gecici = gecici.ok_or(Engel::Desteklenmeyen)?;
                self.baglam.CopyResource(&gecici, &self.doku);

                let mut esleme = D3D11_MAPPED_SUBRESOURCE::default();
                self.baglam
                    .Map(&gecici, 0, D3D11_MAP_READ, 0, Some(&mut esleme))
                    .map_err(|e| engel(&e))?;

                let mut g = crate::scaling::algoritma::Goruntu::yeni(self.genislik, self.yukseklik);
                let satir = self.genislik as usize * 4;
                for y in 0..self.yukseklik as usize {
                    // Kaynağın satır aralığı (pitch) satır uzunluğundan
                    // büyük olabiliyor; dolgu atlanmazsa görüntü kayar.
                    let kaynak = (esleme.pData as *const u8).add(y * esleme.RowPitch as usize);
                    std::ptr::copy_nonoverlapping(
                        kaynak,
                        g.pikseller.as_mut_ptr().add(y * satir),
                        satir,
                    );
                }
                self.baglam.Unmap(&gecici, 0);
                Ok(g)
            }
        }
    }

    /// Bir ekranın çoğaltmasını **verilen** cihazla açar.
    ///
    /// Ayrı fonksiyon, çünkü `yeniden_ac` bunu cihazı değiştirmeden
    /// çağırıyor. Adaptör ve çıkış her seferinde yeniden sayılıyor: ekran
    /// modu değiştiğinde eski çıkış nesnesi geçerliliğini yitiriyor.
    fn cogalt(cihaz: &ID3D11Device, ekran: &Ekran) -> Result<IDXGIOutputDuplication, Engel> {
        unsafe {
            let fabrika: IDXGIFactory1 =
                CreateDXGIFactory1().map_err(|e| Engel::Sistem(e.code().0))?;
            let adaptor = fabrika
                .EnumAdapters1(ekran.adaptor)
                .map_err(|_| Engel::EkranYok)?;
            let cikis = adaptor
                .EnumOutputs(ekran.cikis)
                .map_err(|_| Engel::EkranYok)?;
            let cikis1: IDXGIOutput1 = cikis.cast().map_err(|e| engel(&e))?;
            cikis1.DuplicateOutput(cihaz).map_err(|e| engel(&e))
        }
    }

    /// Kareyi tutacak doku ve gölgelendirici görünümü.
    fn hedef_doku(
        cihaz: &ID3D11Device,
        genislik: u32,
        yukseklik: u32,
    ) -> Result<(ID3D11Texture2D, ID3D11ShaderResourceView), Engel> {
        unsafe {
            let tanim = D3D11_TEXTURE2D_DESC {
                Width: genislik,
                Height: yukseklik,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as u32,
                CPUAccessFlags: 0,
                MiscFlags: 0,
            };
            let mut doku: Option<ID3D11Texture2D> = None;
            cihaz
                .CreateTexture2D(&tanim, None, Some(&mut doku))
                .map_err(|e| engel(&e))?;
            let doku = doku.ok_or(Engel::Desteklenmeyen)?;

            let mut gorunum: Option<ID3D11ShaderResourceView> = None;
            cihaz
                .CreateShaderResourceView(&doku, None, Some(&mut gorunum))
                .map_err(|e| engel(&e))?;
            let gorunum = gorunum.ok_or(Engel::Desteklenmeyen)?;
            Ok((doku, gorunum))
        }
    }
}

// ---------------------------------------------------------------------------
// Windows dışı: derlensin diye. Ürün Windows'a özel (`decisions.md` #2).
// ---------------------------------------------------------------------------

#[cfg(not(windows))]
pub fn ekranlar() -> Result<Vec<Ekran>, Engel> {
    Err(Engel::Platform)
}

#[cfg(not(windows))]
pub struct Yakalayici;

#[cfg(not(windows))]
impl Yakalayici {
    pub fn ac(_indeks: usize) -> Result<Self, Engel> {
        Err(Engel::Platform)
    }
    pub fn yeniden_ac(&mut self) -> Result<bool, Engel> {
        Err(Engel::Platform)
    }
    pub fn kare(&mut self, _zaman_asimi_ms: u32) -> Result<KareDurumu, Engel> {
        Err(Engel::Platform)
    }
    pub fn son_bekleme_us(&self) -> u32 {
        0
    }
    pub fn oku(&self) -> Result<crate::scaling::algoritma::Goruntu, Engel> {
        Err(Engel::Platform)
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn engel_metinleri_ne_yapilacagini_soyluyor() {
        // Kullanıcıya "başarısız" demek yetmiyor; çözümü olan durumlarda
        // çözüm metnin içinde olmalı (tasarım ilkesi 2).
        assert!(Engel::ErisimKesildi.to_string().contains("kenarlıksız"));
        assert!(Engel::BaskaUygulamaKullaniyor.to_string().contains("kapat"));
        // Sistem kodu ham HRESULT olarak değil, en azından biçimlenmiş
        // gösteriliyor ve tek satır kalıyor.
        let m = Engel::Sistem(-2005270490).to_string();
        assert!(!m.contains('\n'));
        assert!(m.contains("0x"));
    }

    #[test]
    fn engel_metinlerinde_sucluma_yok() {
        // Yakalanamayan ekran çoğu zaman kullanıcının hatası değil.
        for e in [
            Engel::ErisimKesildi,
            Engel::BaskaUygulamaKullaniyor,
            Engel::Desteklenmeyen,
            Engel::EkranYok,
            Engel::Platform,
        ] {
            let m = e.to_string();
            assert!(!m.is_empty());
            assert!(!m.to_lowercase().contains("hata:"), "{m}");
        }
    }

    /// Windows'ta ekran listesi okunabilmeli.
    ///
    /// Ekransız bir CI makinesinde `EkranYok` dönebilir; test o durumu da
    /// kabul ediyor. Kontrol ettiği şey, listenin **tutarlı** olması.
    #[cfg(windows)]
    #[test]
    fn ekran_listesi_tutarli() {
        match ekranlar() {
            Ok(liste) => {
                for (i, e) in liste.iter().enumerate() {
                    assert_eq!(e.indeks, i, "indeksler sırayla olmalı");
                    assert!(e.genislik > 0 && e.yukseklik > 0, "{e:?}");
                    assert!(!e.ad.is_empty());
                }
            }
            Err(Engel::EkranYok) => {}
            Err(e) => panic!("beklenmeyen engel: {e}"),
        }
    }
}
