//! Bir kurulum klasöründe "oyunun kendisi hangi exe" sorusuna cevap arar.
//!
//! Bir oyun klasöründe onlarca `.exe` olabiliyor: kurulum sihirbazı, çökme
//! raporlayıcı, hile karşıtı servis, yeniden dağıtılabilir paketler, motorun
//! yardımcı süreçleri. Kullanıcıya bunların hepsini sıralamak, elle yazmaktan
//! daha iyi değil.
//!
//! Bu yüzden iki kademe var:
//!
//! 1. **Eleme** — adı ya da klasörü tanıdık bir gürültü kalıbına uyanlar hiç
//!    gösterilmiyor.
//! 2. **Puanlama** — kalanlar sıralanıyor. En büyük dosya genelde oyunun
//!    kendisidir; adı oyunun adına benzeyen dosya daha da güçlü bir işarettir.
//!
//! Puanlama **öneri sırası** belirliyor, karar vermiyor: liste kullanıcıya
//! olduğu gibi gösteriliyor ve seçim onun. Yanlış sıralama can sıkar, zarar
//! vermez. Bu yüzden sezgiler burada serbestçe ayarlanabilir; kimse bu
//! sayılara güvenerek sistemde bir şey değiştirmiyor.

use std::path::Path;

use crate::system_boost::detect::ad_normalize;

/// Kurulum klasöründe kaç seviye derine inilecek.
///
/// Unreal oyunları exe'yi `<Oyun>/Binaries/Win64/` altında tutuyor: üç
/// seviye. Dört, bunu rahat kapsıyor ve devasa varlık klasörlerinde
/// kaybolmamızı engelliyor.
const AZAMI_DERINLIK: usize = 4;

/// Bir oyunda dolaşılacak azami klasör sayısı.
///
/// Sınırsız yürüyüş, 200 GB'lık bir oyun klasöründe arayüzü dakikalarca
/// bekletebilir. Tarama eksik kalırsa kullanıcı exe'yi elle seçebiliyor;
/// arayüzün donması ise geri dönüşü olmayan bir izlenim bırakır.
///
/// Yürüyüş **genişlik öncelikli**: bütçe biterse elde kalanlar sığ
/// klasörler olur ve oyunun asıl exe'si neredeyse her zaman sığdadır.
/// Derinlik öncelikli olsaydı bütçe tek bir varlık klasöründe tükenebilirdi.
const AZAMI_KLASOR: usize = 800;

/// Kullanıcıya gösterilecek azami aday.
const AZAMI_ADAY: usize = 12;

/// Adında bu parçalardan biri geçen exe hiç gösterilmiyor.
///
/// Hepsi "oyunun kendisi olamaz" diyebildiğimiz dosyalar. Şüpheli olan
/// listeye girmiyor: eksik eleme kullanıcıya fazladan satır gösterir, fazla
/// eleme aradığı exe'yi gizler.
const GURULTU_ADLARI: &[&str] = &[
    "unins",
    "setup",
    "install",
    "vcredist",
    "vc_redist",
    "dxsetup",
    "dxwebsetup",
    "directx",
    "dotnetfx",
    "ndp4",
    "oalinst",
    "crashreport",
    "crashhandler",
    "crashpad",
    // "crash" tek başına elenmiyor: adında geçen gerçek oyunlar var
    // (Crash Bandicoot). Raporlayıcıları eleyen kalıp "reporter".
    "reporter",
    "prereqsetup",
    "easyanticheat",
    "eac_",
    "battleye",
    "be_service",
    "beservice",
    "steamerrorreporter",
    "activationui",
    "cefsubprocess",
    "epicwebhelper",
    "helper",
    "updater",
    "patcher",
    "config",
    "settings",
    "uninstall",
];

/// Bu klasörlerin içine hiç girilmiyor.
const GURULTU_KLASORLERI: &[&str] = &[
    "_commonredist",
    "commonredist",
    "redist",
    "redistributable",
    "redistributables",
    "directx",
    "dotnet",
    "vcredist",
    "easyanticheat",
    "easyanticheat_eos",
    "battleye",
    "engine",
    "installer",
    "installers",
    "_installer",
    "support",
    "thirdparty",
    "third_party",
    "docs",
    "documentation",
    "soundtrack",
    "manual",
];

