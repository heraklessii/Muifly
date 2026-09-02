//! Çeviri belleği: oyun başına bir JSON.
//!
//! ## Karar #22'nin "öğrenme"si burası
//!
//! Model ince ayarı üç gerekçeyle reddedildi (onayla/reddet sinyali seq2seq
//! eğitimi için çok zayıf; ürünün içinde eğitim hattı taşımak gerekirdi;
//! zamanla kayan bir model tasarım ilkesi 2'nin yasakladığı kara kutunun ta
//! kendisi). Yerine konan iki şey aynı dosyada duruyor:
//!
//! - **Birebir eşleşme önbelleği** — aynı metin daha önce çevrildiyse aynen
//!   kullanılıyor. Yan faydası hız: önbellek isabetinde model hiç çalışmıyor
//!   ve oyunlar metni çok tekrarladığı için isabet oranı yüksek olacak.
//! - **Oyuna özel terim sözlüğü** — uygulaması [`super::sozluk`] içinde.
//!
//! ## Neden oyun başına ayrı dosya
//!
//! Profillerle aynı gerekçe (karar #9): kullanıcı dosyayı bir metin
//! editöründe açıp okuyabilsin, düzeltsin, silsin. `ROADMAP.md` Faz 5 kabul
//! kriteri bunu açıkça istiyor. Tek bir veritabanı bunu imkânsız kılardı.
//! "Skyrim'in sözlüğünü sil" de tek dosya silmek oluyor.
//!
//! ## Kaydın kökeni saklanıyor, çünkü şeffaflık bunu gerektiriyor
//!
//! Bir çeviri kullanıcının onayladığı kayıttan mı geldi, makineden mi —
//! arayüz bunu söyleyebilmeli (tasarım ilkesi 2). Karar #22 bunu "daha az
//! değil DAHA şeffaf" diye savunuyor; [`Koken`] o cümlenin kod karşılığı.
//!
//! ## Sıfır geri bildirimle de çalışır
//!
//! Karar #22: "kullanıcıların çoğu hiçbir şeyi puanlamaz; puanlamaya bağımlı
//! bir tasarım kullanılmayacak bir tasarımdır." Bu yüzden makine çevirileri
//! de önbelleğe giriyor — kullanıcı hiçbir şey onaylamasa bile bellek
//! çalışıyor, sadece kayıtların kökeni [`Koken::Makine`] oluyor.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::ceviri::anahtar;
use crate::error::{Error, Result};

/// Diskte tutulan en fazla **makine** kaydı sayısı.
///
/// Kullanıcı kayıtları bu sınıra dahil değil (bkz. [`CeviriBellegi::buda`]).
/// Yıllarca oynanan bir oyunda makine çevirileri sınırsız birikir; kullanıcının
/// elle onayladığı bir düzeltmenin yer açmak için atılması ise kabul edilemez.
pub const MAKINE_KAPASITESI: usize = 2000;

/// Bir kaydın nereden geldiği.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Koken {
    /// Çeviri modelinden geldi, kullanıcı görmedi ya da bir şey demedi.
    Makine,
    /// Kullanıcı onayladı ya da elle düzeltti.
    ///
    /// Karar #22: asıl olan "Düzelt", "Reddet" değil — reddetme yalnızca
    /// yanlış olduğunu söyler, doğrusunu söylemez. Bu yüzden burada
    /// "reddedildi" diye bir köken yok; reddedilen kayıt silinir.
    Kullanici,
}

/// Tek bir çeviri kaydı.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Kayit {
    /// Kaynak metnin özgün hali. Arama anahtarı DEĞİL — anahtar
    /// [`super::anahtar`] ile üretiliyor, bu alan gösterim için duruyor.
    pub metin: String,
    pub ceviri: String,
    pub koken: Koken,
    /// Unix milisaniye. Budama sırasında en eski makine kayıtları düşüyor.
    pub zaman: i64,
}

