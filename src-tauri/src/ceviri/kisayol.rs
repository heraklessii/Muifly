//! Çeviri kısayolu — `RegisterHotKey`.
//!
//! ## Neden kısayol
//!
//! Karar #22: sürekli çeviri yok, tuşa basınca var. Kullanıcı oyunun
//! içindeyken Muifly penceresine geçemez; çeviriyi başlatmanın tek makul
//! yolu sistem geneli bir kısayol.
//!
//! ## Kanca DEĞİL
//!
//! Tasarım ilkesi 3'ün burada uygulanışı, karar #22'de baştan yazılmıştı:
//! `RegisterHotKey` resmî bir Windows API'si, `WH_KEYBOARD_LL` bir kanca.
//! İkisi dışarıdan aynı görünür — biri tuşa basınca çalışır, diğeri de.
//! Fark şu: kanca **bütün** tuş basışlarını görür, bu API yalnızca
//! kaydedilen kombinasyonu. Anti-cheat yazılımlarının baktığı şey de tam
//! olarak bu ayrım.
//!
//! ## İş parçacığı neden ayrı
//!
//! `RegisterHotKey` pencere sahibi olmadan çağrıldığında mesaj **iş
//! parçacığı kuyruğuna** düşüyor ve kaydı yapan iş parçacığı ile kaldıran
//! aynı olmak zorunda. Ölçeklemede bu iş döngü iş parçacığının içinde
//! yapılıyordu (karar #34); burada öyle bir döngü yok, o yüzden kısayolun
//! kendi iş parçacığı var. İş parçacığı yalnızca mesaj bekliyor: uyurken
//! CPU harcamıyor.
//!
//! ## Tetikleyici uzun sürmemeli
//!
//! Geri çağrı mesaj kuyruğunun iş parçacığında koşuyor. Orada çeviri
//! yapılsaydı, çeviri sürerken ikinci bir tuşa basış kuyrukta bekler ve
//! kullanıcı "kısayol çalışmıyor" sanırdı. Bu yüzden geri çağrının işi
//! bir bayrak dikmek; asıl iş `super::Ceviri`nin iş parçacığında.

/// Kısayol adayları: `(değiştirici, tuş, etiket)`.
///
/// Sıralı deneniyor — ilki başka bir uygulamada kayıtlıysa sonraki.
/// Hiçbiri kaydedilemezse özellik **açılmıyor**: çalışmayan bir kısayol,
/// kullanıcının tuşa basıp hiçbir şey olmamasını izlemesi demek.
#[cfg(windows)]
const ADAYLAR: [(u32, u32, &str); 3] = [
    (
        windows::Win32::UI::Input::KeyboardAndMouse::MOD_CONTROL.0
            | windows::Win32::UI::Input::KeyboardAndMouse::MOD_ALT.0,
        windows::Win32::UI::Input::KeyboardAndMouse::VK_T.0 as u32,
        "Ctrl+Alt+T",
    ),
    (
        windows::Win32::UI::Input::KeyboardAndMouse::MOD_CONTROL.0
            | windows::Win32::UI::Input::KeyboardAndMouse::MOD_ALT.0
            | windows::Win32::UI::Input::KeyboardAndMouse::MOD_SHIFT.0,
        windows::Win32::UI::Input::KeyboardAndMouse::VK_T.0 as u32,
        "Ctrl+Alt+Shift+T",
    ),
    (
        windows::Win32::UI::Input::KeyboardAndMouse::MOD_CONTROL.0
            | windows::Win32::UI::Input::KeyboardAndMouse::MOD_ALT.0,
        windows::Win32::UI::Input::KeyboardAndMouse::VK_F10.0 as u32,
        "Ctrl+Alt+F10",
    ),
];

/// Kısayol kimliği. Ölçeklemenin kaçış kısayolundan (0x4D55) farklı: ikisi
/// ayrı iş parçacıklarında ama aynı sayıyı kullanmak, birini taşıyan
/// gelecekteki bir değişiklikte sessiz bir çakışma olurdu.
#[cfg(windows)]
const KIMLIK: i32 = 0x4D56;

#[cfg(windows)]
pub use win::Kisayol;

