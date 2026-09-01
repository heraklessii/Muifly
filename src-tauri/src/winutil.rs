//! Windows tanıtıcıları için ince RAII sarmalayıcı.
//!
//! Ayrı bir dosyada olmasının sebebi: `CloseHandle` unutulan tek bir kod yolu,
//! saatlerce açık duran bir arka plan aracında sessizce tanıtıcı sızdırır.
//! Tanıtıcıyı elle kapatan hiçbir yer olmasın diye tip tek noktada tanımlı.

#![cfg(windows)]

use windows::Win32::Foundation::{CloseHandle, HANDLE};

/// Sahipliği taşıyan tanıtıcı. Kapsam bitince `CloseHandle` çağrılıyor.
pub struct Tanitici(pub HANDLE);

impl Drop for Tanitici {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            // SAFETY: tanıtıcı bu tipin sahipliğinde; başka bir yer kapatmıyor.
            let _ = unsafe { CloseHandle(self.0) };
        }
    }
}

impl Tanitici {
    pub fn ham(&self) -> HANDLE {
        self.0
    }
}
