//! Süreç algılama: öndeki pencere, süreç listesi, oyun sezgisi.
//!
//! Bu dosya hiçbir şeyi değiştirmez, yalnızca okur. Kullandığı API'lerin
//! tamamı resmi ve dokümante: `GetForegroundWindow`, `GetWindowThreadProcessId`,
//! `QueryFullProcessImageNameW`, `CreateToolhelp32Snapshot`. Oyun sürecine
//! hiçbir şey enjekte edilmiyor (tasarım ilkesi 3).

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Sistemde çalışan bir süreç.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Surec {
    pub pid: u32,
    /// Yalnızca dosya adı, küçük harfe indirgenmiş: `discord.exe`.
    /// Profil eşleştirmesi bu alan üzerinden yapılıyor ve Windows dosya
    /// adlarında büyük/küçük harf ayrımı olmadığı için normalleştiriliyor.
    pub ad: String,
}

/// Öndeki pencerenin sahibi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OndekiPencere {
    pub pid: u32,
    pub ad: String,
    /// Pencere, bağlı olduğu monitörü tam kaplıyor mu?
    ///
    /// Kenarlıksız tam ekran (borderless) oyunlar da bu testi geçiyor;
    /// gerçek amaç bu zaten — profil listesinde olmayan bir oyunu "oyun
    /// olabilir" diye işaretleyen sezgi bu alana dayanıyor.
    pub tam_ekran: bool,
}

/// Süreç adını normalleştirir: yalnızca dosya adı, küçük harf.
pub fn ad_normalize(yol: &str) -> String {
    yol.rsplit(['\\', '/']).next().unwrap_or(yol).to_lowercase()
}

/// Windows'un kendi süreçleri ve kabuk bileşenleri.
///
/// Bu liste **dondurma korumasıdır**: bir profil yanlışlıkla `explorer.exe`
/// yazsa bile donduramaz. Sabit kodlanmış olması bilinçli — kullanıcı
/// tarafından düzenlenebilir bir "güvenli liste" olsaydı, paylaşılan bir
/// profil onu boşaltarak sistemi kilitleyebilirdi.
pub const DOKUNULMAZ: &[&str] = &[
    "system",
    "system idle process",
    "registry",
    "smss.exe",
    "csrss.exe",
    "wininit.exe",
    "winlogon.exe",
    "services.exe",
    "lsass.exe",
    "svchost.exe",
    "dwm.exe",
    "explorer.exe",
    "fontdrvhost.exe",
    "audiodg.exe",
    "sihost.exe",
    "ctfmon.exe",
    "taskhostw.exe",
    "runtimebroker.exe",
    "shellexperiencehost.exe",
    "searchhost.exe",
    "startmenuexperiencehost.exe",
    "textinputhost.exe",
    "muifly.exe",
];

/// Süreç dondurulabilir mi?
///
/// Ad zaten normalleştirilmiş kabul ediliyor.
pub fn dondurulabilir(ad: &str) -> bool {
    !DOKUNULMAZ.contains(&ad)
}

