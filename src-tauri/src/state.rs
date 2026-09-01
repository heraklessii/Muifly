//! Motor: modülleri birbirine bağlayan tek durum nesnesi.
//!
//! `system_boost` ve `network_boost` tek tek işleri yapıyor; burası "oyun
//! algılandı, şu profili uygula, kapanınca geri al" akışını kuruyor.
//!
//! ## Değişmez kural
//!
//! Sistemde bir şey değiştiren her yol şu üçlüyü birlikte yapıyor:
//! **uygula → deftere yaz → günlüğe yaz.** Üçünden biri eksik kalırsa ya geri
//! alma kaybolur ya da kullanıcı ne olduğunu göremez. Bu yüzden `uygula_ve_yaz`
//! yardımcısı var ve modüller doğrudan deftere yazmıyor.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::ledger::{Defter, Kapsam, Undo};
use crate::monitor::{self, Duzey, Gunluk, Kategori, Ornek, Ornekleyici, Ozet, Tampon};
use crate::profile_engine::{self, Mod, Profil};
use crate::settings::{self, Ayarlar};
use crate::system_boost::{self, GucPlani, Oncelik};
use crate::{network_boost, revert};

/// Arayüze giden tam durum.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Durum {
    pub mod_: Mod,
    pub mod_adi: String,
    pub ayarlar: Ayarlar,
    /// Bekleyen geri alma sayısı: arayüzdeki "geri al" rozetinin sayısı.
    pub bekleyen_geri_alma: usize,
    pub kalici_degisiklik: usize,
    /// Dondurma özelliği bu makinede kullanılabiliyor mu?
    pub dondurma_destegi: bool,
    pub yonetici: bool,
    pub cpu_hibrit: bool,
    pub mantiksal_cekirdek: u32,
    /// Aktif güç planının görünen adı.
    pub guc_plani: Option<String>,
    pub otomatik_baslatma: bool,
}

/// Bir profil uygulamasının kullanıcıya dönen özeti.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UygulamaSonucu {
    pub uygulanan: Vec<String>,
    pub atlanan: Vec<String>,
    pub hatalar: Vec<String>,
}

/// Bir mod geçişinin sonucu.
#[derive(Debug, Clone, PartialEq)]
pub struct ModDegisimi {
    pub yeni: Mod,
    /// Geçişte geri alınan oturumluk değişiklik sayısı (oyundan çıkışta).
    pub geri_alinan: usize,
}

/// Öncesi/sonrası karşılaştırması için işaretlenmiş an.
#[derive(Debug, Clone, Copy)]
struct Isaret {
    zaman: i64,
}

pub struct Motor {
    pub defter: Defter,
    pub gunluk: Gunluk,
    pub ayarlar: Ayarlar,
    pub profiller: Vec<Profil>,
    pub mod_: Mod,
    ornekler: Tampon,
    ornekleyici: Ornekleyici,
    /// Optimizasyon uygulandığı an — öncesi/sonrası karşılaştırması için.
    isaret: Option<Isaret>,
    /// Oyun önden çıktığında ilk fark edilen an; gecikme sayacı buradan işliyor.
    cikis_baslangici: Option<i64>,
    profil_dizini: PathBuf,
}

impl Motor {
    /// Diskteki durumu okur ve önceki oturumdan kalanları temizler.
    pub fn baslat() -> Self {
        let ayarlar = Ayarlar::yukle(&settings::ayar_yolu());
        let mut defter = Defter::yukle(settings::defter_yolu());
        let mut gunluk = Gunluk::yeni();

        gunluk.bilgi(Kategori::Uygulama, "Muifly başlatıldı");

        // Çökme sonrası bekleyen değişiklikler.
        revert::acilista_temizle(&mut defter, &mut gunluk);

        let profil_dizini = settings::profil_dizini();
        let (profiller, hatalar) = profile_engine::store::hepsini_yukle(&profil_dizini);
        for h in hatalar {
            gunluk.uyari(Kategori::Profil, h);
        }

        Self {
            defter,
            gunluk,
            ayarlar,
            profiller,
            mod_: Mod::Bosta,
            ornekler: Tampon::yeni(monitor::ORNEK_KAPASITESI),
            ornekleyici: Ornekleyici::yeni(),
            isaret: None,
            cikis_baslangici: None,
            profil_dizini,
        }
    }

