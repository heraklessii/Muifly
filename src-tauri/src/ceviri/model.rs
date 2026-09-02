//! Çeviri modelinin dosyaları: nerede duruyor, kurulu mu, nasıl iniyor.
//!
//! ## Model ikiliye GİRMİYOR
//!
//! Karar #1, Electron'u "~120 MB RAM tabanı, 'sistemini hafifleten araç'
//! iddiasıyla çelişir" diye elemişti. Yarım gigabaytlık bir NMT modelini
//! kuruluma koymak aynı argümanı bu sefer bize karşı çalıştırırdı — hem de
//! ekran çevirisini hiç kullanmayacak kullanıcılara ödettirerek. Karar #22
//! bu yüzden modeli **isteğe bağlı indirme** olarak tanımladı; burası o
//! kararın uygulaması.
//!
//! ## Dosyalar ve neden bu dosyalar
//!
//! Model `onnx-community/opus-mt-tc-big-en-tr`, int8 nicelenmiş. Karar
//! #29'un ölçtüğü ve bu makinede bir kez daha koşturulan sürüm bu:
//! niceleme kaliteyi bozmuyor, süreyi yarıya indiriyor ve indirmeyi
//! ~1,1 GB'dan ~507 MiB'a çekiyor.
//!
//! `tokenizer.json` parçalama için, `vocab.json` sayıya çevirme için. İkisi
//! birden gerekiyor çünkü deponun tokenizer'ının id uzayı modelin gömme
//! matrisiyle aynı değil — ölçülen tablo karar #29'da, uygulaması
//! `cevirici::Sozluk`'te.
//!
//! ## Beklenen özetler sabit
//!
//! Her dosyanın boyutu ve SHA-256'sı kodda yazılı. Sebep iki yönlü: yarım
//! inen bir dosya ONNX Runtime'ın içinde anlaşılmaz bir hatayla patlar,
//! değiştirilmiş bir dosya ise hiç patlamaz. Bunlar **bu makinede indirilip
//! çeviri kalitesi sınanmış** dosyaların özetleri; depo bir gün dosyaları
//! yeniden yüklerse özet tutmaz ve kullanıcı denenmemiş bir modelle sessizce
//! çalışmaz.
//!
//! ## İndirme kullanıcının başlattığı tek ağ trafiği
//!
//! Program arkada kendiliğinden hiçbir yere bağlanmıyor
//! (`docs/DISTRIBUTION.md`). Bu dosyalar yalnızca kullanıcı indirmeyi
//! başlattığında iniyor, adresler kodda sabit ve indirilen her şey
//! doğrulanıyor.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use serde::Serialize;

use crate::error::{Error, Result};

/// Deponun kök adresi. Sabit: kullanıcıdan ya da bir ayardan adres almıyoruz.
const KOK: &str = "https://huggingface.co/onnx-community/opus-mt-tc-big-en-tr/resolve/main";

/// İndirilecek tek bir dosya.
pub struct Dosya {
    /// Diskteki adı.
    pub ad: &'static str,
    /// Depodaki yolu (`KOK`'e göre).
    pub uzak: &'static str,
    pub bayt: u64,
    /// Beklenen SHA-256, onaltılık küçük harf.
    pub ozet: &'static str,
}

impl Dosya {
    pub fn url(&self) -> String {
        format!("{KOK}/{}", self.uzak)
    }
}

/// Modelin dört parçası.
pub const DOSYALAR: [Dosya; 4] = [
    Dosya {
        ad: "vocab.json",
        uzak: "vocab.json",
        bayt: 1_501_287,
        ozet: "01ef522455d2bf22b0716416ee34688cc2b3f5fa257f742665d21b6acc450ac1",
    },
    Dosya {
        ad: "tokenizer.json",
        uzak: "tokenizer.json",
        bayt: 5_606_923,
        ozet: "32061d3dd6183010d1317ecdbfce02d747560703cc1fc6a2b2263af79ae9e7df",
    },
    Dosya {
        ad: "encoder.onnx",
        uzak: "onnx/encoder_model_quantized.onnx",
        bayt: 135_730_164,
        ozet: "58bfc076347346049a52a8d53b1f6e4664af74e9737aded374d7d1f78805f46e",
    },
    Dosya {
        ad: "decoder.onnx",
        uzak: "onnx/decoder_model_quantized.onnx",
        bayt: 395_310_868,
        ozet: "44264bac42bc67e6ef5a9f37b9ef364e1b93277026399ca6d90eb5752ee5164b",
    },
];

