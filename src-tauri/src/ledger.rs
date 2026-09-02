//! Geri alma defteri.
//!
//! Tasarım ilkesi 1 (tersine çevrilebilirlik) bu dosyada somutlaşıyor:
//! sistemde bir şey değiştiren **her** kod yolu, değişikliği yapmadan ÖNCE
//! eski değeri buraya yazmak zorunda. "Yeni değeri yaz, eskisini hatırlamaya
//! çalış" kabul edilmiyor.
//!
//! Defter diske yazılıyor. Sebep: program çökerse ya da kullanıcı Görev
//! Yöneticisinden sonlandırırsa, süreç içi bir liste ile birlikte geri alma
//! bilgisi de kaybolurdu ve sistem değiştirilmiş halde kalırdı. Diskteki
//! defter sayesinde bir sonraki açılışta bekleyen kayıtlar görülüp geri
//! alınabiliyor (`state.rs` icinde `acilista_temizle`).
//!
//! Bu dosya **veri**dir: `Undo` bir şeyi nasıl geri alacağını bilmez, sadece
//! neyin eski değerinin ne olduğunu taşır. Uygulayan taraf `revert.rs`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Bir kaydın ne zaman geri alınacağı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Kapsam {
    /// Oyun oturumuna bağlı. Oyun kapanınca otomatik geri alınır.
    /// Örnek: süreç önceliği, dondurulan arka plan süreçleri.
    Oturum,
    /// Kullanıcı açıkça geri alana kadar duran değişiklik.
    /// Örnek: TCP ayarı, açılış modu güç planı.
    Kalici,
}

/// Neyin eski değerinin ne olduğu.
///
/// Her varyant **kendi başına yeterli** olmak zorunda: geri alma anında
/// programın başka hiçbir yerinden bilgi okumaya ihtiyaç duymamalı, çünkü
/// geri alma bazen bambaşka bir program oturumunda (çökme sonrası) çalışıyor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tur", rename_all = "camelCase")]
pub enum Undo {
    /// `SetPriorityClass` öncesi sınıf.
    SurecOnceligi {
        pid: u32,
        /// PID yeniden kullanımına karşı kimlik kontrolü.
        surec: String,
        onceki: u32,
    },
    /// `SetProcessAffinityMask` öncesi maske.
    SurecAffinite {
        pid: u32,
        surec: String,
        onceki: u64,
    },
    /// Dondurulmuş süreç. Geri alma = devam ettir.
    SurecDonduruldu { pid: u32, surec: String },
    /// `PowerSetActiveScheme` öncesi plan.
    GucPlani {
        onceki_guid: String,
        onceki_ad: String,
    },
    /// Registry DWORD. `onceki: None` = değer o an YOKTU, geri alma = sil.
    /// Bu ayrım şart: var olmayan bir değeri 0 yazarak "geri almak" sistemi
    /// eski haline döndürmez, yeni bir duruma sokar.
    RegistryDword {
        yol: String,
        ad: String,
        onceki: Option<u32>,
    },
    /// Muifly'ın **kendi oluşturduğu** registry anahtarı (QoS ilkesi gibi).
    /// Geri alma = anahtarı sil.
    ///
    /// Yalnızca program tarafından oluşturulan anahtarlar için: `vardi: true`
    /// olan bir kayıt hiç yazılmıyor, çünkü önceden var olan bir anahtarın
    /// tüm içeriğini geri yükleyecek bir mekanizma yok ve yarım bir geri alma
    /// hiç geri almamaktan kötü.
    RegistryAnahtari { yol: String },
}

impl Undo {
    /// Kullanıcıya gösterilen "bu geri alınırsa ne olur" cümlesi.
    pub fn geri_alma_ozeti(&self) -> String {
        match self {
            Undo::SurecOnceligi { surec, onceki, .. } => {
                format!("{surec} önceliği {} sınıfına döner", oncelik_adi(*onceki))
            }
            Undo::SurecAffinite { surec, .. } => {
                format!("{surec} tüm çekirdeklere geri açılır")
            }
            Undo::SurecDonduruldu { surec, .. } => format!("{surec} devam ettirilir"),
            Undo::GucPlani { onceki_ad, .. } => format!("güç planı {onceki_ad} olur"),
            Undo::RegistryDword { ad, onceki, .. } => match onceki {
                Some(v) => format!("{ad} kaydı {v} değerine döner"),
                None => format!("{ad} kaydı silinir (önceden yoktu)"),
            },
            Undo::RegistryAnahtari { yol } => {
                let ad = yol.rsplit('\\').next().unwrap_or(yol);
                format!("{ad} ilkesi kaldırılır")
            }
        }
    }
}