    /// Uygula → deftere yaz → günlüğe yaz.
    ///
    /// Sıra sabit ve bu fonksiyonun dışına çıkmıyor. `sonuc` başarısızsa
    /// deftere hiçbir şey yazılmıyor: yapılmamış bir değişikliğin geri alma
    /// kaydı, ilerideki bir geri alma turunu boşuna hataya düşürürdü.
    fn uygula_ve_yaz(
        &mut self,
        kategori: Kategori,
        ozet: impl Into<String>,
        kapsam: Kapsam,
        sonuc: Result<Undo>,
        cikti: &mut UygulamaSonucu,
    ) {
        let ozet = ozet.into();
        match sonuc {
            Ok(undo) => {
                let id = self.defter.kaydet(ozet.clone(), kapsam, undo);
                self.gunluk.aksiyon(kategori, ozet.clone(), id);
                cikti.uygulanan.push(ozet);
            }
            Err(e) => {
                let mesaj = format!("{ozet} — {}", crate::error::tek_satir(&e));
                self.gunluk.uyari(kategori, mesaj.clone());
                cikti.hatalar.push(mesaj);
            }
        }
    }

    /// Bir profili verilen oyun sürecine uygular.
    pub fn profil_uygula(&mut self, profil: &Profil, pid: u32) -> UygulamaSonucu {
        let mut cikti = UygulamaSonucu::default();

        // Karşılaştırma için "öncesi" penceresini burada kapatıyoruz: bu andan
        // sonraki örnekler "sonrası".
        self.isaret = Some(Isaret {
            zaman: chrono::Utc::now().timestamp_millis(),
        });

        self.gunluk.bilgi(
            Kategori::Profil,
            format!("'{}' profili uygulanıyor", profil.display_name),
        );

        // 1. Oyun önceliği.
        match Oncelik::ayristir(&profil.system.priority_class) {
            Ok(oncelik) => {
                let sonuc = system_boost::priority::oncelik_ayarla(pid, oncelik);
                self.uygula_ve_yaz(
                    Kategori::Sistem,
                    format!(
                        "{} önceliği {} yapıldı",
                        profil
                            .executable_names
                            .first()
                            .cloned()
                            .unwrap_or_else(|| format!("pid {pid}")),
                        crate::ledger::oncelik_adi(oncelik.ham())
                    ),
                    Kapsam::Oturum,
                    sonuc,
                    &mut cikti,
                );
            }
            Err(e) => cikti.hatalar.push(crate::error::tek_satir(&e)),
        }

        // 2. CPU affinitesi — yalnızca kullanıcı açıkça istediyse ve CPU
        //    hibritse. `docs/RISKS.md`: yanlış affinite performansı düşürüyor.
        if profil.system.cpu_affinity == profile_engine::AffiniteTercihi::SadecePCore {
            match system_boost::priority::topoloji() {
                Ok(t) => match t.p_core_maskesi {
                    Some(maske) => {
                        let sonuc = system_boost::priority::affinite_ayarla(pid, maske);
                        self.uygula_ve_yaz(
                            Kategori::Sistem,
                            "oyun performans çekirdeklerine sabitlendi",
                            Kapsam::Oturum,
                            sonuc,
                            &mut cikti,
                        );
                    }
                    None => {
                        let m = "bu CPU hibrit değil, çekirdek sabitleme atlandı".to_string();
                        self.gunluk.bilgi(Kategori::Sistem, m.clone());
                        cikti.atlanan.push(m);
                    }
                },
                Err(e) => cikti.hatalar.push(crate::error::tek_satir(&e)),
            }
        }

        // 3. Güç planı.
        if let Some(plan_metni) = &profil.system.power_plan {
            match GucPlani::ayristir(plan_metni) {
                Ok(istenen) => match system_boost::power::plani_uygula(istenen) {
                    Ok((undo, uygulanan)) => {
                        // İstenen plan yoksa düşülen plan söyleniyor: sessiz
                        // bir geri düşüş, kullanıcının profilinde yazanla
                        // gerçeği ayırırdı.
                        if uygulanan != istenen {
                            self.gunluk.uyari(
                                Kategori::Sistem,
                                format!(
                                    "'{}' planı bu Windows'ta yok, '{}' uygulandı",
                                    istenen.ad(),
                                    uygulanan.ad()
                                ),
                            );
                        }
                        let ozet = format!("güç planı '{}' yapıldı", uygulanan.ad());
                        let id = self.defter.kaydet(ozet.clone(), Kapsam::Oturum, undo);
                        self.gunluk.aksiyon(Kategori::Sistem, ozet.clone(), id);
                        cikti.uygulanan.push(ozet);
                    }
                    Err(e) => {
                        let m = format!(
                            "güç planı değiştirilemedi — {}",
                            crate::error::tek_satir(&e)
                        );
                        self.gunluk.uyari(Kategori::Sistem, m.clone());
                        cikti.hatalar.push(m);
                    }
                },
                Err(e) => cikti.hatalar.push(crate::error::tek_satir(&e)),
            }
        }

        // 4. Arka plan süreçlerini dondur.
        let dondurulacaklar = profil.dondurulacaklar();
        if !dondurulacaklar.is_empty() {
            let sonuc = system_boost::suspend::listeyi_dondur(&dondurulacaklar, Some(pid));
            for (_, ad, undo) in sonuc.basarili {
                let ozet = format!("{ad} donduruldu");
                let id = self.defter.kaydet(ozet.clone(), Kapsam::Oturum, undo);
                self.gunluk.aksiyon(Kategori::Sistem, ozet.clone(), id);
                cikti.uygulanan.push(ozet);
            }
            for (ad, sebep) in sonuc.atlanan {
                let m = format!("{ad} atlandı: {sebep}");
                self.gunluk.bilgi(Kategori::Sistem, m.clone());
                cikti.atlanan.push(m);
            }
        }

        // 5. ve 6. adım ağ modülüne ait. Demo ikilisinde kapalı; profil
        // dosyasında istense bile uygulanmıyor ve atlandığı kullanıcıya
        // söyleniyor — sessizce atlamak, profilde yazanla gerçeği ayırırdı.
        let ag_acik = crate::surum::kisitlar().ag_modulu;
        if !ag_acik && (profil.network.qos_priority || profil.network.tcp_nodelay) {
            let m = "ağ ayarları atlandı: Network Boost demo sürümde kapalı".to_string();
            self.gunluk.bilgi(Kategori::Ag, m.clone());
            cikti.atlanan.push(m);
        }

        // 5. QoS ilkesi.
        if ag_acik && profil.network.qos_priority {
            if let Some(exe) = profil.executable_names.first() {
                let sonuc = network_boost::qos::ilke_olustur(exe);
                self.uygula_ve_yaz(
                    Kategori::Ag,
                    format!("{exe} için QoS ilkesi eklendi (oturum açılışında etkin olur)"),
                    // Kalıcı: ilke registry'de duruyor ve oyun kapanınca
                    // kalkması gerekmiyor. Kullanıcı isterse kaldırıyor.
                    Kapsam::Kalici,
                    sonuc,
                    &mut cikti,
                );
            }
        }

        // 6. TCP ayarları.
        if ag_acik && profil.network.tcp_nodelay {
            match network_boost::tcp::nagle_kapat() {
                Ok((kayitlar, hatalar)) => {
                    if !kayitlar.is_empty() {
                        let adet = kayitlar.len();
                        for undo in kayitlar {
                            self.defter
                                .kaydet(undo.geri_alma_ozeti(), Kapsam::Kalici, undo);
                        }
                        let ozet = format!("Nagle paket birleştirmesi kapatıldı ({adet} kayıt)");
                        self.gunluk
                            .yaz(Duzey::Aksiyon, Kategori::Ag, ozet.clone(), None);
                        cikti.uygulanan.push(ozet);
                    }
                    for h in hatalar {
                        self.gunluk.uyari(Kategori::Ag, h.clone());
                        cikti.hatalar.push(h);
                    }
                }
                Err(e) => {
                    let m = format!("TCP ayarı uygulanamadı — {}", crate::error::tek_satir(&e));
                    self.gunluk.uyari(Kategori::Ag, m.clone());
                    cikti.hatalar.push(m);
                }
            }
        }

        cikti
    }

