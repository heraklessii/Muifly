//! DNS ölçümü: hangi çözümleyici bu bağlantıda daha hızlı cevap veriyor.
//!
//! Ölçüm **gerçek bir DNS sorgusu** ile yapılıyor, ICMP ping ile değil. Fark
//! önemli: bir çözümleyici ping'e hızlı cevap verip sorguya yavaş cevap
//! verebilir (ping'ler sınırda karşılanır, sorgu arkadaki çözümleyiciye
//! gider). Kullanıcının hissettiği gecikme ikincisi.
//!
//! Sorgu paketi elle kuruluyor; bir DNS kütüphanesi eklenmedi. Gereken şey
//! tek bir A kaydı sorgusu ve cevabın gelip gelmediği — tam bir çözümleyici
//! değil. Paket kurma ve ayrıştırma bu dosyada saf fonksiyonlar ve
//! doğrudan test ediliyor.
//!
//! ## DNS ayarını Muifly değiştirmiyor
//!
//! Program en hızlı çözümleyiciyi **buluyor ve öneriyor**, sistem ayarını
//! kendiliğinden değiştirmiyor. Sebep `docs/decisions.md` #6'da: adaptör
//! seviyesinde DNS değiştirmenin güvenilir yolu `netsh`, geri alması ise
//! adaptörün önceki durumunun (statik liste mi DHCP mi) doğru okunmasına
//! bağlı. Bu okuma yanlış olursa kullanıcı internetsiz kalıyor ve programın
//! geri alma vaadi tam da en kötü anda tutmuyor. Ölçüp önermek, yanlış
//! uygulamaktan iyi.

use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Karşılaştırılan genel çözümleyiciler.
///
/// Liste kısa tutuldu: on tane sunucuyu sırayla denemek ölçümü uzatıyor ve
/// kullanıcıya karar verdirmiyor. Hepsi ücretsiz ve kayıt gerektirmiyor.
pub const BILINEN_COZUMLEYICILER: &[(&str, &str)] = &[
    ("Cloudflare", "1.1.1.1"),
    ("Google", "8.8.8.8"),
    ("Quad9", "9.9.9.9"),
    ("OpenDNS", "208.67.222.222"),
    ("AdGuard", "94.140.14.14"),
];

/// Ölçümde sorulan alan adı.
///
/// Her turda farklı bir ad kullanılıyor olsaydı önbellek etkisi ölçüme
/// karışırdı; sabit ve yaygın bir ad, her çözümleyicide büyük olasılıkla
/// önbellekte — yani ölçülen şey ağ yolu, çözümleyicinin yükü değil.
const OLCUM_ADI: &str = "example.com";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsSonucu {
    pub ad: String,
    pub adres: String,
    /// Ortalama cevap süresi. Hiç cevap gelmediyse `None`.
    pub ortalama_ms: Option<f32>,
    /// Cevap gelen deneme sayısı / toplam deneme.
    pub cevap: u32,
    pub deneme: u32,
    /// Bu, sistemin şu an kullandığı çözümleyici mi?
    pub sistemin_kullandigi: bool,
}

/// DNS sorgu paketi kurar (A kaydı, recursion desired).
///
/// `kimlik` çağıran tarafından veriliyor: cevabın aynı sorguya ait olduğunu
/// doğrulamak için gerekiyor ve testte sabitlenebilmesi lazım.
pub fn sorgu_paketi(ad: &str, kimlik: u16) -> Result<Vec<u8>> {
    let mut p = Vec::with_capacity(64);
    p.extend_from_slice(&kimlik.to_be_bytes());
    // 0x0100: standart sorgu, recursion desired.
    p.extend_from_slice(&0x0100u16.to_be_bytes());
    p.extend_from_slice(&1u16.to_be_bytes()); // QDCOUNT
    p.extend_from_slice(&0u16.to_be_bytes()); // ANCOUNT
    p.extend_from_slice(&0u16.to_be_bytes()); // NSCOUNT
    p.extend_from_slice(&0u16.to_be_bytes()); // ARCOUNT

    for etiket in ad.split('.') {
        if etiket.is_empty() {
            continue;
        }
        if etiket.len() > 63 {
            return Err(Error::Network(format!("DNS etiketi çok uzun: {etiket}")));
        }
        p.push(etiket.len() as u8);
        p.extend_from_slice(etiket.as_bytes());
    }
    p.push(0); // kök etiket

    p.extend_from_slice(&1u16.to_be_bytes()); // QTYPE = A
    p.extend_from_slice(&1u16.to_be_bytes()); // QCLASS = IN
    Ok(p)
}

