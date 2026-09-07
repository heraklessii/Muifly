//! İkiliye gömülü oyun kataloğu: exe adı → oyunun okunabilir adı.
//!
//! Kütüphane taraması (`crate::library`) bir exe buluyor; katalog o exe'nin
//! hangi oyun olduğunu ve **rekabetçi** bir oyun olup olmadığını söylüyor.
//! İkisi birleşince kullanıcı, profil penceresini açtığında adı ve exe'si
//! dolu bir taslak görüyor.
//!
//! ## Katalogda ne YOK ve neden
//!
//! Hazır öncelik sınıfı, güç planı, dondurma listesi ya da affinite ayarı
//! **bilinçli olarak yok**. Böyle bir alan olsaydı, "Cyberpunk'ta güç
//! planını şuna al" gibi bir tavsiyeyi ölçmeden dağıtıyor olurduk; tasarım
//! ilkesi 4 tam olarak bunu yasaklıyor (`docs/DESIGN_PRINCIPLES.md`).
//! Rakiplerin "oyuna özel optimizasyon veritabanı" pazarlamasının büyük
//! kısmı bu türden ölçülmemiş iddialar. Gerekçe: `docs/decisions.md` #26.
//!
//! Katalogdaki tek "ayar" `rekabetci` bayrağı ve o da bir vaat değil,
//! bir **etiket**: profilin hangi oyun türüne ait olduğunu söylüyor ve
//! arayüzde mod adı olarak görünüyor. Kendi başına sistemde hiçbir şeyi
//! değiştirmiyor — katalog bir şey açmıyor.
//!
//! ## Kaynak
//!
//! `src-tauri/katalog.json` **elle** bakımı yapılan bir dosya —
//! `ucuncu-taraf.json` gibi üretilmiş değil, üreteci yok. Yanlış ya da eski
//! bir satırın bedeli düşük: eşleşme olmaz, kullanıcı adı kendi yazar.
//! Hiçbir satır tek başına sistemde bir değişikliğe yol açmıyor.

use serde::{Deserialize, Serialize};

use super::schema::Profil;

/// Katalogun ikiliye gömülü hali.
const KATALOG_JSON: &str = include_str!("../../katalog.json");

#[derive(Debug, Clone, Deserialize)]
struct Dosya {
    #[allow(dead_code)]
    surum: u32,
    girdiler: Vec<Girdi>,
}

/// Katalogdaki tek bir oyun.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Girdi {
    /// Küçük harfe indirgenmiş exe adı: `"cs2.exe"`.
    pub exe: String,
    pub ad: String,
    #[serde(default)]
    pub rekabetci: bool,
    /// Yalnızca gerçekten bir uyarı gerektiren oyunlarda dolu.
    #[serde(default)]
    pub not: Option<String>,
}

/// Çözümlenmiş katalog. İlk kullanımda bir kez okunuyor.
fn girdiler() -> &'static [Girdi] {
    use std::sync::OnceLock;
    static KATALOG: OnceLock<Vec<Girdi>> = OnceLock::new();
    KATALOG.get_or_init(|| {
        // Bozuk katalog programı açılıştan alıkoymamalı: kütüphane katalogsuz
        // da çalışıyor, sadece oyun adını tanımıyor.
        serde_json::from_str::<Dosya>(KATALOG_JSON)
            .map(|d| d.girdiler)
            .unwrap_or_default()
    })
}

/// Exe adına karşılık gelen katalog girdisi.
pub fn ara(exe: &str) -> Option<&'static Girdi> {
    let hedef = crate::system_boost::detect::ad_normalize(exe);
    girdiler().iter().find(|g| g.exe == hedef)
}

/// Birden çok aday exe içinden katalogda tanınan ilkini bulur.
pub fn ilk_taninan(exeler: &[String]) -> Option<&'static Girdi> {
    exeler.iter().find_map(|e| ara(e))
}

/// Bir profil taslağı ve o taslağın neden böyle olduğunun açıklaması.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Taslak {
    pub profil: Profil,
    /// Kullanıcıya gösterilecek satırlar: taslağın nereden geldiği.
    /// Şeffaflık ilkesi — hazır gelen bir profilin gerekçesi görünmeli.
    pub aciklamalar: Vec<String>,
}

