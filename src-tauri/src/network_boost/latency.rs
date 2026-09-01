//! Gecikme, jitter ve yol (route) ölçümü.
//!
//! ICMP `IcmpSendEcho` üzerinden yapılıyor. Ham soket kullanılmıyor: ham soket
//! yönetici yetkisi ister, `IcmpSendEcho` istemez. Programın sürekli açık
//! duran ölçüm tarafının yükseltilmiş yetki gerektirmemesi tasarım ilkesi 5.
//!
//! Ölçüm sonuçları `monitor::metrics` içindeki saf fonksiyonlara besleniyor;
//! jitter/kayıp hesabı burada tekrarlanmıyor.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Varsayılan ölçüm hedefi.
///
/// Cloudflare'ın anycast adresi: dünyanın her yerinden yakın bir düğüme
/// düşüyor, yani ölçülen şey "kullanıcının bağlantısının kararlılığı", belirli
/// bir sunucuya olan mesafe değil. Oyun sunucusuna ölçüm, kullanıcı profilde
/// bir adres verdiğinde yapılıyor.
pub const VARSAYILAN_HEDEF: &str = "1.1.1.1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YolDugumu {
    /// Kaçıncı atlama (TTL).
    pub atlama: u8,
    /// Cevap veren düğümün adresi. Cevap yoksa `None` — bu normal, birçok
    /// yönlendirici TTL aşımı bildirmiyor.
    pub adres: Option<String>,
    pub gecikme_ms: Option<f32>,
}

/// Yol testinin sonucu.
///
/// **Yol değiştirilmiyor, yalnızca gösteriliyor.** `docs/MODULES.md` "en düşük
/// gecikmeli yol tespiti" diyor; tespit ile yönlendirme farklı şeyler. Statik
/// route eklemek (`route add`) sistemin yönlendirme tablosunu değiştirir ve
/// yanlış bir kayıt kullanıcıyı internetsiz bırakır. Muifly yolu ölçüp
/// gösteriyor; nerede gecikme biriktiğini görmek, kullanıcının ISS'siyle
/// konuşabilmesi için yeterli ve dürüst olan bu.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YolSonucu {
    pub hedef: String,
    pub dugumler: Vec<YolDugumu>,
    /// Hedefe ulaşıldı mı?
    pub ulasildi: bool,
}

#[cfg(windows)]
mod win {
    use super::*;

    use std::net::Ipv4Addr;

    use windows::Win32::Foundation::WAIT_TIMEOUT;
    use windows::Win32::NetworkManagement::IpHelper::{
        IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho, ICMP_ECHO_REPLY, IP_OPTION_INFORMATION,
        IP_SUCCESS, IP_TTL_EXPIRED_TRANSIT,
    };

    /// `IcmpCloseHandle` unutulmasın diye.
    struct IcmpTanitici(windows::Win32::Foundation::HANDLE);

    impl Drop for IcmpTanitici {
        fn drop(&mut self) {
            if !self.0.is_invalid() {
                // SAFETY: tanıtıcı bu tipin sahipliğinde.
                let _ = unsafe { IcmpCloseHandle(self.0) };
            }
        }
    }

    fn ac() -> Result<IcmpTanitici> {
        // SAFETY: parametresiz; başarısızlıkta geçersiz tanıtıcı dönüyor ve
        // hemen altında kontrol ediliyor.
        let h = unsafe { IcmpCreateFile() }
            .map_err(|e| crate::error::win("ICMP tanıtıcısı açma", e))?;
        if h.is_invalid() {
            return Err(Error::Network("ICMP tanıtıcısı açılamadı".into()));
        }
        Ok(IcmpTanitici(h))
    }

    fn adres_coz(hedef: &str) -> Result<Ipv4Addr> {
        if let Ok(ip) = hedef.parse::<Ipv4Addr>() {
            return Ok(ip);
        }
        // Ad çözümlemesi: `to_socket_addrs` sistemin çözümleyicisini kullanıyor.
        use std::net::ToSocketAddrs;
        let adresler = (hedef, 0u16)
            .to_socket_addrs()
            .map_err(|e| Error::Network(format!("{hedef} çözümlenemedi: {e}")))?;
        for a in adresler {
            if let std::net::IpAddr::V4(v4) = a.ip() {
                return Ok(v4);
            }
        }
        Err(Error::Network(format!("{hedef} için IPv4 adresi yok")))
    }

