//! Süreç önceliği ve CPU affinitesi.
//!
//! `REALTIME_PRIORITY_CLASS` bu dosyada **temsil edilemiyor**: `Oncelik`
//! enum'ında böyle bir varyant yok. Bu bir unutkanlık değil, `docs/RISKS.md`
//! içindeki kuralın tip seviyesinde uygulanması — gerçek zamanlı öncelik,
//! fare/klavye sürücüleri dahil sistem servislerini aç bırakıp makineyi
//! kilitleyebiliyor. Bir profil dosyası `"realtime"` yazsa bile
//! ayrıştırılırken reddediliyor.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::ledger::Undo;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Oncelik {
    Dusuk,
    NormalinAlti,
    Normal,
    NormalinUstu,
    /// Oyunlar için kullanılan en yüksek sınıf. Bunun üstü yok.
    Yuksek,
}

impl Oncelik {
    pub fn ham(self) -> u32 {
        match self {
            Oncelik::Dusuk => 0x0000_0040,
            Oncelik::NormalinAlti => 0x0000_4000,
            Oncelik::Normal => 0x0000_0020,
            Oncelik::NormalinUstu => 0x0000_8000,
            Oncelik::Yuksek => 0x0000_0080,
        }
    }

    /// Profil dosyasındaki metni önceliğe çevirir.
    ///
    /// `"realtime"` / `"gerçek zamanlı"` bilinçli olarak TANINMIYOR ve hata
    /// dönüyor — sessizce "yüksek"e düşürmek, kullanıcının profilinde yazanla
    /// programın yaptığı arasında fark yaratırdı (şeffaflık ilkesi).
    pub fn ayristir(metin: &str) -> Result<Self> {
        match metin.to_lowercase().as_str() {
            "low" | "dusuk" | "düşük" => Ok(Oncelik::Dusuk),
            "below_normal" | "normalin_alti" => Ok(Oncelik::NormalinAlti),
            "normal" => Ok(Oncelik::Normal),
            "above_normal" | "normalin_ustu" => Ok(Oncelik::NormalinUstu),
            "high" | "yuksek" | "yüksek" => Ok(Oncelik::Yuksek),
            "realtime" | "gercek_zamanli" => Err(Error::ProfileInvalid(
                "gerçek zamanlı öncelik desteklenmiyor: sistemi kilitleyebilir".into(),
            )),
            diger => Err(Error::ProfileInvalid(format!(
                "bilinmeyen öncelik sınıfı: {diger}"
            ))),
        }
    }
}

/// Sistemdeki mantıksal çekirdek sayısı ve verim sınıfları.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuTopolojisi {
    pub mantiksal_cekirdek: u32,
    /// Hibrit CPU (Intel 12. nesil ve sonrası gibi) tespit edildi mi?
    pub hibrit: bool,
    /// Performans çekirdeklerinin (P-core) maskesi. Hibrit değilse `None`.
    ///
    /// `None` dönmesi önemli: hibrit olmayan bir CPU'da "P-core'a sabitle"
    /// seçeneği anlamsız ve arayüzde gösterilmiyor.
    pub p_core_maskesi: Option<u64>,
}

