//! TCP ve çokluortam zamanlayıcı ayarları — hepsi geri alınabilir.
//!
//! Buradaki her yazma `registry::dword_yaz` üzerinden gidiyor ve bir `Undo`
//! döndürüyor. `docs/RISKS.md` içindeki "registry / TCP tuning geri alma"
//! riskinin karşılığı bu: eski değeri okumadan yazan tek bir satır yok, ve
//! "değer yoktu" ile "değer 0'dı" ayrımı `Option<u32>` ile taşınıyor.
//!
//! ## Ayarların ne yaptığı (ve ne yapmadığı)
//!
//! - **Nagle (`TcpAckFrequency`, `TCPNoDelay`)**: Küçük paketlerin
//!   birleştirilmeden hemen gönderilmesini sağlıyor. Oyun trafiği küçük ve
//!   sık paketlerden oluştuğu için bekletilmemesi mantıklı. Bant genişliğini
//!   artırmıyor, paketlerin gönderilme anını değiştiriyor.
//! - **`NetworkThrottlingIndex`**: Windows'un çokluortam oynatımı sırasında
//!   ağ paketlerini saniyede ~10.000 ile sınırlayan mekanizması. Kapatmak
//!   oyunlarda bu sınırı kaldırıyor.
//! - **`SystemResponsiveness`**: Çokluortam zamanlayıcısının arka plan
//!   görevlerine ayırdığı CPU payı.
//!
//! Hiçbiri "ping'i düşürmüyor". Yaptıkları şey, Windows'un paketleri
//! bekletmesini engellemek. `DESIGN_PRINCIPLES.md` madde 4 gereği arayüz ve
//! günlük metinleri bu dili kullanıyor.

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::ledger::Undo;
use crate::registry::{self, Kok};

/// Ağ arayüzlerinin TCP parametreleri.
const ARAYUZLER: &str = "SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters\\Interfaces";

/// Çokluortam zamanlayıcısının sistem profili.
const SISTEM_PROFILI: &str =
    "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Multimedia\\SystemProfile";

/// Uygulanabilecek ayarlar ve o an ne durumda oldukları.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TcpDurumu {
    /// Nagle kapatması uygulanmış arayüz sayısı / toplam arayüz sayısı.
    pub nagle_kapali_arayuz: usize,
    pub toplam_arayuz: usize,
    /// `NetworkThrottlingIndex` değeri. `None` = ayar yok (Windows varsayılanı).
    pub throttling_index: Option<u32>,
    pub sistem_yanit: Option<u32>,
    /// Bu ayarlar yönetici yetkisi istiyor; şu an var mı?
    pub yonetici_gerekiyor: bool,
}

/// Ağ arayüzü GUID'leri.
///
/// Yalnızca IP adresi ya da DHCP kaydı olanlar alınıyor: registry'de
/// kullanılmayan eski arayüz kayıtları da duruyor ve onlara yazmak hiçbir işe
/// yaramadığı gibi defteri gereksiz kayıtla şişirir.
pub fn arayuzler() -> Result<Vec<String>> {
    let hepsi = registry::alt_anahtarlar(Kok::Makine, ARAYUZLER)?;
    let mut aktif = Vec::new();
    for guid in hepsi {
        let yol = format!("{ARAYUZLER}\\{guid}");
        let dhcp = registry::dword_oku(Kok::Makine, &yol, "EnableDHCP").unwrap_or(None);
        let statik = registry::metin_oku(Kok::Makine, &yol, "IPAddress").unwrap_or(None);
        if dhcp.is_some() || statik.is_some() {
            aktif.push(guid);
        }
    }
    Ok(aktif)
}

/// Nagle algoritmasını tüm aktif arayüzlerde kapatır.
///
/// Kısmi başarı kabul: bir arayüze yazılamazsa diğerleri yine de uygulanıyor
/// ve başarılı olanların geri alma kayıtları döndürülüyor. Hata listesi ayrı
/// dönüyor ki günlükte görünsün.
pub fn nagle_kapat() -> Result<(Vec<Undo>, Vec<String>)> {
    let mut kayitlar = Vec::new();
    let mut hatalar = Vec::new();

    for guid in arayuzler()? {
        let yol = format!("{ARAYUZLER}\\{guid}");
        for (ad, deger) in [("TcpAckFrequency", 1u32), ("TCPNoDelay", 1u32)] {
            match registry::dword_yaz(Kok::Makine, &yol, ad, deger) {
                Ok(u) => kayitlar.push(u),
                Err(e) => hatalar.push(format!("{guid}/{ad}: {}", crate::error::tek_satir(&e))),
            }
        }
    }
    Ok((kayitlar, hatalar))
}

