//! Sistem tepsisi simgesi ve menüsü.
//!
//! `--tepside` bayrağıyla açılan program pencere göstermiyor
//! (`system_boost::startup`). Tepsi simgesi olmadan kullanıcının programı geri
//! getirmesinin yolu kalmıyor; bu yüzden tepsi, otomatik başlatmanın
//! önkoşulu.
//!
//! Menüden tetiklenen her işlem `state::Motor` üzerinden geçiyor: uygulanan
//! değişiklik hem deftere hem günlüğe yazılıyor. Tepsi ikinci bir yol değil,
//! aynı yolun kısayolu.
//!
//! Pencere kapalıyken sonucu görmenin tek yolu bildirim olduğu için menü
//! işlemleri kısa bir OS bildirimi gösteriyor. Bildirim gösterilemezse işlem
//! yine de yapılıyor: asıl kayıt günlükte.

use tauri::menu::{IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_notification::NotificationExt;

use crate::commands::{OLAY_DURUM, OLAY_GUNLUK};
use crate::state::Motor;

/// Tepsi simgesinin kimliği — ipucu güncellemesi bununla buluyor.
pub const TEPSI_ID: &str = "muifly";

pub const OGE_GOSTER: &str = "goster";
pub const OGE_UYGULA: &str = "uygula";
pub const OGE_VARSAYILAN: &str = "varsayilan";
pub const OGE_CIKIS: &str = "cikis";

/// Bildirim gövdesinin üst sınırı. Uzun liste bildirimde okunmuyor; tamamı
/// zaten günlükte.
const BILDIRIM_SINIRI: usize = 160;

/// Menü öğeleri: (kimlik, etiket). Sıra, menüdeki sıra.
///
/// Tek yerde durması bilinçli: menü buradan kuruluyor, testler etiketleri
/// buradan okuyor.
pub fn menu_ogeleri() -> [(&'static str, &'static str); 4] {
    [
        (OGE_GOSTER, "Göster"),
        (OGE_UYGULA, "Öndekine uygula"),
        (OGE_VARSAYILAN, "Varsayılana dön"),
        (OGE_CIKIS, "Çıkış"),
    ]
}

/// Tepsi simgesini kurar.
pub fn kur<R: Runtime>(uygulama: &AppHandle<R>) -> tauri::Result<()> {
    let ogeler = menu_ogeleri()
        .into_iter()
        .map(|(kimlik, etiket)| MenuItem::with_id(uygulama, kimlik, etiket, true, None::<&str>))
        .collect::<tauri::Result<Vec<MenuItem<R>>>>()?;

    // Çıkış'tan önce ayraç: yanlışlıkla tıklanması en pahalı öğe o.
    let ayirac = PredefinedMenuItem::separator(uygulama)?;
    let mut sirali: Vec<&dyn IsMenuItem<R>> = Vec::with_capacity(ogeler.len() + 1);
    for (oge, (kimlik, _)) in ogeler.iter().zip(menu_ogeleri()) {
        if kimlik == OGE_CIKIS {
            sirali.push(&ayirac);
        }
        sirali.push(oge);
    }
    let menu = Menu::with_items(uygulama, &sirali)?;

    let mut kurucu = TrayIconBuilder::with_id(TEPSI_ID)
        .tooltip("Muifly")
        .menu(&menu)
        // Sol tık menüyü değil pencereyi açıyor: Windows'ta tepsi
        // simgesinden beklenen davranış bu.
        .show_menu_on_left_click(false)
        .on_menu_event(menu_olayi)
        .on_tray_icon_event(|tepsi, olay| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = olay
            {
                pencereyi_goster(tepsi.app_handle());
            }
        });

    if let Some(ikon) = uygulama.default_window_icon().cloned() {
        kurucu = kurucu.icon(ikon);
    }

    kurucu.build(uygulama)?;
    Ok(())
}

/// Tepsi ipucunu günceller: simgenin üzerine gelince hangi modda olduğu
/// görünsün.
pub fn ipucu_guncelle<R: Runtime>(uygulama: &AppHandle<R>, mod_adi: &str) {
    if let Some(tepsi) = uygulama.tray_by_id(TEPSI_ID) {
        let _ = tepsi.set_tooltip(Some(format!("Muifly — {mod_adi}")));
    }
}

fn menu_olayi<R: Runtime>(uygulama: &AppHandle<R>, olay: MenuEvent) {
    match olay.id.as_ref() {
        OGE_GOSTER => pencereyi_goster(uygulama),
        OGE_UYGULA => ondekine_uygula(uygulama),
        OGE_VARSAYILAN => varsayilana_don(uygulama),
        // Oturumluk değişikliklerin geri alınması `lib::run` içindeki çıkış
        // olayında: kapanışın hangi yoldan geldiği fark etmesin.
        OGE_CIKIS => uygulama.exit(0),
        _ => {}
    }
}