    /// Oyun kapandı / öne başka bir şey geldi: oturumluk her şeyi geri al.
    pub fn oturumu_kapat(&mut self) -> revert::Sonuc {
        let sonuc = revert::oturumu_kapat(&mut self.defter, &mut self.gunluk);
        if sonuc.geri_alinan > 0 {
            self.gunluk.bilgi(
                Kategori::Sistem,
                format!("{} değişiklik geri alındı", sonuc.geri_alinan),
            );
        }
        self.isaret = None;
        sonuc
    }

    /// Öndeki uygulamaya, profili olmasa da hafif varsayılanı uygular.
    ///
    /// Hem arayüzdeki düğme hem tepsi menüsü buraya geliyor: uygulama yolu
    /// tek olmalı ki defter ve günlük kaydı hiçbir yoldan atlanamasın.
    pub fn ondekine_uygula(&mut self) -> Result<UygulamaSonucu> {
        let onde = system_boost::detect::ondeki_pencere()?;
        let profil = match profile_engine::store::eslesen(&self.profiller, &onde.ad).0 {
            Some(p) => p.clone(),
            None => profile_engine::genel_profil(&onde.ad).dogrula()?.0,
        };
        Ok(self.profil_uygula(&profil, onde.pid))
    }

    /// Kullanıcının "varsayılana dön" düğmesi.
    pub fn hepsini_geri_al(&mut self) -> revert::Sonuc {
        // Muifly'ın kendi QoS ilkeleri deftere yazılıyor ama defter kaybolmuş
        // olabilir (elle silinen dosya). Ön ekli ilkeler ayrıca süpürülüyor.
        if let Ok(adet) = network_boost::qos::bizim_ilkeleri_kaldir() {
            if adet > 0 {
                self.gunluk.yaz(
                    Duzey::GeriAlma,
                    Kategori::Ag,
                    format!("{adet} QoS ilkesi kaldırıldı"),
                    None,
                );
            }
        }
        revert::hepsini_geri_al(&mut self.defter, &mut self.gunluk)
    }

