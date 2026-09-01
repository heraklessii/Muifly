//! Registry okuma/yazma — **her zaman eski değeri döndürerek**.
//!
//! Bu modülün yazma fonksiyonları eski değeri okumadan yazmıyor. İmza bunu
//! zorunlu kılıyor: `dword_yaz` bir `Undo` döndürüyor ve çağıran onu deftere
//! koymak zorunda. "Yaz ve unut" yapan bir yardımcı fonksiyon bu dosyada
//! bilinçli olarak yok — olsaydı, aceleyle yazılan bir kod yolu geri
//! alınamayan bir registry değişikliği bırakabilirdi (`docs/RISKS.md`).
//!
//! `onceki: None` ile `onceki: Some(0)` arasındaki fark hayati: birincisi
//! "değer yoktu, geri alma = SİL", ikincisi "değer 0'dı, geri alma = 0 yaz".
//! Var olmayan bir değeri 0 yazarak geri almak, sistemi eski haline değil
//! yeni bir hale sokar.

use crate::error::{Error, Result};
use crate::ledger::Undo;

/// Desteklenen kökler. Tam bir registry sarmalayıcısı değil: ürünün
/// dokunduğu iki kök bunlar ve listeyi dar tutmak yanlış bir yere yazma
/// olasılığını azaltıyor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kok {
    /// `HKEY_CURRENT_USER` — yönetici yetkisi gerekmiyor.
    Kullanici,
    /// `HKEY_LOCAL_MACHINE` — yönetici yetkisi gerekiyor.
    Makine,
}

impl Kok {
    pub fn kisa(self) -> &'static str {
        match self {
            Kok::Kullanici => "HKCU",
            Kok::Makine => "HKLM",
        }
    }
}

/// Defterde ve günlükte görünen tam yol.
pub fn tam_yol(kok: Kok, alt_yol: &str) -> String {
    format!("{}\\{}", kok.kisa(), alt_yol)
}

/// Defterdeki metin yolu tekrar `(Kok, alt yol)` çiftine çevirir.
///
/// Çökme sonrası defterden okunan bir kaydın geri alınabilmesi için gerekli.
pub fn yolu_coz(tam: &str) -> Result<(Kok, String)> {
    let (kok, kalan) = tam
        .split_once('\\')
        .ok_or_else(|| Error::ProfileInvalid(format!("geçersiz registry yolu: {tam}")))?;
    let kok = match kok {
        "HKCU" => Kok::Kullanici,
        "HKLM" => Kok::Makine,
        diger => {
            return Err(Error::ProfileInvalid(format!(
                "desteklenmeyen registry kökü: {diger}"
            )))
        }
    };
    Ok((kok, kalan.to_string()))
}

