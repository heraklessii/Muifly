//! Model dosyalarının indirilmesi — WinHTTP ile.
//!
//! ## Neden yeni bir HTTP kasası eklenmedi
//!
//! `library::png`'deki gerekçenin aynısı: bu programın ağdan indirdiği tek
//! şey çeviri modeli ve bunun için onlarca kasalık bir bağımlılık ağacı
//! (TLS yığını, sertifika deposu, asenkron çalışma zamanı) taşımanın
//! karşılığı yok. WinHTTP işletim sisteminin kendi yığını; sertifika
//! doğrulaması, vekil sunucu ayarları ve yönlendirme takibi zaten onun işi
//! ve kullanıcının Windows'ta yaptığı ağ ayarlarına uyuyor.
//!
//! ## İndirme kullanıcının başlattığı tek ağ trafiği
//!
//! `docs/DISTRIBUTION.md`: program arkada kendiliğinden bir yere bağlanmıyor.
//! Bu modül de öyle — yalnızca kullanıcı "modeli indir" dediğinde çalışıyor,
//! adres kodda sabit ve indirilen her dosya SHA-256 ile doğrulanıyor
//! (`super::model`).
//!
//! ## Yarım dosya bırakmıyor
//!
//! Dosya `.yarim` uzantısıyla iniyor, doğrulandıktan sonra adına
//! kavuşuyor. Aksi halde yarıda kesilen bir indirme, bir sonraki açılışta
//! "model kurulu" görünür ve hata ONNX Runtime'ın içinden, kullanıcıya
//! hiçbir şey söylemeyen bir mesajla çıkardı.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::{Error, Result};

/// Bir indirmenin o anki durumu — arayüze giden ilerleme.
#[derive(Debug, Clone, Copy)]
pub struct Ilerleme {
    pub inen: u64,
    /// Sunucu `Content-Length` vermezse `None`. Arayüz o zaman yüzde değil
    /// yalnızca inen miktarı gösteriyor: uydurma bir yüzde, biten bir
    /// indirmede %60'ta durur ve kullanıcıya yalan söylerdi.
    pub toplam: Option<u64>,
}

/// Adresin parçaları.
///
/// URL çözümlemesi `WinHttpCrackUrl` yerine burada yapılıyor: girdi kodda
/// sabit birkaç adres, çözümleme saf bir fonksiyon ve böylece Windows
/// olmadan da test edilebiliyor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parcalar {
    pub guvenli: bool,
    pub sunucu: String,
    pub port: u16,
    /// Sorgu dizesi dahil, `/` ile başlar.
    pub yol: String,
}

/// `https://sunucu[:port]/yol` biçimini ayrıştırır.
pub fn ayristir(url: &str) -> Option<Parcalar> {
    let (sema, kalan) = url.split_once("://")?;
    let guvenli = match sema.to_ascii_lowercase().as_str() {
        "https" => true,
        "http" => false,
        _ => return None,
    };
    let (yetki, yol) = match kalan.find('/') {
        Some(i) => (&kalan[..i], &kalan[i..]),
        None => (kalan, "/"),
    };
    if yetki.is_empty() {
        return None;
    }
    // Kullanıcı adı/parola taşıyan bir adres beklemiyoruz ve kabul de
    // etmiyoruz: sabit adreslerimizde yok, gelirse bir şey ters demektir.
    if yetki.contains('@') {
        return None;
    }
    let (sunucu, port) = match yetki.rsplit_once(':') {
        Some((s, p)) => (s.to_string(), p.parse::<u16>().ok()?),
        None => (yetki.to_string(), if guvenli { 443 } else { 80 }),
    };
    if sunucu.is_empty() {
        return None;
    }
    Some(Parcalar {
        guvenli,
        sunucu,
        port,
        yol: yol.to_string(),
    })
}

/// İndirme iptal edildiğinde dönen hata.
pub fn iptal_hatasi() -> Error {
    Error::Indirme("indirme durduruldu".into())
}

#[cfg(windows)]
pub use win::indir;

#[cfg(windows)]
mod win {
    use super::*;
    use std::io::Write;
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::Networking::WinHttp::*;

    /// Kapatılması unutulmayan WinHTTP tutamacı.
    ///
    /// `winutil::Handle` ile aynı gerekçe: erken dönüşlerin her birinde
    /// kapatma çağrısı yazmak, birinde unutmak demektir.
    struct Tutamac(*mut core::ffi::c_void);