    /// Tek bir ICMP echo. `ttl: None` = varsayılan.
    ///
    /// Dönen değer: `(cevap veren adres, gecikme ms)`. Zaman aşımında
    /// `Ok(None)` — kayıp paket bir hata değil, ölçülen veridir.
    fn tek_echo(
        tanitici: &IcmpTanitici,
        hedef: Ipv4Addr,
        ttl: Option<u8>,
        zaman_asimi_ms: u32,
    ) -> Result<Option<(Ipv4Addr, f32)>> {
        // 32 baytlık yük: `ping` komutunun varsayılanı, ara yollarda özel bir
        // işleme takılmayan bilinen bir boyut.
        let veri = [0x4du8; 32];
        // Cevap tamponu: yapı + veri + Windows'un istediği pay.
        let mut cevap = vec![0u8; std::mem::size_of::<ICMP_ECHO_REPLY>() + veri.len() + 8];

        let secenekler = ttl.map(|t| IP_OPTION_INFORMATION {
            Ttl: t,
            Tos: 0,
            Flags: 0,
            OptionsSize: 0,
            OptionsData: std::ptr::null_mut(),
        });

        // SAFETY: tamponlar yerel ve boyutları doğru bildiriliyor; API çıktıyı
        // `cevap` içine yazıyor.
        let adet = unsafe {
            IcmpSendEcho(
                tanitici.0,
                u32::from_ne_bytes(hedef.octets()),
                veri.as_ptr() as *const _,
                veri.len() as u16,
                secenekler.as_ref().map(|s| s as *const _),
                cevap.as_mut_ptr() as *mut _,
                cevap.len() as u32,
                zaman_asimi_ms,
            )
        };

        if adet == 0 {
            let kod = unsafe { windows::Win32::Foundation::GetLastError() };
            // Zaman aşımı ve TTL aşımı ölçümün parçası, hata değil.
            if kod.0 == WAIT_TIMEOUT.0 || kod.0 == IP_REQ_TIMED_OUT {
                return Ok(None);
            }
            return Ok(None);
        }

        // SAFETY: `adet > 0` olduğu için tamponda en az bir cevap yapısı var.
        let r = unsafe { &*(cevap.as_ptr() as *const ICMP_ECHO_REPLY) };
        let adres = Ipv4Addr::from(r.Address.to_ne_bytes());

        // TTL aşımı da geçerli bir yol düğümü cevabı.
        if r.Status == IP_SUCCESS || r.Status == IP_TTL_EXPIRED_TRANSIT {
            // `RoundTripTime` milisaniye cinsinden tam sayı; 1 ms altındaki
            // turlar 0 görünüyor. Bu bir ölçüm sınırı, uydurulmuş bir
            // hassasiyet eklenmiyor.
            Ok(Some((adres, r.RoundTripTime as f32)))
        } else {
            Ok(None)
        }
    }

    /// `IP_REQ_TIMED_OUT` — `windows` crate'inde sabit olarak açılmıyor.
    const IP_REQ_TIMED_OUT: u32 = 11010;

    /// Tek ölçüm. Kayıp paket `Ok(None)`.
    pub fn olc(hedef: &str, zaman_asimi_ms: u32) -> Result<Option<f32>> {
        let ip = adres_coz(hedef)?;
        let t = ac()?;
        Ok(tek_echo(&t, ip, None, zaman_asimi_ms)?.map(|(_, ms)| ms))
    }

    /// `adet` kez ölçer; her sonucu (kayıplar dahil) döner.
    pub fn seri_olc(hedef: &str, adet: u32, zaman_asimi_ms: u32) -> Result<Vec<Option<f32>>> {
        let ip = adres_coz(hedef)?;
        let t = ac()?;
        let mut sonuclar = Vec::with_capacity(adet as usize);
        for _ in 0..adet {
            sonuclar.push(tek_echo(&t, ip, None, zaman_asimi_ms)?.map(|(_, ms)| ms));
        }
        Ok(sonuclar)
    }