/// İndirmenin toplam boyutu.
pub fn toplam_bayt() -> u64 {
    DOSYALAR.iter().map(|d| d.bayt).sum()
}

/// Model dosyalarının klasörü: `%APPDATA%\Muifly\ceviri\model`.
///
/// Çeviri belleklerinin yanında ama ayrı klasörde: bellek kullanıcının
/// emeği, model indirilebilir bir dosya yığını. "Modeli sil" bir klasör
/// silmek oluyor ve kullanıcının düzeltmelerine dokunmuyor.
pub fn dizin() -> PathBuf {
    crate::settings::ceviri_dizini().join("model")
}

pub fn yol(ad: &str) -> PathBuf {
    dizin().join(ad)
}

/// Dosya var ve boyutu beklenen mi?
fn boyutu_dogru(yol: &Path, bayt: u64) -> bool {
    std::fs::metadata(yol).map(|m| m.len()).ok() == Some(bayt)
}

/// Dört dosya da yerinde ve beklenen boyutta mı?
///
/// Özet burada **hesaplanmıyor**: yarım gigabaytı her durum sorgusunda
/// okumak, arayüzü açan herkese saniyeler ödetirdi. Özet indirmeden hemen
/// sonra ve kullanıcı istediğinde (`dogrula`) bakılıyor.
pub fn kurulu() -> bool {
    DOSYALAR.iter().all(|d| boyutu_dogru(&yol(d.ad), d.bayt))
}

/// Arayüze giden model durumu.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelDurumu {
    pub kurulu: bool,
    /// İndirme boyutu — arayüz "bu kadar inecek" diyebilsin diye.
    pub toplam_bayt: u64,
    /// Klasörün yolu. Kullanıcı dosyaların nerede durduğunu görebilmeli.
    pub dizin: String,
    /// Eksik ya da boyutu tutmayan dosyalar.
    pub eksikler: Vec<String>,
    /// Model kimliği — hangi modelin indirileceği saklanmıyor.
    pub kaynak: String,
}

pub fn durum() -> ModelDurumu {
    let eksikler: Vec<String> = DOSYALAR
        .iter()
        .filter(|d| !boyutu_dogru(&yol(d.ad), d.bayt))
        .map(|d| d.ad.to_string())
        .collect();
    ModelDurumu {
        kurulu: eksikler.is_empty(),
        toplam_bayt: toplam_bayt(),
        dizin: dizin().display().to_string(),
        eksikler,
        kaynak: "onnx-community/opus-mt-tc-big-en-tr (int8)".into(),
    }
}

/// Bütün dosyaların özetini hesaplayıp beklenenle karşılaştırır.
///
/// Yavaş (yarım gigabayt okunuyor) ve bilerek ayrı: indirmeden sonra bir kez
/// ve kullanıcı "doğrula" dediğinde çalışıyor.
pub fn dogrula() -> Result<()> {
    for d in DOSYALAR.iter() {
        let y = yol(d.ad);
        if !boyutu_dogru(&y, d.bayt) {
            return Err(Error::Indirme(format!("{} eksik ya da boyutu tutmuyor", d.ad)));
        }
        dosyayi_dogrula(&y, d)?;
    }
    Ok(())
}

fn dosyayi_dogrula(yol: &Path, d: &Dosya) -> Result<()> {
    let bulunan = super::sha256::dosya(yol)?;
    if bulunan != d.ozet {
        return Err(Error::Indirme(format!(
            "{} beklenen dosya değil (SHA-256 tutmuyor) — indirme bozulmuş ya da \
             depodaki dosya değişmiş olabilir",
            d.ad
        )));
    }
    Ok(())
}

/// İndirmenin hangi adımda olduğu.
#[derive(Debug, Clone)]
pub struct Adim {
    /// Kaçıncı dosya (1'den başlar) ve kaç dosya var.
    pub sira: usize,
    pub adet: usize,
    pub ad: &'static str,
    /// Bütün indirmenin toplamı üzerinden inen bayt.
    pub inen: u64,
    pub toplam: u64,
}

