//! Defterdeki kayıtları gerçekten geri alan taraf.
//!
//! `ledger.rs` neyin eski değerinin ne olduğunu tutuyor; burası onu sisteme
//! geri yazıyor. Ayrım bilinçli: defter, geri alma kodunu tanımadan
//! serileştirilebiliyor ve çökme sonrası bambaşka bir program oturumunda
//! okunabiliyor.
//!
//! ## Kısmi başarısızlık nasıl ele alınıyor
//!
//! Bir kaydın geri alınamaması diğerlerini durdurmuyor. Başarısız kayıt
//! deftere **iade ediliyor**: sistemde duran bir değişiklik, kullanıcı
//! arayüzünde görünmeye devam etmeli. Sessizce düşürmek, "her şey geri
//! alındı" diyip bir şeyi bırakmak olurdu.

use crate::error::{Error, Result};
use crate::ledger::{Defter, Kapsam, Kayit, Undo};
use crate::monitor::{Gunluk, Kategori};
use crate::{registry, system_boost};

/// Tek bir kaydı geri alır.
pub fn tek(undo: &Undo) -> Result<()> {
    match undo {
        Undo::SurecOnceligi { pid, surec, onceki } => {
            kimlik_dogrula(*pid, surec)?;
            system_boost::priority::oncelik_ham_yaz(*pid, *onceki)
        }
        Undo::SurecAffinite { pid, surec, onceki } => {
            kimlik_dogrula(*pid, surec)?;
            system_boost::priority::affinite_ham_yaz(*pid, *onceki)
        }
        Undo::SurecDonduruldu { pid, surec } => system_boost::suspend::devam_ettir(*pid, surec),
        Undo::GucPlani { onceki_guid, .. } => system_boost::power::guid_ile_uygula(onceki_guid),
        Undo::RegistryDword { yol, ad, onceki } => registry::dword_geri_al(yol, ad, *onceki),
        Undo::RegistryAnahtari { yol } => {
            let (kok, alt) = registry::yolu_coz(yol)?;
            registry::anahtar_sil(kok, &alt)
        }
    }
}

/// PID yeniden kullanımına karşı kimlik kontrolü.
///
/// Süreç artık yoksa geri alınacak bir şey de yok: `Ok(())` dönüyor ve kayıt
/// defterden düşüyor. Süreç VAR ama başkasıysa hata dönüyor — yanlış sürecin
/// önceliğini değiştirmektense kaydı defterde bırakmak doğru.
fn kimlik_dogrula(pid: u32, beklenen: &str) -> Result<()> {
    match system_boost::detect::surec_adi(pid) {
        Ok(simdiki) if simdiki == beklenen => Ok(()),
        Ok(simdiki) => Err(Error::ProcessIdentityMismatch {
            pid,
            expected: beklenen.to_string(),
            actual: simdiki,
        }),
        // Süreç kapanmış.
        Err(_) => Err(Error::ProcessNotFound(pid)),
    }
}

/// Geri alma turunun sonucu.
#[derive(Debug, Default)]
pub struct Sonuc {
    pub geri_alinan: usize,
    /// Süreç kapandığı için gereksiz kalan kayıtlar. Hata değil.
    pub gereksiz: usize,
    /// (özet, sebep)
    pub basarisiz: Vec<(String, String)>,
}

/// Verilen kayıtları geri alır; başarısızları deftere iade eder.
///
/// `gunluk` her adımı yazıyor — geri alma da bir aksiyondur ve şeffaflık
/// ilkesi onu da kapsıyor.
pub fn kayitlari_isle(kayitlar: Vec<Kayit>, defter: &mut Defter, gunluk: &mut Gunluk) -> Sonuc {
    let mut sonuc = Sonuc::default();

    for kayit in kayitlar {
        match tek(&kayit.undo) {
            Ok(()) => {
                sonuc.geri_alinan += 1;
                gunluk.yaz(
                    crate::monitor::Duzey::GeriAlma,
                    kategori(&kayit.undo),
                    kayit.undo.geri_alma_ozeti(),
                    None,
                );
                gunluk.geri_alma_isaretle(kayit.id);
            }
            // Süreç kapanmışsa geri alınacak bir şey kalmamış demektir;
            // kayıt düşüyor ve kullanıcıya hata gösterilmiyor.
            Err(Error::ProcessNotFound(_)) => {
                sonuc.gereksiz += 1;
                gunluk.geri_alma_isaretle(kayit.id);
            }
            Err(e) => {
                let sebep = crate::error::tek_satir(&e);
                gunluk.hata(
                    kategori(&kayit.undo),
                    format!("geri alınamadı — {}: {sebep}", kayit.ozet),
                );
                sonuc.basarisiz.push((kayit.ozet.clone(), sebep));
                defter.iade(kayit);
            }
        }
    }
    sonuc
}

fn kategori(undo: &Undo) -> Kategori {
    match undo {
        Undo::SurecOnceligi { .. }
        | Undo::SurecAffinite { .. }
        | Undo::SurecDonduruldu { .. }
        | Undo::GucPlani { .. } => Kategori::Sistem,
        Undo::RegistryDword { .. } | Undo::RegistryAnahtari { .. } => Kategori::Ag,
    }
}

/// Oyun kapandığında: oturum kapsamındaki her şeyi geri al.
pub fn oturumu_kapat(defter: &mut Defter, gunluk: &mut Gunluk) -> Sonuc {
    let kayitlar = defter.kapsami_al(Kapsam::Oturum);
    if kayitlar.is_empty() {
        return Sonuc::default();
    }
    kayitlari_isle(kayitlar, defter, gunluk)
}

