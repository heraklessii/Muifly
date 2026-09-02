//! Çeviri akışı: okunan metinden gösterilecek sonuca.
//!
//! ```text
//! ham OCR metni
//!   → onisleme::hazirla     BÜYÜK HARF küçültülür, şüpheler işaretlenir
//!   → cumle::bol            birimlere ayrılır (karar #29 zaaf 3)
//!   → her birim için:
//!       bellek.ara          daha önce çevrildiyse model hiç çalışmaz
//!       sozluk::koru        oyun terimleri modelden gizlenir
//!       motor.cevir
//!       sozluk::geri_koy    terimler geri konur, KAYBOLANLAR raporlanır
//!       bellek.ekle         makine kaydı olarak önbelleğe girer
//! ```
//!
//! ## Ekran yakalama burada YOK, bilerek
//!
//! Bu modül `&str` alıyor, ekran değil. Sebep test edilebilirlik: akışın
//! bütün kararları (önbellek isabeti, terim koruma, kayıp raporu, boş
//! çeviri) Windows olmadan sınanabiliyor. Yakalama ve OCR bir katman
//! yukarıda, `super::Ceviri`de.
//!
//! ## Hiçbir şey sessizce yutulmuyor
//!
//! Karar #29'un üçüncü zaafı ölçülmüş bir gerçekti: model bir cümleyi
//! **hata vermeden** düşürebiliyor ve çıktı akıcı göründüğü için kullanıcı
//! fark etmiyor. Bu akış üç ayrı yerde ona karşı duruyor:
//!
//! 1. Birim sayısı girişte ve çıkışta aynı — düşen bir cümle boş bir birim
//!    olarak görünüyor, hiç olmamış gibi değil.
//! 2. `sozluk::geri_koy`un kaybettiği işaretler birime yazılıyor.
//! 3. Kaynak metin her birimde saklanıyor; arayüz çeviriyi kaynaksız
//!    göstermiyor.

use serde::Serialize;

use super::bellek::{CeviriBellegi, Koken};
use super::{cumle, onisleme, sozluk};
use crate::error::Result;

/// Bir metni çeviren şey.
///
/// Arayüz (trait) olması test içindi ve orada kaldı: akışın kararlarını
/// yarım gigabaytlık bir model yüklemeden sınayabilmenin başka yolu yok.
pub trait Motor {
    fn cevir(&mut self, kaynak: &str) -> Result<String>;
}

/// Çevrilmiş tek bir birim.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Birim {
    /// Kullanıcının ekranda gördüğü hali (ön işlemeden ÖNCE).
    pub kaynak: String,
    pub ceviri: String,
    pub koken: Koken,
    /// Sözlükten korunup geri konan terimler.
    pub korunan_terimler: Vec<String>,
    /// Çıktıda işareti bulunamayan terimler — "çeviri eksik" demek.
    pub kayip_terimler: Vec<String>,
    /// Model bu birim için hiçbir şey üretmedi.
    ///
    /// Ayrı bir alan, çünkü boş bir çeviri satırı arayüzde kolayca
    /// gözden kaçar; karar #29 zaaf 3'ün tam olarak göründüğü yer burası.
    pub bos: bool,
}

/// Bir çeviri isteğinin tamamı.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sonuc {
    /// OCR'ın verdiği ham metin, hiç dokunulmamış.
    pub ham: String,
    /// Ön işlemenin ürettiği uyarılar, kullanıcıya gösterilecek cümleler.
    pub uyarilar: Vec<String>,
    pub birimler: Vec<Birim>,
    /// Kaç birim modele hiç gitmedi (önbellek isabeti).
    pub bellekten: usize,
    /// Yalnızca modelde geçen süre. OCR süresi ayrı taşınıyor
    /// (`super::Ceviri`), çünkü ikisi ayrı sorunun cevabı.
    pub ceviri_ms: u64,
}

