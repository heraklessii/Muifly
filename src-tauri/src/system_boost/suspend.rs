//! Arka plan süreçlerini **dondurma** (kapatma değil).
//!
//! Kapatmak yerine dondurmak, tasarım ilkesi 1'in en görünür uygulaması:
//! Discord dondurulduğunda sohbet penceresi kapanmıyor, sesli görüşme
//! düşmüyor — süreç sadece CPU almayı bırakıyor ve devam ettirildiğinde
//! kaldığı yerden çalışıyor.
//!
//! ## Bilinen risk: `NtSuspendProcess` resmi olarak dokümante edilmemiş
//!
//! Fonksiyon `ntdll.dll` içinde ve yıllardır kararlı, ama Microsoft'un resmi
//! belgelerinde yok (`docs/RISKS.md`). Bu yüzden:
//!
//! - Çağrı bu tek dosyada izole; Windows davranışı değişirse tek nokta güncellenir.
//! - Fonksiyon çalışma anında aranıyor, bulunamazsa program çalışmaya devam
//!   ediyor ve dondurma özelliği kapanıyor — bir Windows güncellemesi ürünü
//!   açılmaz hale getirmemeli.
//! - Alternatif yok: `SuspendThread` ile bütün thread'leri tek tek dondurmak
//!   yarış koşulu üretiyor (dondurma sırasında yeni thread açılabilir).

use crate::error::{Error, Result};
use crate::ledger::Undo;
use crate::system_boost::detect::dondurulabilir;

/// Dondurma denemesinin sonucu.
///
/// Kısmi başarı normal: bir profil on süreç listeler, üçü zaten kapalıdır,
/// biri yükseltilmiş olduğu için erişilemez. Bunların hiçbiri hata değil ve
/// işlemi durdurmamalı — ama kullanıcıya görünmeli.
#[derive(Debug, Default)]
pub struct DondurmaSonucu {
    pub basarili: Vec<(u32, String, Undo)>,
    /// (süreç adı, sebep)
    pub atlanan: Vec<(String, String)>,
}

/// Bir süreci dondurmadan önceki güvenlik kontrolü.
///
/// `oyun_pid`: o an korunan oyun süreci. Oyunun kendisini dondurmak, aracın
/// yapabileceği en kötü hata olurdu; bu yüzden ayrı bir parametre olarak
/// zorunlu tutuluyor, çağıranın unutması mümkün değil.
pub fn dondurma_engeli(pid: u32, ad: &str, oyun_pid: Option<u32>) -> Option<String> {
    if Some(pid) == oyun_pid {
        return Some("oyunun kendisi".into());
    }
    if pid <= 4 {
        // 0 = System Idle, 4 = System.
        return Some("çekirdek sistem süreci".into());
    }
    if !dondurulabilir(ad) {
        return Some("Windows sistem süreci".into());
    }
    if pid == std::process::id() {
        return Some("Muifly'ın kendisi".into());
    }
    None
}

#[cfg(windows)]
mod win {
    use super::*;

    use std::sync::OnceLock;