    /// Yol testi: TTL'i 1'den başlatıp hedefe kadar artırır.
    pub fn yol_testi(hedef: &str, azami_atlama: u8, zaman_asimi_ms: u32) -> Result<YolSonucu> {
        let ip = adres_coz(hedef)?;
        let t = ac()?;
        let mut dugumler = Vec::new();
        let mut ulasildi = false;

        for atlama in 1..=azami_atlama {
            match tek_echo(&t, ip, Some(atlama), zaman_asimi_ms)? {
                Some((adres, ms)) => {
                    dugumler.push(YolDugumu {
                        atlama,
                        adres: Some(adres.to_string()),
                        gecikme_ms: Some(ms),
                    });
                    if adres == ip {
                        ulasildi = true;
                        break;
                    }
                }
                None => dugumler.push(YolDugumu {
                    atlama,
                    adres: None,
                    gecikme_ms: None,
                }),
            }
        }

        Ok(YolSonucu {
            hedef: hedef.to_string(),
            dugumler,
            ulasildi,
        })
    }
}

#[cfg(windows)]
pub use win::{olc, seri_olc, yol_testi};

#[cfg(not(windows))]
pub fn olc(_hedef: &str, _zaman_asimi_ms: u32) -> Result<Option<f32>> {
    Err(Error::Unsupported("ICMP ölçümü"))
}

#[cfg(not(windows))]
pub fn seri_olc(_hedef: &str, _adet: u32, _zaman_asimi_ms: u32) -> Result<Vec<Option<f32>>> {
    Err(Error::Unsupported("ICMP ölçümü"))
}

#[cfg(not(windows))]
pub fn yol_testi(_hedef: &str, _azami: u8, _zaman_asimi_ms: u32) -> Result<YolSonucu> {
    Err(Error::Unsupported("yol testi"))
}

/// Yol testinde gecikmenin en çok arttığı atlama.
///
/// Kullanıcıya "sorun sende değil, şu düğümde" diyebilmek için. Sayısal bir
/// vaat değil, ölçülen bir gözlem — `DESIGN_PRINCIPLES.md` madde 4 ile uyumlu.
pub fn en_buyuk_sicrama(dugumler: &[YolDugumu]) -> Option<(u8, f32)> {
    let mut onceki: Option<f32> = None;
    let mut en_iyi: Option<(u8, f32)> = None;

    for d in dugumler {
        let Some(ms) = d.gecikme_ms else { continue };
        if let Some(o) = onceki {
            let artis = ms - o;
            if artis > 0.0 && en_iyi.map(|(_, a)| artis > a).unwrap_or(true) {
                en_iyi = Some((d.atlama, artis));
            }
        }
        onceki = Some(ms);
    }
    en_iyi
}

#[cfg(test)]
mod testler {
    use super::*;

    fn dugum(atlama: u8, ms: Option<f32>) -> YolDugumu {
        YolDugumu {
            atlama,
            adres: ms.map(|_| "10.0.0.1".to_string()),
            gecikme_ms: ms,
        }
    }

    #[test]
    fn en_buyuk_sicrama_bulunuyor() {
        // 5 → 8 (+3), 8 → 40 (+32), 40 → 42 (+2)
        let yol = vec![
            dugum(1, Some(5.0)),
            dugum(2, Some(8.0)),
            dugum(3, Some(40.0)),
            dugum(4, Some(42.0)),
        ];
        let (atlama, artis) = en_buyuk_sicrama(&yol).unwrap();
        assert_eq!(atlama, 3);
        assert!((artis - 32.0).abs() < 0.001);
    }

    #[test]
    fn cevapsiz_dugumler_sicramayi_bozmuyor() {
        // Ortadaki cevapsız düğüm atlanıyor; 5 → 40 farkı 3. atlamada.
        let yol = vec![dugum(1, Some(5.0)), dugum(2, None), dugum(3, Some(40.0))];
        let (atlama, _) = en_buyuk_sicrama(&yol).unwrap();
        assert_eq!(atlama, 3);
    }

    #[test]
    fn tek_dugumde_sicrama_yok() {
        assert!(en_buyuk_sicrama(&[dugum(1, Some(5.0))]).is_none());
        assert!(en_buyuk_sicrama(&[]).is_none());
    }

    #[test]
    fn azalan_gecikme_sicrama_saymiyor() {
        // Gecikme düşüyorsa "sıçrama" yok; ölçüm gürültüsü artış gibi
        // gösterilmemeli.
        let yol = vec![dugum(1, Some(40.0)), dugum(2, Some(20.0))];
        assert!(en_buyuk_sicrama(&yol).is_none());
    }

    #[test]
    fn varsayilan_hedef_gecerli_ip() {
        assert!(VARSAYILAN_HEDEF.parse::<std::net::Ipv4Addr>().is_ok());
    }
}
