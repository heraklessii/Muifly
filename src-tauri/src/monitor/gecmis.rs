//! Oturum geçmişi: Muifly'ın bir oyun için sistemde bir şey tuttuğu her
//! aralığın diske yazılmış kaydı.
//!
//! ## Neden var
//!
//! Şeffaflık günlüğü (`log.rs`) **yalnızca bellekte** duruyor ve programla
//! birlikte ölüyor. Ama Muifly'ın normal kullanımı tepside beklemek: kullanıcı
//! akşam oynuyor, sabah bilgisayarı yeniden başlatıyor ve "dün gece ne
//! yapıldı" sorusunun cevabı hiçbir yerde kalmıyor. Tasarım ilkesi 2 "ne
//! değişti, kullanıcıya gösterilir" diyor; program kapandığında kaybolan bir
//! kayıt bu vaadin yarısını tutuyor.
//!
//! ## Ne DEĞİL
//!
//! Bu bir telemetri değil. Dosya kullanıcının kendi makinesinde, kendi veri
//! klasöründe duruyor; hiçbir yere gönderilmiyor, hiçbir yerden okunmuyor.
//! Ayarlardan kapatılabiliyor ve tek düğmeyle silinebiliyor.
//!
//! Bir "skor tablosu" da değil. `decisions.md` #15 gereği burada da öncesi ve
//! sonrası **iki ayrı özet** olarak taşınıyor; tek bir "şu kadar iyileşti"
//! oranı üretilmiyor. Kayıt ölçüm taşır, iddia taşımaz.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::monitor::{KareOzeti, Ozet};

/// Diskte tutulan en fazla oturum sayısı.
///
/// Sınırsız bir dosya, yıllarca tepside duran bir araçta sessizce büyür.
/// 200 oturum, günde bir oyun oynayan biri için yarım yıldan uzun bir
/// geçmiş; dosya boyutu birkaç yüz kilobayt.
pub const KAPASITE: usize = 200;

/// Bir oturumun kaydı.
///
/// "Oturum" = profilin uygulandığı andan oturumluk değişikliklerin geri
/// alındığı ana kadar geçen süre. Oyunun açık kalma süresi DEĞİL: Muifly'ın
/// sistemde bir şey tuttuğu süre. İkisi aynı olmak zorunda değil (kullanıcı
/// oyunun ortasında elle geri alabilir) ve kayıt hangisini anlattığını
/// bilmek zorunda.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OturumKaydi {
    /// Başlangıç zaman damgası; listede anahtar olarak da kullanılıyor.
    pub id: u64,
    /// Unix milisaniye.
    pub baslangic: i64,
    pub bitis: i64,
    /// Oyunun çalıştırılabilir dosya adı.
    pub surec: String,
    /// Katalogdan ya da profilden gelen okunur ad. Bilinmiyorsa `None` —
    /// exe adını "oyun adı" diye göstermek uydurma olurdu.
    pub oyun_adi: Option<String>,
    pub profil_adi: Option<String>,
    pub mod_adi: String,
    /// Uygulanan değişikliklerin özetleri — günlükteki cümlelerin aynısı.
    pub uygulanan: Vec<String>,
    /// Oturum kapanırken gerçekten geri alınan kayıt sayısı.
    pub geri_alinan: usize,
    /// Profil uygulanmadan önceki ölçüm penceresinin özeti.
    pub onceki: Option<Ozet>,
    /// Uygulandıktan sonraki pencerenin özeti.
    pub sonraki: Option<Ozet>,
    /// Oturum sırasında kare ölçümü yapıldıysa son ölçümün özeti.
    pub kare: Option<KareOzeti>,
}

impl OturumKaydi {
    /// Oturumun süresi, saniye. Negatif olamaz: saat geri alınmış bir
    /// makinede bitiş başlangıçtan önce görünebilir.
    pub fn sure_sn(&self) -> i64 {
        ((self.bitis - self.baslangic) / 1000).max(0)
    }

    /// Listede gösterilen ad.
    pub fn ad(&self) -> &str {
        self.oyun_adi.as_deref().unwrap_or(&self.surec)
    }
}

