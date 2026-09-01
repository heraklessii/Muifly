//! ETW ile sunum (present) olaylarını dinleme.
//!
//! Bu, `frames.rs`'in veri kaynağı. Kare sürelerinin **nereden geldiği**
//! burada, **ne anlama geldiği** orada; ayrım bilinçli, çünkü istatistik
//! tarafı yükseltilmiş yetki olmadan test edilebiliyor, burası edilemiyor.
//!
//! # Neden ETW
//!
//! Hook'suz kare ölçümünün tek yolu bu (`decisions.md` #14). Overlay
//! kütüphanelerinin yaptığı gibi oyunun `Present` çağrısına atlamak tasarım
//! ilkesi 3'ü doğrudan ihlal ederdi. ETW ise oyunun kendi yaydığı olayları
//! işletim sisteminin sağladığı kanaldan **dışarıdan** dinliyor: oyunun
//! adres alanına hiçbir şey yazılmıyor, hiçbir çağrı yönlendirilmiyor.
//!
//! # Neden her zaman açık değil
//!
//! Gerçek zamanlı bir ETW oturumu açmak (`StartTrace`) yönetici ya da
//! `Performance Log Users` üyeliği istiyor; yükseltilmemiş bir süreçte
//! `ERROR_ACCESS_DENIED` dönüyor. Bu ölçüldü, varsayılmadı — bkz.
//! `decisions.md` #27. Tasarım ilkesi 5 arka plan izlemesinin yükseltilmiş
//! çalışmasını yasakladığı için kare ölçümü **sürekli değil, kullanıcının
//! başlattığı süreli bir ölçüm** olarak kurgulanıyor.
//!
//! Bu yüzden bu modül kendi başına UAC istemiyor ve kendini otomatik
//! başlatmıyor. Yetkisi olmayan bir süreçte `baslat` `Engel::YetkiYok`
//! dönüyor; ne yapılacağına çağıran karar veriyor.

use crate::monitor::frames::{KareOzeti, KareTamponu, KARE_KAPASITESI};

/// Ölçümün neden başlayamadığı.
///
/// Hata metni değil, tip: arayüzün her durumda ne yazacağına kendisi karar
/// versin. "Erişim reddedildi (5)" gibi bir Win32 cümlesini kullanıcıya
/// göstermek, şeffaflık değil kalabalık olurdu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engel {
    /// Yükseltilmiş yetki yok. Beklenen ve normal durum.
    YetkiYok,
    /// Aynı adlı bir oturum açık kalmış ve durdurulamadı.
    OturumAcik,
    /// Başka bir Win32 hatası.
    Sistem(u32),
    /// Windows dışı derleme.
    Desteklenmiyor,
}

impl std::fmt::Display for Engel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Engel::YetkiYok => write!(f, "kare ölçümü için yükseltilmiş yetki gerekiyor"),
            Engel::OturumAcik => write!(f, "önceki ölçüm oturumu kapanmamış"),
            Engel::Sistem(k) => write!(f, "ölçüm oturumu açılamadı (sistem kodu {k})"),
            Engel::Desteklenmiyor => write!(f, "kare ölçümü bu platformda yok"),
        }
    }
}

/// Ölçüm oturumunun adı.
///
/// Sabit: program çökerse arkada kalan oturumu bir sonraki açılışta bu adla
/// bulup durdurabilmek gerekiyor. Rastgele bir ad, sistemde sessizce çalışan
/// yetim bir ETW oturumu bırakırdı — geri alma defterinin çözdüğü problemin
/// aynısı (`decisions.md` #3).
pub const OTURUM_ADI: &str = "Muifly-Kare-Olcumu";