#[cfg(windows)]
mod win {
    use super::*;

    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS, WIN32_ERROR};
    use windows::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW,
        RegSetValueExW, HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_SET_VALUE,
        REG_DWORD, REG_OPTION_NON_VOLATILE, REG_SAM_FLAGS, REG_SZ, REG_VALUE_TYPE,
    };

    /// `RegCloseKey` unutulmasın diye.
    struct Anahtar(HKEY);

    impl Drop for Anahtar {
        fn drop(&mut self) {
            if !self.0.is_invalid() {
                // SAFETY: anahtar bu tipin sahipliğinde.
                let _ = unsafe { RegCloseKey(self.0) };
            }
        }
    }

    fn ham_kok(kok: Kok) -> HKEY {
        match kok {
            Kok::Kullanici => HKEY_CURRENT_USER,
            Kok::Makine => HKEY_LOCAL_MACHINE,
        }
    }

    fn ac(kok: Kok, alt_yol: &str, yetki: REG_SAM_FLAGS) -> Result<Anahtar> {
        let yol = HSTRING::from(alt_yol);
        let mut anahtar = HKEY::default();
        // SAFETY: yol geçerli bir geniş karakter dizisi; çıktı yerel.
        let kod = unsafe {
            RegOpenKeyExW(
                ham_kok(kok),
                PCWSTR(yol.as_ptr()),
                None,
                yetki,
                &mut anahtar,
            )
        };
        if kod != ERROR_SUCCESS {
            return Err(hata(kod, "registry anahtarı açma", kok, alt_yol));
        }
        Ok(Anahtar(anahtar))
    }

    fn olustur(kok: Kok, alt_yol: &str) -> Result<Anahtar> {
        let yol = HSTRING::from(alt_yol);
        let mut anahtar = HKEY::default();
        // SAFETY: yol geçerli; `REG_OPTION_NON_VOLATILE` kalıcı anahtar demek.
        let kod = unsafe {
            RegCreateKeyExW(
                ham_kok(kok),
                PCWSTR(yol.as_ptr()),
                None,
                None,
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE | KEY_READ,
                None,
                &mut anahtar,
                None,
            )
        };
        if kod != ERROR_SUCCESS {
            return Err(hata(kod, "registry anahtarı oluşturma", kok, alt_yol));
        }
        Ok(Anahtar(anahtar))
    }

    fn hata(kod: WIN32_ERROR, islem: &'static str, kok: Kok, yol: &str) -> Error {
        // HKLM'e yazma denemesi yetki hatasıyla dönüyorsa kullanıcıya
        // "bozuk" değil "yükseltme gerekiyor" demek gerekiyor.
        if kod.0 == 5 {
            return Error::NeedsElevation(if kok == Kok::Makine {
                "sistem ayarını değiştirme"
            } else {
                "kullanıcı ayarını değiştirme"
            });
        }
        Error::Windows {
            islem,
            kod: format!("{} ({})", kod.0, tam_yol(kok, yol)),
        }
    }

    /// DWORD okur. Değer yoksa `Ok(None)` — hata değil.
    pub fn dword_oku(kok: Kok, alt_yol: &str, ad: &str) -> Result<Option<u32>> {
        let anahtar = match ac(kok, alt_yol, KEY_READ) {
            Ok(a) => a,
            // Anahtarın kendisi yoksa değer de yok.
            Err(Error::Windows { .. }) => return Ok(None),
            Err(e) => return Err(e),
        };

        let ad_w = HSTRING::from(ad);
        let mut tur = REG_VALUE_TYPE::default();
        let mut deger = 0u32;
        let mut boy = std::mem::size_of::<u32>() as u32;

        // SAFETY: tampon `u32` boyutunda ve `boy` onu doğru bildiriyor.
        let kod = unsafe {
            RegQueryValueExW(
                anahtar.0,
                PCWSTR(ad_w.as_ptr()),
                None,
                Some(&mut tur),
                Some(&mut deger as *mut u32 as *mut u8),
                Some(&mut boy),
            )
        };
        if kod == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        if kod != ERROR_SUCCESS {
            return Err(hata(kod, "registry değeri okuma", kok, alt_yol));
        }
        if tur != REG_DWORD {
            return Err(Error::Windows {
                islem: "registry değeri okuma",
                kod: format!("{ad} beklenen DWORD değil"),
            });
        }
        Ok(Some(deger))
    }

    /// DWORD yazar ve geri alma kaydını döner.
    pub fn dword_yaz(kok: Kok, alt_yol: &str, ad: &str, deger: u32) -> Result<Undo> {
        let onceki = dword_oku(kok, alt_yol, ad)?;
        let anahtar = olustur(kok, alt_yol)?;
        let ad_w = HSTRING::from(ad);

        // SAFETY: dilim `deger`in baytları; ömrü çağrı boyunca geçerli.
        let kod = unsafe {
            RegSetValueExW(
                anahtar.0,
                PCWSTR(ad_w.as_ptr()),
                None,
                REG_DWORD,
                Some(&deger.to_le_bytes()),
            )
        };
        if kod != ERROR_SUCCESS {
            return Err(hata(kod, "registry değeri yazma", kok, alt_yol));
        }

        Ok(Undo::RegistryDword {
            yol: tam_yol(kok, alt_yol),
            ad: ad.to_string(),
            onceki,
        })
    }

    /// Değeri siler. Zaten yoksa başarılı sayılıyor.
    pub fn deger_sil(kok: Kok, alt_yol: &str, ad: &str) -> Result<()> {
        let anahtar = match ac(kok, alt_yol, KEY_SET_VALUE) {
            Ok(a) => a,
            Err(Error::Windows { .. }) => return Ok(()),
            Err(e) => return Err(e),
        };
        let ad_w = HSTRING::from(ad);
        // SAFETY: geçerli anahtar ve ad.
        let kod = unsafe { RegDeleteValueW(anahtar.0, PCWSTR(ad_w.as_ptr())) };
        if kod == ERROR_SUCCESS || kod == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            Err(hata(kod, "registry değeri silme", kok, alt_yol))
        }
    }

    /// Defterdeki bir DWORD kaydını geri alır.
    ///
    /// `onceki: None` → değeri SİL. Bu satır, bu dosyanın var oluş sebebi.
    pub fn dword_geri_al(yol: &str, ad: &str, onceki: Option<u32>) -> Result<()> {
        let (kok, alt) = yolu_coz(yol)?;
        match onceki {
            Some(v) => {
                dword_yaz(kok, &alt, ad, v)?;
                Ok(())
            }
            None => deger_sil(kok, &alt, ad),
        }
    }

    pub fn metin_oku(kok: Kok, alt_yol: &str, ad: &str) -> Result<Option<String>> {
        let anahtar = match ac(kok, alt_yol, KEY_READ) {
            Ok(a) => a,
            Err(Error::Windows { .. }) => return Ok(None),
            Err(e) => return Err(e),
        };
        let ad_w = HSTRING::from(ad);
        let mut boy = 0u32;

        // Boyut sorgusu.
        // SAFETY: tampon `None`, yalnızca boyut isteniyor.
        let kod = unsafe {
            RegQueryValueExW(
                anahtar.0,
                PCWSTR(ad_w.as_ptr()),
                None,
                None,
                None,
                Some(&mut boy),
            )
        };
        if kod == ERROR_FILE_NOT_FOUND || boy == 0 {
            return Ok(None);
        }
        if kod != ERROR_SUCCESS {
            return Err(hata(kod, "registry metni okuma", kok, alt_yol));
        }

        let mut tampon = vec![0u8; boy as usize];
        // SAFETY: tampon `boy` kadar ayrıldı.
        let kod = unsafe {
            RegQueryValueExW(
                anahtar.0,
                PCWSTR(ad_w.as_ptr()),
                None,
                None,
                Some(tampon.as_mut_ptr()),
                Some(&mut boy),
            )
        };
        if kod != ERROR_SUCCESS {
            return Err(hata(kod, "registry metni okuma", kok, alt_yol));
        }

        let genis: Vec<u16> = tampon
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .take_while(|c| *c != 0)
            .collect();
        Ok(Some(String::from_utf16_lossy(&genis)))
    }

    pub fn metin_yaz(kok: Kok, alt_yol: &str, ad: &str, deger: &str) -> Result<()> {
        let anahtar = olustur(kok, alt_yol)?;
        let ad_w = HSTRING::from(ad);
        let deger_w: Vec<u16> = deger.encode_utf16().chain(std::iter::once(0)).collect();
        let baytlar: Vec<u8> = deger_w.iter().flat_map(|c| c.to_le_bytes()).collect();

        // SAFETY: bayt dilimi yerel ve NUL sonlandırmalı UTF-16 içeriyor.
        let kod = unsafe {
            RegSetValueExW(
                anahtar.0,
                PCWSTR(ad_w.as_ptr()),
                None,
                REG_SZ,
                Some(&baytlar),
            )
        };
        if kod != ERROR_SUCCESS {
            return Err(hata(kod, "registry metni yazma", kok, alt_yol));
        }
        Ok(())
    }

    /// Bir anahtarın doğrudan alt anahtarlarının adları.
    ///
    /// Ağ arayüzleri registry'de GUID adlı alt anahtarlar olarak duruyor ve
    /// hangi arayüzün olduğu makineden makineye değişiyor; sabit bir yol
    /// yazılamıyor.
    pub fn alt_anahtarlar(kok: Kok, alt_yol: &str) -> Result<Vec<String>> {
        use windows::Win32::System::Registry::RegEnumKeyExW;

        let anahtar = match ac(kok, alt_yol, KEY_READ) {
            Ok(a) => a,
            Err(Error::Windows { .. }) => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };

        let mut adlar = Vec::new();
        let mut indeks = 0u32;
        loop {
            let mut tampon = [0u16; 256];
            let mut boy = tampon.len() as u32;
            // SAFETY: tampon yerel; `boy` gerçek uzunluğu bildiriyor ve API
            // yazdığı kadarını geri yazıyor.
            let kod = unsafe {
                RegEnumKeyExW(
                    anahtar.0,
                    indeks,
                    Some(windows::core::PWSTR(tampon.as_mut_ptr())),
                    &mut boy,
                    None,
                    None,
                    None,
                    None,
                )
            };
            if kod != ERROR_SUCCESS {
                break;
            }
            adlar.push(String::from_utf16_lossy(&tampon[..boy as usize]));
            indeks += 1;
        }
        Ok(adlar)
    }

    /// Anahtar var mı?
    pub fn anahtar_var(kok: Kok, alt_yol: &str) -> bool {
        ac(kok, alt_yol, KEY_READ).is_ok()
    }

    /// Anahtarı ve altındaki her şeyi siler.
    ///
    /// Yalnızca **Muifly'ın kendi oluşturduğu** anahtarlar için kullanılıyor
    /// (QoS ilkesi). Defterde "bu anahtar önceden yoktu" kaydı olmadan
    /// çağrılmamalı; çağıran taraf bunu garanti ediyor.
    pub fn anahtar_sil(kok: Kok, alt_yol: &str) -> Result<()> {
        use windows::core::HSTRING;
        use windows::Win32::System::Registry::RegDeleteTreeW;

        let yol = HSTRING::from(alt_yol);
        // SAFETY: kök tanıtıcısı sabit, yol geçerli geniş karakter dizisi.
        let kod = unsafe { RegDeleteTreeW(ham_kok(kok), PCWSTR(yol.as_ptr())) };
        if kod == ERROR_SUCCESS || kod == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            Err(hata(kod, "registry anahtarı silme", kok, alt_yol))
        }
    }
}