/// Geçmişin tamamının özeti — arayüzdeki başlık şeridi.
///
/// Hepsi sayım ve toplam; hiçbiri türetilmiş bir "başarı" ölçüsü değil.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GecmisOzeti {
    pub oturum_sayisi: usize,
    pub toplam_sure_sn: i64,
    pub toplam_degisiklik: usize,
    /// Kare ölçümü içeren oturum sayısı.
    pub olculen_oturum: usize,
    /// En çok süre geçirilen oyun.
    pub en_cok: Option<EnCok>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnCok {
    pub ad: String,
    pub sure_sn: i64,
    pub oturum: usize,
}

/// Diskteki oturum kayıtları.
///
/// Sıra **yeniden eskiye**: arayüzün ilk sorusu "en son ne oldu" ve kapasite
/// dolduğunda düşmesi gereken taraf sonu. İkisi de aynı yöne bakıyor.
#[derive(Debug, Default)]
pub struct Gecmis {
    kayitlar: Vec<OturumKaydi>,
    yol: Option<PathBuf>,
}

impl Gecmis {
    /// Diske yazmayan geçmiş — testler için.
    pub fn bellekte() -> Self {
        Self::default()
    }

    /// Dosyayı okur; yoksa boş geçmişle başlar.
    ///
    /// Bozuk dosyada `Defter::yukle` ile aynı davranış: hata dönmüyor, dosya
    /// `.bozuk` uzantısıyla kenara alınıyor ve program açılıyor. Bir geçmiş
    /// dosyası yüzünden programın hiç açılmaması, kaybının kendisinden çok
    /// daha kötü olurdu.
    pub fn yukle(yol: impl AsRef<Path>) -> Self {
        let yol = yol.as_ref().to_path_buf();
        let mut gecmis = match std::fs::read_to_string(&yol) {
            Ok(icerik) => match serde_json::from_str::<Vec<OturumKaydi>>(&icerik) {
                Ok(kayitlar) => Gecmis {
                    kayitlar,
                    yol: None,
                },
                Err(_) => {
                    let _ = std::fs::rename(&yol, yol.with_extension("bozuk"));
                    Gecmis::default()
                }
            },
            Err(_) => Gecmis::default(),
        };
        gecmis.kayitlar.truncate(KAPASITE);
        gecmis.yol = Some(yol);
        gecmis
    }

    /// Yeni kaydı başa ekler, kapasiteyi aşanı düşürür ve diske yazar.
    pub fn ekle(&mut self, kayit: OturumKaydi) {
        self.kayitlar.insert(0, kayit);
        self.kayitlar.truncate(KAPASITE);
        self.diske_yaz();
    }

    pub fn liste(&self) -> &[OturumKaydi] {
        &self.kayitlar
    }

    /// Geçmişi siler. Kullanıcının "geçmişi temizle" düğmesi.
    ///
    /// Dosya da siliniyor, boş bir dizi yazılmıyor: kullanıcı "sildim"
    /// dediğinde diskte artakalan bir dosya olmamalı.
    pub fn temizle(&mut self) {
        self.kayitlar.clear();
        if let Some(yol) = &self.yol {
            let _ = std::fs::remove_file(yol);
        }
    }

    pub fn ozet(&self) -> GecmisOzeti {
        ozetle(&self.kayitlar)
    }

    /// Diske yazma hatası yutuluyor ve loglanıyor: geçmiş kaydedilemedi diye
    /// oyun oturumunu kesmenin bir anlamı yok (`Defter::diske_yaz` ile aynı
    /// gerekçe).
    fn diske_yaz(&self) {
        let Some(yol) = &self.yol else { return };
        match serde_json::to_string_pretty(&self.kayitlar) {
            Ok(metin) => {
                if let Err(e) = crate::settings::atomik_yaz(yol, &metin) {
                    log::warn!("oturum geçmişi diske yazılamadı: {e}");
                }
            }
            Err(e) => log::warn!("oturum geçmişi serileştirilemedi: {e}"),
        }
    }
}