/// Cevabın beklenen sorguya ait olup olmadığını söyler.
///
/// İçerik ayrıştırılmıyor: ölçülen şey cevabın gelme süresi, cevabın doğru
/// IP'yi taşıyıp taşımadığı değil. Kimlik ve QR biti yeterli.
pub fn cevap_gecerli(paket: &[u8], beklenen_kimlik: u16) -> bool {
    if paket.len() < 12 {
        return false;
    }
    let kimlik = u16::from_be_bytes([paket[0], paket[1]]);
    let bayraklar = u16::from_be_bytes([paket[2], paket[3]]);
    // En üst bit QR: 1 = cevap.
    kimlik == beklenen_kimlik && (bayraklar & 0x8000) != 0
}

/// Tek bir çözümleyiciye tek sorgu; turu milisaniye olarak döner.
fn tek_sorgu(adres: IpAddr, zaman_asimi: Duration) -> Result<f32> {
    let soket = UdpSocket::bind(if adres.is_ipv4() {
        "0.0.0.0:0"
    } else {
        "[::]:0"
    })
    .map_err(|e| Error::Network(format!("soket açılamadı: {e}")))?;
    soket
        .set_read_timeout(Some(zaman_asimi))
        .map_err(|e| Error::Network(format!("zaman aşımı ayarlanamadı: {e}")))?;

    // Kimlik rastgele: aynı kimlikle art arda sorgu, ara yollardaki
    // önbelleklerin cevabı eşleştirmesine yol açabilir.
    let kimlik = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0)
        % 65536) as u16;

    let paket = sorgu_paketi(OLCUM_ADI, kimlik)?;
    let hedef = SocketAddr::new(adres, 53);

    let baslangic = Instant::now();
    soket
        .send_to(&paket, hedef)
        .map_err(|e| Error::Network(format!("gönderilemedi: {e}")))?;

    let mut tampon = [0u8; 512];
    loop {
        let (n, kimden) = soket
            .recv_from(&mut tampon)
            .map_err(|e| Error::Network(format!("cevap gelmedi: {e}")))?;
        // Başka bir kaynaktan gelen paketi saymıyoruz.
        if kimden.ip() != adres {
            continue;
        }
        if cevap_gecerli(&tampon[..n], kimlik) {
            return Ok(baslangic.elapsed().as_secs_f32() * 1000.0);
        }
        // Kimlik tutmuyorsa eski bir cevap; beklemeye devam (zaman aşımı
        // döngüyü zaten sonlandırır).
    }
}

/// Bir çözümleyiciyi `deneme` kez ölçer.
pub fn cozumleyici_olc(
    ad: &str,
    adres_metni: &str,
    deneme: u32,
    zaman_asimi_ms: u64,
    sistemin_kullandigi: bool,
) -> DnsSonucu {
    let adres: Option<IpAddr> = adres_metni.parse().ok();
    let mut toplam = 0.0f32;
    let mut cevap = 0u32;

    if let Some(adres) = adres {
        for _ in 0..deneme {
            if let Ok(ms) = tek_sorgu(adres, Duration::from_millis(zaman_asimi_ms)) {
                toplam += ms;
                cevap += 1;
            }
        }
    }

    DnsSonucu {
        ad: ad.to_string(),
        adres: adres_metni.to_string(),
        ortalama_ms: if cevap == 0 {
            None
        } else {
            Some(toplam / cevap as f32)
        },
        cevap,
        deneme,
        sistemin_kullandigi,
    }
}

