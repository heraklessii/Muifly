//! Sıkıştırmasız PNG yazıcı.
//!
//! Windows'tan çıkarılan exe ikonları ham piksel dizisi olarak geliyor
//! (`super::ikon`). Arayüzde göstermek için tarayıcının tanıdığı bir biçime
//! çevrilmeleri gerekiyor.
//!
//! Neden yeni bir bağımlılık değil: `image` ya da `png` kasası bu iş için
//! devasa. Neden PNG, ham BMP değil: BMP'nin saydamlık taşıyan sürümünün
//! (BITMAPV5) WebView2'de doğru çizileceğine güvenmek gerekirdi, PNG'de
//! böyle bir soru yok.
//!
//! PNG'nin veri akışı zlib; zlib'in de **sıkıştırmasız blok** kipi var.
//! Böylece deflate uygulamadan geçerli bir PNG üretilebiliyor. Dosya
//! büyüyor ama tek kullanıcı bir ikon: 128x128 ikon ~66 KB ve yalnızca
//! kapak görseli olmayan oyunlarda, kart görününce üretiliyor.

/// Bir zlib sıkıştırmasız bloğunun taşıyabileceği azami bayt.
const BLOK: usize = 65535;

/// RGBA piksellerden PNG üretir. `veri` uzunluğu `genislik * yukseklik * 4`
/// olmalı; değilse `None`.
pub fn kodla(genislik: u32, yukseklik: u32, veri: &[u8]) -> Option<Vec<u8>> {
    let beklenen = (genislik as usize)
        .checked_mul(yukseklik as usize)?
        .checked_mul(4)?;
    if genislik == 0 || yukseklik == 0 || veri.len() != beklenen {
        return None;
    }

    let mut cikti = Vec::with_capacity(veri.len() + 1024);
    cikti.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&genislik.to_be_bytes());
    ihdr.extend_from_slice(&yukseklik.to_be_bytes());
    ihdr.push(8); // bit derinliği
    ihdr.push(6); // renk tipi: RGBA
    ihdr.push(0); // sıkıştırma: deflate
    ihdr.push(0); // süzgeç yöntemi
    ihdr.push(0); // taramasız
    parca_yaz(&mut cikti, b"IHDR", &ihdr);

    // Her satırın başına süzgeç baytı (0 = süzgeç yok) ekleniyor.
    let mut ham = Vec::with_capacity(beklenen + yukseklik as usize);
    for satir in veri.chunks_exact(genislik as usize * 4) {
        ham.push(0);
        ham.extend_from_slice(satir);
    }

    parca_yaz(&mut cikti, b"IDAT", &zlib_sar(&ham));
    parca_yaz(&mut cikti, b"IEND", &[]);
    Some(cikti)
}

/// Veriyi sıkıştırmasız zlib akışına sarar.
fn zlib_sar(ham: &[u8]) -> Vec<u8> {
    // 0x78 0x01: deflate, 32K pencere, en düşük sıkıştırma. İki baytın
    // 31'e bölünmesi zlib'in başlık sağlamasıdır ve 0x7801 bunu sağlıyor.
    let mut cikti = vec![0x78, 0x01];

    if ham.is_empty() {
        cikti.extend_from_slice(&[0x01, 0x00, 0x00, 0xff, 0xff]);
    } else {
        let mut kalan = ham;
        while !kalan.is_empty() {
            let n = kalan.len().min(BLOK);
            let son = n == kalan.len();
            cikti.push(if son { 1 } else { 0 });
            cikti.extend_from_slice(&(n as u16).to_le_bytes());
            cikti.extend_from_slice(&(!(n as u16)).to_le_bytes());
            cikti.extend_from_slice(&kalan[..n]);
            kalan = &kalan[n..];
        }
    }

    cikti.extend_from_slice(&adler32(ham).to_be_bytes());
    cikti
}

fn parca_yaz(cikti: &mut Vec<u8>, tur: &[u8; 4], veri: &[u8]) {
    cikti.extend_from_slice(&(veri.len() as u32).to_be_bytes());
    cikti.extend_from_slice(tur);
    cikti.extend_from_slice(veri);

    let mut crc = crc32(0xffff_ffff, tur);
    crc = crc32(crc, veri);
    cikti.extend_from_slice(&(crc ^ 0xffff_ffff).to_be_bytes());
}

/// PNG parça sağlaması (IEEE CRC-32), tablosuz.
fn crc32(baslangic: u32, veri: &[u8]) -> u32 {
    let mut c = baslangic;
    for &b in veri {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xedb8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
    }
    c
}

/// zlib akış sağlaması.
fn adler32(veri: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &x in veri {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn imza_ve_parcalar_yerinde() {
        let png = kodla(2, 2, &[0u8; 16]).unwrap();
        assert_eq!(&png[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
        assert!(png.windows(4).any(|p| p == b"IHDR"));
        assert!(png.windows(4).any(|p| p == b"IDAT"));
        assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
    }

    #[test]
    fn ihdr_boyutlari_dogru() {
        let png = kodla(7, 3, &[0u8; 7 * 3 * 4]).unwrap();
        // 8 imza + 4 uzunluk + 4 tür = 16. Sonrasında genişlik ve yükseklik.
        assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), 7);
        assert_eq!(u32::from_be_bytes(png[20..24].try_into().unwrap()), 3);
    }

    #[test]
    fn yanlis_uzunluk_reddediliyor() {
        assert!(kodla(2, 2, &[0u8; 15]).is_none());
        assert!(kodla(0, 4, &[]).is_none());
        assert!(kodla(4, 0, &[]).is_none());
    }

    #[test]
    fn zlib_basligi_31e_bolunuyor() {
        let akis = zlib_sar(b"deneme");
        let bas = ((akis[0] as u32) << 8) | akis[1] as u32;
        assert_eq!(bas % 31, 0, "zlib basligi gecersiz");
    }

    #[test]
    fn bilinen_adler32() {
        // RFC 1950 örneği: "Wikipedia" -> 0x11E60398
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn bilinen_crc32() {
        // "123456789" için IEEE CRC-32 = 0xCBF43926
        assert_eq!(crc32(0xffff_ffff, b"123456789") ^ 0xffff_ffff, 0xCBF4_3926);
    }

    #[test]
    fn tek_bloktan_buyuk_veri_bolunuyor() {
        // 200x200 RGBA + süzgeç baytları 65535'i aşıyor: birden çok zlib
        // bloğu gerekiyor. Bu yol bozulursa PNG sessizce geçersiz olur.
        let png = kodla(200, 200, &vec![7u8; 200 * 200 * 4]).unwrap();
        assert!(png.len() > 160_000);
    }
}