/// Saf özet fonksiyonu — `Gecmis::ozet` bunu çağırıyor, testler de.
pub fn ozetle(kayitlar: &[OturumKaydi]) -> GecmisOzeti {
    let mut sureler: Vec<(String, i64, usize)> = Vec::new();
    for k in kayitlar {
        let ad = k.ad().to_string();
        match sureler.iter_mut().find(|(a, _, _)| *a == ad) {
            Some(giris) => {
                giris.1 += k.sure_sn();
                giris.2 += 1;
            }
            None => sureler.push((ad, k.sure_sn(), 1)),
        }
    }
    // Eşitlikte ilk gelen kazanıyor; liste yeniden eskiye sıralı olduğu için
    // bu "daha yakın zamanda oynanan" demek.
    let en_cok = sureler
        .into_iter()
        .max_by_key(|(_, sure, _)| *sure)
        .filter(|(_, sure, _)| *sure > 0)
        .map(|(ad, sure_sn, oturum)| EnCok {
            ad,
            sure_sn,
            oturum,
        });

    GecmisOzeti {
        oturum_sayisi: kayitlar.len(),
        toplam_sure_sn: kayitlar.iter().map(|k| k.sure_sn()).sum(),
        toplam_degisiklik: kayitlar.iter().map(|k| k.uygulanan.len()).sum(),
        olculen_oturum: kayitlar.iter().filter(|k| k.kare.is_some()).count(),
        en_cok,
    }
}

/// Saniyeyi okunur süreye çevirir. Arayüzdeki `format.ts` içindeki `sure` ile
/// aynı kurallar; rapor metni arayüzden bağımsız üretilebilsin diye burada da
/// bir kopyası var.
pub fn sure_metni(saniye: i64) -> String {
    if saniye <= 0 {
        return "0 sn".to_string();
    }
    if saniye < 60 {
        return format!("{saniye} sn");
    }
    let dakika = saniye / 60;
    let kalan = saniye % 60;
    if dakika >= 60 {
        return format!("{} sa {} dk", dakika / 60, dakika % 60);
    }
    if kalan == 0 {
        format!("{dakika} dk")
    } else {
        format!("{dakika} dk {kalan} sn")
    }
}

/// Yerel saat biçimi. Zaman damgası okunamazsa uydurulmuş bir tarih
/// basılmıyor.
fn tarih_metni(zaman_ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(zaman_ms)
        .map(|d| {
            d.with_timezone(&chrono::Local)
                .format("%d.%m.%Y %H:%M")
                .to_string()
        })
        .unwrap_or_else(|| "zaman okunamadı".into())
}

/// Geçmişin düz metin raporu — kullanıcının dosyaya kaydettiği şey.
///
/// Biçim bilinçli olarak düz metin: kullanıcı bir destek başlığına ya da
/// arkadaşına yapıştırabilsin, açmak için başka bir program gerekmesin.
/// İçeride tek bir yorum cümlesi yok — hangi ayarın uygulandığı ve o
/// pencerede ne ölçüldüğü yazıyor, "iyi/kötü" demiyor (ilke 4).
pub fn rapor_metni(kayitlar: &[OturumKaydi]) -> String {
    let ozet = ozetle(kayitlar);
    let mut s = String::new();
    s.push_str("Muifly — oturum geçmişi\n");
    s.push_str(&format!("Sürüm: {}\n", env!("CARGO_PKG_VERSION")));
    s.push_str(&format!(
        "Dışa aktarma: {}\n",
        chrono::Local::now().format("%d.%m.%Y %H:%M")
    ));
    s.push_str(&format!(
        "{} oturum · toplam {} · {} değişiklik\n",
        ozet.oturum_sayisi,
        sure_metni(ozet.toplam_sure_sn),
        ozet.toplam_degisiklik
    ));
    s.push_str(
        "\nBu dosyadaki sayılar bu makinede, bu oturumlarda ölçülen\n\
         değerlerdir; başka bir makinede aynısı beklenemez.\n",
    );

    if kayitlar.is_empty() {
        s.push_str("\n(kayıt yok)\n");
        return s;
    }

    for k in kayitlar {
        s.push_str("\n----------------------------------------\n");
        s.push_str(&format!("{} — {}\n", k.ad(), tarih_metni(k.baslangic)));
        s.push_str(&format!(
            "Süre: {} · Mod: {}\n",
            sure_metni(k.sure_sn()),
            k.mod_adi
        ));
        if let Some(p) = &k.profil_adi {
            s.push_str(&format!("Profil: {p}\n"));
        }
        if k.uygulanan.is_empty() {
            s.push_str("Uygulanan: yok\n");
        } else {
            s.push_str("Uygulanan:\n");
            for u in &k.uygulanan {
                s.push_str(&format!("  - {u}\n"));
            }
        }
        s.push_str(&format!("Geri alınan: {}\n", k.geri_alinan));

        if let (Some(o), Some(y)) = (&k.onceki, &k.sonraki) {
            s.push_str("Ölçüm (öncesi, sonrası — iki ayrı pencere):\n");
            s.push_str(&format!(
                "  CPU  {:.0}% / {:.0}%\n  RAM  {:.0}% / {:.0}%\n",
                o.cpu_ort, y.cpu_ort, o.bellek_ort, y.bellek_ort
            ));
            if let (Some(a), Some(b)) = (o.gecikme_ort_ms, y.gecikme_ort_ms) {
                s.push_str(&format!("  Gecikme  {a:.0} ms / {b:.0} ms\n"));
            }
        }

        if let Some(kare) = &k.kare {
            s.push_str(&format!(
                "Kare ölçümü: ortalama {:.1} kare/sn · en kötü %1 {:.1} ms · \
                 oynama {:.1} ms ({} kare)\n",
                kare.ort_fps, kare.p1_kotu_ms, kare.kare_jitter_ms, kare.kare_sayisi
            ));
        }
    }
    s
}

