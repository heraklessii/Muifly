//! Muifly — Windows için oyun performans optimizasyon aracı.
//!
//! Modül haritası (`docs/ARCHITECTURE.md`):
//!
//! ```text
//! commands  ← arayüzün tek girişi
//!    │
//! state (Motor)  ← akış: oyun algılandı → profil uygula → kapanınca geri al
//!    ├── profile_engine   mod seçimi, profil şeması, disk deposu
//!    ├── system_boost     öncelik, affinite, dondurma, güç planı, açılış
//!    ├── network_boost    DNS ölçümü, gecikme/jitter, TCP, QoS
//!    ├── monitor          şeffaflık günlüğü + ölçüm
//!    ├── scaling          ekran yakalama + ölçekleme + sunum (Faz 3)
//!    ├── ledger           geri alma defteri (veri)
//!    └── revert           geri alma uygulayıcısı
//! ```
//!
//! Değişmez kural: sistemde bir şey değiştiren her yol, `state::Motor`
//! üzerinden geçiyor ve deftere + günlüğe yazıyor.

pub mod ceviri;
pub mod commands;
pub mod error;
pub mod ledger;
pub mod library;
pub mod monitor;
pub mod network_boost;
pub mod profile_engine;
pub mod registry;
pub mod revert;
pub mod scaling;
pub mod settings;
pub mod state;
pub mod surum;
pub mod system_boost;
pub mod tray;
pub mod ucuncu_taraf;
#[cfg(windows)]
pub mod winutil;

use std::time::Duration;

use tauri::{Emitter, Manager};

use crate::state::Motor;