/// Okunan çekirdek kayıtlarından topolojiyi çıkarır.
///
/// Girdi: her fiziksel çekirdek için `(verim sınıfı, işlemci grubu, maske)`.
/// Windows API'sinden ayrı tutuldu ki kural — özellikle çok gruplu makinedeki
/// davranış — testle korunabilsin.
///
/// **Çok gruplu makinede P-core maskesi verilmiyor.** 64'ten fazla mantıksal
/// çekirdeği olan sistemlerde çekirdekler işlemci gruplarına bölünüyor ve
/// farklı grupların maskeleri aynı bitleri kullanıyor; hepsini VEYA'lamak
/// başka bir grubun çekirdeğini işaret eden anlamsız bir maske üretirdi.
/// `SetProcessAffinityMask` yalnızca sürecin kendi grubunu adresleyebiliyor,
/// dolayısıyla doğru cevap "maske yok" (`docs/RISKS.md`: yanlış affinite
/// performansı düşürür).
pub fn topolojiyi_hesapla(cekirdekler: &[(u8, u16, u64)]) -> CpuTopolojisi {
    let mantiksal = cekirdekler.iter().map(|(_, _, m)| m.count_ones()).sum();

    let sinif_sayisi: std::collections::BTreeSet<u8> =
        cekirdekler.iter().map(|(s, _, _)| *s).collect();
    let hibrit = sinif_sayisi.len() > 1;

    let tek_grup = cekirdekler.iter().map(|(_, g, _)| *g).all(|g| g == 0);

    let p_core_maskesi = match (hibrit, tek_grup, sinif_sayisi.iter().next_back()) {
        (true, true, Some(&en_yuksek)) => Some(
            cekirdekler
                .iter()
                .filter(|(s, _, _)| *s == en_yuksek)
                .fold(0u64, |a, (_, _, m)| a | m),
        ),
        _ => None,
    };

    CpuTopolojisi {
        mantiksal_cekirdek: mantiksal,
        hibrit,
        p_core_maskesi,
    }
}