    impl Drop for Tutamac {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    let _ = WinHttpCloseHandle(self.0);
                }
            }
        }
    }

    /// Okuma tamponu. 256 KB: her turda bir ilerleme bildirimi düşüyor,
    /// yarım gigabaytta ~2000 bildirim — arayüzü boğmadan akıcı bir çubuk.
    const TAMPON: usize = 256 * 1024;

    /// Zaman aşımları (ms). Çözümleme/bağlanma kısa, okuma uzun: yavaş bir
    /// bağlantıda 400 MB'lık dosya saatler sürebilir ve bu bir hata değil.
    const ZAMAN_ASIMI: (i32, i32, i32, i32) = (15_000, 30_000, 30_000, 120_000);

    fn hata(ne: &str) -> Error {
        Error::Indirme(format!(
            "{ne} ({})",
            windows::core::Error::from_thread().message()
        ))
    }

    /// Dosyayı indirir. Hedef dosya ancak indirme bittiğinde oluşuyor.
    ///
    /// `iptal` her tamponda bir okunuyor: kullanıcı vazgeçtiğinde saniyeler
    /// değil, bir tamponluk süre içinde duruyor.
    pub fn indir(
        url: &str,
        hedef: &Path,
        iptal: &AtomicBool,
        mut bildir: impl FnMut(Ilerleme),
    ) -> Result<()> {
        let p = ayristir(url).ok_or_else(|| Error::Indirme(format!("adres anlaşılmadı: {url}")))?;

        unsafe {
            let oturum = Tutamac(WinHttpOpen(
                &HSTRING::from("Muifly"),
                WINHTTP_ACCESS_TYPE_AUTOMATIC_PROXY,
                PCWSTR::null(),
                PCWSTR::null(),
                0,
            ));
            if oturum.0.is_null() {
                return Err(hata("ağ oturumu açılamadı"));
            }
            let _ = WinHttpSetTimeouts(
                oturum.0,
                ZAMAN_ASIMI.0,
                ZAMAN_ASIMI.1,
                ZAMAN_ASIMI.2,
                ZAMAN_ASIMI.3,
            );

            let baglanti = Tutamac(WinHttpConnect(
                oturum.0,
                &HSTRING::from(p.sunucu.as_str()),
                p.port,
                0,
            ));
            if baglanti.0.is_null() {
                return Err(hata("sunucuya bağlanılamadı"));
            }

            let istek = Tutamac(WinHttpOpenRequest(
                baglanti.0,
                &HSTRING::from("GET"),
                &HSTRING::from(p.yol.as_str()),
                PCWSTR::null(),
                PCWSTR::null(),
                std::ptr::null(),
                if p.guvenli {
                    WINHTTP_FLAG_SECURE
                } else {
                    WINHTTP_OPEN_REQUEST_FLAGS(0)
                },
            ));
            if istek.0.is_null() {
                return Err(hata("istek açılamadı"));
            }

            WinHttpSendRequest(istek.0, None, None, 0, 0, 0)
                .map_err(|_| hata("istek gönderilemedi"))?;
            WinHttpReceiveResponse(istek.0, std::ptr::null_mut())
                .map_err(|_| hata("sunucu cevap vermedi"))?;

            let kod = sayisal_baslik(istek.0, WINHTTP_QUERY_STATUS_CODE).unwrap_or(0);
            if kod != 200 {
                return Err(Error::Indirme(format!(
                    "sunucu {kod} döndürdü — adres değişmiş ya da geçici olarak erişilemiyor"
                )));
            }
            let toplam = sayisal_baslik(istek.0, WINHTTP_QUERY_CONTENT_LENGTH);

            // Yarım dosya kalıcı olmasın diye ayrı ad; doğrulama `model`de.
            let gecici = hedef.with_extension("yarim");
            if let Some(dizin) = gecici.parent() {
                std::fs::create_dir_all(dizin)?;
            }
            let mut dosya = std::fs::File::create(&gecici)?;

            let mut tampon = vec![0u8; TAMPON];
            let mut inen: u64 = 0;
            bildir(Ilerleme { inen, toplam });

            loop {
                if iptal.load(Ordering::Relaxed) {
                    drop(dosya);
                    let _ = std::fs::remove_file(&gecici);
                    return Err(iptal_hatasi());
                }

                let mut okunan: u32 = 0;
                if WinHttpReadData(
                    istek.0,
                    tampon.as_mut_ptr() as *mut core::ffi::c_void,
                    TAMPON as u32,
                    &mut okunan,
                )
                .is_err()
                {
                    drop(dosya);
                    let _ = std::fs::remove_file(&gecici);
                    return Err(hata("bağlantı koptu"));
                }
                if okunan == 0 {
                    break;
                }

                if let Err(e) = dosya.write_all(&tampon[..okunan as usize]) {
                    drop(dosya);
                    let _ = std::fs::remove_file(&gecici);
                    return Err(e.into());
                }
                inen += okunan as u64;
                bildir(Ilerleme { inen, toplam });
            }

            dosya.flush()?;
            drop(dosya);

            // Beklenenden kısa bir dosya, sessizce "indi" sayılmamalı.
            if let Some(t) = toplam {
                if inen != t {
                    let _ = std::fs::remove_file(&gecici);
                    return Err(Error::Indirme(format!(
                        "dosya eksik indi ({inen} / {t} bayt)"
                    )));
                }
            }

            let _ = std::fs::remove_file(hedef);
            std::fs::rename(&gecici, hedef)?;
            Ok(())
        }
    }

    /// Sayı olarak istenen bir cevap başlığı.
    fn sayisal_baslik(istek: *mut core::ffi::c_void, hangi: u32) -> Option<u64> {
        unsafe {
            // Content-Length 4 GB'ı aşabilir; 64 bitlik sorgu bu yüzden.
            let mut deger: u64 = 0;
            let mut boy = std::mem::size_of::<u64>() as u32;
            if WinHttpQueryHeaders(
                istek,
                hangi | WINHTTP_QUERY_FLAG_NUMBER64,
                PCWSTR::null(),
                Some(&mut deger as *mut u64 as *mut core::ffi::c_void),
                &mut boy,
                std::ptr::null_mut(),
            )
            .is_ok()
            {
                return Some(deger);
            }

            let mut kucuk: u32 = 0;
            let mut boy = std::mem::size_of::<u32>() as u32;
            if WinHttpQueryHeaders(
                istek,
                hangi | WINHTTP_QUERY_FLAG_NUMBER,
                PCWSTR::null(),
                Some(&mut kucuk as *mut u32 as *mut core::ffi::c_void),
                &mut boy,
                std::ptr::null_mut(),
            )
            .is_ok()
            {
                return Some(kucuk as u64);
            }
            None
        }
    }
}