/// Pencereyi geri getirir. Simge durumuna küçültülmüşse onu da açıyor.
pub fn pencereyi_goster<R: Runtime>(uygulama: &AppHandle<R>) {
    if let Some(pencere) = uygulama.get_webview_window("main") {
        let _ = pencere.unminimize();
        let _ = pencere.show();
        let _ = pencere.set_focus();
    }
}

fn ondekine_uygula<R: Runtime>(uygulama: &AppHandle<R>) {
    let Some(kilit) = uygulama.try_state::<parking_lot::Mutex<Motor>>() else {
        return;
    };

    match kilit.lock().ondekine_uygula() {
        Ok(cikti) => {
            let govde = if cikti.uygulanan.is_empty() {
                "Uygulanacak bir değişiklik çıkmadı".to_string()
            } else {
                cikti.uygulanan.join(", ")
            };
            bildir(uygulama, "Profil uygulandı", &govde);
        }
        Err(e) => bildir(uygulama, "Uygulanamadı", &crate::error::tek_satir(&e)),
    }
    yayinla(uygulama);
}

fn varsayilana_don<R: Runtime>(uygulama: &AppHandle<R>) {
    let Some(kilit) = uygulama.try_state::<parking_lot::Mutex<Motor>>() else {
        return;
    };

    let sonuc = kilit.lock().hepsini_geri_al();
    let mut govde = format!("{} değişiklik geri alındı", sonuc.geri_alinan);
    if !sonuc.basarisiz.is_empty() {
        govde.push_str(&format!(
            ", {} tanesi geri alınamadı",
            sonuc.basarisiz.len()
        ));
    }
    bildir(uygulama, "Varsayılana dönüldü", &govde);
    yayinla(uygulama);
}

/// Arayüz açıksa tepsiden yapılan işlemi de görsün.
fn yayinla<R: Runtime>(uygulama: &AppHandle<R>) {
    let Some(kilit) = uygulama.try_state::<parking_lot::Mutex<Motor>>() else {
        return;
    };
    let durum = kilit.lock().durum();
    let _ = uygulama.emit(OLAY_DURUM, durum);
    let satirlar = kilit.lock().gunluk.son(20);
    let _ = uygulama.emit(OLAY_GUNLUK, satirlar);
}

/// Mod değişiminin bildirim metni. `None` = bildirilecek bir şey yok.
///
/// Saf fonksiyon: metnin ne diyeceği bildirim altyapısından bağımsız test
/// edilebilsin. İki kural:
///
/// 1. **Olmayan iş bildirilmiyor.** Oyundan çıkışta hiçbir şey geri
///    alınmadıysa (profil hiç uygulanmamıştı) bildirim de yok — "geri alındı"
///    demek yanlış beyan, boş bir bildirim ise gürültü.
/// 2. Otomatik uygulama kapalıyken bildirimin ne söyleyeceği belli:
///    program bir şey YAPMADI, sadece oyunu gördü.
pub fn mod_bildirim_metni(
    mod_: &crate::profile_engine::Mod,
    geri_alinan: usize,
    otomatik_uygula: bool,
) -> Option<(String, String)> {
    match mod_.surec() {
        Some(surec) => {
            let govde = if otomatik_uygula {
                format!("{surec} algılandı, profil uygulandı. Ayrıntılar günlükte.")
            } else {
                format!("{surec} algılandı. Otomatik uygulama kapalı; uygulamak için Muifly'ı aç.")
            };
            Some((mod_.ad().to_string(), govde))
        }
        None if geri_alinan > 0 => Some((
            "Oyun kapandı".to_string(),
            format!("{geri_alinan} değişiklik geri alındı."),
        )),
        None => None,
    }
}

/// Mod değişimini bildirir. Ayar kapalıyken hiç çağrılmıyor (`lib.rs`).
pub fn mod_bildir<R: Runtime>(
    uygulama: &AppHandle<R>,
    mod_: &crate::profile_engine::Mod,
    geri_alinan: usize,
    otomatik_uygula: bool,
) {
    if let Some((baslik, govde)) = mod_bildirim_metni(mod_, geri_alinan, otomatik_uygula) {
        bildir(uygulama, &baslik, &govde);
    }
}

fn bildir<R: Runtime>(uygulama: &AppHandle<R>, baslik: &str, govde: &str) {
    let _ = uygulama
        .notification()
        .builder()
        .title(baslik)
        .body(kisalt(govde, BILDIRIM_SINIRI))
        .show();
}