    /// Öndeki pencereyi okuyup mod geçişine karar verir.
    ///
    /// Dönen değer: mod değiştiyse `Some(ModDegisimi)`. Değişmediyse `None` —
    /// arayüze her saniye aynı olayı yollamamak için.
    ///
    /// Geri alınan değişiklik sayısı da dönüyor: bildirim metni "geri alındı"
    /// diyebilmek için gerçekten bir şeyin geri alındığını bilmek zorunda
    /// (şeffaflık ilkesi — olmayan bir işi bildirmek yanlış beyandır).
    pub fn mod_guncelle(&mut self) -> Option<ModDegisimi> {
        let onde = system_boost::detect::ondeki_pencere().ok();
        let yeni =
            profile_engine::mod_sec(onde.as_ref(), &self.profiller, self.ayarlar.rekabetci_mod);

        // Oyundan çıkış: gecikme sayacı. Alt+Tab yapan kullanıcı için tampon.
        let simdi = chrono::Utc::now().timestamp_millis();
        let onceki_oyunda = self.mod_.oyun_pid().is_some();
        let yeni_oyunda = yeni.oyun_pid().is_some();

        if onceki_oyunda && !yeni_oyunda {
            match self.cikis_baslangici {
                None => {
                    self.cikis_baslangici = Some(simdi);
                    return None;
                }
                Some(basladi) => {
                    let gecen = (simdi - basladi) / 1000;
                    if gecen < self.ayarlar.oyun_cikis_gecikmesi_sn as i64 {
                        return None;
                    }
                }
            }
        }
        self.cikis_baslangici = None;

        if yeni == self.mod_ {
            return None;
        }

        // Oyundan çıkılıyorsa oturumluk değişiklikleri geri al.
        let geri_alinan = if onceki_oyunda && !yeni_oyunda {
            self.oturumu_kapat().geri_alinan
        } else {
            0
        };

        self.gunluk.bilgi(
            Kategori::Profil,
            format!("mod: {} → {}", self.mod_.ad(), yeni.ad()),
        );
        self.mod_ = yeni.clone();
        Some(ModDegisimi { yeni, geri_alinan })
    }