// ---------------------------------------------------------------------------
// Windows tarafı
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod win {
    use super::*;

    use windows::Win32::System::SystemInformation::{
        GetLogicalProcessorInformationEx, RelationProcessorCore,
        SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
    };
    use windows::Win32::System::Threading::{
        GetPriorityClass, OpenProcess, SetPriorityClass, SetProcessAffinityMask,
        PROCESS_CREATION_FLAGS, PROCESS_QUERY_INFORMATION, PROCESS_SET_INFORMATION,
    };

    use crate::system_boost::detect::surec_adi;
    use crate::winutil::Tanitici;

    fn yazma_icin_ac(pid: u32) -> Result<Tanitici> {
        // SAFETY: sabit bayraklar; sahiplik `Tanitici`ye geçiyor.
        let h = unsafe {
            OpenProcess(
                PROCESS_QUERY_INFORMATION | PROCESS_SET_INFORMATION,
                false,
                pid,
            )
        }
        .map_err(|e| {
            // Yükseltilmiş bir sürecin önceliği, yükseltilmemiş bir programdan
            // değiştirilemiyor. Kullanıcıya "bozuk" değil "yetki gerekiyor"
            // demek gerekiyor.
            match crate::error::win("süreç açma", e) {
                Error::NeedsElevation(_) => Error::NeedsElevation("süreç önceliğini değiştirme"),
                diger => diger,
            }
        })?;
        Ok(Tanitici(h))
    }

    pub fn oncelik_oku(pid: u32) -> Result<u32> {
        let t = yazma_icin_ac(pid)?;
        // SAFETY: geçerli tanıtıcı.
        let sinif = unsafe { GetPriorityClass(t.0) };
        if sinif == 0 {
            return Err(Error::ProcessNotFound(pid));
        }
        Ok(sinif)
    }

    /// Önceliği ayarlar ve geri alma kaydını döner.
    ///
    /// Sıra kritik: ÖNCE eski değer okunuyor, sonra yenisi yazılıyor. Ters
    /// sıra, yazma başarılı olup okuma başarısız olduğunda geri alınamayan
    /// bir değişiklik bırakırdı.
    pub fn oncelik_ayarla(pid: u32, yeni: Oncelik) -> Result<Undo> {
        let onceki = oncelik_oku(pid)?;
        let ad = surec_adi(pid).unwrap_or_else(|_| format!("pid {pid}"));

        let t = yazma_icin_ac(pid)?;
        // SAFETY: geçerli tanıtıcı, `Oncelik::ham` yalnızca dokümante edilmiş
        // sınıf sabitlerini üretiyor (realtime üretemiyor).
        unsafe { SetPriorityClass(t.0, PROCESS_CREATION_FLAGS(yeni.ham())) }
            .map_err(|e| crate::error::win("öncelik ayarlama", e))?;

        Ok(Undo::SurecOnceligi {
            pid,
            surec: ad,
            onceki,
        })
    }

    /// Ham sınıf değeriyle geri yazma — yalnızca geri alma yolu kullanıyor.
    pub fn oncelik_ham_yaz(pid: u32, sinif: u32) -> Result<()> {
        let t = yazma_icin_ac(pid)?;
        // SAFETY: değer defterden geliyor ve oraya yalnızca `GetPriorityClass`
        // çıktısı yazılıyor.
        unsafe { SetPriorityClass(t.0, PROCESS_CREATION_FLAGS(sinif)) }
            .map_err(|e| crate::error::win("öncelik geri yükleme", e))
    }

    pub fn affinite_ayarla(pid: u32, maske: u64) -> Result<Undo> {
        if maske == 0 {
            return Err(Error::ProfileInvalid(
                "boş affinite maskesi: süreç hiçbir çekirdekte çalışamaz".into(),
            ));
        }
        let onceki = affinite_oku(pid)?;
        let ad = surec_adi(pid).unwrap_or_else(|_| format!("pid {pid}"));

        let t = yazma_icin_ac(pid)?;
        // SAFETY: geçerli tanıtıcı; maskenin sıfır olmadığı yukarıda kontrol
        // edildi (sıfır maske API tarafından reddedilir ama hata mesajı
        // kullanıcıya bir şey anlatmaz).
        unsafe { SetProcessAffinityMask(t.0, maske as usize) }
            .map_err(|e| crate::error::win("affinite ayarlama", e))?;

        Ok(Undo::SurecAffinite {
            pid,
            surec: ad,
            onceki,
        })
    }

    pub fn affinite_oku(pid: u32) -> Result<u64> {
        use windows::Win32::System::Threading::GetProcessAffinityMask;

        let t = yazma_icin_ac(pid)?;
        let mut surec = 0usize;
        let mut sistem = 0usize;
        // SAFETY: iki çıktı da yerel ve geçerli.
        unsafe { GetProcessAffinityMask(t.0, &mut surec, &mut sistem) }
            .map_err(|e| crate::error::win("affinite okuma", e))?;
        Ok(surec as u64)
    }

    pub fn affinite_ham_yaz(pid: u32, maske: u64) -> Result<()> {
        let t = yazma_icin_ac(pid)?;
        // SAFETY: değer defterden geliyor.
        unsafe { SetProcessAffinityMask(t.0, maske as usize) }
            .map_err(|e| crate::error::win("affinite geri yükleme", e))
    }

    /// CPU topolojisini okur ve hibrit ise P-core maskesini çıkarır.
    ///
    /// Yöntem: `GetLogicalProcessorInformationEx(RelationProcessorCore)` her
    /// fiziksel çekirdek için bir kayıt veriyor ve `EfficiencyClass` alanında
    /// verim sınıfını bildiriyor. Birden fazla farklı sınıf varsa CPU hibrit;
    /// en YÜKSEK sınıf performans çekirdeği (Microsoft'un tanımı böyle).
    pub fn topoloji() -> Result<CpuTopolojisi> {
        let mut boy = 0u32;
        // İlk çağrı yalnızca gereken tampon boyutunu öğrenmek için; hata
        // dönmesi bekleniyor (ERROR_INSUFFICIENT_BUFFER).
        // SAFETY: `None` tampon + boyut çıktısı, API'nin dokümante ettiği kullanım.
        let _ = unsafe { GetLogicalProcessorInformationEx(RelationProcessorCore, None, &mut boy) };
        if boy == 0 {
            return Err(Error::Windows {
                islem: "CPU topolojisi",
                kod: "tampon boyutu alınamadı".into(),
            });
        }

        let mut tampon = vec![0u8; boy as usize];
        // SAFETY: tampon `boy` kadar ayrıldı ve hizalama için `u8` vektörü
        // API'nin döndürdüğü değişken uzunluklu kayıtlar için yeterli
        // (kayıtlar `Size` alanıyla kendi uzunluklarını taşıyor).
        unsafe {
            GetLogicalProcessorInformationEx(
                RelationProcessorCore,
                Some(tampon.as_mut_ptr() as *mut SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX),
                &mut boy,
            )
        }
        .map_err(|e| crate::error::win("CPU topolojisi", e))?;

        let mut ofset = 0usize;
        // (verim sınıfı, işlemci grubu, maske) üçlüleri.
        let mut cekirdekler: Vec<(u8, u16, u64)> = Vec::new();

        while ofset + std::mem::size_of::<u32>() * 2 <= boy as usize {
            // SAFETY: `ofset` her adımda kaydın kendi `Size` alanı kadar
            // ilerliyor ve döngü koşulu tampon sınırını aşmıyor.
            let kayit = unsafe {
                &*(tampon.as_ptr().add(ofset) as *const SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX)
            };
            let kayit_boyu = kayit.Size as usize;
            if kayit_boyu == 0 || ofset + kayit_boyu > boy as usize {
                break;
            }

            // SAFETY: `RelationProcessorCore` istendiği için birlik alanı
            // `Processor` olarak geçerli.
            let cekirdek = unsafe { &kayit.Anonymous.Processor };
            let grup_sayisi = (cekirdek.GroupCount as usize).min(cekirdek.GroupMask.len());
            // Her grup ayrı bir kayıt satırı: maskeler gruplar arasında
            // VEYA'lanmıyor (bkz. `topolojiyi_hesapla`).
            for i in 0..grup_sayisi {
                let grup = cekirdek.GroupMask[i];
                cekirdekler.push((cekirdek.EfficiencyClass, grup.Group, grup.Mask as u64));
            }

            ofset += kayit_boyu;
        }

        Ok(topolojiyi_hesapla(&cekirdekler))
    }
}

