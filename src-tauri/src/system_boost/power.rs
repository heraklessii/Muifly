//! Güç planı: oku, değiştir, geri yükle.
//!
//! Değişiklik `PowerSetActiveScheme` ile yapılıyor, `powercfg.exe` kabuk
//! çağrısıyla değil. Sebep: harici süreç her çağrıda bir konsol penceresi
//! parlatıyor ve çıktısı yerelleştirilmiş metin — ayrıştırmak kırılgan.
//!
//! **Yeni güç planı OLUŞTURULMUYOR.** "Ultimate Performance" planı Windows'un
//! bazı sürümlerinde gizli ve `powercfg -duplicatescheme` ile üretiliyor; bu
//! üretim sistemde kalıcı bir plan bırakıyor ve tam olarak geri alınması
//! (planı silmek) kullanıcının kendi oluşturduğu bir planı silme riskini
//! taşıyor. Plan yoksa yüksek performansa düşülüyor ve sebep günlüğe yazılıyor
//! (tasarım ilkesi 1 ve 2).

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::ledger::Undo;

/// Bilinen Windows güç planları.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GucPlani {
    GucTasarrufu,
    Dengeli,
    YuksekPerformans,
    /// Windows'un her kurulumunda görünmüyor. Yoksa yüksek performansa düşülür.
    UstunPerformans,
}

impl GucPlani {
    pub fn guid_metni(self) -> &'static str {
        match self {
            GucPlani::GucTasarrufu => "a1841308-3541-4fab-bc81-f71556f20b4a",
            GucPlani::Dengeli => "381b4222-f694-41f0-9685-ff5bb260df2e",
            GucPlani::YuksekPerformans => "8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c",
            GucPlani::UstunPerformans => "e9a42b02-d5df-448d-aa00-03f14749eb61",
        }
    }

    pub fn ad(self) -> &'static str {
        match self {
            GucPlani::GucTasarrufu => "Güç tasarrufu",
            GucPlani::Dengeli => "Dengeli",
            GucPlani::YuksekPerformans => "Yüksek performans",
            GucPlani::UstunPerformans => "Üstün performans",
        }
    }

    pub fn ayristir(metin: &str) -> Result<Self> {
        match metin.to_lowercase().as_str() {
            "power_saver" | "guc_tasarrufu" => Ok(GucPlani::GucTasarrufu),
            "balanced" | "dengeli" => Ok(GucPlani::Dengeli),
            "high_performance" | "yuksek_performans" => Ok(GucPlani::YuksekPerformans),
            "ultimate_performance" | "ustun_performans" => Ok(GucPlani::UstunPerformans),
            diger => Err(Error::ProfileInvalid(format!(
                "bilinmeyen güç planı: {diger}"
            ))),
        }
    }

    pub fn tumu() -> [GucPlani; 4] {
        [
            GucPlani::GucTasarrufu,
            GucPlani::Dengeli,
            GucPlani::YuksekPerformans,
            GucPlani::UstunPerformans,
        ]
    }
}

/// Metin GUID'i 16 bayta çevirir.
///
/// Kendi ayrıştırıcımız var çünkü GUID'ler bu dosyada sabit metin olarak
/// duruyor ve deftere de metin olarak yazılıyor — çökme sonrası okunan bir
/// defterdeki GUID'i geri çevirebilmek gerekiyor.
pub fn guid_ayristir(metin: &str) -> Result<[u8; 16]> {
    let temiz: String = metin
        .chars()
        .filter(|c| *c != '-' && *c != '{' && *c != '}')
        .collect();
    if temiz.len() != 32 {
        return Err(Error::ProfileInvalid(format!("geçersiz GUID: {metin}")));
    }
    let mut bayt = [0u8; 16];
    for i in 0..16 {
        bayt[i] = u8::from_str_radix(&temiz[i * 2..i * 2 + 2], 16)
            .map_err(|_| Error::ProfileInvalid(format!("geçersiz GUID: {metin}")))?;
    }
    Ok(bayt)
}

/// 16 baytı standart GUID metnine çevirir.
pub fn guid_metne(bayt: &[u8; 16]) -> String {
    let h = |b: &[u8]| -> String { b.iter().map(|x| format!("{x:02x}")).collect() };
    format!(
        "{}-{}-{}-{}-{}",
        h(&bayt[0..4]),
        h(&bayt[4..6]),
        h(&bayt[6..8]),
        h(&bayt[8..10]),
        h(&bayt[10..16])
    )
}