/// Kurulum klasöründe bulunmuş bir çalıştırılabilir.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aday {
    /// Küçük harfe indirgenmiş dosya adı — profil eşleştirmesi bunu kullanıyor.
    pub ad: String,
    /// Tam yol. Yalnızca ikon çıkarmak için; profile yazılmıyor.
    pub yol: String,
    pub puan: i64,
}

/// Kurulum klasörünü tarar ve aday exe'leri puanlarına göre sıralı döndürür.
///
/// Klasör okunamıyorsa (silinmiş oyun, izin yok) boş liste döner — hata
/// değil: kullanıcı exe'yi elle seçebilir.
pub fn adaylar(kok: &Path, oyun_adi: &str) -> Vec<Aday> {
    let mut bulunanlar = Vec::new();
    let mut kuyruk = std::collections::VecDeque::from([(kok.to_path_buf(), 0usize)]);
    let mut gezilen = 0usize;

    while let Some((klasor, derinlik)) = kuyruk.pop_front() {
        gezilen += 1;
        if gezilen > AZAMI_KLASOR {
            break;
        }
        let Ok(girdiler) = std::fs::read_dir(&klasor) else {
            continue;
        };
        for girdi in girdiler.flatten() {
            let yol = girdi.path();
            let Ok(tur) = girdi.file_type() else { continue };
            if tur.is_dir() {
                if derinlik + 1 > AZAMI_DERINLIK {
                    continue;
                }
                let ad = girdi.file_name().to_string_lossy().to_lowercase();
                if GURULTU_KLASORLERI.contains(&ad.as_str()) {
                    continue;
                }
                kuyruk.push_back((yol, derinlik + 1));
            } else if tur.is_file() {
                let ad = ad_normalize(&girdi.file_name().to_string_lossy());
                if !ad.ends_with(".exe") {
                    continue;
                }
                let boyut = girdi.metadata().map(|m| m.len()).unwrap_or(0);
                let Some(puan) = puanla(&ad, boyut, derinlik, oyun_adi) else {
                    continue;
                };
                bulunanlar.push(Aday {
                    ad,
                    yol: yol.to_string_lossy().into_owned(),
                    puan,
                });
            }
        }
    }

    // Aynı ada sahip birden çok dosya olabilir (x86/x64 klasörleri). En yüksek
    // puanlı olan kalıyor ki listede tekrar görünmesin.
    bulunanlar.sort_by(|a, b| b.puan.cmp(&a.puan).then_with(|| a.ad.cmp(&b.ad)));
    let mut gorulen = std::collections::HashSet::new();
    bulunanlar.retain(|a| gorulen.insert(a.ad.clone()));
    bulunanlar.truncate(AZAMI_ADAY);
    bulunanlar
}

/// Tek bir exe'yi puanlar. `None` = elendi, gösterilmeyecek.
///
/// Dosya sistemine dokunmuyor: sıralama sezgisinin tamamı burada ve testten
/// geçiyor.
pub fn puanla(ad: &str, boyut: u64, derinlik: usize, oyun_adi: &str) -> Option<i64> {
    let govde = ad.strip_suffix(".exe").unwrap_or(ad);
    if govde.is_empty() {
        return None;
    }
    if GURULTU_ADLARI.iter().any(|g| govde.contains(g)) {
        return None;
    }

    // Megabayt, 400 MB'de tavanlanıyor. Tavansız bırakılırsa 8 GB'lık tek bir
    // paket dosyası, adı birebir eşleşen 300 MB'lık gerçek exe'yi geçer.
    let mb = (boyut / (1024 * 1024)).min(400) as i64;
    let mut puan = mb;

    // Ad benzerliği boyuttan daha güçlü bir işaret: "eldenring.exe" 60 MB
    // olsa bile o klasördeki 300 MB'lık yardımcıyı geçmeli.
    let sade_oyun = sadelestir(oyun_adi);
    let sade_govde = sadelestir(govde);
    if !sade_oyun.is_empty() && !sade_govde.is_empty() {
        if sade_govde == sade_oyun {
            puan += 600;
        } else if sade_govde.contains(&sade_oyun) || sade_oyun.contains(&sade_govde) {
            puan += 300;
        }
    }

    // Unreal'in "-win64-shipping" son eki oyunun asıl ikilisini işaretler.
    if govde.ends_with("-win64-shipping") || govde.ends_with("-shipping") {
        puan += 250;
    }

    // Başlatıcılar gerçek oyun olabilir (Epic/Ubisoft oyunları) ama önce
    // gerçek ikiliye bakılsın.
    if govde.contains("launcher") || govde.contains("start") {
        puan -= 150;
    }

    // Kökteki dosya, beş klasör içeridekinden daha muhtemel.
    puan -= (derinlik as i64) * 20;

    Some(puan)
}