/// Kütüphaneden gelen bir oyundan profil taslağı üretir.
///
/// **Hiçbir şey kaydetmiyor ve hiçbir şey uygulamıyor.** Dönen taslak
/// arayüzde açılıyor, kullanıcı değiştirip kaydediyor. Otomatik uygulama
/// yok: karar #10'un (otomatik uygulama varsayılan kapalı) aynı gerekçesi.
pub fn taslak(oyun_adi: &str, exeler: &[String]) -> Taslak {
    let mut aciklamalar = Vec::new();

    let taninan = ilk_taninan(exeler);
    let secilen_exe = taninan
        .map(|g| g.exe.clone())
        .or_else(|| exeler.first().cloned())
        .unwrap_or_default();

    let ad = taninan
        .map(|g| g.ad.clone())
        .unwrap_or_else(|| oyun_adi.to_string());
    let mut profil = Profil::yeni(kimlik_uret(&ad), ad.clone(), secilen_exe.clone());

    match taninan {
        Some(g) => {
            aciklamalar.push(format!("'{}' katalogda tanındı: {}", g.exe, g.ad));
            if g.rekabetci {
                profil.competitive = true;
                aciklamalar.push(
                    "Rekabetçi oyun olarak işaretlendi: mod adı arayüzde böyle görünür.".into(),
                );
            }
            if let Some(not) = &g.not {
                aciklamalar.push(not.clone());
            }
        }
        None => {
            aciklamalar.push(format!(
                "'{secilen_exe}' katalogda yok; ad ve ayarlar kütüphaneden geldiği gibi bırakıldı."
            ));
        }
    }

    if exeler.len() > 1 {
        aciklamalar.push(format!(
            "Kurulum klasöründe {} aday bulundu; profile yalnızca en olası olan eklendi.",
            exeler.len()
        ));
    }
    aciklamalar.push("Hiçbir şey uygulanmadı — kaydetmeden önce gözden geçir.".into());

    Taslak {
        profil,
        aciklamalar,
    }
}