// ---------------------------------------------------------------------------
// Windows uygulaması
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod win {
    use super::*;
    use std::mem::size_of;
    use std::sync::{Arc, Mutex};
    use windows::core::{GUID, PCWSTR, PWSTR};
    use windows::Win32::Foundation::{
        ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS, ERROR_SUCCESS, ERROR_WMI_INSTANCE_NOT_FOUND,
    };
    use windows::Win32::System::Diagnostics::Etw::*;
    use windows::Win32::System::Performance::QueryPerformanceFrequency;

    /// Microsoft-Windows-DXGI — D3D10/11/12 sunumları.
    const DXGI: GUID = GUID::from_u128(0xCA11C036_0102_4A2D_A6AD_F03CFED5D3C9);
    /// Microsoft-Windows-D3D9 — eski oyunlar.
    const D3D9: GUID = GUID::from_u128(0x783ACA0A_790E_4D7F_8451_AA850511C6B9);

    /// DXGI: `IDXGISwapChain::Present` başlangıcı.
    const DXGI_PRESENT: u16 = 42;
    /// DXGI: `PresentMultiplaneOverlay` başlangıcı — tam ekran kaplama yolu.
    const DXGI_PRESENT_MPO: u16 = 55;
    /// D3D9: `IDirect3DDevice9::Present` başlangıcı.
    const D3D9_PRESENT: u16 = 1;

    /// Yalnızca sunum **başlangıcı** sayılıyor, bitişi değil: iki başlangıç
    /// arasındaki süre bir karenin süresi. Bitişi de saymak her kareyi iki
    /// kez sayardı.
    fn sunum_mu(saglayici: &GUID, id: u16) -> bool {
        if *saglayici == DXGI {
            id == DXGI_PRESENT || id == DXGI_PRESENT_MPO
        } else if *saglayici == D3D9 {
            id == D3D9_PRESENT
        } else {
            false
        }
    }

    /// Geri çağrıya `EVENT_TRACE_LOGFILEW.Context` üzerinden taşınan durum.
    struct Paylasim {
        tampon: Mutex<KareTamponu>,
    }

    fn utf16(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// ETW her olay için bunu çağırıyor. Burada **hiçbir şey ayrılmıyor ve
    /// hiçbir I/O yapılmıyor**: geri çağrı ölçülen sürecin sunum hızında
    /// koşuyor, ağır bir iş buraya konursa ölçüm aracın kendisi yüzünden
    /// bozulur.
    unsafe extern "system" fn olay_geldi(kayit: *mut EVENT_RECORD) {
        if kayit.is_null() {
            return;
        }
        let e = unsafe { &*kayit };
        if !sunum_mu(&e.EventHeader.ProviderId, e.EventHeader.EventDescriptor.Id) {
            return;
        }
        if e.UserContext.is_null() {
            return;
        }
        let p = unsafe { &*(e.UserContext as *const Paylasim) };
        if let Ok(mut t) = p.tampon.lock() {
            t.ekle(e.EventHeader.TimeStamp);
        }
    }

    /// `EVENT_TRACE_PROPERTIES` + isim için ayrılmış tampon.
    ///
    /// Tek parça bir `Vec<u8>` olmak zorunda: `StartTrace` oturum adını
    /// yapının hemen ardına, `LoggerNameOffset` ile gösterilen yere kopyalıyor.
    struct Ozellikler {
        tampon: Vec<u8>,
    }

    impl Ozellikler {
        fn yeni(ad_uzunlugu: usize) -> Self {
            let boy = size_of::<EVENT_TRACE_PROPERTIES>() + ad_uzunlugu * 2 + 64;
            let mut o = Self {
                tampon: vec![0u8; boy],
            };
            o.doldur();
            o
        }

        fn doldur(&mut self) {
            let boy = self.tampon.len();
            self.tampon.iter_mut().for_each(|b| *b = 0);
            let p = self.ptr();
            // SAFETY: `p` yeterince büyük, sıfırlanmış ve hizalı bir tampona
            // bakıyor; yazılan alanların hepsi yapının içinde.
            unsafe {
                (*p).Wnode.BufferSize = boy as u32;
                (*p).Wnode.Flags = WNODE_FLAG_TRACED_GUID;
                // 1 = QPC. Kare süresi hesabı bu sayacın frekansına dayanıyor.
                (*p).Wnode.ClientContext = 1;
                (*p).LogFileMode = EVENT_TRACE_REAL_TIME_MODE;
                (*p).LoggerNameOffset = size_of::<EVENT_TRACE_PROPERTIES>() as u32;
                // Saniyede bir boşalt: gerçek zamanlı gösterim için olayların
                // varsayılan tamponlanma süresi (birkaç saniye) fazla uzun.
                (*p).FlushTimer = 1;
            }
        }

        fn ptr(&mut self) -> *mut EVENT_TRACE_PROPERTIES {
            self.tampon.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES
        }
    }

    /// Arkada kalmış bir oturumu durdurur. Yoksa sessizce geçer.
    fn kalinti_temizle(ad: &[u16], ozellikler: &mut Ozellikler) {
        ozellikler.doldur();
        // SAFETY: isim sonlandırılmış, özellik tamponu geçerli.
        unsafe {
            let _ = ControlTraceW(
                CONTROLTRACE_HANDLE::default(),
                PCWSTR(ad.as_ptr()),
                ozellikler.ptr(),
                EVENT_TRACE_CONTROL_STOP,
            );
        }
    }

    /// Çalışan bir kare ölçümü.
    ///
    /// `Drop` oturumu durduruyor: bu tip düşerse sistemde açık ETW oturumu
    /// kalmaz. Yetim oturum, programın kendi ilkesine (her değişiklik geri
    /// alınabilir) aykırı olurdu.
    pub struct Olcum {
        kolu: CONTROLTRACE_HANDLE,
        ad: Vec<u16>,
        ozellikler: Ozellikler,
        paylasim: Arc<Paylasim>,
        is: Option<std::thread::JoinHandle<()>>,
        frekans: i64,
    }

    impl Olcum {
        pub fn baslat(hedef_pid: u32) -> Result<Self, Engel> {
            let ad = utf16(OTURUM_ADI);
            let mut ozellikler = Ozellikler::yeni(ad.len());

            kalinti_temizle(&ad, &mut ozellikler);
            ozellikler.doldur();

            let mut kolu = CONTROLTRACE_HANDLE::default();
            // SAFETY: üç argüman da geçerli ve yaşam süresi çağrıyı kapsıyor.
            let hata = unsafe { StartTraceW(&mut kolu, PCWSTR(ad.as_ptr()), ozellikler.ptr()) };
            match hata {
                ERROR_SUCCESS => {}
                ERROR_ACCESS_DENIED => return Err(Engel::YetkiYok),
                ERROR_ALREADY_EXISTS => return Err(Engel::OturumAcik),
                k => return Err(Engel::Sistem(k.0)),
            }

            let paylasim = Arc::new(Paylasim {
                tampon: Mutex::new(KareTamponu::yeni(KARE_KAPASITESI)),
            });

            let mut olcum = Olcum {
                kolu,
                ad,
                ozellikler,
                paylasim,
                is: None,
                frekans: frekans(),
            };

            // Oturum açıldı ama sağlayıcı açılamadıysa `Drop` temizliyor.
            olcum.saglayicilari_ac(hedef_pid)?;
            olcum.tuketiciyi_baslat();
            Ok(olcum)
        }

        /// Sağlayıcıları **yalnızca hedef süreç için** açar.
        ///
        /// PID süzgeci çekirdek tarafında uygulanıyor; olayı alıp sonra elemek
        /// yerine hiç almamak, ölçümün kendi maliyetini oyunun üstünden
        /// kaldırıyor. Ölçüm aracının ölçtüğü şeyi bozmaması gerekiyor.
        fn saglayicilari_ac(&mut self, hedef_pid: u32) -> Result<(), Engel> {
            let pidler = [hedef_pid];
            let mut suzgec = EVENT_FILTER_DESCRIPTOR {
                Ptr: pidler.as_ptr() as u64,
                Size: std::mem::size_of_val(&pidler) as u32,
                Type: EVENT_FILTER_TYPE_PID,
            };
            let parametreler = ENABLE_TRACE_PARAMETERS {
                Version: ENABLE_TRACE_PARAMETERS_VERSION_2,
                EnableFilterDesc: &mut suzgec,
                FilterDescCount: 1,
                ..Default::default()
            };

            let mut acilan = 0usize;
            let mut son_hata = ERROR_SUCCESS;
            for g in [DXGI, D3D9] {
                // SAFETY: kol geçerli, GUID ve parametreler bu çağrı boyunca
                // yaşıyor (`pidler` ve `suzgec` yerel, çağrı senkron).
                let e = unsafe {
                    EnableTraceEx2(
                        self.kolu,
                        &g,
                        EVENT_CONTROL_CODE_ENABLE_PROVIDER.0,
                        TRACE_LEVEL_INFORMATION as u8,
                        0,
                        0,
                        0,
                        Some(&parametreler),
                    )
                };
                if e == ERROR_SUCCESS {
                    acilan += 1;
                } else {
                    son_hata = e;
                }
            }

            // Biri açıldıysa yeter: bir oyun ya DXGI ya D3D9 kullanıyor,
            // ikisini birden değil. Hiçbiri açılmadıysa ölçüm yapılamaz.
            if acilan == 0 {
                Err(Engel::Sistem(son_hata.0))
            } else {
                Ok(())
            }
        }

        fn tuketiciyi_baslat(&mut self) {
            let paylasim = self.paylasim.clone();
            let is = std::thread::Builder::new()
                .name("muifly-etw".into())
                .spawn(move || {
                    let mut ad = utf16(OTURUM_ADI);
                    // Arc'ın bir sayımı geri çağrıya işaretçi olarak gidiyor;
                    // `ProcessTrace` dönünce geri alınıp düşürülüyor.
                    let ctx = Arc::into_raw(paylasim);

                    let mut lf = EVENT_TRACE_LOGFILEW {
                        LoggerName: PWSTR(ad.as_mut_ptr()),
                        Context: ctx as *mut std::ffi::c_void,
                        ..Default::default()
                    };
                    lf.Anonymous1.ProcessTraceMode =
                        PROCESS_TRACE_MODE_REAL_TIME | PROCESS_TRACE_MODE_EVENT_RECORD;
                    lf.Anonymous2.EventRecordCallback = Some(olay_geldi);

                    // SAFETY: `lf` bu iş parçacığında yaşıyor; `ProcessTrace`
                    // dönene kadar `ctx` geçerli.
                    let h = unsafe { OpenTraceW(&mut lf) };
                    if h.Value != u64::MAX {
                        unsafe {
                            let _ = ProcessTrace(&[h], None, None);
                            let _ = CloseTrace(h);
                        }
                    }
                    // SAFETY: `ctx` yukarıda `Arc::into_raw` ile üretildi ve
                    // geri çağrı artık koşmuyor.
                    unsafe { drop(Arc::from_raw(ctx)) };
                })
                .ok();
            self.is = is;
        }

        pub fn kare_sayisi(&self) -> usize {
            self.paylasim.tampon.lock().map(|t| t.len()).unwrap_or(0)
        }

        pub fn ozet(&self) -> Option<KareOzeti> {
            self.paylasim
                .tampon
                .lock()
                .ok()
                .and_then(|t| t.ozet(self.frekans))
        }

        pub fn temizle(&self) {
            if let Ok(mut t) = self.paylasim.tampon.lock() {
                t.temizle();
            }
        }

        /// Oturumu durdurur ve tüketici iş parçacığını bekler.
        fn kapat(&mut self) {
            self.ozellikler.doldur();
            // SAFETY: kol ve isim geçerli. Hata yutulmuyor değil, anlamsız:
            // oturum zaten kapalıysa `ERROR_WMI_INSTANCE_NOT_FOUND` dönüyor
            // ve yapılacak bir şey yok.
            unsafe {
                let hata = ControlTraceW(
                    self.kolu,
                    PCWSTR(self.ad.as_ptr()),
                    self.ozellikler.ptr(),
                    EVENT_TRACE_CONTROL_STOP,
                );
                debug_assert!(
                    hata == ERROR_SUCCESS || hata == ERROR_WMI_INSTANCE_NOT_FOUND,
                    "beklenmeyen ControlTrace hatası: {}",
                    hata.0
                );
            }
            // Oturum durunca `ProcessTrace` dönüyor, iş parçacığı bitiyor.
            if let Some(is) = self.is.take() {
                let _ = is.join();
            }
        }
    }

    impl Drop for Olcum {
        fn drop(&mut self) {
            self.kapat();
        }
    }

    /// QPC frekansı. Kare süresi hesabı bunu bölen olarak kullanıyor.
    pub fn frekans() -> i64 {
        let mut f = 0i64;
        // SAFETY: çıktı işaretçisi yerel ve geçerli.
        unsafe {
            let _ = QueryPerformanceFrequency(&mut f);
        }
        f
    }

    /// Açılışta çağrılıyor: önceki çalışmadan kalmış oturumu durdurur.
    ///
    /// Yetkisi yoksa hiçbir şey yapmıyor — ki bu normal durum, çünkü
    /// yetkisiz bir süreç zaten oturum açamamıştı.
    pub fn kalintiyi_durdur() {
        let ad = utf16(OTURUM_ADI);
        let mut o = Ozellikler::yeni(ad.len());
        kalinti_temizle(&ad, &mut o);
    }
}

#[cfg(windows)]
pub use win::{frekans, kalintiyi_durdur, Olcum};

// ---------------------------------------------------------------------------
// Windows dışı: derlensin diye. Ürün Windows'a özel (`decisions.md` #2).
// ---------------------------------------------------------------------------

#[cfg(not(windows))]
pub struct Olcum;

#[cfg(not(windows))]
impl Olcum {
    pub fn baslat(_hedef_pid: u32) -> Result<Self, Engel> {
        Err(Engel::Desteklenmiyor)
    }
    pub fn kare_sayisi(&self) -> usize {
        0
    }
    pub fn ozet(&self) -> Option<KareOzeti> {
        None
    }
    pub fn temizle(&self) {}
}

#[cfg(not(windows))]
pub fn frekans() -> i64 {
    0
}

#[cfg(not(windows))]
pub fn kalintiyi_durdur() {}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn engel_metinleri_sayisal_vaat_icermiyor() {
        // Tasarım ilkesi 4: kullanıcıya giden hiçbir cümle sayı vaat etmez.
        for e in [
            Engel::YetkiYok,
            Engel::OturumAcik,
            Engel::Sistem(5),
            Engel::Desteklenmiyor,
        ] {
            let m = e.to_string();
            for yasak in ["ms", "fps", "%", "daha hızlı", "kazan"] {
                assert!(
                    !m.to_lowercase().contains(yasak),
                    "engel metninde yasak ifade '{yasak}': {m}"
                );
            }
        }
    }

    #[test]
    fn yetkisiz_surecte_olcum_baslamiyor() {
        // Bu test yükseltilmemiş bir CI koşucusunda `YetkiYok` bekliyor.
        // Yönetici olarak koşulursa ölçüm gerçekten başlar; o durumda da
        // düşerken oturumu kapatması gerekiyor — `Drop` sınanmış olur.
        match Olcum::baslat(std::process::id()) {
            Err(Engel::YetkiYok) => {}
            Err(baska) => panic!("beklenmeyen engel: {baska:?}"),
            Ok(olcum) => {
                // Yükseltilmiş ortam: kendi sürecimiz sunum yapmıyor, kare
                // gelmemeli ama oturum açılıp temiz kapanmalı.
                assert_eq!(olcum.kare_sayisi(), 0);
                drop(olcum);
            }
        }
    }

    #[test]
    fn oturum_adi_sabit() {
        // Çökme sonrası kalıntı temizliği bu ada dayanıyor; değişirse
        // yetim bir ETW oturumu kalır.
        assert_eq!(OTURUM_ADI, "Muifly-Kare-Olcumu");
    }
}