/// Arka plan döngüsünün temel adımı.
///
/// Mod kontrolü her saniye, ölçüm ise kullanıcının seçtiği aralıkta. İkisi
/// ayrı: oyun algılamanın hızlı olması lazım (kullanıcı Alt+Tab yapınca
/// arayüz hemen doğruyu göstermeli), ölçümün ise sık olması gerekmiyor ve her
/// ölçüm bir ICMP paketi demek.
const DONGU_ADIMI: Duration = Duration::from_secs(1);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|uygulama, _, _| {
            // İkinci kez açılırsa var olan pencereyi öne getir: arka planda
            // duran bir araçta simge çift tıklaması bunu bekliyor.
            if let Some(pencere) = uygulama.get_webview_window("main") {
                let _ = pencere.show();
                let _ = pencere.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .setup(|uygulama| {
            let motor = Motor::baslat();
            uygulama.manage(parking_lot::Mutex::new(motor));

            // Tepsi önce kuruluyor: `--tepside` ile pencere gizlenecekse,
            // gizlemeden önce kullanıcının programı geri getirebileceği bir
            // yol var olmalı. Kurulum başarısızsa pencere gizlenmiyor.
            let tepsi_var = match tray::kur(uygulama.handle()) {
                Ok(()) => true,
                Err(e) => {
                    log::warn!("tepsi simgesi kurulamadı: {e}");
                    false
                }
            };

            // Tepside açılış: `--tepside` bayrağıyla başlatıldıysa pencere
            // gösterilmiyor (`system_boost::startup`).
            if tepsi_var && std::env::args().any(|a| a == "--tepside") {
                if let Some(pencere) = uygulama.get_webview_window("main") {
                    let _ = pencere.hide();
                }
            }

            pencere_kapanisini_bagla(uygulama.handle());

            arka_plan_dongusu(uygulama.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::surum,
            commands::kisitlar,
            commands::ucuncu_taraf_listesi,
            commands::ucuncu_taraf_metni,
            commands::durum,
            commands::gunluk,
            commands::gunlugu_temizle,
            commands::gecmis,
            commands::gecmis_ozeti,
            commands::gecmisi_temizle,
            commands::gecmis_disa_aktar,
            commands::ornekler,
            commands::ozet,
            commands::karsilastirma,
            commands::bekleyen_geri_almalar,
            commands::geri_al,
            commands::hepsini_geri_al,
            commands::profiller,
            commands::profil_kaydet,
            commands::profil_sil,
            commands::profil_disa_aktar,
            commands::profil_etkileri,
            commands::profil_onizle,
            commands::profil_ice_aktar,
            commands::profil_uygula,
            commands::ondekine_uygula,
            commands::oturumu_kapat,
            commands::ondeki_pencere,
            commands::surecler,
            commands::dondurma_adaylari,
            commands::oyunlari_tara,
            commands::oyun_gorseli,
            commands::oyun_elle_ekle,
            commands::profil_taslagi,
            commands::katalog_girdisi,
            commands::taninan_surecler,
            commands::dns_karsilastir,
            commands::yol_testi,
            commands::kare_olcum_durumu,
            commands::oyunu_olc,
            commands::tcp_durumu,
            commands::tcp_uygula,
            commands::qos_ilkeleri,
            commands::qos_kaldir,
            commands::ag_aciklamalari,
            commands::ayarlar,
            commands::ayarlari_yaz,
            commands::otomatik_baslatma_ayarla,
            commands::otomatik_baslatma_komutu,
            commands::yapilmayanlar,
            commands::olcekleme_ekranlari,
            commands::olcekleme_algoritmalari,
            commands::olcekleme_durumu,
            commands::olcekleme_baslat,
            commands::olcekleme_durdur,
            commands::olcekleme_algoritma,
            commands::olcekleme_denemesi,
        ])
        .build(tauri::generate_context!())
        .expect("Muifly başlatılamadı")
        .run(|kol, olay| {
            if let tauri::RunEvent::Exit = olay {
                cikista_geri_al(kol);
            }
        });
}

/// Pencere kapatıldığında ne olacağı.
///
/// `tepsiye_kucult` açıkken (varsayılan) kapatma programı sonlandırmıyor,
/// pencereyi gizliyor: oyun bekleyen bir aracın kapat düğmesiyle ölmesi,
/// otomatik uygulamayı da sessizce kapatırdı. Ayar kapalıysa davranış
/// standart — kapat, çık.
fn pencere_kapanisini_bagla(uygulama: &tauri::AppHandle) {
    let Some(pencere) = uygulama.get_webview_window("main") else {
        return;
    };
    let kol = uygulama.clone();
    pencere.clone().on_window_event(move |olay| {
        let tauri::WindowEvent::CloseRequested { api, .. } = olay else {
            return;
        };
        let tepsiye = kol
            .try_state::<parking_lot::Mutex<Motor>>()
            .map(|k| k.lock().ayarlar.tepsiye_kucult)
            .unwrap_or(false);
        if tepsiye {
            api.prevent_close();
            let _ = pencere.hide();
        }
    });
}

/// Çıkışta oturumluk değişiklikleri geri alır.
///
/// Kalıcı kayıtlar defterde kalıyor — kullanıcı onları açıkça istedi ve
/// arayüzde "varsayılana dön" ile duruyorlar. Oturumluk olanlar (dondurulmuş
/// süreçler, güç planı, öncelik) programla birlikte kalkmalı: yoksa
/// dondurulmuş bir uygulama, Muifly bir daha açılana kadar dondurulmuş
/// kalırdı. Açılıştaki `revert::acilista_temizle` bunun ağı, ilk savunması
/// değil.
fn cikista_geri_al(uygulama: &tauri::AppHandle) {
    let Some(kilit) = uygulama.try_state::<parking_lot::Mutex<Motor>>() else {
        return;
    };
    let sonuc = kilit.lock().oturumu_kapat();
    if sonuc.geri_alinan > 0 {
        log::info!("çıkışta {} değişiklik geri alındı", sonuc.geri_alinan);
    }
    for (ozet, sebep) in sonuc.basarisiz {
        log::warn!("çıkışta geri alınamadı — {ozet}: {sebep}");
    }
}

/// Mod izleme ve ölçüm döngüsü.
///
/// Ayrı bir iş parçacığında: Tauri'nin olay döngüsünü bloke eden bir ölçüm,
/// arayüzü dondururdu. Motor kilidi her adımda kısa süre alınıyor.
fn arka_plan_dongusu(uygulama: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut sayac: u32 = 0;

        loop {
            std::thread::sleep(DONGU_ADIMI);
            sayac = sayac.wrapping_add(1);

            let Some(kilit) = uygulama.try_state::<parking_lot::Mutex<Motor>>() else {
                continue;
            };

            // 1. Mod kontrolü — her saniye.
            let (mod_degisti, otomatik, bildirim, olcum_araligi) = {
                let mut motor = kilit.lock();
                let degisti = motor.mod_guncelle();
                (
                    degisti,
                    motor.ayarlar.otomatik_uygula,
                    motor.ayarlar.mod_bildirimi,
                    motor.ayarlar.olcum_araligi_sn.max(1),
                )
            };

            if let Some(degisim) = mod_degisti {
                let yeni_mod = degisim.yeni;
                // Otomatik uygulama açıksa ve bir oyuna geçildiyse profili
                // uygula. Varsayılan kapalı (`settings::Ayarlar`).
                if otomatik {
                    if let Some(pid) = yeni_mod.oyun_pid() {
                        let mut motor = kilit.lock();
                        let profil = match &yeni_mod {
                            profile_engine::Mod::OyunProfili { profil_id, .. }
                            | profile_engine::Mod::Rekabetci {
                                profil_id: Some(profil_id),
                                ..
                            } => motor
                                .profiller
                                .iter()
                                .find(|p| p.profile_id == *profil_id)
                                .cloned(),
                            profile_engine::Mod::OyunGenel { surec, .. }
                            | profile_engine::Mod::Rekabetci { surec, .. } => {
                                profile_engine::genel_profil(surec)
                                    .dogrula()
                                    .ok()
                                    .map(|(p, _)| p)
                            }
                            _ => None,
                        };
                        if let Some(profil) = profil {
                            motor.profil_uygula(&profil, pid);
                        }
                    }
                }

                let durum = kilit.lock().durum();
                // Tepsi ipucu: simgenin üzerine gelen kullanıcı, pencereyi
                // açmadan hangi modda olduğunu görsün.
                tray::ipucu_guncelle(&uygulama, &durum.mod_adi);
                // Bildirim varsayılan kapalı (`settings::Ayarlar`): pencere
                // kapalıyken ne olduğunu görmenin tek yolu bu, ama her
                // Alt+Tab'da bildirim atmak gürültü olurdu.
                if bildirim {
                    tray::mod_bildir(&uygulama, &yeni_mod, degisim.geri_alinan, otomatik);
                }
                let _ = uygulama.emit(commands::OLAY_DURUM, durum);
                let satirlar = kilit.lock().gunluk.son(20);
                let _ = uygulama.emit(commands::OLAY_GUNLUK, satirlar);
            }

            // 2. Ölçüm — kullanıcının seçtiği aralıkta.
            if sayac % olcum_araligi == 0 {
                let ornek = kilit.lock().ornek_al();
                let _ = uygulama.emit(commands::OLAY_ORNEK, ornek);
            }
        }
    });
}