#[cfg(test)]
mod testler {
    use super::*;

    fn ozet_ornegi(cpu: f32) -> Ozet {
        Ozet {
            ornek_sayisi: 10,
            cpu_ort: cpu,
            bellek_ort: 50.0,
            gecikme_ort_ms: Some(20.0),
            jitter_ms: Some(2.0),
            kayip_yuzde: 0.0,
        }
    }

    fn kayit(id: u64, ad: &str, sure_sn: i64) -> OturumKaydi {
        OturumKaydi {
            id,
            baslangic: id as i64,
            bitis: id as i64 + sure_sn * 1000,
            surec: format!("{ad}.exe"),
            oyun_adi: Some(ad.to_string()),
            profil_adi: None,
            mod_adi: "Oyun Profili".into(),
            uygulanan: vec!["öncelik yükseltildi".into()],
            geri_alinan: 1,
            onceki: None,
            sonraki: None,
            kare: None,
        }
    }

    #[test]
    fn sure_negatif_olmuyor() {
        // Saat geri alınmış bir makinede bitiş başlangıçtan önce görünebilir.
        let mut k = kayit(1000, "oyun", 10);
        k.bitis = 0;
        assert_eq!(k.sure_sn(), 0);
    }

    #[test]
    fn ad_bilinmiyorsa_exe_gosteriliyor() {
        let mut k = kayit(1, "oyun", 5);
        k.oyun_adi = None;
        assert_eq!(k.ad(), "oyun.exe");
    }

    #[test]
    fn yeni_kayit_basa_giriyor() {
        let mut g = Gecmis::bellekte();
        g.ekle(kayit(1, "ilk", 10));
        g.ekle(kayit(2, "ikinci", 10));
        assert_eq!(g.liste()[0].id, 2);
    }

    #[test]
    fn kapasite_asilinca_en_eski_dusuyor() {
        let mut g = Gecmis::bellekte();
        for i in 0..(KAPASITE + 5) {
            g.ekle(kayit(i as u64, "oyun", 1));
        }
        assert_eq!(g.liste().len(), KAPASITE);
        assert_eq!(g.liste()[0].id, (KAPASITE + 4) as u64);
        assert!(g.liste().iter().all(|k| k.id >= 5));
    }

    #[test]
    fn ozet_sureleri_oyun_basina_topluyor() {
        let kayitlar = vec![kayit(3, "A", 100), kayit(2, "B", 300), kayit(1, "A", 100)];
        let o = ozetle(&kayitlar);
        assert_eq!(o.oturum_sayisi, 3);
        assert_eq!(o.toplam_sure_sn, 500);
        assert_eq!(o.toplam_degisiklik, 3);
        let en = o.en_cok.unwrap();
        assert_eq!(en.ad, "B");
        assert_eq!(en.sure_sn, 300);
        assert_eq!(en.oturum, 1);
    }