/// Çokluortam ağ kısıtlamasını kaldırır.
///
/// `0xFFFFFFFF` "kısıtlama yok" anlamına gelen dokümante edilmiş değer.
pub fn throttling_kaldir() -> Result<Vec<Undo>> {
    Ok(vec![
        registry::dword_yaz(
            Kok::Makine,
            SISTEM_PROFILI,
            "NetworkThrottlingIndex",
            0xFFFF_FFFF,
        )?,
        // `SystemResponsiveness` 10: arka plana ayrılan pay. 0 yazmak Windows
        // tarafından zaten 10 gibi işleniyor ve bazı sürücülerle sorun
        // çıkarıyor; dokümante edilen en düşük anlamlı değer kullanılıyor.
        registry::dword_yaz(Kok::Makine, SISTEM_PROFILI, "SystemResponsiveness", 10)?,
    ])
}

/// Şu anki durumu okur — hiçbir şey değiştirmez.
pub fn durum() -> Result<TcpDurumu> {
    let arayuz_listesi = arayuzler().unwrap_or_default();
    let mut nagle_kapali = 0usize;

    for guid in &arayuz_listesi {
        let yol = format!("{ARAYUZLER}\\{guid}");
        let ack = registry::dword_oku(Kok::Makine, &yol, "TcpAckFrequency").unwrap_or(None);
        let nodelay = registry::dword_oku(Kok::Makine, &yol, "TCPNoDelay").unwrap_or(None);
        if ack == Some(1) && nodelay == Some(1) {
            nagle_kapali += 1;
        }
    }

    Ok(TcpDurumu {
        nagle_kapali_arayuz: nagle_kapali,
        toplam_arayuz: arayuz_listesi.len(),
        throttling_index: registry::dword_oku(
            Kok::Makine,
            SISTEM_PROFILI,
            "NetworkThrottlingIndex",
        )
        .unwrap_or(None),
        sistem_yanit: registry::dword_oku(Kok::Makine, SISTEM_PROFILI, "SystemResponsiveness")
            .unwrap_or(None),
        yonetici_gerekiyor: !yonetici_mi(),
    })
}

/// Program yönetici yetkisiyle mi çalışıyor?
#[cfg(windows)]
pub fn yonetici_mi() -> bool {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::Security::TOKEN_QUERY;
    use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    let mut jeton = HANDLE::default();
    // SAFETY: kendi sürecimizin jetonunu açıyoruz; çıktı yerel.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut jeton) }.is_err() {
        return false;
    }
    let jeton = crate::winutil::Tanitici(jeton);

    let mut yukseltme = TOKEN_ELEVATION::default();
    let mut boy = std::mem::size_of::<TOKEN_ELEVATION>() as u32;
    // SAFETY: tampon `TOKEN_ELEVATION` boyutunda ve `boy` onu bildiriyor.
    let ok = unsafe {
        GetTokenInformation(
            jeton.0,
            TokenElevation,
            Some(&mut yukseltme as *mut _ as *mut _),
            boy,
            &mut boy,
        )
    }
    .is_ok();

    ok && yukseltme.TokenIsElevated != 0
}

#[cfg(not(windows))]
pub fn yonetici_mi() -> bool {
    false
}

/// Ayarların kullanıcıya nasıl anlatıldığı.
///
/// Metinler burada, arayüzde değil: `DESIGN_PRINCIPLES.md` madde 4'e karşı
/// gözden geçirmenin tek bir yerde yapılabilmesi için.
pub const ACIKLAMALAR: &[(&str, &str)] = &[
    (
        "Nagle birleştirmesi",
        "Küçük paketlerin gönderilmeden önce birleştirilmesini engeller. Oyun trafiği küçük ve sık paketlerden oluşuyor; beklemeden gönderilmeleri gecikme oynamasını azaltabilir.",
    ),
    (
        "Ağ kısıtlaması",
        "Windows, çokluortam oynatımı sırasında ağ paketlerini sınırlıyor. Bu sınır kaldırılır.",
    ),
    (
        "Sistem yanıt payı",
        "Çokluortam zamanlayıcısının arka plan görevlerine ayırdığı payı düşürür.",
    ),
];

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn aciklamalarda_sayisal_vaat_yok() {
        // Tasarım ilkesi 4'ün otomatik kontrolü. Metne "%20 hızlanır" gibi
        // bir cümle sızarsa test düşer.
        let yasakli = ["% ", "ms düşür", "kat hızl", "garanti", "artırır"];
        for (baslik, metin) in ACIKLAMALAR {
            for y in yasakli {
                assert!(
                    !metin.to_lowercase().contains(y),
                    "'{baslik}' açıklaması sayısal/garantili vaat içeriyor: {y}"
                );
            }
        }
    }

    #[test]
    fn registry_yollari_makine_kokunde() {
        // Bu ayarlar sistem geneli; kullanıcı köküne yazmak sessizce
        // etkisiz kalırdı.
        assert!(ARAYUZLER.starts_with("SYSTEM\\"));
        assert!(SISTEM_PROFILI.starts_with("SOFTWARE\\"));
    }

    #[test]
    fn durum_okumasi_cokmuyor() {
        // Yönetici olmadan da okuma çalışmalı: arayüz açılışta durumu
        // gösterebilmeli, kullanıcı yükseltme yapmadan da ne olduğunu
        // görebilmeli.
        let d = durum().expect("durum okuması hata vermemeli");
        assert!(d.nagle_kapali_arayuz <= d.toplam_arayuz);
    }
}