#[cfg(windows)]
mod win {
    use super::*;

    use windows::core::GUID;
    use windows::Win32::Foundation::{LocalFree, ERROR_SUCCESS, HLOCAL};
    use windows::Win32::System::Power::{
        PowerGetActiveScheme, PowerReadFriendlyName, PowerSetActiveScheme,
    };

    fn guid_yap(metin: &str) -> Result<GUID> {
        let b = guid_ayristir(metin)?;
        Ok(GUID::from_values(
            u32::from_be_bytes([b[0], b[1], b[2], b[3]]),
            u16::from_be_bytes([b[4], b[5]]),
            u16::from_be_bytes([b[6], b[7]]),
            [b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]],
        ))
    }

    fn guid_metne_win(g: &GUID) -> String {
        let mut b = [0u8; 16];
        b[0..4].copy_from_slice(&g.data1.to_be_bytes());
        b[4..6].copy_from_slice(&g.data2.to_be_bytes());
        b[6..8].copy_from_slice(&g.data3.to_be_bytes());
        b[8..16].copy_from_slice(&g.data4);
        guid_metne(&b)
    }

    /// Aktif planın GUID'i ve adı.
    pub fn aktif_plan() -> Result<(String, String)> {
        let mut isaretci: *mut GUID = std::ptr::null_mut();
        // SAFETY: API kendi ayırdığı belleğin işaretçisini yazıyor; altta
        // `LocalFree` ile bırakılıyor.
        let kod = unsafe { PowerGetActiveScheme(None, &mut isaretci) };
        if kod != ERROR_SUCCESS || isaretci.is_null() {
            return Err(Error::Windows {
                islem: "aktif güç planı okuma",
                kod: format!("hata {}", kod.0),
            });
        }

        // SAFETY: işaretçi az önce doğrulandı; kopya alınıp hemen serbest
        // bırakılıyor, sonrasında kullanılmıyor.
        let guid = unsafe { *isaretci };
        // SAFETY: bellek `PowerGetActiveScheme` tarafından LocalAlloc ile
        // ayrıldı; dokümante edilen serbest bırakma yolu bu.
        unsafe {
            let _ = LocalFree(Some(HLOCAL(isaretci as *mut _)));
        };

        let metin = guid_metne_win(&guid);
        Ok((metin.clone(), plan_adi(&guid).unwrap_or(metin)))
    }

    /// Planın Windows'taki görünen adı.
    ///
    /// Kullanıcının kendi oluşturduğu planlar da var; sabit bir eşleme yetmez.
    fn plan_adi(guid: &GUID) -> Option<String> {
        let mut boy = 0u32;
        // İlk çağrı boyut için.
        // SAFETY: tampon `None`, yalnızca boyut isteniyor.
        let _ = unsafe { PowerReadFriendlyName(None, Some(guid), None, None, None, &mut boy) };
        if boy == 0 {
            return None;
        }
        let mut tampon = vec![0u8; boy as usize];
        // SAFETY: tampon `boy` kadar ayrıldı.
        let kod = unsafe {
            PowerReadFriendlyName(
                None,
                Some(guid),
                None,
                None,
                Some(tampon.as_mut_ptr()),
                &mut boy,
            )
        };
        if kod != ERROR_SUCCESS {
            return None;
        }
        // Çıktı UTF-16, sonda NUL var.
        let genis: Vec<u16> = tampon
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|c| *c != 0)
            .collect();
        Some(String::from_utf16_lossy(&genis))
    }

    /// Plan sistemde var mı? ("Üstün performans" her kurulumda yok.)
    pub fn plan_mevcut(plan: GucPlani) -> bool {
        match guid_yap(plan.guid_metni()) {
            Ok(g) => plan_adi(&g).is_some(),
            Err(_) => false,
        }
    }

    /// Planı uygular ve geri alma kaydını döner.
    ///
    /// Önce mevcut plan okunuyor: okuma başarısız olursa değişiklik hiç
    /// yapılmıyor. Geri alınamayacak bir değişikliği yapmaktansa hiç
    /// yapmamak doğru (tasarım ilkesi 1).
    pub fn plani_uygula(plan: GucPlani) -> Result<(Undo, GucPlani)> {
        let (onceki_guid, onceki_ad) = aktif_plan()?;

        // İstenen plan yoksa bir alt basamağa düşülüyor. Sessizce değil:
        // çağıran hangi planın gerçekten uygulandığını geri alıyor ve günlüğe
        // yazıyor.
        let uygulanan = if plan == GucPlani::UstunPerformans && !plan_mevcut(plan) {
            GucPlani::YuksekPerformans
        } else {
            plan
        };

        let guid = guid_yap(uygulanan.guid_metni())?;
        // SAFETY: GUID yerel ve geçerli; API yalnızca okuyor.
        let kod = unsafe { PowerSetActiveScheme(None, Some(&guid)) };
        if kod != ERROR_SUCCESS {
            return Err(Error::Windows {
                islem: "güç planı değiştirme",
                kod: format!("hata {}", kod.0),
            });
        }

        Ok((
            Undo::GucPlani {
                onceki_guid,
                onceki_ad,
            },
            uygulanan,
        ))
    }

    /// Ham GUID metniyle geri yükleme — yalnızca geri alma yolu kullanıyor.
    pub fn guid_ile_uygula(guid_metni: &str) -> Result<()> {
        let guid = guid_yap(guid_metni)?;
        // SAFETY: yukarıdakiyle aynı sözleşme.
        let kod = unsafe { PowerSetActiveScheme(None, Some(&guid)) };
        if kod != ERROR_SUCCESS {
            return Err(Error::Windows {
                islem: "güç planı geri yükleme",
                kod: format!("hata {}", kod.0),
            });
        }
        Ok(())
    }
}

