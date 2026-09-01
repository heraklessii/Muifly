//! ATILACAK SONDA - Muifly ETW / FPS fizibilitesi (docs/decisions.md #14).
//!
//! Uc soruyu cevapliyor:
//!   1. Yukseltilmemis surec gercek zamanli ETW oturumu acabiliyor mu?
//!   2. Yukseltilmis surec, BASKA bir surecin DXGI Present olaylarini goruyor mu?
//!   3. Present araliklarindan anlamli bir kare suresi cikiyor mu?
//!
//! Kullanim:  etw-sonda.exe [saniye]     (varsayilan 10)

use std::collections::HashMap;
use std::mem::size_of;
use std::sync::{Mutex, OnceLock};
use windows::core::{GUID, PCWSTR, PWSTR};
use windows::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS, ERROR_SUCCESS};
use windows::Win32::System::Diagnostics::Etw::*;
use windows::Win32::System::Performance::QueryPerformanceFrequency;

const OTURUM: &str = "Muifly-ETW-Sonda";

// Microsoft-Windows-DXGI  (D3D10/11/12 yolu)
const DXGI: GUID = GUID::from_u128(0xCA11C036_0102_4A2D_A6AD_F03CFED5D3C9);
// Microsoft-Windows-D3D9  (eski oyunlar)
const D3D9: GUID = GUID::from_u128(0x783ACA0A_790E_4D7F_8451_AA850511C6B9);

// DXGI: IDXGISwapChain::Present baslangici / PresentMultiplaneOverlay baslangici
const OLAY_PRESENT_START: u16 = 42;
const OLAY_PRESENT_MPO_START: u16 = 55;
// D3D9: IDirect3DDevice9::Present baslangici
const OLAY_D3D9_PRESENT_START: u16 = 1;

/// PID -> Present olaylarinin QPC damgalari.
static TOPLAM: OnceLock<Mutex<HashMap<u32, Vec<i64>>>> = OnceLock::new();

fn kayit() -> &'static Mutex<HashMap<u32, Vec<i64>>> {
    TOPLAM.get_or_init(|| Mutex::new(HashMap::new()))
}

fn utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

unsafe extern "system" fn olay_geldi(kayit_ptr: *mut EVENT_RECORD) {
    if kayit_ptr.is_null() {
        return;
    }
    let e = unsafe { &*kayit_ptr };
    let saglayici = e.EventHeader.ProviderId;
    let id = e.EventHeader.EventDescriptor.Id;

    let ilgili = if saglayici == DXGI {
        id == OLAY_PRESENT_START || id == OLAY_PRESENT_MPO_START
    } else if saglayici == D3D9 {
        id == OLAY_D3D9_PRESENT_START
    } else {
        false
    };
    if !ilgili {
        return;
    }

    let pid = e.EventHeader.ProcessId;
    let t = e.EventHeader.TimeStamp;
    if let Ok(mut m) = kayit().lock() {
        m.entry(pid).or_default().push(t);
    }
}

fn surec_adi(pid: u32) -> String {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    unsafe {
        let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return format!("<pid {pid}>");
        };
        let mut tampon = [0u16; 512];
        let mut boy = tampon.len() as u32;
        let ad = if QueryFullProcessImageNameW(
            h,
            PROCESS_NAME_WIN32,
            PWSTR(tampon.as_mut_ptr()),
            &mut boy,
        )
        .is_ok()
        {
            let tam = String::from_utf16_lossy(&tampon[..boy as usize]);
            tam.rsplit(['\\', '/']).next().unwrap_or(&tam).to_string()
        } else {
            format!("<pid {pid}>")
        };
        let _ = CloseHandle(h);
        ad
    }
}

/// Sirali kare suresi dizisinde yuzdelik. "%1 kotu" icin p=99.
fn yuzdelik(sirali: &[f64], p: f64) -> f64 {
    if sirali.is_empty() {
        return 0.0;
    }
    let k = ((p / 100.0) * (sirali.len() - 1) as f64).round() as usize;
    sirali[k.min(sirali.len() - 1)]
}

fn props_doldur(props: *mut EVENT_TRACE_PROPERTIES, toplam_boyut: usize, props_boyut: usize) {
    unsafe {
        (*props).Wnode.BufferSize = toplam_boyut as u32;
        (*props).Wnode.Flags = WNODE_FLAG_TRACED_GUID;
        (*props).Wnode.ClientContext = 1; // QPC
        (*props).LogFileMode = EVENT_TRACE_REAL_TIME_MODE;
        (*props).LoggerNameOffset = props_boyut as u32;
    }
}

