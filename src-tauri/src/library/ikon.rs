//! Bir `.exe` dosyasının gömülü ikonunu okur.
//!
//! Kapak görseli olmayan her oyun için görsel kaynağı bu: Epic oyunları,
//! kütüphane dışı kurulumlar, elle seçilmiş exe'ler. Steam'in önbelleğinde
//! kapak varsa oraya hiç gelinmiyor (`super::steam`).
//!
//! **Ağa çıkmıyor.** Bu, IGDB/RAWG/SteamGridDB gibi servislerin yerine
//! seçilen yolun ikinci yarısı: anahtar yok, kota yok, kullanıcının hangi
//! oyunlara sahip olduğu hiçbir sunucuya gitmiyor (`docs/decisions.md` #25).
//!
//! Kullanılan API'ler resmî ve salt okunur: `PrivateExtractIconsW`,
//! `GetIconInfo`, `GetDIBits`. Hedef sürece hiçbir şey yazılmıyor, hiçbir şey
//! enjekte edilmiyor (tasarım ilkesi 3).

/// İstenen ikon kenar uzunlukları, büyükten küçüğe.
///
/// Modern exe'ler 256'lık ikon gömüyor ama hepsi değil; istenen boy yoksa
/// Windows ölçekliyor ve sonuç bulanık oluyor. Sırayla denemek, gerçekten
/// var olan en büyük boya yaklaşıyor. 128 kart görselinde yeterli ve
/// 256'nın dörtte biri kadar yer tutuyor.
#[cfg(windows)]
const BOYLAR: &[i32] = &[128, 64, 48, 32];

/// Exe'nin ikonunu PNG olarak döndürür. Bulunamazsa `None`.
#[cfg(windows)]
pub fn exe_ikonu(yol: &str) -> Option<Vec<u8>> {
    use windows::Win32::Foundation::MAX_PATH;
    use windows::Win32::Graphics::Gdi::{
        DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HGDIOBJ,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        DestroyIcon, GetIconInfo, PrivateExtractIconsW, HICON, ICONINFO,
    };

    // `PrivateExtractIconsW` sabit boyutlu bir MAX_PATH tamponu istiyor.
    // Daha uzun yollar (\\?\ ön ekli) bu API ile okunamıyor; ikon yerine
    // `None` dönüp arayüzün yer tutucusuna düşüyoruz.
    let genis: Vec<u16> = yol.encode_utf16().chain(std::iter::once(0)).collect();
    if genis.len() > MAX_PATH as usize {
        return None;
    }
    let mut tampon = [0u16; MAX_PATH as usize];
    tampon[..genis.len()].copy_from_slice(&genis);

    for &boy in BOYLAR {
        let mut ikonlar = [HICON::default(); 1];
        // SAFETY: tampon MAX_PATH boyutunda ve sonlandırılmış; `ikonlar`
        // yerel bir dizi ve API yazdığı adedi geri bildiriyor.
        let adet =
            unsafe { PrivateExtractIconsW(&tampon, 0, boy, boy, Some(&mut ikonlar), None, 0) };
        if adet == 0 || ikonlar[0].is_invalid() {
            continue;
        }
        let hicon = ikonlar[0];

        let sonuc = (|| -> Option<Vec<u8>> {
            let mut bilgi = ICONINFO::default();
            // SAFETY: `hicon` geçerli; `bilgi` yerel.
            unsafe { GetIconInfo(hicon, &mut bilgi) }.ok()?;

            // Maske bitmap'i her zaman dönüyor ve serbest bırakılması bize
            // düşüyor; renk bitmap'i tek renkli ikonlarda boş olabiliyor.
            let renk = bilgi.hbmColor;
            let temizle = || unsafe {
                if !bilgi.hbmMask.is_invalid() {
                    let _ = DeleteObject(HGDIOBJ(bilgi.hbmMask.0));
                }
                if !renk.is_invalid() {
                    let _ = DeleteObject(HGDIOBJ(renk.0));
                }
            };
            if renk.is_invalid() {
                temizle();
                return None;
            }

            let mut bm = BITMAP::default();
            // SAFETY: `renk` geçerli bir HBITMAP, `bm` yerel ve boyutu doğru.
            let okundu = unsafe {
                GetObjectW(
                    HGDIOBJ(renk.0),
                    std::mem::size_of::<BITMAP>() as i32,
                    Some(&mut bm as *mut _ as *mut _),
                )
            };
            if okundu == 0 || bm.bmWidth <= 0 || bm.bmHeight <= 0 {
                temizle();
                return None;
            }
            let (g, y) = (bm.bmWidth as u32, bm.bmHeight as u32);

            let mut basliklar = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: bm.bmWidth,
                    // Negatif yükseklik = yukarıdan aşağı satır sırası. PNG de
                    // böyle bekliyor; işaret unutulursa ikon ters çıkar.
                    biHeight: -bm.bmHeight,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut pikseller = vec![0u8; (g as usize) * (y as usize) * 4];
            // SAFETY: ekran DC'si yalnızca biçim dönüşümü için; tampon
            // boyutu başlıktaki genişlik/yükseklikle birebir.
            let satir = unsafe {
                let hdc = GetDC(None);
                let n = GetDIBits(
                    hdc,
                    renk,
                    0,
                    y,
                    Some(pikseller.as_mut_ptr() as *mut _),
                    &mut basliklar,
                    DIB_RGB_COLORS,
                );
                ReleaseDC(None, hdc);
                n
            };
            temizle();
            if satir == 0 {
                return None;
            }

            // Windows BGRA veriyor, PNG RGBA istiyor.
            for p in pikseller.chunks_exact_mut(4) {
                p.swap(0, 2);
            }
            // 24 bit ikonlarda alfa kanalı sıfır geliyor; olduğu gibi
            // bırakılırsa ikon tamamen saydam görünür. Tümü sıfırsa
            // "saydamlık bilgisi yok" kabul edip opak yapıyoruz.
            if pikseller.chunks_exact(4).all(|p| p[3] == 0) {
                for p in pikseller.chunks_exact_mut(4) {
                    p[3] = 255;
                }
            }

            super::png::kodla(g, y, &pikseller)
        })();

        // SAFETY: `hicon` bu döngüde üretildi ve başka kimse sahibi değil.
        unsafe {
            let _ = DestroyIcon(hicon);
        }

        if sonuc.is_some() {
            return sonuc;
        }
    }
    None
}

#[cfg(not(windows))]
pub fn exe_ikonu(_yol: &str) -> Option<Vec<u8>> {
    None
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn olmayan_dosya_none_donduruyor() {
        assert!(exe_ikonu("Z:\\boyle-bir-dosya-yok-42.exe").is_none());
    }

    #[test]
    fn asiri_uzun_yol_cokertmiyor() {
        let uzun = format!("C:\\{}\\x.exe", "a".repeat(400));
        assert!(exe_ikonu(&uzun).is_none());
    }

    #[cfg(windows)]
    #[test]
    fn sistem_exesinin_ikonu_okunuyor() {
        // notepad.exe her Windows kurulumunda var ve ikon taşıyor. Bu test
        // GDI zincirinin (çıkar → biçim çevir → PNG) uçtan uca çalıştığını
        // gösteriyor; PNG imzası da doğrulanıyor.
        let yol = format!(
            "{}\\System32\\notepad.exe",
            std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into())
        );
        if !std::path::Path::new(&yol).is_file() {
            return;
        }
        let png = exe_ikonu(&yol).expect("notepad.exe ikonu okunamadi");
        assert_eq!(&png[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    }
}