    use windows::core::s;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SUSPEND_RESUME};

    use crate::system_boost::detect::surec_adi;
    use crate::winutil::Tanitici;

    type NtSurecFn = unsafe extern "system" fn(HANDLE) -> i32;

    struct NtdllGirisleri {
        dondur: Option<NtSurecFn>,
        devam: Option<NtSurecFn>,
    }

    fn girisler() -> &'static NtdllGirisleri {
        static GIRISLER: OnceLock<NtdllGirisleri> = OnceLock::new();
        GIRISLER.get_or_init(|| {
            // SAFETY: ntdll her süreçte zaten yüklü; `GetModuleHandleA` yeni
            // bir kütüphane yüklemiyor, var olanın tanıtıcısını veriyor —
            // bu yüzden tanıtıcı serbest bırakılmıyor (ve bırakılmamalı).
            let modul = match unsafe { GetModuleHandleA(s!("ntdll.dll")) } {
                Ok(m) => m,
                Err(_) => {
                    return NtdllGirisleri {
                        dondur: None,
                        devam: None,
                    }
                }
            };

            // SAFETY: adresler ntdll'den geliyor ve imza `NTSTATUS(HANDLE)`
            // olarak biliniyor. Bulunamazsa `None` kalıyor ve çağrılmıyor.
            let dondur = unsafe { GetProcAddress(modul, s!("NtSuspendProcess")) }
                .map(|p| unsafe { std::mem::transmute::<_, NtSurecFn>(p) });
            let devam = unsafe { GetProcAddress(modul, s!("NtResumeProcess")) }
                .map(|p| unsafe { std::mem::transmute::<_, NtSurecFn>(p) });

            NtdllGirisleri { dondur, devam }
        })
    }

    /// Dondurma özelliği bu Windows sürümünde kullanılabilir mi?
    ///
    /// Arayüz bunu açılışta soruyor: kullanılamıyorsa ilgili ayarlar gri
    /// gösteriliyor, kullanıcı çalışmayacak bir düğmeye basmıyor.
    pub fn destekleniyor() -> bool {
        let g = girisler();
        g.dondur.is_some() && g.devam.is_some()
    }

    fn dondurma_icin_ac(pid: u32) -> Result<Tanitici> {
        // SAFETY: yalnızca dondurma/devam yetkisi isteniyor — bellek okuma
        // ya da yazma yetkisi ALINMIYOR (tasarım ilkesi 3).
        let h = unsafe { OpenProcess(PROCESS_SUSPEND_RESUME, false, pid) }.map_err(|e| {
            match crate::error::win("süreç açma", e) {
                Error::NeedsElevation(_) => Error::NeedsElevation("arka plan sürecini dondurma"),
                _ => Error::ProcessNotFound(pid),
            }
        })?;
        Ok(Tanitici(h))
    }

    pub fn dondur(pid: u32, oyun_pid: Option<u32>) -> Result<Undo> {
        let ad = surec_adi(pid)?;
        if let Some(sebep) = dondurma_engeli(pid, &ad, oyun_pid) {
            return Err(Error::ProfileInvalid(format!("{ad} dondurulamaz: {sebep}")));
        }

        let f = girisler()
            .dondur
            .ok_or(Error::Unsupported("dondurma (NtSuspendProcess bulunamadı)"))?;

        let t = dondurma_icin_ac(pid)?;
        // SAFETY: tanıtıcı geçerli ve `PROCESS_SUSPEND_RESUME` yetkisiyle
        // açıldı; fonksiyon imzası ntdll'in bilinen imzası.
        let durum = unsafe { f(t.0) };
        if durum < 0 {
            return Err(Error::Windows {
                islem: "süreç dondurma",
                kod: format!("NTSTATUS 0x{durum:08x}"),
            });
        }

        Ok(Undo::SurecDonduruldu { pid, surec: ad })
    }

    /// Dondurulmuş süreci devam ettirir.
    ///
    /// `beklenen_ad` PID yeniden kullanımına karşı: dondurulan süreç kapanmış
    /// ve Windows aynı PID'i başka bir sürece vermiş olabilir. O sürece
    /// dokunmak — özellikle çökme sonrası açılışta defter işlenirken — kabul
    /// edilemez. Ad tutmuyorsa işlem yapılmıyor ve durum bildiriliyor.
    pub fn devam_ettir(pid: u32, beklenen_ad: &str) -> Result<()> {
        let simdiki = match surec_adi(pid) {
            Ok(a) => a,
            // Süreç zaten yok: devam ettirilecek bir şey de yok. Bu bir hata
            // değil, defterdeki kayıt temizlenebilir.
            Err(_) => return Ok(()),
        };
        if simdiki != beklenen_ad {
            return Err(Error::ProcessIdentityMismatch {
                pid,
                expected: beklenen_ad.to_string(),
                actual: simdiki,
            });
        }

        let f = girisler()
            .devam
            .ok_or(Error::Unsupported("devam ettirme (NtResumeProcess yok)"))?;

        let t = dondurma_icin_ac(pid)?;
        // SAFETY: dondur() ile aynı sözleşme.
        let durum = unsafe { f(t.0) };
        if durum < 0 {
            return Err(Error::Windows {
                islem: "süreç devam ettirme",
                kod: format!("NTSTATUS 0x{durum:08x}"),
            });
        }
        Ok(())
    }

    /// Bir ad listesindeki tüm süreçleri dondurur.
    ///
    /// Tek bir sürecin başarısız olması diğerlerini durdurmuyor: kısmi sonuç
    /// döndürülüyor ve çağıran hem başarılıları deftere yazıyor hem atlananları
    /// günlüğe. "Ya hep ya hiç" davranışı burada yanlış olurdu — kullanıcının
    /// listesindeki bir uygulama kapalıysa geri kalanı da mı atlanacak?
    pub fn listeyi_dondur(adlar: &[String], oyun_pid: Option<u32>) -> DondurmaSonucu {
        let mut sonuc = DondurmaSonucu::default();

        let surecler = match crate::system_boost::detect::surec_listesi() {
            Ok(s) => s,
            Err(e) => {
                sonuc
                    .atlanan
                    .push(("(tümü)".into(), crate::error::tek_satir(&e)));
                return sonuc;
            }
        };

        let hedefler: std::collections::HashSet<String> =
            adlar.iter().map(|a| a.to_lowercase()).collect();

        for surec in surecler.into_iter().filter(|s| hedefler.contains(&s.ad)) {
            if let Some(sebep) = dondurma_engeli(surec.pid, &surec.ad, oyun_pid) {
                sonuc.atlanan.push((surec.ad, sebep));
                continue;
            }
            match dondur(surec.pid, oyun_pid) {
                Ok(undo) => sonuc.basarili.push((surec.pid, surec.ad, undo)),
                Err(e) => sonuc.atlanan.push((surec.ad, crate::error::tek_satir(&e))),
            }
        }
        sonuc
    }
}