/// Metni karakter sınırında kesiyor.
///
/// Bayt sınırında kesmek Türkçe metinde paniğe yol açardı: pek çok harf çok
/// baytlı.
pub fn kisalt(metin: &str, sinir: usize) -> String {
    if metin.chars().count() <= sinir {
        return metin.to_string();
    }
    let kesilen: String = metin.chars().take(sinir.saturating_sub(1)).collect();
    format!("{kesilen}…")
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn menu_kimlikleri_benzersiz() {
        let ogeler = menu_ogeleri();
        for (i, (kimlik, _)) in ogeler.iter().enumerate() {
            for (j, (digeri, _)) in ogeler.iter().enumerate() {
                assert!(
                    i == j || kimlik != digeri,
                    "yinelenen menü kimliği: {kimlik}"
                );
            }
        }
    }

    #[test]
    fn menu_etiketleri_dolu() {
        for (kimlik, etiket) in menu_ogeleri() {
            assert!(!etiket.trim().is_empty(), "{kimlik} etiketsiz");
        }
    }

    #[test]
    fn menude_sayisal_vaat_yok() {
        // DESIGN_PRINCIPLES.md madde 4: kullanıcıya gösterilen metinde
        // rakamla verilmiş performans vaadi olmaz.
        for (_, etiket) in menu_ogeleri() {
            assert!(
                !etiket.chars().any(|c| c.is_ascii_digit()),
                "menü etiketinde sayı: {etiket}"
            );
        }
    }

    #[test]
    fn goster_ve_cikis_menude() {
        let kimlikler: Vec<&str> = menu_ogeleri().iter().map(|(k, _)| *k).collect();
        // İkisi de zorunlu: biri pencereyi geri getiriyor, diğeri pencere
        // gizliyken programdan çıkmanın tek yolu.
        assert!(kimlikler.contains(&OGE_GOSTER));
        assert!(kimlikler.contains(&OGE_CIKIS));
    }

    fn oyun_modu() -> crate::profile_engine::Mod {
        crate::profile_engine::Mod::OyunGenel {
            pid: 1234,
            surec: "cs2.exe".into(),
        }
    }

    #[test]
    fn oyun_bildiriminde_surec_adi_var() {
        let (baslik, govde) = mod_bildirim_metni(&oyun_modu(), 0, true).unwrap();
        assert_eq!(baslik, "Oyun Algılandı");
        assert!(govde.contains("cs2.exe"));
        assert!(govde.contains("uygulandı"));
    }

    #[test]
    fn otomatik_kapaliyken_yapilmadigi_soyleniyor() {
        // Program bir şey YAPMADI, sadece oyunu gördü. Bildirimin bunu
        // karıştırmaması gerekiyor.
        let (_, govde) = mod_bildirim_metni(&oyun_modu(), 0, false).unwrap();
        assert!(govde.contains("Otomatik uygulama kapalı"));
        assert!(!govde.contains("uygulandı."));
    }

    #[test]
    fn geri_alma_yoksa_bildirim_de_yok() {
        // "0 değişiklik geri alındı" diye bir bildirim, hiç uygulanmamış bir
        // profilden sonra kullanıcıyı boşuna rahatsız ederdi.
        assert!(mod_bildirim_metni(&crate::profile_engine::Mod::Bosta, 0, true).is_none());
    }

    #[test]
    fn geri_alinan_sayisi_bildiriliyor() {
        let (baslik, govde) =
            mod_bildirim_metni(&crate::profile_engine::Mod::Bosta, 3, true).unwrap();
        assert_eq!(baslik, "Oyun kapandı");
        assert!(govde.contains('3'));
    }

    #[test]
    fn bildirimde_sayisal_vaat_yok() {
        // DESIGN_PRINCIPLES.md madde 4. Bildirimdeki tek sayı, gerçekten
        // sayılmış bir şey olabilir (geri alınan kayıt adedi); performans
        // iddiası olamaz.
        for (mod_, geri, otomatik) in [
            (oyun_modu(), 0, true),
            (oyun_modu(), 0, false),
            (crate::profile_engine::Mod::Bosta, 2, true),
        ] {
            let Some((baslik, govde)) = mod_bildirim_metni(&mod_, geri, otomatik) else {
                continue;
            };
            let metin = format!("{baslik} {govde}").to_lowercase();
            for yasak in ["ms", "fps", "%", "daha hızlı", "kazan"] {
                assert!(!metin.contains(yasak), "bildirimde sayısal vaat: {metin}");
            }
        }
    }

    #[test]
    fn kisa_metin_degismiyor() {
        assert_eq!(kisalt("kısa", 10), "kısa");
    }

    #[test]
    fn uzun_metin_karakter_sinirinda_kesiliyor() {
        let metin = "ışığ".repeat(100);
        let kesilen = kisalt(&metin, 20);
        assert_eq!(kesilen.chars().count(), 20);
        assert!(kesilen.ends_with('…'));
    }

    #[test]
    fn tam_sinirdaki_metin_kesilmiyor() {
        let metin = "ğüşiöç";
        assert_eq!(kisalt(metin, 6), metin);
    }
}