#[cfg(not(windows))]
pub fn indir(
    _url: &str,
    _hedef: &Path,
    _iptal: &AtomicBool,
    _bildir: impl FnMut(Ilerleme),
) -> Result<()> {
    Err(Error::Unsupported("model indirme"))
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn https_varsayilan_port() {
        let p = ayristir("https://huggingface.co/model/dosya.onnx").unwrap();
        assert!(p.guvenli);
        assert_eq!(p.sunucu, "huggingface.co");
        assert_eq!(p.port, 443);
        assert_eq!(p.yol, "/model/dosya.onnx");
    }

    #[test]
    fn acik_port_okunuyor() {
        let p = ayristir("https://ornek.test:8443/a/b").unwrap();
        assert_eq!(p.port, 8443);
        assert_eq!(p.sunucu, "ornek.test");
    }

    #[test]
    fn yolsuz_adres_koke_dusuyor() {
        assert_eq!(ayristir("https://ornek.test").unwrap().yol, "/");
    }

    #[test]
    fn sorgu_dizesi_yolda_kaliyor() {
        let p = ayristir("https://ornek.test/dosya?download=true").unwrap();
        assert_eq!(p.yol, "/dosya?download=true");
    }

    #[test]
    fn taninmayan_sema_reddediliyor() {
        assert!(ayristir("ftp://ornek.test/x").is_none());
        assert!(ayristir("file:///C:/gizli").is_none());
        assert!(ayristir("ornek.test/x").is_none());
    }

    #[test]
    fn kimlik_tasiyan_adres_reddediliyor() {
        // Sabit adreslerimizde yok; gelirse bir şey ters demektir.
        assert!(ayristir("https://kullanici:parola@ornek.test/x").is_none());
    }

    #[test]
    fn bos_sunucu_reddediliyor() {
        assert!(ayristir("https:///yol").is_none());
    }
}