// ---------------------------------------------------------------------------
// Windows tarafı
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod win {
    use super::*;

    use crate::winutil::Tanitici;
    use windows::Win32::Foundation::{HWND, MAX_PATH, RECT};
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId,
    };

    /// Yalnızca sorgulama yetkisiyle süreç açar.
    ///
    /// `PROCESS_QUERY_LIMITED_INFORMATION` bilinçli: adı okumak için daha
    /// geniş bir yetkiye gerek yok ve dar yetki, yükseltilmiş süreçlerde de
    /// çoğu zaman başarılı oluyor.
    pub fn sorgu_icin_ac(pid: u32) -> Result<Tanitici> {
        // SAFETY: sabit bayraklar, sahiplik `Tanitici`ye geçiyor.
        let h = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }
            .map_err(|_| Error::ProcessNotFound(pid))?;
        Ok(Tanitici(h))
    }

    pub fn surec_adi(pid: u32) -> Result<String> {
        let t = sorgu_icin_ac(pid)?;
        let mut tampon = [0u16; MAX_PATH as usize];
        let mut boy = tampon.len() as u32;

        // SAFETY: tampon yerel ve `boy` gerçek uzunluğu; API yazdığı kadarını
        // `boy` üzerinden geri bildiriyor.
        unsafe {
            QueryFullProcessImageNameW(
                t.0,
                PROCESS_NAME_WIN32,
                windows::core::PWSTR(tampon.as_mut_ptr()),
                &mut boy,
            )
        }
        .map_err(|e| crate::error::win("süreç adı okuma", e))?;

        let yol = String::from_utf16_lossy(&tampon[..boy as usize]);
        Ok(ad_normalize(&yol))
    }

    pub fn ondeki_pencere() -> Result<OndekiPencere> {
        // SAFETY: parametresiz okuma; boş masaüstünde geçersiz HWND dönebilir,
        // hemen altında kontrol ediliyor.
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.is_invalid() {
            return Err(Error::Network("öndeki pencere yok".into()));
        }

        let mut pid = 0u32;
        // SAFETY: `pid` yerel ve geçerli.
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        if pid == 0 {
            return Err(Error::ProcessNotFound(0));
        }

        Ok(OndekiPencere {
            pid,
            ad: surec_adi(pid)?,
            tam_ekran: tam_ekran_mi(hwnd),
        })
    }

    /// Pencere, bağlı olduğu monitörün tamamını kaplıyor mu?
    ///
    /// Monitör dikdörtgeni ile pencere dikdörtgeni karşılaştırılıyor; DPI
    /// ölçekleme yüzünden bir-iki piksellik sapma olabildiği için tam eşitlik
    /// aranmıyor.
    fn tam_ekran_mi(hwnd: HWND) -> bool {
        let mut pencere = RECT::default();
        // SAFETY: `pencere` yerel; hata durumunda değer okunmuyor.
        if unsafe { GetWindowRect(hwnd, &mut pencere) }.is_err() {
            return false;
        }

        // SAFETY: geçerli HWND; `MONITOR_DEFAULTTONEAREST` her zaman bir
        // monitör döndürüyor.
        let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
        let mut bilgi = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        // SAFETY: `cbSize` doldurulmuş.
        if !unsafe { GetMonitorInfoW(monitor, &mut bilgi) }.as_bool() {
            return false;
        }

        let m = bilgi.rcMonitor;
        const TOLERANS: i32 = 2;
        (pencere.left - m.left).abs() <= TOLERANS
            && (pencere.top - m.top).abs() <= TOLERANS
            && (pencere.right - m.right).abs() <= TOLERANS
            && (pencere.bottom - m.bottom).abs() <= TOLERANS
    }

    pub fn surec_listesi() -> Result<Vec<Surec>> {
        // SAFETY: sabit bayrak, ikinci parametre 0 = tüm sistem.
        let anlik = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }
            .map_err(|e| crate::error::win("süreç listesi", e))?;
        let anlik = Tanitici(anlik);

        let mut girdi = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        let mut sonuc = Vec::new();
        // SAFETY: `dwSize` doldurulmuş, tanıtıcı geçerli.
        if unsafe { Process32FirstW(anlik.0, &mut girdi) }.is_err() {
            return Ok(sonuc);
        }

        loop {
            let ham = String::from_utf16_lossy(
                &girdi.szExeFile[..girdi
                    .szExeFile
                    .iter()
                    .position(|c| *c == 0)
                    .unwrap_or(girdi.szExeFile.len())],
            );
            sonuc.push(Surec {
                pid: girdi.th32ProcessID,
                ad: ad_normalize(&ham),
            });

            // SAFETY: aynı tanıtıcı ve yapı.
            if unsafe { Process32NextW(anlik.0, &mut girdi) }.is_err() {
                break;
            }
        }
        Ok(sonuc)
    }
}

#[cfg(windows)]
pub use win::{ondeki_pencere, surec_listesi};

#[cfg(windows)]
pub(crate) use win::surec_adi;

#[cfg(not(windows))]
pub fn ondeki_pencere() -> Result<OndekiPencere> {
    Err(Error::Unsupported("öndeki pencere algılama"))
}

#[cfg(not(windows))]
pub fn surec_listesi() -> Result<Vec<Surec>> {
    Err(Error::Unsupported("süreç listesi"))
}

#[cfg(not(windows))]
pub(crate) fn surec_adi(pid: u32) -> Result<String> {
    let _ = pid;
    Err(Error::Unsupported("süreç adı"))
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn ad_yoldan_ayikliyor() {
        assert_eq!(ad_normalize("C:\\Games\\Steam\\Game.exe"), "game.exe");
        assert_eq!(ad_normalize("/usr/bin/Foo"), "foo");
        assert_eq!(ad_normalize("Discord.EXE"), "discord.exe");
    }

    #[test]
    fn sistem_surecleri_dondurulamiyor() {
        // Bu testin görevi, listenin yanlışlıkla boşaltılmasını yakalamak.
        assert!(!dondurulabilir("explorer.exe"));
        assert!(!dondurulabilir("csrss.exe"));
        assert!(!dondurulabilir("dwm.exe"));
        assert!(!dondurulabilir("lsass.exe"));
    }

    #[test]
    fn program_kendini_donduramiyor() {
        assert!(!dondurulabilir("muifly.exe"));
    }

    #[test]
    fn siradan_uygulama_dondurulabilir() {
        assert!(dondurulabilir("discord.exe"));
        assert!(dondurulabilir("spotify.exe"));
        assert!(dondurulabilir("chrome.exe"));
    }

    #[test]
    fn dokunulmaz_liste_normalize_edilmis() {
        // Karşılaştırma küçük harf üzerinden yapılıyor; listede büyük harf
        // olsaydı kontrol sessizce delinirdi.
        for ad in DOKUNULMAZ {
            assert_eq!(*ad, ad.to_lowercase(), "'{ad}' küçük harf olmalı");
        }
    }
}