fn main() {
    let saniye: u64 = std::env::args()
        .nth(1)
        .and_then(|a| a.parse().ok())
        .unwrap_or(10);

    let ad = utf16(OTURUM);
    let props_boyut = size_of::<EVENT_TRACE_PROPERTIES>();
    let toplam_boyut = props_boyut + ad.len() * 2 + 64;
    let mut tampon = vec![0u8; toplam_boyut];
    let props = tampon.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES;

    // Onceki calismadan kalmis bir oturum olabilir.
    props_doldur(props, toplam_boyut, props_boyut);
    unsafe {
        let _ = ControlTraceW(
            CONTROLTRACE_HANDLE::default(),
            PCWSTR(ad.as_ptr()),
            props,
            EVENT_TRACE_CONTROL_STOP,
        );
    }

    tampon.iter_mut().for_each(|b| *b = 0);
    props_doldur(props, toplam_boyut, props_boyut);

    let mut kolu = CONTROLTRACE_HANDLE::default();
    let hata = unsafe { StartTraceW(&mut kolu, PCWSTR(ad.as_ptr()), props) };
    println!("[1] StartTraceW -> {}", hata.0);
    match hata {
        ERROR_SUCCESS | ERROR_ALREADY_EXISTS => {}
        ERROR_ACCESS_DENIED => {
            println!("    ERISIM REDDEDILDI. Gercek zamanli ETW oturumu yonetici ya da");
            println!("    'Performance Log Users' uyeligi istiyor. 2. ve 3. sorular");
            println!("    bu sonda yonetici olarak calistirilmadan cevaplanamaz.");
            return;
        }
        _ => {
            println!("    beklenmeyen hata, cikiliyor");
            return;
        }
    }

    for (isim, g) in [("DXGI", DXGI), ("D3D9", D3D9)] {
        let e = unsafe {
            EnableTraceEx2(
                kolu,
                &g,
                EVENT_CONTROL_CODE_ENABLE_PROVIDER.0,
                TRACE_LEVEL_VERBOSE as u8,
                0,
                0,
                0,
                None,
            )
        };
        println!("[2] EnableTraceEx2({isim}) -> {}", e.0);
    }

    // Tuketici ayri is parcaciginda: ProcessTrace bloke ediyor.
    let is = std::thread::spawn(move || {
        let mut ad_t = utf16(OTURUM);
        let mut lf = EVENT_TRACE_LOGFILEW {
            LoggerName: PWSTR(ad_t.as_mut_ptr()),
            ..Default::default()
        };
        lf.Anonymous1.ProcessTraceMode =
            PROCESS_TRACE_MODE_REAL_TIME | PROCESS_TRACE_MODE_EVENT_RECORD;
        lf.Anonymous2.EventRecordCallback = Some(olay_geldi);

        let h = unsafe { OpenTraceW(&mut lf) };
        if h.Value == u64::MAX {
            println!("[3] OpenTraceW BASARISIZ");
            return;
        }
        let r = unsafe { ProcessTrace(&[h], None, None) };
        println!("[3] ProcessTrace bitti -> {}", r.0);
        unsafe {
            let _ = CloseTrace(h);
        }
    });

    println!("[4] {saniye} saniye dinleniyor... (3B sunum yapan bir uygulama acik olsun)");
    std::thread::sleep(std::time::Duration::from_secs(saniye));

    unsafe {
        let _ = ControlTraceW(kolu, PCWSTR(ad.as_ptr()), props, EVENT_TRACE_CONTROL_STOP);
    }
    let _ = is.join();

    let mut frekans: i64 = 0;
    unsafe {
        let _ = QueryPerformanceFrequency(&mut frekans);
    }
    println!("[5] QPC frekansi: {frekans}");

    let m = kayit().lock().unwrap();
    let mut satirlar: Vec<(u32, Vec<i64>)> = m.iter().map(|(k, v)| (*k, v.clone())).collect();
    satirlar.sort_by_key(|(_, v)| std::cmp::Reverse(v.len()));

    if satirlar.is_empty() {
        println!();
        println!("HIC PRESENT OLAYI GELMEDI.");
        println!("Oturum acildi ama olay yok: ya ekranda 3B sunum yapan uygulama yoktu,");
        println!("ya da saglayici/olay secimi yanlis.");
        return;
    }

    println!();
    println!(
        "{:<28} {:>7} {:>9} {:>9} {:>11}",
        "surec", "present", "ort FPS", "ort ms", "%1 kotu ms"
    );
    println!("{}", "-".repeat(68));
    for (pid, mut damgalar) in satirlar.into_iter().take(8) {
        damgalar.sort_unstable();
        if damgalar.len() < 2 || frekans == 0 {
            continue;
        }
        let mut kare_ms: Vec<f64> = damgalar
            .windows(2)
            .map(|p| (p[1] - p[0]) as f64 * 1000.0 / frekans as f64)
            .collect();
        let sure_s = (damgalar[damgalar.len() - 1] - damgalar[0]) as f64 / frekans as f64;
        let fps = if sure_s > 0.0 {
            (damgalar.len() - 1) as f64 / sure_s
        } else {
            0.0
        };
        let ort_ms = kare_ms.iter().sum::<f64>() / kare_ms.len() as f64;
        kare_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p99 = yuzdelik(&kare_ms, 99.0);
        println!(
            "{:<28} {:>7} {:>9.1} {:>9.2} {:>11.2}",
            surec_adi(pid),
            damgalar.len(),
            fps,
            ort_ms,
            p99
        );
    }
}