    /// Bir ölçüm örneği alır.
    pub fn ornek_al(&mut self) -> Ornek {
        let gecikme = if self.ayarlar.gecikme_olcumu {
            network_boost::latency::olc(&self.ayarlar.gecikme_hedefi, 1000)
                .ok()
                .flatten()
        } else {
            None
        };
        let ornek = self.ornekleyici.ornek_al(gecikme);
        self.ornekler.ekle(ornek);
        ornek
    }

    pub fn ornekler(&self) -> Vec<Ornek> {
        self.ornekler.hepsi()
    }

    /// Optimizasyon öncesi ve sonrası özetleri.
    ///
    /// İşaret yoksa (henüz profil uygulanmadıysa) `None`: uydurulmuş bir
    /// "öncesi" göstermektense hiç göstermemek doğru.
    pub fn karsilastirma(&self) -> Option<monitor::Karsilastirma> {
        let isaret = self.isaret?;
        let hepsi = self.ornekler.hepsi();
        let (onceki, sonraki): (Vec<Ornek>, Vec<Ornek>) =
            hepsi.into_iter().partition(|o| o.zaman < isaret.zaman);

        if onceki.is_empty() || sonraki.is_empty() {
            return None;
        }

        Some(monitor::Karsilastirma {
            onceki_saniye: sure_saniye(&onceki),
            sonraki_saniye: sure_saniye(&sonraki),
            onceki: monitor::metrics::ozetle(&onceki),
            sonraki: monitor::metrics::ozetle(&sonraki),
        })
    }

    pub fn ozet(&self) -> Ozet {
        monitor::metrics::ozetle(&self.ornekler.hepsi())
    }

    pub fn durum(&self) -> Durum {
        let topoloji = system_boost::priority::topoloji().ok();
        Durum {
            mod_adi: self.mod_.ad().to_string(),
            mod_: self.mod_.clone(),
            ayarlar: self.ayarlar.clone(),
            bekleyen_geri_alma: self.defter.liste().len(),
            kalici_degisiklik: self
                .defter
                .liste()
                .iter()
                .filter(|k| k.kapsam == Kapsam::Kalici)
                .count(),
            dondurma_destegi: system_boost::suspend::destekleniyor(),
            yonetici: network_boost::tcp::yonetici_mi(),
            cpu_hibrit: topoloji.as_ref().map(|t| t.hibrit).unwrap_or(false),
            mantiksal_cekirdek: topoloji.map(|t| t.mantiksal_cekirdek).unwrap_or(0),
            guc_plani: system_boost::power::aktif_plan().ok().map(|(_, ad)| ad),
            otomatik_baslatma: system_boost::startup::otomatik_baslatma_acik().unwrap_or(false),
        }
    }

    pub fn profilleri_yenile(&mut self) {
        let (profiller, hatalar) = profile_engine::store::hepsini_yukle(&self.profil_dizini);
        for h in hatalar {
            self.gunluk.uyari(Kategori::Profil, h);
        }
        self.profiller = profiller;
    }

    pub fn profil_dizini(&self) -> &PathBuf {
        &self.profil_dizini
    }

    pub fn ayarlari_kaydet(&self) -> Result<()> {
        self.ayarlar.kaydet(&settings::ayar_yolu())
    }
}