/// Bir oyunun çeviri belleği ve terim sözlüğü.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CeviriBellegi {
    /// Oyunun kimliği — çalıştırılabilir dosya adı ya da profil kimliği.
    pub oyun: String,
    /// BCP-47 etiketleri, örn. `en` ve `tr`.
    ///
    /// Dosyada duruyorlar çünkü bir kaydın hangi dil çiftine ait olduğu
    /// dosyanın kendisinden anlaşılabilmeli; kullanıcı bunu bir editörde
    /// açtığında tahmin etmek zorunda kalmamalı.
    pub kaynak_dil: String,
    pub hedef_dil: String,
    /// Terim sözlüğü: kaynak terim → hedef terim.
    ///
    /// `BTreeMap` bilinçli: dosya her kaydedişte aynı sırada yazılsın ki
    /// kullanıcı iki sürümü karşılaştırabilsin ve gereksiz fark görmesin.
    #[serde(default)]
    pub terimler: BTreeMap<String, String>,
    #[serde(default)]
    pub kayitlar: Vec<Kayit>,
}

impl CeviriBellegi {
    /// Yeni, boş bir bellek.
    pub fn yeni(
        oyun: impl Into<String>,
        kaynak_dil: impl Into<String>,
        hedef_dil: impl Into<String>,
    ) -> Self {
        Self {
            oyun: oyun.into(),
            kaynak_dil: kaynak_dil.into(),
            hedef_dil: hedef_dil.into(),
            terimler: BTreeMap::new(),
            kayitlar: Vec::new(),
        }
    }

    /// Metnin daha önce çevrilmiş hali.
    ///
    /// Arama [`super::anahtar`] üzerinden: boşluk ve büyük/küçük harf farkı
    /// isabeti kaçırmasın diye. OCR aynı diyalog kutusunu iki kez okuduğunda
    /// çıktı birebir aynı olmak zorunda değil.
    pub fn ara(&self, metin: &str) -> Option<&Kayit> {
        let a = anahtar(metin);
        self.kayitlar.iter().find(|k| anahtar(&k.metin) == a)
    }

    /// Kayıt ekler ya da günceller.
    ///
    /// **Kullanıcı kaydının üstüne makine yazamaz.** Kullanıcı bir çeviriyi
    /// düzelttiyse, aynı metin bir daha geldiğinde modelin çıktısı o
    /// düzeltmeyi sessizce ezmemeli — ezerse "onayladığın kayıttan geldi"
    /// vaadi yalan olur. Tersi serbest: kullanıcı düzeltmesi makine kaydının
    /// üstüne yazar.
    ///
    /// Dönen değer: kayıt gerçekten yazıldı mı.
    pub fn ekle(&mut self, metin: &str, ceviri: &str, koken: Koken) -> bool {
        let a = anahtar(metin);
        let simdi = chrono::Utc::now().timestamp_millis();

        if let Some(mevcut) = self.kayitlar.iter_mut().find(|k| anahtar(&k.metin) == a) {
            if mevcut.koken == Koken::Kullanici && koken == Koken::Makine {
                return false;
            }
            mevcut.metin = metin.to_string();
            mevcut.ceviri = ceviri.to_string();
            mevcut.koken = koken;
            mevcut.zaman = simdi;
            return true;
        }

        self.kayitlar.push(Kayit {
            metin: metin.to_string(),
            ceviri: ceviri.to_string(),
            koken,
            zaman: simdi,
        });
        self.buda();
        true
    }

    /// Bir kaydı siler. Karar #22'de "Reddet"in karşılığı bu: reddedilen
    /// kayıt bir puana dönüşmüyor, ortadan kalkıyor.
    pub fn sil(&mut self, metin: &str) -> bool {
        let a = anahtar(metin);
        let onceki = self.kayitlar.len();
        self.kayitlar.retain(|k| anahtar(&k.metin) != a);
        self.kayitlar.len() != onceki
    }