/// Eksik dosyaları indirir ve her birini doğrular.
///
/// Zaten yerinde ve boyutu doğru olan dosya **yeniden inmiyor**: yarıda
/// kesilen bir indirmeyi baştan başlatmak, 400 MB'ı ikinci kez indirmek
/// olurdu.
pub fn indir(iptal: &AtomicBool, mut bildir: impl FnMut(Adim)) -> Result<()> {
    let toplam = toplam_bayt();
    let mut onceki: u64 = 0;

    for (i, d) in DOSYALAR.iter().enumerate() {
        let hedef = yol(d.ad);
        if boyutu_dogru(&hedef, d.bayt) {
            onceki += d.bayt;
            bildir(Adim {
                sira: i + 1,
                adet: DOSYALAR.len(),
                ad: d.ad,
                inen: onceki,
                toplam,
            });
            continue;
        }

        let baslangic = onceki;
        super::indirme::indir(&d.url(), &hedef, iptal, |il| {
            bildir(Adim {
                sira: i + 1,
                adet: DOSYALAR.len(),
                ad: d.ad,
                inen: baslangic + il.inen,
                toplam,
            });
        })?;

        // Doğrulama indirmenin parçası: bozuk bir dosyayı diskte bırakmak,
        // bir sonraki açılışta "model kurulu" demek olurdu.
        if let Err(e) = dosyayi_dogrula(&hedef, d) {
            let _ = std::fs::remove_file(&hedef);
            return Err(e);
        }
        onceki += d.bayt;
    }
    Ok(())
}

/// Model klasörünü siler.
///
/// Çeviri belleğine dokunmuyor: kullanıcının onayladığı düzeltmeler modelin
/// değil kullanıcının malı (karar #22).
pub fn sil() -> Result<()> {
    let d = dizin();
    if d.exists() {
        std::fs::remove_dir_all(&d)?;
    }
    Ok(())
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn dosya_adresleri_depoya_bagli() {
        for d in DOSYALAR.iter() {
            let u = d.url();
            assert!(u.starts_with("https://"), "güvensiz adres: {u}");
            assert!(u.starts_with(KOK), "adres depo dışına çıkıyor: {u}");
            // Adresin çözümlenebildiği de burada sınanıyor: sabit bir dizgede
            // yapılan bir yazım hatası, ancak indirme anında ortaya çıkardı.
            assert!(super::super::indirme::ayristir(&u).is_some(), "adres bozuk: {u}");
        }
    }

    #[test]
    fn ozetler_sha256_bicimi() {
        for d in DOSYALAR.iter() {
            assert_eq!(d.ozet.len(), 64, "{} özeti 64 karakter değil", d.ad);
            assert!(
                d.ozet.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
                "{} özeti küçük harf onaltılık değil",
                d.ad
            );
            assert!(d.bayt > 0);
        }
    }

    #[test]
    fn dosya_adlari_tekil_ve_yolsuz() {
        let mut adlar: Vec<&str> = DOSYALAR.iter().map(|d| d.ad).collect();
        adlar.sort_unstable();
        let onceki = adlar.len();
        adlar.dedup();
        assert_eq!(adlar.len(), onceki, "aynı ada iki dosya yazılıyor");
        for d in DOSYALAR.iter() {
            // Ad doğrudan yol birleştirmede kullanılıyor; dizin ayracı taşımamalı.
            assert!(!d.ad.contains('/') && !d.ad.contains('\\') && !d.ad.contains(".."));
        }
    }

    /// Ürün duruşu: model ikiliye gömülmüyor, indiriliyor (karar #1, #22).
    ///
    /// Kod seviyesinde korunması gereken şey bu dosyaya modelin baytlarını
    /// gömen bir makronun girmemesi. Testin ölçtüğü şey bir davranış değil,
    /// bir duruş.
    #[test]
    fn model_ikiliye_gomulmuyor() {
        let kaynak = include_str!("model.rs");
        // Aranan dize parça parça kuruluyor: kaynağın içine olduğu gibi
        // yazılsaydı test kendi kendini yakalar ve hiçbir zaman geçmezdi.
        let yasak = format!("include{}", "_bytes!");
        assert!(
            !kaynak.contains(&yasak),
            "model ikiliye gömülmüş — karar #1 ve #22'ye aykırı"
        );
    }

    #[test]
    fn model_dizini_veri_kokunun_altinda() {
        assert!(dizin().starts_with(crate::settings::veri_dizini()));
        assert_ne!(dizin(), crate::settings::ceviri_dizini());
    }

    #[test]
    fn toplam_dosyalarin_toplami() {
        assert_eq!(
            toplam_bayt(),
            DOSYALAR.iter().map(|d| d.bayt).sum::<u64>()
        );
        // İndirme yarım gigabayt mertebesinde; mağaza sayfasında yazılacak
        // sayı buradan geliyor (karar #29).
        assert!(toplam_bayt() > 500_000_000);
    }
}