impl Sonuc {
    /// Bütün birimlerin çevirisi, satır satır.
    pub fn duz_metin(&self) -> String {
        self.birimler
            .iter()
            .map(|b| b.ceviri.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Kullanıcının bakması gereken bir şey var mı?
    pub fn kusurlu(&self) -> bool {
        !self.uyarilar.is_empty()
            || self
                .birimler
                .iter()
                .any(|b| b.bos || !b.kayip_terimler.is_empty())
    }
}

/// Bir uyarıyı kullanıcıya gösterilecek cümleye çevirir.
///
/// Metin Rust tarafında (karar #17). Cümleler bir şeyi **anlatıyor**, bir
/// şey vaat etmiyor: "şu satır küçültüldü" doğrulanabilir bir ifade,
/// "çeviri daha iyi olacak" değil (tasarım ilkesi 4).
fn uyari_metni(u: &onisleme::Uyari) -> String {
    match u {
        onisleme::Uyari::BuyukHarfKucultuldu { satir } => format!(
            "'{satir}' tamamı büyük harfti; çeviriye cümle düzenine indirilmiş hali verildi."
        ),
        onisleme::Uyari::BitisikKelimeSuphesi { parca } => format!(
            "'{parca}' okunurken kelimeler birbirine geçmiş olabilir; çeviri bu yüzden \
             tuhaf çıkabilir."
        ),
        onisleme::Uyari::NoktalamaSuphesi { parca } => format!(
            "'{parca}' içinde beklenmedik bir noktalama var; okuma hatası olabilir."
        ),
    }
}

/// Metni çevirir ve belleği günceller.
///
/// `bellek` hem okunuyor hem yazılıyor: isabet eden birim modele
/// gitmiyor, etmeyen birimin sonucu makine kaydı olarak giriyor. Karar #22:
/// özellik **sıfır geri bildirimle tam çalışmak zorunda**, o yüzden
/// kullanıcı hiçbir şey onaylamasa bile önbellek doluyor.
pub fn cevir(ham: &str, bellek: &mut CeviriBellegi, motor: &mut dyn Motor) -> Result<Sonuc> {
    let hazirlik = onisleme::hazirla(ham);
    let uyarilar: Vec<String> = hazirlik.uyarilar.iter().map(uyari_metni).collect();

    // Kaynak, kullanıcının ekranda gördüğü metin olmalı (karar #29 zaaf 3);
    // çeviriye giren düzeltilmiş hali değil. İkisi birim birim eşleşsin
    // diye ikisi de aynı kurallarla bölünüyor.
    let girdiler = cumle::bol(&hazirlik.metin);
    let ozgunler = cumle::bol(&hazirlik.ozgun);

    let baslangic = std::time::Instant::now();
    let mut birimler = Vec::with_capacity(girdiler.len());
    let mut bellekten = 0usize;

    for (i, girdi) in girdiler.iter().enumerate() {
        // Bölme iki metinde farklı sayıda birim üretirse (küçültme
        // noktalamayı değiştirmiyor ama emin olmanın bedeli yok) düzeltilmiş
        // metne düşülüyor: kaynaksız bir çeviri göstermektense çeviriye
        // giren metni göstermek doğru.
        let ozgun = ozgunler.get(i).unwrap_or(girdi).clone();

        if let Some(kayit) = bellek.ara(girdi) {
            bellekten += 1;
            birimler.push(Birim {
                kaynak: ozgun,
                bos: kayit.ceviri.trim().is_empty(),
                ceviri: kayit.ceviri.clone(),
                koken: kayit.koken,
                korunan_terimler: Vec::new(),
                kayip_terimler: Vec::new(),
            });
            continue;
        }

        let (korunmus, yerlesimler) = sozluk::koru(girdi, &bellek.terimler);
        let ham_ceviri = motor.cevir(&korunmus)?;
        let (ceviri, kayip) = sozluk::geri_koy(&ham_ceviri, &yerlesimler);

        // Boş çeviri belleğe **yazılmıyor**: yazılsaydı bir daha hiç
        // denenmez ve model bir cümleyi bir kez düşürdüğü için o cümle
        // kalıcı olarak çevrilemez hâle gelirdi.
        if !ceviri.trim().is_empty() {
            bellek.ekle(girdi, &ceviri, Koken::Makine);
        }

        birimler.push(Birim {
            kaynak: ozgun,
            bos: ceviri.trim().is_empty(),
            ceviri,
            koken: Koken::Makine,
            korunan_terimler: yerlesimler.iter().map(|y| y.terim.clone()).collect(),
            kayip_terimler: kayip,
        });
    }

    Ok(Sonuc {
        ham: ham.to_string(),
        uyarilar,
        birimler,
        bellekten,
        ceviri_ms: baslangic.elapsed().as_millis() as u64,
    })
}

#[cfg(test)]
mod testler {
    use super::*;

    /// Modelin taklidi: verilen eşlemeye bakıyor, yoksa `<<girdi>>` üretiyor.
    struct SahteMotor {
        eslemeler: Vec<(String, String)>,
        cagri: usize,
    }

    impl SahteMotor {
        fn yeni(ciftler: &[(&str, &str)]) -> Self {
            Self {
                eslemeler: ciftler
                    .iter()
                    .map(|(a, b)| ((*a).to_string(), (*b).to_string()))
                    .collect(),
                cagri: 0,
            }
        }
    }

    impl Motor for SahteMotor {
        fn cevir(&mut self, kaynak: &str) -> Result<String> {
            self.cagri += 1;
            for (a, b) in &self.eslemeler {
                if a == kaynak {
                    return Ok(b.clone());
                }
            }
            Ok(format!("<<{kaynak}>>"))
        }
    }

    fn bellek() -> CeviriBellegi {
        CeviriBellegi::yeni("test.exe", "en", "tr")
    }

    // --- Karar #29 zaaf 3: düşen cümle görünür kalıyor ---

    #[test]
    fn iki_cumle_iki_birim_oluyor() {
        let mut b = bellek();
        let mut m = SahteMotor::yeni(&[
            ("Keep your guard up.", "Tetikte ol."),
            ("This one bites back.", "Bu karşılık verir."),
        ]);
        let s = cevir("Keep your guard up. This one bites back.", &mut b, &mut m).unwrap();
        assert_eq!(s.birimler.len(), 2);
        assert_eq!(m.cagri, 2, "model tek parça çağrıldı");
    }

    #[test]
    fn bos_ceviri_isaretleniyor() {
        let mut b = bellek();
        let mut m = SahteMotor::yeni(&[("This one bites back.", "")]);
        let s = cevir("This one bites back.", &mut b, &mut m).unwrap();
        assert!(s.birimler[0].bos, "boş çeviri sessizce geçti");
        assert!(s.kusurlu());
    }

    #[test]
    fn bos_ceviri_bellege_yazilmiyor() {
        // Yazılsaydı o cümle bir daha hiç denenmezdi.
        let mut b = bellek();
        let mut m = SahteMotor::yeni(&[("Nothing.", "")]);
        cevir("Nothing.", &mut b, &mut m).unwrap();
        assert!(b.ara("Nothing.").is_none());
    }

    #[test]
    fn kaynak_her_birimde_duruyor() {
        let mut b = bellek();
        let mut m = SahteMotor::yeni(&[]);
        let s = cevir("Open the gate.", &mut b, &mut m).unwrap();
        assert_eq!(s.birimler[0].kaynak, "Open the gate.");
    }

    // --- Karar #22: önbellek ---

    #[test]
    fn ikinci_seferde_model_calismiyor() {
        let mut b = bellek();
        let mut m = SahteMotor::yeni(&[("Open the gate.", "Kapıyı aç.")]);
        cevir("Open the gate.", &mut b, &mut m).unwrap();
        let s = cevir("Open the gate.", &mut b, &mut m).unwrap();
        assert_eq!(m.cagri, 1, "aynı metin için model ikinci kez çalıştı");
        assert_eq!(s.bellekten, 1);
        assert_eq!(s.birimler[0].ceviri, "Kapıyı aç.");
    }

    #[test]
    fn kullanici_kaydi_makineye_ezdirilmiyor() {
        let mut b = bellek();
        b.ekle("Open the gate.", "Kapıyı açsana.", Koken::Kullanici);
        let mut m = SahteMotor::yeni(&[("Open the gate.", "Kapıyı aç.")]);
        let s = cevir("Open the gate.", &mut b, &mut m).unwrap();
        assert_eq!(s.birimler[0].ceviri, "Kapıyı açsana.");
        assert_eq!(s.birimler[0].koken, Koken::Kullanici);
        assert_eq!(m.cagri, 0);
    }

    // --- Karar #29 zaaf 2: terim sözlüğü ---

    #[test]
    fn terim_modele_gitmiyor_ve_geri_konuyor() {
        let mut b = bellek();
        b.terim_ekle("Longsword", "Uzun Kılıç");
        // Model işareti olduğu gibi bırakıyor (beklenen davranış).
        let mut m = SahteMotor::yeni(&[("Take the #0#.", "#0# al.")]);
        let s = cevir("Take the Longsword.", &mut b, &mut m).unwrap();
        assert_eq!(s.birimler[0].ceviri, "Uzun Kılıç al.");
        assert_eq!(s.birimler[0].korunan_terimler, vec!["Longsword".to_string()]);
        assert!(s.birimler[0].kayip_terimler.is_empty());
    }

    #[test]
    fn kaybolan_terim_bildiriliyor() {
        // Karar #30'un ölçülmemiş varsayımı: işaret modelden sağ çıkar.
        // Çıkmazsa sessiz kalmıyor.
        let mut b = bellek();
        b.terim_ekle("Stamina", "Dayanıklılık");
        let mut m = SahteMotor::yeni(&[("Your #0# is low.", "Bir şey azaldı.")]);
        let s = cevir("Your Stamina is low.", &mut b, &mut m).unwrap();
        assert_eq!(s.birimler[0].kayip_terimler, vec!["Stamina".to_string()]);
        assert!(s.kusurlu());
    }

    // --- Karar #29 zaaf 1: büyük harf ---

    #[test]
    fn buyuk_harf_kucultuluyor_ve_soyleniyor() {
        let mut b = bellek();
        let mut m = SahteMotor::yeni(&[]);
        let s = cevir("MISSION FAILED", &mut b, &mut m).unwrap();
        assert!(!s.uyarilar.is_empty(), "küçültme sessizce yapıldı");
        // Modele küçültülmüş hali gitti, kullanıcıya özgün hali gösteriliyor.
        assert_eq!(s.birimler[0].kaynak, "MISSION FAILED");
        assert!(s.birimler[0].ceviri.contains("Mission failed"));
    }

    // --- Sınır durumları ---

    #[test]
    fn bos_metin_bos_sonuc() {
        let mut b = bellek();
        let mut m = SahteMotor::yeni(&[]);
        let s = cevir("   \n  ", &mut b, &mut m).unwrap();
        assert!(s.birimler.is_empty());
        assert_eq!(m.cagri, 0);
    }

    #[test]
    fn duz_metin_birimleri_satirlara_yaziyor() {
        let mut b = bellek();
        let mut m = SahteMotor::yeni(&[("A.", "A çevrildi."), ("B.", "B çevrildi.")]);
        let s = cevir("A.\nB.", &mut b, &mut m).unwrap();
        assert_eq!(s.duz_metin(), "A çevrildi.\nB çevrildi.");
    }

    #[test]
    fn kusursuz_sonuc_kusurlu_degil() {
        let mut b = bellek();
        let mut m = SahteMotor::yeni(&[("Open the gate.", "Kapıyı aç.")]);
        let s = cevir("Open the gate.", &mut b, &mut m).unwrap();
        assert!(!s.kusurlu());
    }
}