#[cfg(windows)]
pub use win::{
    affinite_ayarla, affinite_ham_yaz, affinite_oku, oncelik_ayarla, oncelik_ham_yaz, oncelik_oku,
    topoloji,
};

#[cfg(not(windows))]
mod stub {
    use super::*;

    pub fn oncelik_oku(_pid: u32) -> Result<u32> {
        Err(Error::Unsupported("öncelik okuma"))
    }
    pub fn oncelik_ayarla(_pid: u32, _y: Oncelik) -> Result<Undo> {
        Err(Error::Unsupported("öncelik ayarlama"))
    }
    pub fn oncelik_ham_yaz(_pid: u32, _s: u32) -> Result<()> {
        Err(Error::Unsupported("öncelik geri yükleme"))
    }
    pub fn affinite_oku(_pid: u32) -> Result<u64> {
        Err(Error::Unsupported("affinite okuma"))
    }
    pub fn affinite_ayarla(_pid: u32, _m: u64) -> Result<Undo> {
        Err(Error::Unsupported("affinite ayarlama"))
    }
    pub fn affinite_ham_yaz(_pid: u32, _m: u64) -> Result<()> {
        Err(Error::Unsupported("affinite geri yükleme"))
    }
    pub fn topoloji() -> Result<CpuTopolojisi> {
        Err(Error::Unsupported("CPU topolojisi"))
    }
}