#[cfg(windows)]
pub use win::{
    alt_anahtarlar, anahtar_sil, anahtar_var, deger_sil, dword_geri_al, dword_oku, dword_yaz,
    metin_oku, metin_yaz,
};

#[cfg(not(windows))]
mod stub {
    use super::*;

    pub fn dword_oku(_k: Kok, _y: &str, _a: &str) -> Result<Option<u32>> {
        Err(Error::Unsupported("registry okuma"))
    }
    pub fn dword_yaz(_k: Kok, _y: &str, _a: &str, _d: u32) -> Result<Undo> {
        Err(Error::Unsupported("registry yazma"))
    }
    pub fn deger_sil(_k: Kok, _y: &str, _a: &str) -> Result<()> {
        Err(Error::Unsupported("registry silme"))
    }
    pub fn dword_geri_al(_y: &str, _a: &str, _o: Option<u32>) -> Result<()> {
        Err(Error::Unsupported("registry geri alma"))
    }
    pub fn metin_oku(_k: Kok, _y: &str, _a: &str) -> Result<Option<String>> {
        Err(Error::Unsupported("registry okuma"))
    }
    pub fn metin_yaz(_k: Kok, _y: &str, _a: &str, _d: &str) -> Result<()> {
        Err(Error::Unsupported("registry yazma"))
    }
    pub fn alt_anahtarlar(_k: Kok, _y: &str) -> Result<Vec<String>> {
        Err(Error::Unsupported("registry listeleme"))
    }
    pub fn anahtar_var(_k: Kok, _y: &str) -> bool {
        false
    }
    pub fn anahtar_sil(_k: Kok, _y: &str) -> Result<()> {
        Err(Error::Unsupported("registry anahtarı silme"))
    }
}

