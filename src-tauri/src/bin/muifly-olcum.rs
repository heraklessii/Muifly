//! `muifly-olcum` — yükseltilmiş kare ölçümü yardımcısı.
//!
//! Tek işi var: verilen PID için verilen süre boyunca ETW ile sunum
//! olaylarını dinlemek, özeti bir JSON dosyasına yazmak ve kapanmak.
//! Gerekçesi `docs/decisions.md` #27 ve `monitor::olcum`.
//!
//! **Bu ikili neden ayrı**: gerçek zamanlı ETW oturumu yükseltilmiş yetki
//! istiyor, tasarım ilkesi 5 ise ana uygulamanın yükseltilmiş çalışmasını
//! yasaklıyor. Ayrı ve kısa ömürlü bir süreç ikisini uzlaştırıyor.
//!
//! Burada **hiçbir sistem ayarı değiştirilmiyor**: yardımcı yalnızca okuyor.
//! Yükseltilmiş yetki ETW oturumu açabilmek için, sistemi değiştirmek için
//! değil. Bu yüzden bu ikilinin geri alma defterine yazacak bir şeyi yok.

// Konsol penceresi parlamasın: ana uygulama bunu gizli başlatıyor ve
// kullanıcının göreceği tek şey UAC istemi olmalı.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

use muifly_lib::monitor::olcum::{self, Istek};

/// Çıkış kodları. Ana uygulama sonucu JSON dosyasından okuyor; bunlar
/// yalnızca dosya hiç yazılamadığında bir şey söyleyebilmek için.
const TAMAM: u8 = 0;
const ARGUMAN_HATASI: u8 = 2;
const YETKI_YOK: u8 = 3;
const OLCUM_HATASI: u8 = 4;
const YAZILAMADI: u8 = 5;

fn main() -> ExitCode {
    let istek = match Istek::ayristir(std::env::args().skip(1)) {
        Ok(i) => i,
        Err(_) => return ExitCode::from(ARGUMAN_HATASI),
    };

    match olcum::yardimci_calistir(&istek) {
        Ok(sonuc) => match olcum::sonucu_yaz(&istek.cikti, &sonuc) {
            Ok(()) => ExitCode::from(TAMAM),
            Err(_) => ExitCode::from(YAZILAMADI),
        },
        Err(muifly_lib::monitor::etw::Engel::YetkiYok) => ExitCode::from(YETKI_YOK),
        Err(_) => ExitCode::from(OLCUM_HATASI),
    }
}
