//! Sistem Açılışı modu.
//!
//! Bu modun iki parçası var:
//!
//! 1. **Muifly'ın Windows ile başlaması.** `HKCU\...\Run` altına bir değer.
//!    Kullanıcı kökü bilinçli: `HKLM` yönetici yetkisi ister ve programın
//!    kendini başlatması için sistem geneli bir kayıt gereksiz geniş bir
//!    ayrıcalık olurdu (tasarım ilkesi 5).
//!
//! 2. **Açılışta güç planı.** `power.rs` üzerinden, `Kalici` kapsamda
//!    deftere yazılıyor.
//!
//! ## Servis geciktirme neden burada yok
//!
//! `docs/ROADMAP.md` Faz 1 kapsamında "startup servis gecikmesi" yazıyor.
//! Servisleri geciktirmek `HKLM\SYSTEM\CurrentControlSet\Services\<ad>`
//! altındaki `DelayedAutostart` / `Start` değerlerini değiştirmek demek ve
//! üç sorunu var:
//!
//! - Yanlış servisi geciktirmek (antivirüs, sürücü, VPN) makineyi
//!   açılışta savunmasız ya da çalışmaz bırakabilir.
//! - Servis listesi makineden makineye tamamen farklı; "gereksiz" kararını
//!   program veremez, kullanıcı da çoğu servisin ne olduğunu bilmez.
//! - Geri alma tek başına yetmiyor: kullanıcı arada servisi elle değiştirirse
//!   defterdeki eski değer artık doğru değil.
//!
//! Bu yüzden bilinçli olarak ertelendi. Ürünün "bir şeyi yapmıyorsak
//! yapmıyoruz deriz" duruşu gereği arayüzde de böyle görünüyor: özellik
//! gri değil, hiç yok.

use crate::error::Result;
use crate::registry::{self, Kok};

/// Windows'un çalıştırma listesi.
const RUN_YOLU: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const RUN_ADI: &str = "Muifly";

/// Program Windows ile başlıyor mu?
pub fn otomatik_baslatma_acik() -> Result<bool> {
    Ok(registry::metin_oku(Kok::Kullanici, RUN_YOLU, RUN_ADI)?.is_some())
}

/// Windows ile başlatmayı açar.
///
/// Yazılan komut satırı `--tepside` bayrağını taşıyor: açılışta pencere
/// açılmıyor, program doğrudan sistem tepsisine iniyor. Kullanıcının her
/// açılışta bir pencere kapatması gerekmemeli.
pub fn otomatik_baslatmayi_ac() -> Result<()> {
    let exe = std::env::current_exe()?;
    let komut = format!("\"{}\" --tepside", exe.display());
    registry::metin_yaz(Kok::Kullanici, RUN_YOLU, RUN_ADI, &komut)
}

pub fn otomatik_baslatmayi_kapat() -> Result<()> {
    registry::deger_sil(Kok::Kullanici, RUN_YOLU, RUN_ADI)
}

/// Kayıtlı komut satırı — arayüz "ne yazılmış" diye gösterebilsin
/// (şeffaflık ilkesi: kullanıcı registry'sine ne konduğunu görebilmeli).
pub fn otomatik_baslatma_komutu() -> Result<Option<String>> {
    registry::metin_oku(Kok::Kullanici, RUN_YOLU, RUN_ADI)
}

/// Servis geciktirme desteği — bilinçli olarak yok.
///
/// Fonksiyon duruyor ki arayüz "bu özellik yok" diyebilsin ve ileride
/// eklenirse tek bir yer değişsin.
pub fn servis_geciktirme_destegi() -> bool {
    false
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn run_yolu_kullanici_kokunde() {
        // Yönetici yetkisi gerektirmemesi bir tasarım kararı; yol yanlışlıkla
        // HKLM'e taşınırsa bu test düşmez ama sabit bir gözden geçirme
        // noktası bırakır.
        assert!(!RUN_YOLU.starts_with('\\'));
        assert!(RUN_YOLU.contains("CurrentVersion\\Run"));
    }

    #[test]
    fn servis_geciktirme_kapali() {
        assert!(!servis_geciktirme_destegi());
    }

    #[cfg(windows)]
    #[test]
    fn otomatik_baslatma_acilip_kapaniyor() {
        // Testten çıkarken sistemi bulduğu gibi bırakması gerekiyor.
        let baslangicta = otomatik_baslatma_acik().unwrap();

        otomatik_baslatmayi_ac().unwrap();
        assert!(otomatik_baslatma_acik().unwrap());
        let komut = otomatik_baslatma_komutu().unwrap().unwrap();
        assert!(komut.contains("--tepside"), "tepsi bayrağı yazılmalı");

        otomatik_baslatmayi_kapat().unwrap();
        assert!(!otomatik_baslatma_acik().unwrap());

        if baslangicta {
            otomatik_baslatmayi_ac().unwrap();
        }
    }
}