/// `PRIORITY_CLASS` sabitlerinin okunur karşılığı.
pub fn oncelik_adi(sinif: u32) -> &'static str {
    match sinif {
        0x0000_0040 => "düşük",
        0x0000_4000 => "normalin altı",
        0x0000_0020 => "normal",
        0x0000_8000 => "normalin üstü",
        0x0000_0080 => "yüksek",
        0x0000_0100 => "gerçek zamanlı",
        _ => "bilinmeyen",
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Kayit {
    pub id: u64,
    /// Unix milisaniye. Arayüzde saat olarak gösteriliyor.
    pub zaman: i64,
    /// Değişikliğin kendisi ("Discord donduruldu"), geri alması değil.
    pub ozet: String,
    pub kapsam: Kapsam,
    pub undo: Undo,
}

/// Bekleyen geri alma kayıtları ve diske yazma.
#[derive(Debug, Default)]
pub struct Defter {
    kayitlar: Vec<Kayit>,
    sonraki_id: u64,
    yol: Option<PathBuf>,
}

impl Defter {
    /// Diske yazmayan defter — testler için.
    pub fn bellekte() -> Self {
        Self::default()
    }

    /// Verilen yoldaki defteri okur; dosya yoksa boş defterle başlar.
    ///
    /// Dosya BOZUKSA hata dönmüyor, boş defterle devam ediyor ve bozuk dosyayı
    /// `.bozuk` uzantısıyla saklıyor. Sebep: bozuk bir defter yüzünden program
    /// hiç açılmazsa kullanıcı geri alma arayüzüne de ulaşamaz — en kötü
    /// senaryo bu olurdu.
    pub fn yukle(yol: impl AsRef<Path>) -> Self {
        let yol = yol.as_ref().to_path_buf();
        let mut defter = match std::fs::read_to_string(&yol) {
            Ok(icerik) => match serde_json::from_str::<Vec<Kayit>>(&icerik) {
                Ok(kayitlar) => Defter {
                    sonraki_id: kayitlar.iter().map(|k| k.id).max().unwrap_or(0) + 1,
                    kayitlar,
                    yol: None,
                },
                Err(_) => {
                    let _ = std::fs::rename(&yol, yol.with_extension("bozuk"));
                    Defter::default()
                }
            },
            Err(_) => Defter::default(),
        };
        defter.yol = Some(yol);
        defter
    }

    /// Kaydı deftere ekler ve diske yazar; kayıt kimliğini döner.
    ///
    /// Diske yazma hatası **yutuluyor** ve `log` ile bildiriliyor: defterin
    /// diske yazılamaması, o an yapılmakta olan optimizasyonu iptal etmek
    /// için yeterli bir sebep değil — bellekteki kopya hâlâ geri alabilir.
    pub fn kaydet(&mut self, ozet: impl Into<String>, kapsam: Kapsam, undo: Undo) -> u64 {
        let id = self.sonraki_id;
        self.sonraki_id += 1;
        self.kayitlar.push(Kayit {
            id,
            zaman: chrono::Utc::now().timestamp_millis(),
            ozet: ozet.into(),
            kapsam,
            undo,
        });
        self.diske_yaz();
        id
    }

    pub fn liste(&self) -> &[Kayit] {
        &self.kayitlar
    }

    pub fn bos_mu(&self) -> bool {
        self.kayitlar.is_empty()
    }

    /// Belirli bir kaydı defterden çıkarır (geri alma başarılı olduğunda).
    pub fn dus(&mut self, id: u64) -> Option<Kayit> {
        let i = self.kayitlar.iter().position(|k| k.id == id)?;
        let kayit = self.kayitlar.remove(i);
        self.diske_yaz();
        Some(kayit)
    }

    /// Verilen kapsamdaki kayıtları defterden alır.
    ///
    /// Sıra TERS: en son yapılan değişiklik ilk geri alınıyor. Sıra önemli,
    /// çünkü değişiklikler birbirine bağlı olabilir (önce güç planı sonra
    /// öncelik değiştiyse, ters sırada geri almak ara durumları atlar).
    pub fn kapsami_al(&mut self, kapsam: Kapsam) -> Vec<Kayit> {
        let mut alinan = Vec::new();
        let mut kalan = Vec::new();
        for kayit in self.kayitlar.drain(..) {
            if kayit.kapsam == kapsam {
                alinan.push(kayit);
            } else {
                kalan.push(kayit);
            }
        }
        self.kayitlar = kalan;
        alinan.reverse();
        self.diske_yaz();
        alinan
    }

    /// Tüm kayıtları alır (ters sırada).
    pub fn tumunu_al(&mut self) -> Vec<Kayit> {
        let mut alinan = std::mem::take(&mut self.kayitlar);
        alinan.reverse();
        self.diske_yaz();
        alinan
    }

    /// Geri alması başarısız olan kaydı deftere iade eder.
    ///
    /// Kayıt kaybolmuyor: başarısız bir geri alma, sistemde duran bir
    /// değişiklik demek ve kullanıcı onu arayüzde görmeye devam etmeli.
    pub fn iade(&mut self, kayit: Kayit) {
        self.kayitlar.push(kayit);
        self.diske_yaz();
    }

    /// Defteri diske yazar.
    ///
    /// Yazma **atomik** (`settings::atomik_yaz`): yarım yazılmış bir defter,
    /// karar #3'ün korumak istediği şeyi tam da en gerekli anda — yazarken
    /// ölen bir programda — kaybettirirdi. Bozuk defter `.bozuk` diye kenara
    /// konuyor ve bekleyen geri almalar onunla birlikte gidiyor.
    fn diske_yaz(&self) {
        let Some(yol) = &self.yol else { return };
        match serde_json::to_string_pretty(&self.kayitlar) {
            Ok(metin) => {
                if let Err(e) = crate::settings::atomik_yaz(yol, &metin) {
                    log::warn!("geri alma defteri diske yazılamadı: {e}");
                }
            }
            Err(e) => log::warn!("geri alma defteri serileştirilemedi: {e}"),
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn ornek_undo(pid: u32) -> Undo {
        Undo::SurecDonduruldu {
            pid,
            surec: "discord.exe".into(),
        }
    }

    #[test]
    fn kayit_kimlikleri_artiyor() {
        let mut d = Defter::bellekte();
        let a = d.kaydet("a", Kapsam::Oturum, ornek_undo(1));
        let b = d.kaydet("b", Kapsam::Oturum, ornek_undo(2));
        assert_ne!(a, b);
        assert_eq!(d.liste().len(), 2);
    }

    #[test]
    fn kapsam_ayirimi_dogru() {
        let mut d = Defter::bellekte();
        d.kaydet("oturum", Kapsam::Oturum, ornek_undo(1));
        d.kaydet(
            "kalici",
            Kapsam::Kalici,
            Undo::RegistryDword {
                yol: "HKLM\\X".into(),
                ad: "Y".into(),
                onceki: None,
            },
        );

        let oturumluk = d.kapsami_al(Kapsam::Oturum);
        assert_eq!(oturumluk.len(), 1);
        // Kalıcı kayıt defterde kalmalı: oyun kapanması onu geri almaz.
        assert_eq!(d.liste().len(), 1);
        assert_eq!(d.liste()[0].kapsam, Kapsam::Kalici);
    }

    #[test]
    fn geri_alma_sirasi_ters() {
        let mut d = Defter::bellekte();
        d.kaydet("ilk", Kapsam::Oturum, ornek_undo(1));
        d.kaydet("ikinci", Kapsam::Oturum, ornek_undo(2));
        let sirali = d.tumunu_al();
        assert_eq!(sirali[0].ozet, "ikinci");
        assert_eq!(sirali[1].ozet, "ilk");
    }

    #[test]
    fn basarisiz_geri_alma_iade_ediliyor() {
        let mut d = Defter::bellekte();
        d.kaydet("a", Kapsam::Oturum, ornek_undo(1));
        let mut alinan = d.tumunu_al();
        assert!(d.bos_mu());
        d.iade(alinan.pop().unwrap());
        assert_eq!(d.liste().len(), 1);
    }

    #[test]
    fn disk_gidis_donusu() {
        let dizin = tempfile::tempdir().unwrap();
        let yol = dizin.path().join("defter.json");

        let mut d = Defter::yukle(&yol);
        d.kaydet("dondurma", Kapsam::Oturum, ornek_undo(4242));

        // Ayrı bir program oturumu gibi yeniden okuyoruz: çökme senaryosu.
        let d2 = Defter::yukle(&yol);
        assert_eq!(d2.liste().len(), 1);
        assert_eq!(d2.liste()[0].ozet, "dondurma");
        assert_eq!(d2.liste()[0].undo, ornek_undo(4242));
    }

    #[test]
    fn bozuk_defter_programi_kilitlemiyor() {
        let dizin = tempfile::tempdir().unwrap();
        let yol = dizin.path().join("defter.json");
        std::fs::write(&yol, "{ bu json degil").unwrap();

        let d = Defter::yukle(&yol);
        assert!(d.bos_mu());
        assert!(yol.with_extension("bozuk").exists());
    }

    #[test]
    fn kayip_registry_degeri_silme_olarak_ozetleniyor() {
        let u = Undo::RegistryDword {
            yol: "HKLM\\...".into(),
            ad: "TcpAckFrequency".into(),
            onceki: None,
        };
        assert!(u.geri_alma_ozeti().contains("silinir"));
    }
}