#[cfg(windows)]
mod win {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    use windows::Win32::Foundation::{LPARAM, WPARAM};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_NOREPEAT,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetMessageW, PostThreadMessageW, MSG, WM_HOTKEY, WM_QUIT,
    };

    /// Kayıtlı bir kısayol ve onu dinleyen iş parçacığı.
    pub struct Kisayol {
        /// Kullanıcıya gösterilen kombinasyon, örn. `Ctrl+Alt+T`.
        pub etiket: &'static str,
        is_parcasi_kimlik: u32,
        is_parcasi: Option<std::thread::JoinHandle<()>>,
        dur: Arc<AtomicBool>,
    }

    impl Kisayol {
        /// Adayları sırayla dener; hiçbiri kaydedilemezse `None`.
        ///
        /// `tetik` kısayola basıldığında çağrılıyor ve **hızlı dönmek
        /// zorunda** (bkz. modül belgesi).
        pub fn kaydet(tetik: impl Fn() + Send + 'static) -> Option<Self> {
            let (gonderici, alici) = std::sync::mpsc::channel::<Option<(&'static str, u32)>>();
            let dur = Arc::new(AtomicBool::new(false));
            let dur_is = Arc::clone(&dur);

            let is = std::thread::Builder::new()
                .name("muifly-ceviri-kisayol".into())
                .spawn(move || dongu(gonderici, tetik, dur_is))
                .ok()?;

            // Kayıt cevabı bekleniyor: başarısızsa özellik hiç açılmamalı.
            match alici.recv_timeout(std::time::Duration::from_secs(3)) {
                Ok(Some((etiket, kimlik))) => Some(Self {
                    etiket,
                    is_parcasi_kimlik: kimlik,
                    is_parcasi: Some(is),
                    dur,
                }),
                _ => {
                    dur.store(true, Ordering::Relaxed);
                    let _ = is.join();
                    None
                }
            }
        }
    }

    impl Drop for Kisayol {
        fn drop(&mut self) {
            self.dur.store(true, Ordering::Relaxed);
            // Mesaj döngüsü `GetMessageW`de uyuyor; uyandırmadan bitmez.
            unsafe {
                let _ = PostThreadMessageW(
                    self.is_parcasi_kimlik,
                    WM_QUIT,
                    WPARAM(0),
                    LPARAM(0),
                );
            }
            if let Some(is) = self.is_parcasi.take() {
                let _ = is.join();
            }
        }
    }

    /// Kısayolu kaydeder ve mesaj bekler.
    ///
    /// Kaydı **bu** iş parçacığı yapıyor ve kaldırıyor: `RegisterHotKey`
    /// pencere sahibi olmadan çağrıldığında mesaj iş parçacığı kuyruğuna
    /// düşüyor ve kayıt iş parçacığa bağlı.
    fn dongu(
        gonderici: std::sync::mpsc::Sender<Option<(&'static str, u32)>>,
        tetik: impl Fn(),
        dur: Arc<AtomicBool>,
    ) {
        let kimlik_is = unsafe { windows::Win32::System::Threading::GetCurrentThreadId() };

        let mut kayitli: Option<(i32, &'static str)> = None;
        for (sira, (modlar, tus, etiket)) in ADAYLAR.iter().enumerate() {
            let kimlik = KIMLIK + sira as i32;
            let sonuc = unsafe {
                RegisterHotKey(
                    None,
                    kimlik,
                    // `MOD_NOREPEAT`: tuş basılı tutulunca tek mesaj gelsin.
                    // Olmasaydı bir saniyelik basış onlarca çeviri isteği
                    // üretirdi.
                    HOT_KEY_MODIFIERS(modlar | MOD_NOREPEAT.0),
                    *tus,
                )
            };
            if sonuc.is_ok() {
                kayitli = Some((kimlik, etiket));
                break;
            }
        }

        let Some((kimlik, etiket)) = kayitli else {
            let _ = gonderici.send(None);
            return;
        };
        if gonderici.send(Some((etiket, kimlik_is))).is_err() {
            unsafe {
                let _ = UnregisterHotKey(None, kimlik);
            }
            return;
        }

        unsafe {
            let mut mesaj = MSG::default();
            // `GetMessageW` 0 döndüğünde `WM_QUIT` gelmiş demektir.
            while GetMessageW(&mut mesaj, None, 0, 0).as_bool() {
                if dur.load(Ordering::Relaxed) {
                    break;
                }
                if mesaj.message == WM_HOTKEY {
                    tetik();
                }
            }
            // Kaldırılmazsa kombinasyon süreç ömrü boyunca bizde kalır ve
            // çeviri kapalıyken de başka uygulamalardan çalınmış olur.
            let _ = UnregisterHotKey(None, kimlik);
        }
    }
}

// ---------------------------------------------------------------------------
// Windows dışı: derlensin diye. Ürün Windows'a özel (`decisions.md` #2).
// ---------------------------------------------------------------------------

#[cfg(not(windows))]
pub struct Kisayol {
    pub etiket: &'static str,
}

#[cfg(not(windows))]
impl Kisayol {
    pub fn kaydet(_tetik: impl Fn() + Send + 'static) -> Option<Self> {
        None
    }
}