#[cfg(not(windows))]
pub use stub::{
    affinite_ayarla, affinite_ham_yaz, affinite_oku, oncelik_ayarla, oncelik_ham_yaz, oncelik_oku,
    topoloji,
};

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn gercek_zamanli_oncelik_ayristirilamiyor() {
        // Bu test ürünün güvenlik sınırı: geçerse, bir profil dosyası sistemi
        // kilitleyebilecek bir öncelik isteyebiliyor demektir.
        assert!(Oncelik::ayristir("realtime").is_err());
        assert!(Oncelik::ayristir("gercek_zamanli").is_err());
    }

    #[test]
    fn hibrit_olmayan_cpuda_p_core_maskesi_yok() {
        // Tek verim sınıfı: "P-core'a sabitle" anlamsız, maske verilmiyor.
        let t = topolojiyi_hesapla(&[(0, 0, 0b0011), (0, 0, 0b1100)]);
        assert_eq!(t.mantiksal_cekirdek, 4);
        assert!(!t.hibrit);
        assert_eq!(t.p_core_maskesi, None);
    }

    #[test]
    fn hibrit_cpuda_en_yuksek_sinif_p_core() {
        // İki P-core (sınıf 1, SMT'li) + iki E-core (sınıf 0).
        let t = topolojiyi_hesapla(&[
            (1, 0, 0b0000_0011),
            (1, 0, 0b0000_1100),
            (0, 0, 0b0001_0000),
            (0, 0, 0b0010_0000),
        ]);
        assert!(t.hibrit);
        assert_eq!(t.mantiksal_cekirdek, 6);
        assert_eq!(t.p_core_maskesi, Some(0b0000_1111));
    }

    #[test]
    fn cok_gruplu_makinede_p_core_maskesi_verilmiyor() {
        // 64'ten fazla çekirdekli makine: grup 0 ve grup 1'in maskeleri aynı
        // bitleri kullanıyor. Bunları birleştiren maske yanlış çekirdeği
        // işaret ederdi; doğru cevap "maske yok".
        let t = topolojiyi_hesapla(&[(1, 0, 0b0011), (1, 1, 0b0011), (0, 1, 0b1100)]);
        assert!(t.hibrit);
        assert_eq!(t.p_core_maskesi, None);
        // Çekirdek sayımı yine de doğru: sayım gruplardan bağımsız.
        assert_eq!(t.mantiksal_cekirdek, 6);
    }

    #[test]
    fn bos_topoloji_cokmuyor() {
        let t = topolojiyi_hesapla(&[]);
        assert_eq!(t.mantiksal_cekirdek, 0);
        assert!(!t.hibrit);
        assert_eq!(t.p_core_maskesi, None);
    }

    #[test]
    fn tanidik_siniflar_ayristiriliyor() {
        assert_eq!(Oncelik::ayristir("high").unwrap(), Oncelik::Yuksek);
        assert_eq!(Oncelik::ayristir("YÜKSEK").unwrap(), Oncelik::Yuksek);
        assert_eq!(Oncelik::ayristir("normal").unwrap(), Oncelik::Normal);
        assert_eq!(Oncelik::ayristir("low").unwrap(), Oncelik::Dusuk);
    }

    #[test]
    fn bilinmeyen_sinif_hata() {
        assert!(Oncelik::ayristir("çok yüksek").is_err());
        assert!(Oncelik::ayristir("").is_err());
    }

    #[test]
    fn ham_degerler_windows_sabitleriyle_ayni() {
        // Değerler `winnt.h`den; yanlış yazılırsa süreç beklenmedik bir
        // sınıfa girer ve hata vermez — bu yüzden sabitleniyorlar.
        assert_eq!(Oncelik::Yuksek.ham(), 0x0000_0080);
        assert_eq!(Oncelik::Normal.ham(), 0x0000_0020);
        assert_eq!(Oncelik::Dusuk.ham(), 0x0000_0040);
        assert_eq!(Oncelik::NormalinUstu.ham(), 0x0000_8000);
        assert_eq!(Oncelik::NormalinAlti.ham(), 0x0000_4000);
    }

    #[test]
    fn hicbir_sinif_realtime_degerini_uretmiyor() {
        const REALTIME: u32 = 0x0000_0100;
        for o in [
            Oncelik::Dusuk,
            Oncelik::NormalinAlti,
            Oncelik::Normal,
            Oncelik::NormalinUstu,
            Oncelik::Yuksek,
        ] {
            assert_ne!(o.ham(), REALTIME);
        }
    }
}