/// Bilinen çözümleyicileri ve sistemin kendi çözümleyicilerini ölçer.
///
/// Sıralama: cevap verenler süreye göre, cevap vermeyenler sona. Cevap
/// vermeyen bir çözümleyici "sonsuz hızlı" gibi listenin başına çıkmamalı.
pub fn karsilastir(deneme: u32, zaman_asimi_ms: u64) -> Vec<DnsSonucu> {
    let sistem = sistem_cozumleyicileri();
    let mut sonuclar = Vec::new();

    // Sistemin kendi çözümleyicisi listede yoksa ayrıca ölçülüyor: kullanıcı
    // "şu an kullandığım ne kadar hızlı" sorusunun cevabını görmeden karar
    // veremez.
    for adres in &sistem {
        if !BILINEN_COZUMLEYICILER.iter().any(|(_, a)| a == adres) {
            sonuclar.push(cozumleyici_olc(
                "Mevcut (ISS)",
                adres,
                deneme,
                zaman_asimi_ms,
                true,
            ));
        }
    }

    for (ad, adres) in BILINEN_COZUMLEYICILER {
        let kullaniliyor = sistem.iter().any(|s| s == adres);
        sonuclar.push(cozumleyici_olc(
            ad,
            adres,
            deneme,
            zaman_asimi_ms,
            kullaniliyor,
        ));
    }

    sirala(&mut sonuclar);
    sonuclar
}