    /// Makine kayıtlarını temizler, kullanıcı kayıtlarına dokunmaz.
    ///
    /// "Önbelleği temizle" düğmesinin karşılığı. Kullanıcının kendi
    /// düzeltmelerini de silen bir temizlik, düğmeye basanın beklediği şey
    /// değildir; onun yolu dosyayı silmek.
    pub fn makine_kayitlarini_temizle(&mut self) {
        self.kayitlar.retain(|k| k.koken == Koken::Kullanici);
    }

    /// Makine kayıtlarını kapasiteye indirir; en eskiler düşer.
    fn buda(&mut self) {
        let makine = self
            .kayitlar
            .iter()
            .filter(|k| k.koken == Koken::Makine)
            .count();
        if makine <= MAKINE_KAPASITESI {
            return;
        }
        let dusecek = makine - MAKINE_KAPASITESI;

        let mut zamanlar: Vec<i64> = self
            .kayitlar
            .iter()
            .filter(|k| k.koken == Koken::Makine)
            .map(|k| k.zaman)
            .collect();
        zamanlar.sort_unstable();
        let esik = zamanlar[dusecek - 1];

        let mut kalan_dusecek = dusecek;
        self.kayitlar.retain(|k| {
            if k.koken == Koken::Makine && k.zaman <= esik && kalan_dusecek > 0 {
                kalan_dusecek -= 1;
                false
            } else {
                true
            }
        });
    }

    pub fn terim_ekle(&mut self, terim: &str, karsilik: &str) {
        let terim = terim.trim();
        if terim.is_empty() {
            return;
        }
        self.terimler
            .insert(terim.to_string(), karsilik.trim().to_string());
    }

    pub fn terim_sil(&mut self, terim: &str) -> bool {
        self.terimler.remove(terim.trim()).is_some()
    }

    /// Diske yazar.
    ///
    /// Atomik: dosyada kullanıcının kendi düzeltmeleri var ve yarım yazılmış
    /// bir dosya onların hepsini birden kaybettirirdi.
    pub fn kaydet(&self, yol: &Path) -> Result<()> {
        // `to_string_pretty`: dosya kullanıcı tarafından okunabilir olmak
        // zorunda (kabul kriteri), tek satırlık JSON o vaadi tutmaz.
        crate::settings::atomik_yaz(yol, &serde_json::to_string_pretty(self)?)
    }
}

/// Bir oyunun bellek dosyasının yolu.
///
/// Dosya adı temizliği `profile_engine::store`'dan alınıyor, kopyalanmıyor:
/// oradaki `..\..\` koruması burada da gerekli ve iki yerde iki koruma
/// tutmanın anlamı yok.
pub fn yolu(dizin: &Path, oyun: &str) -> PathBuf {
    dizin.join(crate::profile_engine::store::dosya_adi(oyun))
}