/// Karşılaştırma için ad sadeleştirme: yalnızca harf ve rakam, küçük harf.
///
/// "NTE: Neverness to Everness" ile "NTEGlobal" arasındaki ilişkiyi ancak
/// noktalama ve boşluk atıldıktan sonra görebiliyoruz.
fn sadelestir(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn gurultu_eleniyor() {
        for ad in [
            "unins000.exe",
            "vcredist_x64.exe",
            "easyanticheat_setup.exe",
            "crashreportclient.exe",
            "ue4prereqsetup_x64.exe",
            "steamerrorreporter.exe",
        ] {
            assert!(
                puanla(ad, 50 * 1024 * 1024, 0, "Bir Oyun").is_none(),
                "{ad} elenmeliydi"
            );
        }
    }

    #[test]
    fn adi_eslesen_kucuk_dosya_buyuk_yardimciyi_geciyor() {
        let oyun = puanla("eldenring.exe", 60 * 1024 * 1024, 1, "ELDEN RING").unwrap();
        let yardimci = puanla(
            "start_protected_game.exe",
            380 * 1024 * 1024,
            1,
            "ELDEN RING",
        )
        .unwrap();
        assert!(
            oyun > yardimci,
            "adı eşleşen exe önde olmalı: {oyun} vs {yardimci}"
        );
    }

    #[test]
    fn shipping_soneki_odullendiriliyor() {
        let shipping = puanla(
            "factorygame-win64-shipping.exe",
            200 * 1024 * 1024,
            3,
            "Satisfactory",
        )
        .unwrap();
        let duz = puanla("digeri.exe", 200 * 1024 * 1024, 3, "Satisfactory").unwrap();
        assert!(shipping > duz);
    }

    #[test]
    fn kokteki_dosya_derindekinden_onde() {
        let sig = puanla("oyun.exe", 100 * 1024 * 1024, 0, "Baska").unwrap();
        let derin = puanla("oyun.exe", 100 * 1024 * 1024, 4, "Baska").unwrap();
        assert!(sig > derin);
    }

    #[test]
    fn baslatici_geride_ama_elenmiyor() {
        let baslatici = puanla("ntegloballauncher.exe", 20 * 1024 * 1024, 1, "NTE").unwrap();
        let oyun = puanla("nteglobal.exe", 20 * 1024 * 1024, 1, "NTE").unwrap();
        assert!(oyun > baslatici);
    }

    #[test]
    fn exe_olmayan_hicbir_sey_puanlanmiyor() {
        // Yürüyüş zaten süzüyor; burada gövdesi boş kalan uç durum korunuyor.
        assert!(puanla(".exe", 1024, 0, "Oyun").is_none());
    }

    #[test]
    fn sadelestirme_noktalama_atiyor() {
        assert_eq!(
            sadelestir("NTE: Neverness to Everness"),
            "ntenevernesstoeverness"
        );
        assert_eq!(sadelestir("Counter-Strike 2"), "counterstrike2");
    }

    #[test]
    fn olmayan_klasor_bos_liste_donduruyor() {
        let sonuc = adaylar(Path::new("Z:\\boyle-bir-klasor-yok-42"), "Yok");
        assert!(sonuc.is_empty());
    }
}