    #[test]
    fn bos_gecmiste_en_cok_yok() {
        let o = ozetle(&[]);
        assert_eq!(o.oturum_sayisi, 0);
        assert!(o.en_cok.is_none());
    }

    #[test]
    fn sifir_sureli_oturumlar_en_cok_uretmiyor() {
        // Hepsi anında kapanmışsa "en çok oynanan" diye bir şey yok; sıfır
        // saniyeyi zirveye yazmak uydurma olurdu.
        let o = ozetle(&[kayit(1, "A", 0), kayit(2, "B", 0)]);
        assert!(o.en_cok.is_none());
    }

    #[test]
    fn disk_gidis_donusu() {
        let dizin = tempfile::tempdir().unwrap();
        let yol = dizin.path().join("gecmis.json");
        let mut g = Gecmis::yukle(&yol);
        g.ekle(kayit(7, "oyun", 60));
        let tekrar = Gecmis::yukle(&yol);
        assert_eq!(tekrar.liste().len(), 1);
        assert_eq!(tekrar.liste()[0].id, 7);
    }

    #[test]
    fn temizleme_dosyayi_da_siliyor() {
        let dizin = tempfile::tempdir().unwrap();
        let yol = dizin.path().join("gecmis.json");
        let mut g = Gecmis::yukle(&yol);
        g.ekle(kayit(1, "oyun", 10));
        assert!(yol.exists());
        g.temizle();
        assert!(g.liste().is_empty());
        assert!(!yol.exists(), "kullanıcı sildiyse diskte kalıntı olmamalı");
    }

    #[test]
    fn bozuk_dosya_programi_kilitlemiyor() {
        let dizin = tempfile::tempdir().unwrap();
        let yol = dizin.path().join("gecmis.json");
        std::fs::write(&yol, "{ bu json degil").unwrap();
        let g = Gecmis::yukle(&yol);
        assert!(g.liste().is_empty());
        assert!(yol.with_extension("bozuk").exists());
    }

    #[test]
    fn sure_metni_esikleri() {
        assert_eq!(sure_metni(0), "0 sn");
        assert_eq!(sure_metni(-5), "0 sn");
        assert_eq!(sure_metni(45), "45 sn");
        assert_eq!(sure_metni(120), "2 dk");
        assert_eq!(sure_metni(125), "2 dk 5 sn");
        assert_eq!(sure_metni(7200), "2 sa 0 dk");
        assert_eq!(sure_metni(5400), "1 sa 30 dk");
    }

    #[test]
    fn rapor_bos_gecmiste_de_uretiliyor() {
        let m = rapor_metni(&[]);
        assert!(m.contains("Muifly"));
        assert!(m.contains("(kayıt yok)"));
    }

    #[test]
    fn rapor_uygulananlari_ve_olcumu_yaziyor() {
        let mut k = kayit(1, "Oyun", 600);
        k.onceki = Some(ozet_ornegi(80.0));
        k.sonraki = Some(ozet_ornegi(40.0));
        let m = rapor_metni(&[k]);
        assert!(m.contains("Oyun"));
        assert!(m.contains("öncelik yükseltildi"));
        assert!(m.contains("80% / 40%"));
    }

    /// Ürün duruşu (ilke 4, karar #15): rapor tek bir iyileşme oranı
    /// üretmiyor. İki pencere yan yana yazılıyor, yorumu kullanıcı yapıyor.
    #[test]
    fn raporda_iyilesme_iddiasi_yok() {
        let mut k = kayit(1, "Oyun", 600);
        k.onceki = Some(ozet_ornegi(80.0));
        k.sonraki = Some(ozet_ornegi(40.0));
        let m = rapor_metni(&[k]).to_lowercase();
        for yasak in [
            "iyileşme",
            "iyileşti",
            "kazanç",
            "hızlandı",
            "daha hızlı",
            "performans artışı",
        ] {
            assert!(!m.contains(yasak), "raporda iddia cümlesi var: {yasak}");
        }
    }
}