#[cfg(windows)]
pub use win::{aktif_plan, guid_ile_uygula, plan_mevcut, plani_uygula};

#[cfg(not(windows))]
pub fn aktif_plan() -> Result<(String, String)> {
    Err(Error::Unsupported("güç planı okuma"))
}

#[cfg(not(windows))]
pub fn plan_mevcut(_plan: GucPlani) -> bool {
    false
}

#[cfg(not(windows))]
pub fn plani_uygula(_plan: GucPlani) -> Result<(Undo, GucPlani)> {
    Err(Error::Unsupported("güç planı değiştirme"))
}

#[cfg(not(windows))]
pub fn guid_ile_uygula(_guid: &str) -> Result<()> {
    Err(Error::Unsupported("güç planı geri yükleme"))
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn bilinen_guidler_ayristirilabiliyor() {
        for plan in GucPlani::tumu() {
            let b = guid_ayristir(plan.guid_metni())
                .unwrap_or_else(|_| panic!("{} GUID'i bozuk", plan.ad()));
            assert_eq!(guid_metne(&b), plan.guid_metni());
        }
    }

    #[test]
    fn guid_gidis_donusu() {
        let m = "381b4222-f694-41f0-9685-ff5bb260df2e";
        assert_eq!(guid_metne(&guid_ayristir(m).unwrap()), m);
    }

    #[test]
    fn suslu_parantezli_guid_kabul_ediliyor() {
        // Windows bazı yerlerde `{...}` biçimini veriyor; defterden okunan
        // değerin bu biçimde olması geri almayı bozmamalı.
        let a = guid_ayristir("{381b4222-f694-41f0-9685-ff5bb260df2e}").unwrap();
        let b = guid_ayristir("381b4222-f694-41f0-9685-ff5bb260df2e").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn bozuk_guid_reddediliyor() {
        assert!(guid_ayristir("").is_err());
        assert!(guid_ayristir("381b4222").is_err());
        assert!(guid_ayristir("zzzzzzzz-f694-41f0-9685-ff5bb260df2e").is_err());
    }

    #[test]
    fn plan_metinleri_ayristiriliyor() {
        assert_eq!(
            GucPlani::ayristir("ultimate_performance").unwrap(),
            GucPlani::UstunPerformans
        );
        assert_eq!(
            GucPlani::ayristir("high_performance").unwrap(),
            GucPlani::YuksekPerformans
        );
        assert!(GucPlani::ayristir("turbo").is_err());
    }

    #[test]
    fn planlarin_guidleri_benzersiz() {
        let mut gorulen = std::collections::HashSet::new();
        for plan in GucPlani::tumu() {
            assert!(
                gorulen.insert(plan.guid_metni()),
                "{} GUID'i başka bir planla aynı",
                plan.ad()
            );
        }
    }
}