/// Görünen addan profil kimliği türetir.
///
/// Arayüzdeki `kimlikUret` ile aynı kuralları uyguluyor
/// (`src/components/ProfilDiyalogu.tsx`): taslak arayüzde açıldığında
/// kimliğin değişmemesi gerekiyor.
pub fn kimlik_uret(ad: &str) -> String {
    let mut cikti = String::with_capacity(ad.len());
    let mut son_alt_cizgi = false;
    for c in ad.chars() {
        let d = match c {
            'ğ' | 'Ğ' => 'g',
            'ü' | 'Ü' => 'u',
            'ş' | 'Ş' => 's',
            'ı' | 'I' => 'i',
            'ö' | 'Ö' => 'o',
            'ç' | 'Ç' => 'c',
            _ => c.to_ascii_lowercase(),
        };
        if d.is_ascii_alphanumeric() {
            cikti.push(d);
            son_alt_cizgi = false;
        } else if !son_alt_cizgi && !cikti.is_empty() {
            cikti.push('_');
            son_alt_cizgi = true;
        }
    }
    let temiz = cikti.trim_matches('_').to_string();
    if temiz.is_empty() {
        "profil".to_string()
    } else {
        temiz
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn katalog_cozumleniyor() {
        assert!(
            girdiler().len() > 20,
            "gömülü katalog okunamadı ya da boş: {}",
            girdiler().len()
        );
    }

    #[test]
    fn exe_adlari_normal_bicimde() {
        for g in girdiler() {
            assert_eq!(g.exe, g.exe.to_lowercase(), "'{}' küçük harf değil", g.exe);
            assert!(g.exe.ends_with(".exe"), "'{}' .exe ile bitmiyor", g.exe);
            assert!(
                !g.exe.contains('\\') && !g.exe.contains('/'),
                "'{}' yol içeriyor — eşleşme yalnızca dosya adıyla yapılıyor",
                g.exe
            );
            assert!(!g.ad.trim().is_empty(), "'{}' adsız", g.exe);
        }
    }

    #[test]
    fn tekrar_eden_exe_yok() {
        let mut gorulen = std::collections::HashSet::new();
        for g in girdiler() {
            assert!(gorulen.insert(&g.exe), "'{}' katalogda iki kez var", g.exe);
        }
    }

    #[test]
    fn cok_genel_exe_adi_yok() {
        // Katalog büyüdükçe asıl tehlike yanlış bir satır değil, FAZLA GENEL
        // bir satır: "game.exe" gibi bir ad alakasız bir süreci oyun diye
        // etiketler ve kullanıcıya hiç kurmadığı bir oyunun adı gösterilir.
        // Eşleşme yalnızca dosya adıyla yapıldığı için (bkz. `ara`) tek
        // koruma bu liste.
        let cok_genel = [
            "game.exe",
            "launcher.exe",
            "client.exe",
            "start.exe",
            "main.exe",
            "app.exe",
            "player.exe",
            "java.exe",
            "javaw.exe",
            "shootergame.exe",
            "unrealgame.exe",
            "game-win64-shipping.exe",
            "client-win64-shipping.exe",
        ];
        for g in girdiler() {
            assert!(
                !cok_genel.contains(&g.exe.as_str()),
                "'{}' başka programlarla çakışacak kadar genel bir ad",
                g.exe
            );
        }
    }

    #[test]
    fn katalog_notlarinda_sayisal_vaat_yok() {
        // Tasarım ilkesi 4'ün katalog tarafındaki karşılığı — `tcp.rs`
        // içindeki testin aynısı. Katalog büyüdükçe asıl koruma bu olacak.
        let yasakli = [
            "% ",
            "ms düşür",
            "kat hızl",
            "garanti",
            "artırır",
            "fps kazan",
        ];
        for g in girdiler() {
            let Some(not) = &g.not else { continue };
            for y in yasakli {
                assert!(
                    !not.to_lowercase().contains(y),
                    "'{}' notu sayısal/garantili vaat içeriyor: {y}",
                    g.exe
                );
            }
        }
    }

    #[test]
    fn taslak_hicbir_sureci_dondurmuyor() {
        // Ürün kararı: katalog dondurma listesi ÖNERMİYOR. Karar #26.
        // Katalog dosyasına böyle bir alan eklenirse bu test düşer.
        for exe in ["cs2.exe", "bilinmeyen_oyun.exe"] {
            let t = taslak("Bir Oyun", &[exe.to_string()]);
            assert!(t.profil.system.suspend_process_list.is_empty());
            assert_eq!(t.profil.system.power_plan, None);
        }
    }

    #[test]
    fn rekabetci_oyun_isaretleniyor() {
        let t = taslak("cs2", &["cs2.exe".to_string()]);
        assert!(t.profil.competitive);
        assert_eq!(t.profil.display_name, "Counter-Strike 2");
        assert!(t.aciklamalar.iter().any(|a| a.contains("Rekabetçi")));
    }

    #[test]
    fn taninmayan_oyun_kutuphane_adini_koruyor() {
        let t = taslak("Benim Oyunum", &["benimoyunum.exe".to_string()]);
        assert_eq!(t.profil.display_name, "Benim Oyunum");
        assert!(!t.profil.competitive);
        assert_eq!(t.profil.executable_names, vec!["benimoyunum.exe"]);
    }

    #[test]
    fn taslaga_yalnizca_tek_exe_giriyor() {
        // Bütün adayları eklemek, başlatıcıya ve yardımcı süreçlere de
        // oyun ayarı uygulardı.
        let t = taslak(
            "Oyun",
            &[
                "launcher.exe".to_string(),
                "cs2.exe".to_string(),
                "yardimci.exe".to_string(),
            ],
        );
        assert_eq!(t.profil.executable_names, vec!["cs2.exe"]);
    }

    #[test]
    fn bos_aday_listesi_cokertmiyor() {
        let t = taslak("Adsiz", &[]);
        assert!(
            t.profil.executable_names.iter().all(|e| e.is_empty())
                || t.profil.executable_names.is_empty()
        );
        assert!(!t.aciklamalar.is_empty());
    }

    #[test]
    fn taslak_dogrulamadan_geciyor() {
        // Ürettiğimiz taslak, şemanın kendi doğrulamasını geçmeli; yoksa
        // kullanıcı kaydetmeye çalışınca hata alır.
        let t = taslak("cs2", &["cs2.exe".to_string()]);
        let (_, duzeltmeler) = t.profil.dogrula().expect("taslak doğrulamayı geçmedi");
        assert!(
            duzeltmeler.is_empty(),
            "beklenmedik düzeltme: {duzeltmeler:?}"
        );
    }

    #[test]
    fn kimlik_uretimi_arayuzle_ayni() {
        assert_eq!(kimlik_uret("Counter-Strike 2"), "counter_strike_2");
        assert_eq!(kimlik_uret("Baldur's Gate 3"), "baldur_s_gate_3");
        assert_eq!(kimlik_uret("Çılgın Şığır"), "cilgin_sigir");
        assert_eq!(kimlik_uret("   "), "profil");
        assert_eq!(kimlik_uret("!!!"), "profil");
    }
}