/// Kullanıcının "varsayılana dön" düğmesi: kalıcılar dahil her şey.
pub fn hepsini_geri_al(defter: &mut Defter, gunluk: &mut Gunluk) -> Sonuc {
    let kayitlar = defter.tumunu_al();
    kayitlari_isle(kayitlar, defter, gunluk)
}

/// Açılışta bekleyen kayıtları temizler.
///
/// Program çökerse ya da sonlandırılırsa oturum kapsamındaki kayıtlar
/// defterde asılı kalıyor. Bir sonraki açılışta bunlar geri alınıyor:
/// dondurulmuş süreçler devam ettiriliyor, güç planı geri yükleniyor.
///
/// **Kalıcı kayıtlara dokunulmuyor** — TCP ayarı gibi bilinçli değişiklikler,
/// program çöktü diye kendiliğinden kalkmamalı.
pub fn acilista_temizle(defter: &mut Defter, gunluk: &mut Gunluk) -> Sonuc {
    let bekleyen = defter.kapsami_al(Kapsam::Oturum);
    if bekleyen.is_empty() {
        return Sonuc::default();
    }

    gunluk.bilgi(
        Kategori::Uygulama,
        format!(
            "önceki oturumdan {} bekleyen değişiklik bulundu, geri alınıyor",
            bekleyen.len()
        ),
    );
    kayitlari_isle(bekleyen, defter, gunluk)
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn kategori_dogru_dagitiliyor() {
        assert_eq!(
            kategori(&Undo::SurecDonduruldu {
                pid: 1,
                surec: "a.exe".into()
            }),
            Kategori::Sistem
        );
        assert_eq!(
            kategori(&Undo::RegistryDword {
                yol: "HKLM\\X".into(),
                ad: "Y".into(),
                onceki: None
            }),
            Kategori::Ag
        );
    }

    #[test]
    fn bos_defterde_oturum_kapatma_cokmuyor() {
        let mut d = Defter::bellekte();
        let mut g = Gunluk::yeni();
        let s = oturumu_kapat(&mut d, &mut g);
        assert_eq!(s.geri_alinan, 0);
        assert_eq!(g.uzunluk(), 0, "boş turda günlüğe satır yazılmamalı");
    }

    #[test]
    fn kapanan_surec_kaydi_hata_uretmiyor() {
        // Var olmayan bir PID: süreç kapanmış demek, kullanıcıya hata
        // gösterilmemeli ve kayıt defterde kalmamalı.
        let mut d = Defter::bellekte();
        let mut g = Gunluk::yeni();
        d.kaydet(
            "hayalet süreç",
            Kapsam::Oturum,
            Undo::SurecDonduruldu {
                // Windows PID'leri 4'ün katı; bu değerin canlı olma olasılığı
                // pratikte yok.
                pid: 0xFFFF_FFF0,
                surec: "olmayan.exe".into(),
            },
        );

        let s = oturumu_kapat(&mut d, &mut g);
        assert!(d.bos_mu(), "kayıt defterden düşmeli");
        assert!(s.basarisiz.is_empty(), "kapanmış süreç hata sayılmamalı");
    }

    #[test]
    fn basarisiz_geri_alma_deftere_iade_ediliyor() {
        // Bozuk registry yolu: geri alma hata verecek ve kayıt iade edilecek.
        let mut d = Defter::bellekte();
        let mut g = Gunluk::yeni();
        d.kaydet(
            "bozuk yol",
            Kapsam::Kalici,
            Undo::RegistryAnahtari {
                yol: "GECERSIZKOK\\Bir\\Yol".into(),
            },
        );

        let s = hepsini_geri_al(&mut d, &mut g);
        assert_eq!(s.basarisiz.len(), 1);
        assert_eq!(d.liste().len(), 1, "kayıt kaybolmamalı");
        assert!(g.son(5).iter().any(|l| l.mesaj.contains("geri alınamadı")));
    }

    #[test]
    fn oturum_kapatma_kalici_kayitlara_dokunmuyor() {
        let mut d = Defter::bellekte();
        let mut g = Gunluk::yeni();
        d.kaydet(
            "TCP ayarı",
            Kapsam::Kalici,
            Undo::RegistryDword {
                yol: "HKCU\\Software\\MuiflyTest".into(),
                ad: "Yok".into(),
                onceki: None,
            },
        );

        oturumu_kapat(&mut d, &mut g);
        assert_eq!(d.liste().len(), 1, "kalıcı kayıt oturumla birlikte kalkmaz");
    }

    #[test]
    fn acilis_temizligi_kalicilari_birakiyor() {
        let mut d = Defter::bellekte();
        let mut g = Gunluk::yeni();
        d.kaydet(
            "kalıcı",
            Kapsam::Kalici,
            Undo::RegistryDword {
                yol: "HKCU\\Software\\MuiflyTest".into(),
                ad: "Yok".into(),
                onceki: None,
            },
        );
        d.kaydet(
            "oturumluk",
            Kapsam::Oturum,
            Undo::SurecDonduruldu {
                pid: 0xFFFF_FFF0,
                surec: "olmayan.exe".into(),
            },
        );

        acilista_temizle(&mut d, &mut g);
        assert_eq!(d.liste().len(), 1);
        assert_eq!(d.liste()[0].kapsam, Kapsam::Kalici);
    }
}