/// Diskten okur; dosya yoksa boş bir bellek döndürür.
///
/// **Bozuk dosya sessizce yutulmuyor.** `monitor::gecmis` bozuk geçmişi
/// atlayıp devam ediyor, çünkü orada kaybolan şey bir kayıt listesi. Burada
/// kaybolan şey kullanıcının elle yaptığı düzeltmeler: sessizce boş bir
/// bellekle devam etmek, kullanıcının gözünde emeğinin buharlaşması olurdu.
/// Dosya `.bozuk` uzantısıyla saklanıyor ve hata döndürülüyor — kullanıcı ne
/// olduğunu ve dosyasının nerede durduğunu görüyor.
pub fn yukle(yol: &Path, oyun: &str, kaynak_dil: &str, hedef_dil: &str) -> Result<CeviriBellegi> {
    let icerik = match std::fs::read_to_string(yol) {
        Ok(i) => i,
        // Dosya henüz yok: ilk kullanım, hata değil.
        Err(_) => return Ok(CeviriBellegi::yeni(oyun, kaynak_dil, hedef_dil)),
    };

    match serde_json::from_str::<CeviriBellegi>(&icerik) {
        Ok(mut bellek) => {
            bellek.buda();
            Ok(bellek)
        }
        Err(e) => {
            let yedek = yol.with_extension("bozuk");
            let _ = std::fs::rename(yol, &yedek);
            Err(Error::ProfileInvalid(format!(
                "çeviri belleği okunamadı ({e}); eski dosya {} olarak saklandı",
                yedek.display()
            )))
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn bellek() -> CeviriBellegi {
        CeviriBellegi::yeni("skyrim.exe", "en", "tr")
    }

    // --- Karar #22: birebir eşleşme önbelleği ---

    #[test]
    fn ayni_metin_bellekten_donuyor() {
        let mut b = bellek();
        b.ekle(
            "Press F to open.",
            "Açmak için F tuşuna basın.",
            Koken::Makine,
        );
        let bulunan = b.ara("Press F to open.").expect("kayıt bulunamadı");
        assert_eq!(bulunan.ceviri, "Açmak için F tuşuna basın.");
    }

    #[test]
    fn bosluk_ve_harf_farki_isabeti_kacirmiyor() {
        // OCR aynı kutuyu iki kez birebir aynı okumak zorunda değil.
        let mut b = bellek();
        b.ekle("The bridge collapsed.", "Köprü çöktü.", Koken::Makine);
        assert!(b.ara("the  bridge   collapsed.").is_some());
    }

    #[test]
    fn baska_metin_yanlislikla_eslesmiyor() {
        let mut b = bellek();
        b.ekle("Open the door.", "Kapıyı aç.", Koken::Makine);
        assert!(b.ara("Close the door.").is_none());
    }

    // --- Ürün duruşu: kullanıcının düzeltmesi ezilmez ---

    #[test]
    fn makine_kullanici_kaydinin_ustune_yazamiyor() {
        let mut b = bellek();
        b.ekle("Longsword", "Uzun kılıç", Koken::Kullanici);
        let yazildi = b.ekle("Longsword", "Uzunsöz", Koken::Makine);
        assert!(!yazildi);
        assert_eq!(b.ara("Longsword").unwrap().ceviri, "Uzun kılıç");
    }

    #[test]
    fn kullanici_makine_kaydini_duzeltebiliyor() {
        let mut b = bellek();
        b.ekle("Longsword", "Uzunsöz", Koken::Makine);
        assert!(b.ekle("Longsword", "Uzun kılıç", Koken::Kullanici));
        let k = b.ara("Longsword").unwrap();
        assert_eq!(k.ceviri, "Uzun kılıç");
        assert_eq!(k.koken, Koken::Kullanici);
    }

    #[test]
    fn kaydin_nereden_geldigi_saklanmiyor() {
        // Tasarım ilkesi 2: arayüz "bu çeviri senin onayladığın kayıttan
        // geldi" diyebilmeli.
        let mut b = bellek();
        b.ekle("Yes", "Evet", Koken::Kullanici);
        assert_eq!(b.ara("Yes").unwrap().koken, Koken::Kullanici);
    }

    // --- Sıfır geri bildirimle çalışma ---

    #[test]
    fn hicbir_sey_onaylanmasa_da_bellek_calisiyor() {
        let mut b = bellek();
        b.ekle("Hello there.", "Merhaba.", Koken::Makine);
        assert!(b.ara("Hello there.").is_some());
    }

    // --- Silme ve temizleme ---

    #[test]
    fn kayit_silinebiliyor() {
        let mut b = bellek();
        b.ekle("Hello", "Merhaba", Koken::Makine);
        assert!(b.sil("Hello"));
        assert!(b.ara("Hello").is_none());
    }

    #[test]
    fn onbellek_temizligi_kullanici_kayitlarina_dokunmuyor() {
        let mut b = bellek();
        b.ekle("A", "a", Koken::Makine);
        b.ekle("B", "b", Koken::Kullanici);
        b.makine_kayitlarini_temizle();
        assert!(b.ara("A").is_none());
        assert!(b.ara("B").is_some());
    }

    // --- Budama ---

    #[test]
    fn kullanici_kayitlari_yer_acmak_icin_atilmiyor() {
        let mut b = bellek();
        b.ekle("korunacak", "korunuyor", Koken::Kullanici);
        for i in 0..(MAKINE_KAPASITESI + 50) {
            b.ekle(&format!("makine {i}"), "x", Koken::Makine);
        }
        assert!(
            b.ara("korunacak").is_some(),
            "kullanıcı kaydı budamada düştü"
        );
        let makine = b
            .kayitlar
            .iter()
            .filter(|k| k.koken == Koken::Makine)
            .count();
        assert_eq!(makine, MAKINE_KAPASITESI);
    }

    // --- Terim sözlüğü ---

    #[test]
    fn terim_eklenip_silinebiliyor() {
        let mut b = bellek();
        b.terim_ekle("  Stamina ", " Dayanıklılık ");
        assert_eq!(
            b.terimler.get("Stamina").map(String::as_str),
            Some("Dayanıklılık")
        );
        assert!(b.terim_sil("Stamina"));
        assert!(b.terimler.is_empty());
    }

    #[test]
    fn bos_terim_eklenmiyor() {
        let mut b = bellek();
        b.terim_ekle("   ", "bir şey");
        assert!(b.terimler.is_empty());
    }

    // --- Disk ---

    #[test]
    fn kaydedilip_geri_okunuyor() {
        let dizin = tempfile::tempdir().unwrap();
        let yol = yolu(dizin.path(), "skyrim.exe");

        let mut b = bellek();
        b.ekle("Yes", "Evet", Koken::Kullanici);
        b.terim_ekle("Whiterun", "Whiterun");
        b.kaydet(&yol).unwrap();

        let okunan = yukle(&yol, "skyrim.exe", "en", "tr").unwrap();
        assert_eq!(okunan, b);
    }

    #[test]
    fn dosya_yoksa_bos_bellek_donuyor() {
        let dizin = tempfile::tempdir().unwrap();
        let yol = yolu(dizin.path(), "hic-acilmamis.exe");
        let b = yukle(&yol, "hic-acilmamis.exe", "en", "tr").unwrap();
        assert!(b.kayitlar.is_empty());
        assert_eq!(b.oyun, "hic-acilmamis.exe");
    }

    #[test]
    fn bozuk_dosya_sessizce_yutulmuyor() {
        let dizin = tempfile::tempdir().unwrap();
        let yol = yolu(dizin.path(), "bozuk.exe");
        std::fs::write(&yol, "{ bu JSON degil").unwrap();

        let sonuc = yukle(&yol, "bozuk.exe", "en", "tr");
        assert!(sonuc.is_err(), "bozuk dosya sessizce boş bellek döndürdü");
        // Kullanıcının emeği silinmiyor, kenara alınıyor.
        assert!(yol.with_extension("bozuk").exists());
    }

    #[test]
    fn dosya_adi_klasor_disina_yazmiyor() {
        let dizin = tempfile::tempdir().unwrap();
        let yol = yolu(dizin.path(), "..\\..\\kotu");
        assert_eq!(yol.parent().unwrap(), dizin.path());
    }

    #[test]
    fn dosya_okunabilir_json() {
        // Kabul kriteri: kullanıcı dosyayı bir editörde açıp okuyabilmeli.
        let dizin = tempfile::tempdir().unwrap();
        let yol = yolu(dizin.path(), "okunur.exe");
        let mut b = CeviriBellegi::yeni("okunur.exe", "en", "tr");
        b.ekle("Yes", "Evet", Koken::Kullanici);
        b.kaydet(&yol).unwrap();

        let icerik = std::fs::read_to_string(&yol).unwrap();
        assert!(icerik.contains('\n'), "tek satırlık JSON okunabilir değil");
        assert!(icerik.contains("Evet"));
    }
}