/// Cevap verenler önce ve hızlıya göre; cevapsızlar sonda.
pub fn sirala(sonuclar: &mut [DnsSonucu]) {
    sonuclar.sort_by(|a, b| match (a.ortalama_ms, b.ortalama_ms) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
}

/// Sistemin şu an kullandığı DNS sunucuları.
#[cfg(windows)]
pub fn sistem_cozumleyicileri() -> Vec<String> {
    use windows::Win32::NetworkManagement::IpHelper::{GetNetworkParams, FIXED_INFO_W2KSP1};

    let mut boy = 0u32;
    // İlk çağrı boyut için; hata dönmesi bekleniyor.
    // SAFETY: tampon `None`, yalnızca boyut isteniyor.
    let _ = unsafe { GetNetworkParams(None, &mut boy) };
    if boy == 0 {
        return Vec::new();
    }

    let mut tampon = vec![0u8; boy as usize];
    // SAFETY: tampon `boy` kadar ayrıldı; yapı değişken uzunluklu bağlı liste
    // içeriyor ve tamponun içinde kalıyor.
    if unsafe {
        GetNetworkParams(
            Some(tampon.as_mut_ptr() as *mut FIXED_INFO_W2KSP1),
            &mut boy,
        )
    } != windows::Win32::Foundation::ERROR_SUCCESS
    {
        return Vec::new();
    }

    let mut adresler = Vec::new();
    // SAFETY: tampon `FIXED_INFO_W2KSP1` ile başlıyor.
    let bilgi = unsafe { &*(tampon.as_ptr() as *const FIXED_INFO_W2KSP1) };

    let mut dugum: *const windows::Win32::NetworkManagement::IpHelper::IP_ADDR_STRING =
        &bilgi.DnsServerList;
    while !dugum.is_null() {
        // SAFETY: düğüm ya gömülü ilk kayıt ya da API'nin ayırdığı bağlı
        // liste elemanı; her ikisi de çağrı boyunca geçerli.
        let d = unsafe { &*dugum };
        // `IP_ADDRESS_STRING` alanı C'de `char[16]`; Rust'ta `i8` diziliyor.
        // ASCII olduğu için baytlara yeniden yorumlamak güvenli.
        let ham = &d.IpAddress.String;
        let bayt: Vec<u8> = ham
            .iter()
            .take_while(|c| **c != 0)
            .map(|c| *c as u8)
            .collect();
        let metin = String::from_utf8_lossy(&bayt).to_string();
        if !metin.is_empty() {
            adresler.push(metin);
        }
        dugum = d.Next;
    }
    adresler
}

#[cfg(not(windows))]
pub fn sistem_cozumleyicileri() -> Vec<String> {
    Vec::new()
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn sorgu_paketi_basligi_dogru() {
        let p = sorgu_paketi("example.com", 0x1234).unwrap();
        assert_eq!(&p[0..2], &[0x12, 0x34], "kimlik");
        assert_eq!(&p[2..4], &[0x01, 0x00], "recursion desired");
        assert_eq!(&p[4..6], &[0x00, 0x01], "tek soru");
        assert_eq!(&p[6..12], &[0, 0, 0, 0, 0, 0], "cevap bölümleri boş");
    }

    #[test]
    fn sorgu_paketi_adi_etiketliyor() {
        let p = sorgu_paketi("example.com", 1).unwrap();
        // 12 bayt başlıktan sonra: 7 'example' 3 'com' 0
        assert_eq!(p[12], 7);
        assert_eq!(&p[13..20], b"example");
        assert_eq!(p[20], 3);
        assert_eq!(&p[21..24], b"com");
        assert_eq!(p[24], 0);
        // Sonda QTYPE=A, QCLASS=IN.
        assert_eq!(&p[25..29], &[0, 1, 0, 1]);
    }

    #[test]
    fn cok_uzun_etiket_reddediliyor() {
        let uzun = "a".repeat(64);
        assert!(sorgu_paketi(&uzun, 1).is_err());
    }

    #[test]
    fn cevap_kimligi_dogrulaniyor() {
        let mut p = vec![0u8; 12];
        p[0..2].copy_from_slice(&0x1234u16.to_be_bytes());
        p[2] = 0x80; // QR = 1
        assert!(cevap_gecerli(&p, 0x1234));
        assert!(
            !cevap_gecerli(&p, 0x9999),
            "başka sorgunun cevabı sayılmamalı"
        );
    }

    #[test]
    fn sorgu_cevap_sayilmiyor() {
        // QR biti 0: bu bir sorgu, cevap değil.
        let mut p = vec![0u8; 12];
        p[0..2].copy_from_slice(&0x1234u16.to_be_bytes());
        assert!(!cevap_gecerli(&p, 0x1234));
    }

    #[test]
    fn kirik_paket_cokmuyor() {
        assert!(!cevap_gecerli(&[], 1));
        assert!(!cevap_gecerli(&[0, 1, 2], 1));
    }

    #[test]
    fn cevapsiz_cozumleyici_listenin_basina_cikmiyor() {
        let mut s = vec![
            DnsSonucu {
                ad: "Cevapsız".into(),
                adres: "10.0.0.1".into(),
                ortalama_ms: None,
                cevap: 0,
                deneme: 3,
                sistemin_kullandigi: false,
            },
            DnsSonucu {
                ad: "Yavaş".into(),
                adres: "1.1.1.1".into(),
                ortalama_ms: Some(90.0),
                cevap: 3,
                deneme: 3,
                sistemin_kullandigi: false,
            },
            DnsSonucu {
                ad: "Hızlı".into(),
                adres: "8.8.8.8".into(),
                ortalama_ms: Some(12.0),
                cevap: 3,
                deneme: 3,
                sistemin_kullandigi: false,
            },
        ];
        sirala(&mut s);
        assert_eq!(s[0].ad, "Hızlı");
        assert_eq!(s[1].ad, "Yavaş");
        assert_eq!(s[2].ad, "Cevapsız", "cevap vermeyen sonda olmalı");
    }

    #[test]
    fn bilinen_cozumleyici_adresleri_gecerli() {
        for (ad, adres) in BILINEN_COZUMLEYICILER {
            assert!(
                adres.parse::<IpAddr>().is_ok(),
                "{ad} adresi ayrıştırılamıyor: {adres}"
            );
        }
    }

    #[test]
    fn cozumleyici_listesinde_tekrar_yok() {
        let mut gorulen = std::collections::HashSet::new();
        for (_, adres) in BILINEN_COZUMLEYICILER {
            assert!(gorulen.insert(*adres), "{adres} listede iki kez var");
        }
    }
}