fn sure_saniye(ornekler: &[Ornek]) -> f32 {
    match (ornekler.first(), ornekler.last()) {
        (Some(ilk), Some(son)) => ((son.zaman - ilk.zaman) as f32 / 1000.0).max(0.0),
        _ => 0.0,
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn ornek(zaman: i64, cpu: f32) -> Ornek {
        Ornek {
            zaman,
            cpu,
            bellek: 50.0,
            gecikme_ms: Some(20.0),
        }
    }

    #[test]
    fn sure_hesabi_dogru() {
        let o = vec![ornek(1000, 1.0), ornek(4000, 1.0)];
        assert!((sure_saniye(&o) - 3.0).abs() < 0.001);
    }

    #[test]
    fn tek_ornekte_sure_sifir() {
        assert_eq!(sure_saniye(&[ornek(1000, 1.0)]), 0.0);
        assert_eq!(sure_saniye(&[]), 0.0);
    }

    #[test]
    fn isaret_yoksa_karsilastirma_yok() {
        // Bu bir ürün kuralı: "öncesi" yoksa uydurulmuş bir karşılaştırma
        // gösterilmiyor.
        let mut motor = bos_motor();
        motor.ornekler.ekle(ornek(1000, 10.0));
        assert!(motor.karsilastirma().is_none());
    }

    #[test]
    fn tek_tarafli_veride_karsilastirma_yok() {
        let mut motor = bos_motor();
        motor.isaret = Some(Isaret { zaman: 5000 });
        // Yalnızca "sonrası" var.
        motor.ornekler.ekle(ornek(6000, 10.0));
        assert!(motor.karsilastirma().is_none());
    }

    #[test]
    fn karsilastirma_isarete_gore_bolunuyor() {
        let mut motor = bos_motor();
        motor.isaret = Some(Isaret { zaman: 5000 });
        motor.ornekler.ekle(ornek(1000, 80.0));
        motor.ornekler.ekle(ornek(2000, 80.0));
        motor.ornekler.ekle(ornek(6000, 20.0));
        motor.ornekler.ekle(ornek(7000, 20.0));

        let k = motor.karsilastirma().unwrap();
        assert_eq!(k.onceki.ornek_sayisi, 2);
        assert_eq!(k.sonraki.ornek_sayisi, 2);
        assert!((k.onceki.cpu_ort - 80.0).abs() < 0.001);
        assert!((k.sonraki.cpu_ort - 20.0).abs() < 0.001);
    }

    /// Diske dokunmayan motor — yalnızca saf mantık testleri için.
    fn bos_motor() -> Motor {
        Motor {
            defter: Defter::bellekte(),
            gunluk: Gunluk::yeni(),
            ayarlar: Ayarlar::default(),
            profiller: Vec::new(),
            mod_: Mod::Bosta,
            ornekler: Tampon::yeni(100),
            ornekleyici: Ornekleyici::yeni(),
            isaret: None,
            cikis_baslangici: None,
            profil_dizini: std::env::temp_dir().join("muifly-test-profiller"),
        }
    }

    #[test]
    fn basarisiz_islem_deftere_yazilmiyor() {
        let mut motor = bos_motor();
        let mut cikti = UygulamaSonucu::default();
        motor.uygula_ve_yaz(
            Kategori::Sistem,
            "olmayacak işlem",
            Kapsam::Oturum,
            Err(crate::error::Error::ProcessNotFound(999_999)),
            &mut cikti,
        );
        assert!(
            motor.defter.bos_mu(),
            "yapılmamış değişiklik deftere girmemeli"
        );
        assert_eq!(cikti.hatalar.len(), 1);
        assert!(cikti.uygulanan.is_empty());
    }

    #[test]
    fn basarili_islem_hem_deftere_hem_gunluge_yaziliyor() {
        let mut motor = bos_motor();
        let mut cikti = UygulamaSonucu::default();
        motor.uygula_ve_yaz(
            Kategori::Sistem,
            "discord.exe donduruldu",
            Kapsam::Oturum,
            Ok(Undo::SurecDonduruldu {
                pid: 42,
                surec: "discord.exe".into(),
            }),
            &mut cikti,
        );
        assert_eq!(motor.defter.liste().len(), 1);
        assert_eq!(motor.gunluk.uzunluk(), 1);
        assert_eq!(cikti.uygulanan.len(), 1);
        // Günlük satırı geri alma düğmesi taşımalı.
        assert!(motor.gunluk.son(1)[0].geri_alma_id.is_some());
    }
}