#[cfg(not(windows))]
pub use stub::{
    alt_anahtarlar, anahtar_sil, anahtar_var, deger_sil, dword_geri_al, dword_oku, dword_yaz,
    metin_oku, metin_yaz,
};

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn yol_bicimi() {
        assert_eq!(
            tam_yol(Kok::Makine, "SYSTEM\\CurrentControlSet"),
            "HKLM\\SYSTEM\\CurrentControlSet"
        );
        assert_eq!(tam_yol(Kok::Kullanici, "Software"), "HKCU\\Software");
    }

    #[test]
    fn yol_gidis_donusu() {
        let tam = tam_yol(Kok::Makine, "SYSTEM\\Foo\\Bar");
        let (kok, alt) = yolu_coz(&tam).unwrap();
        assert_eq!(kok, Kok::Makine);
        assert_eq!(alt, "SYSTEM\\Foo\\Bar");
    }

    #[test]
    fn bilinmeyen_kok_reddediliyor() {
        // Defterde bozuk bir kök varsa, geri alma yanlış yere yazmaktansa
        // hata vermeli.
        assert!(yolu_coz("HKCR\\Foo").is_err());
        assert!(yolu_coz("kok yok").is_err());
    }

    #[cfg(windows)]
    #[test]
    fn kullanici_kokunde_yaz_oku_geri_al() {
        // HKCU altında geçici bir anahtar: yönetici yetkisi gerektirmiyor,
        // bu yüzden testte gerçekten koşabiliyor.
        let yol = "Software\\MuiflyTest\\registry_testi";
        let ad = "deneme";

        // Başlangıçta yok.
        let _ = deger_sil(Kok::Kullanici, yol, ad);
        assert_eq!(dword_oku(Kok::Kullanici, yol, ad).unwrap(), None);

        let undo = dword_yaz(Kok::Kullanici, yol, ad, 42).unwrap();
        assert_eq!(dword_oku(Kok::Kullanici, yol, ad).unwrap(), Some(42));

        // Kayıt "önceden yoktu" demeli.
        match &undo {
            Undo::RegistryDword { onceki, .. } => assert_eq!(*onceki, None),
            _ => panic!("yanlış geri alma tipi"),
        }

        // Geri alma değeri SİLMELİ, 0 yazmamalı.
        if let Undo::RegistryDword {
            yol: y,
            ad: a,
            onceki,
        } = undo
        {
            dword_geri_al(&y, &a, onceki).unwrap();
        }
        assert_eq!(
            dword_oku(Kok::Kullanici, yol, ad).unwrap(),
            None,
            "önceden olmayan değer silinmeli, 0 yazılmamalı"
        );
    }

    #[cfg(windows)]
    #[test]
    fn var_olan_deger_eski_haline_donuyor() {
        let yol = "Software\\MuiflyTest\\registry_testi2";
        let ad = "deneme";
        let _ = deger_sil(Kok::Kullanici, yol, ad);

        dword_yaz(Kok::Kullanici, yol, ad, 7).unwrap();
        let undo = dword_yaz(Kok::Kullanici, yol, ad, 99).unwrap();
        assert_eq!(dword_oku(Kok::Kullanici, yol, ad).unwrap(), Some(99));

        if let Undo::RegistryDword {
            yol: y,
            ad: a,
            onceki,
        } = undo
        {
            assert_eq!(onceki, Some(7));
            dword_geri_al(&y, &a, onceki).unwrap();
        }
        assert_eq!(dword_oku(Kok::Kullanici, yol, ad).unwrap(), Some(7));

        let _ = deger_sil(Kok::Kullanici, yol, ad);
    }
}