#[cfg(windows)]
pub use win::{destekleniyor, devam_ettir, dondur, listeyi_dondur};

#[cfg(not(windows))]
pub fn destekleniyor() -> bool {
    false
}

#[cfg(not(windows))]
pub fn dondur(_pid: u32, _oyun_pid: Option<u32>) -> Result<Undo> {
    Err(Error::Unsupported("süreç dondurma"))
}

#[cfg(not(windows))]
pub fn devam_ettir(_pid: u32, _ad: &str) -> Result<()> {
    Err(Error::Unsupported("süreç devam ettirme"))
}

#[cfg(not(windows))]
pub fn listeyi_dondur(_adlar: &[String], _oyun_pid: Option<u32>) -> DondurmaSonucu {
    DondurmaSonucu::default()
}

/// Varsayılan dondurma listesi.
///
/// **Boş.** `docs/PROFILES.md`: güvenli varsayılan, kullanıcı kendi listesini
/// kuruyor. Buraya "genelde gereksiz" diye bir liste koymak, kullanıcının
/// istemediği bir şeyi sessizce yapmak olurdu.
pub const VARSAYILAN_DONDURMA_LISTESI: &[&str] = &[];

/// Arayüzün "bunları eklemek ister misin" diye ÖNERDİĞİ adaylar.
///
/// Öneri ile varsayılan arasındaki fark ürünün karakteri: bunlar listede
/// görünüyor, kullanıcı işaretlerse ekleniyor. Kendiliğinden eklenmiyor.
pub const ONERILEN_ADAYLAR: &[&str] = &[
    "discord.exe",
    "spotify.exe",
    "steamwebhelper.exe",
    "epicgameslauncher.exe",
    "onedrive.exe",
    "dropbox.exe",
    "slack.exe",
    "teams.exe",
    "msedge.exe",
    "chrome.exe",
    "firefox.exe",
];

#[cfg(test)]
mod testler {
    use super::*;
    use crate::system_boost::detect::DOKUNULMAZ;

    #[test]
    fn oyunun_kendisi_dondurulamiyor() {
        let engel = dondurma_engeli(1234, "oyun.exe", Some(1234));
        assert!(engel.is_some(), "oyun süreci dondurma listesine giremez");
    }

    #[test]
    fn sistem_pidleri_dondurulamiyor() {
        assert!(dondurma_engeli(0, "system idle process", None).is_some());
        assert!(dondurma_engeli(4, "system", None).is_some());
    }

    #[test]
    fn windows_surecleri_dondurulamiyor() {
        assert!(dondurma_engeli(900, "explorer.exe", None).is_some());
        assert!(dondurma_engeli(901, "dwm.exe", None).is_some());
    }

    #[test]
    fn program_kendini_donduramiyor() {
        assert!(dondurma_engeli(std::process::id(), "test.exe", None).is_some());
    }

    #[test]
    fn siradan_uygulama_gecebiliyor() {
        assert!(dondurma_engeli(5000, "discord.exe", Some(1234)).is_none());
    }

    #[test]
    fn varsayilan_liste_bos() {
        // Bu test bir ürün kararını koruyor: varsayılan olarak hiçbir süreç
        // dondurulmuyor. Listeye bir şey eklenirse test düşer ve kararın
        // bilinçli değiştiğine dair bir tartışma başlar.
        assert!(VARSAYILAN_DONDURMA_LISTESI.is_empty());
    }

    #[test]
    fn onerilen_adaylarin_hicbiri_dokunulmaz_degil() {
        for aday in ONERILEN_ADAYLAR {
            assert!(
                !DOKUNULMAZ.contains(aday),
                "'{aday}' hem öneriliyor hem dokunulmaz — çelişki"
            );
        }
    }

    #[test]
    fn onerilen_adaylar_normalize_edilmis() {
        for aday in ONERILEN_ADAYLAR {
            assert_eq!(*aday, aday.to_lowercase());
        }
    }
}
