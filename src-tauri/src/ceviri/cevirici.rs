//! Çeviri modelinin çalıştırılması — ONNX Runtime üzerinde greedy çözümleme.
//!
//! ## Kod nereden geliyor
//!
//! `arac/ceviri-sonda/` atılacak bir fizibilite denemesiydi ve sorusunu
//! cevapladı (karar #29). Buradaki akış onun ürün hâli; sondanın iki tuzağı
//! da burada **çözülmüş olarak** duruyor, çünkü ikisi de saatler yedi ve
//! ikisinin de belirtisi sessizdi:
//!
//! 1. **Tokenizer'ın id uzayı modelin gömme matrisiyle aynı değil.**
//!    Parçalamayı `tokenizer.json` yapıyor (parça **dizeleri** doğru),
//!    sayıya çevirmeyi `vocab.json` (bkz. [`Sozluk`]). İd'ler karışırsa
//!    model çökmüyor, NaN üretmiyor — akıcı ama anlamsız Türkçe üretiyor.
//!    Bir hata olduğu ancak çıktıyı okuyunca anlaşılıyor.
//! 2. **`Precompiled` normalleştiricinin eşlemesi boş** ve `tokenizers`
//!    kasası bunu çözemeyip panikliyor. Anlamca "uygulanacak eşleme yok"
//!    demek, o yüzden `normalizer` alanı yükleme sırasında `null`lanıyor.
//!    Diske ikinci bir dosya yazılmıyor: yamalama bellekte yapılıyor.
//!
//! ## Oyunun çekirdeklerini almıyor
//!
//! ONNX Runtime varsayılan olarak makinedeki bütün çekirdeklere yayılıyor.
//! Kimliği "oyununu daha iyi çalıştırır" olan bir araçta, çeviri isteği
//! sırasında bütün çekirdekleri doldurmak ironinin ötesinde bir hata olurdu
//! (karar #22 bu riski "frame hitch" diye adlandırmıştı). İş parçacığı
//! sayısı bu yüzden [`IS_PARCACIGI`] ile sınırlı.
//!
//! ## Boştayken bellekten düşüyor
//!
//! Karar #22: "Model isteğe bağlı indirilir, boştayken bellekten düşer."
//! Düşürme kararı burada değil `super::Ceviri`de: bu tip yalnızca yüklenip
//! bırakılabilir olmak zorunda, ne zaman bırakılacağına karar vermek onun
//! işi değil.

use std::collections::HashMap;
use std::path::Path;

use ort::session::Session;
use ort::value::Tensor;
use tokenizers::Tokenizer;

use crate::error::{Error, Result};

/// Çözümlemenin başlangıç token'ı (`decoder_start_token_id`, `<pad>`).
const BASLANGIC_PARCA: &str = "<pad>";
/// Cümle sonu (`</s>`).
const SON_PARCA: &str = "</s>";

/// Bir çeviride üretilecek en fazla token.
///
/// Girdi zaten cümle cümle geliyor (`super::cumle`), yani 96 token bir
/// cümle için geniş. Sınırın işi kaliteyi kısmak değil, tekrara giren bir
/// çözümlemenin sonsuza kadar dönmesini engellemek.
pub const EN_FAZLA_TOKEN: usize = 96;

/// Çıkarımın kullanacağı iş parçacığı sayısı.
///
/// İki: çeviri isteği tuşa basınca ve tek seferlik (karar #22), yani
/// gecikmeyi son milisaniyesine kadar sıkıştırmanın karşılığı yok. Buna
/// karşılık oyun çalışırken serbest bırakılan her çekirdek, kullanıcının bu
/// programı açma sebebine doğrudan aykırı.
const IS_PARCACIGI: usize = 2;

/// Modelin kelime dağarcığı: parça dizesi ↔ id.
///
/// Neden `tokenizer.json`'un id'leri kullanılmıyor — ölçülen tablo karar
/// #29'da:
///
/// | token | vocab.json (modelin uzayı) | tokenizer.json |
/// |---|---|---|
/// | `▁The` | 50392 | 23901 |
/// | `</s>` | 43741 | 1072 |
///
/// `tokenizer.json` kendi içinde tutarlı, ama id'leri modele verilince
/// model bambaşka gömme satırlarını okuyor.
pub struct Sozluk {
    ileri: HashMap<String, i64>,
    geri: Vec<String>,
}

impl Sozluk {
    pub fn yukle(yol: &Path) -> Result<Self> {
        let ham = std::fs::read_to_string(yol)?;
        let ileri: HashMap<String, i64> = serde_json::from_str(&ham)?;
        if ileri.is_empty() {
            return Err(Error::Indirme("kelime dağarcığı boş".into()));
        }
        let en_buyuk = ileri.values().copied().max().unwrap_or(0).max(0) as usize;
        let mut geri = vec![String::new(); en_buyuk + 1];
        for (k, v) in &ileri {
            if *v >= 0 {
                geri[*v as usize] = k.clone();
            }
        }
        Ok(Self { ileri, geri })
    }

    pub fn id(&self, parca: &str) -> i64 {
        *self
            .ileri
            .get(parca)
            .or_else(|| self.ileri.get("<unk>"))
            .unwrap_or(&0)
    }

    /// Özel bir token'ın id'si; sözlükte yoksa `None`.
    ///
    /// `id`den ayrı: orada bulunamayan parça `<unk>`e düşüyor ve bu, cümle
    /// sonu token'ı için felaket olurdu — çözümleme hiç durmazdı.
    fn ozel(&self, parca: &str) -> Option<i64> {
        self.ileri.get(parca).copied()
    }

    fn parca(&self, id: i64) -> &str {
        if id < 0 {
            return "";
        }
        self.geri.get(id as usize).map(|s| s.as_str()).unwrap_or("")
    }

    /// Parça dizisini okunabilir metne çevirir.
    ///
    /// SentencePiece'te `▁` kelime başı demek.
    pub fn metin(&self, idler: &[i64]) -> String {
        let mut s = String::new();
        for id in idler {
            let p = self.parca(*id);
            if let Some(kalan) = p.strip_prefix('\u{2581}') {
                if !s.is_empty() {
                    s.push(' ');
                }
                s.push_str(kalan);
            } else {
                s.push_str(p);
            }
        }
        s.trim().to_string()
    }
}

/// Yüklenmiş model.
pub struct Cevirici {
    kodlayici: Session,
    cozucu: Session,
    tokenlestirici: Tokenizer,
    sozluk: Sozluk,
    baslangic: i64,
    son: i64,
}

fn ort_hata(ne: &str, e: impl std::fmt::Display) -> Error {
    Error::Ceviri(format!("{ne}: {e}"))
}

impl Cevirici {
    /// Model dosyalarını yükler. Saniyeler sürüyor; çağıran ayrı bir iş
    /// parçacığında olmak zorunda.
    pub fn yukle() -> Result<Self> {
        let dizin = super::model::dizin();
        if !super::model::kurulu() {
            return Err(Error::Ceviri(
                "çeviri modeli kurulu değil — Çeviri ekranından indirilebilir".into(),
            ));
        }

        let sozluk = Sozluk::yukle(&dizin.join("vocab.json"))?;
        let baslangic = sozluk.ozel(BASLANGIC_PARCA).ok_or_else(|| {
            Error::Ceviri("kelime dağarcığında başlangıç token'ı yok".into())
        })?;
        let son = sozluk
            .ozel(SON_PARCA)
            .ok_or_else(|| Error::Ceviri("kelime dağarcığında cümle sonu token'ı yok".into()))?;

        let tokenlestirici = tokenlestirici_yukle(&dizin.join("tokenizer.json"))?;

        let kodlayici = oturum(&dizin.join("encoder.onnx"))?;
        let cozucu = oturum(&dizin.join("decoder.onnx"))?;

        Ok(Self {
            kodlayici,
            cozucu,
            tokenlestirici,
            sozluk,
            baslangic,
            son,
        })
    }

    /// Tek bir cümleyi çevirir.
    ///
    /// Girdi **bir cümle** olmak zorunda değil ama olması bekleniyor:
    /// çok cümleli girdide modelin bir cümleyi sessizce düşürdüğü ölçüldü
    /// (karar #29 zaaf 3). Bölme işi `super::cumle`de.
    pub fn cevir(&mut self, kaynak: &str) -> Result<String> {
        let kaynak = kaynak.trim();
        if kaynak.is_empty() {
            return Ok(String::new());
        }

        // `false`: özel token'ı tokenizer eklemesin — id'si yanlış uzayda olur.
        let kodlama = self
            .tokenlestirici
            .encode(kaynak, false)
            .map_err(|e| ort_hata("metin parçalanamadı", e))?;

        let mut girdi: Vec<i64> = kodlama
            .get_tokens()
            .iter()
            .map(|p| self.sozluk.id(p))
            .collect();
        girdi.push(self.son);
        let maske: Vec<i64> = vec![1; girdi.len()];
        let n = girdi.len() as i64;

        let cikti = self
            .kodlayici
            .run(ort::inputs![
                "input_ids" => Tensor::from_array((vec![1i64, n], girdi))
                    .map_err(|e| ort_hata("girdi tensörü kurulamadı", e))?,
                "attention_mask" => Tensor::from_array((vec![1i64, n], maske.clone()))
                    .map_err(|e| ort_hata("maske tensörü kurulamadı", e))?,
            ])
            .map_err(|e| ort_hata("kodlayıcı çalışmadı", e))?;

        let (gizli_sekil, gizli) = cikti["last_hidden_state"]
            .try_extract_tensor::<f32>()
            .map_err(|e| ort_hata("kodlayıcı çıktısı okunamadı", e))?;
        let gizli_sekil: Vec<i64> = gizli_sekil.iter().copied().collect();
        let gizli: Vec<f32> = gizli.to_vec();

        let mut uretilen: Vec<i64> = vec![self.baslangic];
        loop {
            let t = uretilen.len() as i64;
            let c = self
                .cozucu
                .run(ort::inputs![
                    "input_ids" => Tensor::from_array((vec![1i64, t], uretilen.clone()))
                        .map_err(|e| ort_hata("çözücü girdisi kurulamadı", e))?,
                    "encoder_attention_mask" => Tensor::from_array((vec![1i64, n], maske.clone()))
                        .map_err(|e| ort_hata("çözücü maskesi kurulamadı", e))?,
                    "encoder_hidden_states" => Tensor::from_array((gizli_sekil.clone(), gizli.clone()))
                        .map_err(|e| ort_hata("çözücü durumu kurulamadı", e))?,
                ])
                .map_err(|e| ort_hata("çözücü çalışmadı", e))?;

            let (sekil, logit) = c["logits"]
                .try_extract_tensor::<f32>()
                .map_err(|e| ort_hata("çözücü çıktısı okunamadı", e))?;
            let kelime = *sekil.last().unwrap_or(&0) as usize;
            if kelime == 0 || logit.len() < kelime {
                return Err(Error::Ceviri("çözücü beklenmedik bir çıktı verdi".into()));
            }
            let son_adim = &logit[logit.len() - kelime..];

            let mut en_iyi = 0usize;
            let mut en_iyi_deger = f32::NEG_INFINITY;
            for (i, v) in son_adim.iter().enumerate() {
                // `bad_words_ids`: `<pad>` üretilmesi yasak.
                if i as i64 == self.baslangic {
                    continue;
                }
                if *v > en_iyi_deger {
                    en_iyi_deger = *v;
                    en_iyi = i;
                }
            }
            uretilen.push(en_iyi as i64);
            if en_iyi as i64 == self.son || uretilen.len() >= EN_FAZLA_TOKEN {
                break;
            }
        }

        let idler: Vec<i64> = uretilen
            .iter()
            .skip(1)
            .filter(|t| **t != self.son && **t != self.baslangic)
            .copied()
            .collect();
        Ok(self.sozluk.metin(&idler))
    }
}

/// Akışın beklediği arayüz.
///
/// Ayrı bir `impl`, çünkü `akis` bu tipi tanımıyor: orası ONNX'suz test
/// edilebilsin diye bir arayüz üzerinden çalışıyor.
impl super::akis::Motor for Cevirici {
    fn cevir(&mut self, kaynak: &str) -> Result<String> {
        Cevirici::cevir(self, kaynak)
    }
}

/// Tokenizer'ı yükler ve normalleştiriciyi bellekte `null`lar.
///
/// Depodaki `tokenizer.json`'un `Precompiled` normalleştiricisinin
/// `precompiled_charsmap` alanı `null` ve `tokenizers` kasası bunu
/// çözemeyip panikliyor. Anlamca "uygulanacak eşleme yok" demek; alan
/// düşürülünce parçalama doğru çalışıyor (karar #29).
///
/// Diske yamalı bir ikinci dosya yazılmıyor: kullanıcının klasöründe
/// açıklaması olmayan bir dosya bırakmak, "dosyalar okunabilir olsun"
/// ilkesine aykırı olurdu.
fn tokenlestirici_yukle(yol: &Path) -> Result<Tokenizer> {
    let ham = std::fs::read_to_string(yol)?;
    let mut agac: serde_json::Value = serde_json::from_str(&ham)?;
    if let Some(nesne) = agac.as_object_mut() {
        nesne.insert("normalizer".into(), serde_json::Value::Null);
    }
    let yamali = serde_json::to_vec(&agac)?;
    Tokenizer::from_bytes(&yamali).map_err(|e| ort_hata("tokenizer okunamadı", e))
}

fn oturum(yol: &Path) -> Result<Session> {
    let olusturucu = Session::builder().map_err(|e| ort_hata("ONNX oturumu kurulamadı", e))?;
    let olusturucu = olusturucu
        .with_intra_threads(IS_PARCACIGI)
        .map_err(|e| ort_hata("iş parçacığı sayısı ayarlanamadı", e))?;
    let mut olusturucu = olusturucu
        .with_inter_threads(1)
        .map_err(|e| ort_hata("iş parçacığı sayısı ayarlanamadı", e))?;
    olusturucu
        .commit_from_file(yol)
        .map_err(|e| ort_hata("model yüklenemedi", e))
}

#[cfg(test)]
mod testler {
    use super::*;

    fn sozluk_yaz(dizin: &Path, ciftler: &[(&str, i64)]) -> std::path::PathBuf {
        let yol = dizin.join("vocab.json");
        let nesne: serde_json::Map<String, serde_json::Value> = ciftler
            .iter()
            .map(|(k, v)| ((*k).to_string(), serde_json::json!(v)))
            .collect();
        std::fs::write(&yol, serde_json::to_string(&nesne).unwrap()).unwrap();
        yol
    }

    #[test]
    fn sozluk_gidis_donus() {
        let d = tempfile::tempdir().unwrap();
        let yol = sozluk_yaz(
            d.path(),
            &[
                ("\u{2581}Merhaba", 5),
                ("\u{2581}dünya", 9),
                ("!", 3),
                ("<unk>", 1),
                ("</s>", 7),
            ],
        );
        let s = Sozluk::yukle(&yol).unwrap();
        assert_eq!(s.id("\u{2581}Merhaba"), 5);
        assert_eq!(s.metin(&[5, 9, 3]), "Merhaba dünya!");
    }

    #[test]
    fn bilinmeyen_parca_unk_oluyor() {
        let d = tempfile::tempdir().unwrap();
        let yol = sozluk_yaz(d.path(), &[("<unk>", 1), ("\u{2581}bir", 2)]);
        let s = Sozluk::yukle(&yol).unwrap();
        assert_eq!(s.id("hicboyleparcayok"), 1);
    }

    /// Cümle sonu token'ı `<unk>`e düşmemeli.
    ///
    /// Düşseydi çözümleme durma koşulunu hiç göremez ve her çeviri
    /// [`EN_FAZLA_TOKEN`] tur dönerdi — yavaş, ama daha kötüsü sonunda
    /// anlamsız bir kuyruk üreten bir çıktı.
    #[test]
    fn ozel_token_bulunamazsa_unk_donmuyor() {
        let d = tempfile::tempdir().unwrap();
        let yol = sozluk_yaz(d.path(), &[("<unk>", 1), ("\u{2581}bir", 2)]);
        let s = Sozluk::yukle(&yol).unwrap();
        assert_eq!(s.ozel("</s>"), None);
        assert_eq!(s.ozel("\u{2581}bir"), Some(2));
    }

    #[test]
    fn bos_sozluk_reddediliyor() {
        let d = tempfile::tempdir().unwrap();
        let yol = d.path().join("vocab.json");
        std::fs::write(&yol, "{}").unwrap();
        assert!(Sozluk::yukle(&yol).is_err());
    }

    #[test]
    fn tanimsiz_id_metni_bozmuyor() {
        let d = tempfile::tempdir().unwrap();
        let yol = sozluk_yaz(d.path(), &[("\u{2581}bir", 2)]);
        let s = Sozluk::yukle(&yol).unwrap();
        // Sözlükte olmayan bir id çıktıda boş kalıyor, panik yok.
        assert_eq!(s.metin(&[2, 9999, -1]), "bir");
    }


    // -----------------------------------------------------------------
    // Gerçek modelle uçtan uca — `--ignored` ile koşuyor
    // -----------------------------------------------------------------
    //
    // `scaling::testler::gercek_ekranda_bir_tur` ile aynı gerekçe (karar
    // #32): birim testleri bu boru hattının doğru çalıştığını gösteremez.
    // Modelin yüklendiğini, tokenizer id uzayının doğru olduğunu ve
    // çıktının anlamlı Türkçe olduğunu ancak gerçek dosyalarla koşup
    // okuyarak anlıyoruz. Yarım gigabaytlık dosyalara bağlı olduğu için
    // CI'da koşamaz; elle:
    //
    // ```text
    // cargo test ceviri::cevirici::testler::gercek_modelle -- --ignored --nocapture
    // ```

    #[test]
    #[ignore = "yarım gigabaytlık model dosyalarına ihtiyaç duyuyor"]
    fn gercek_modelle_uctan_uca() {
        if !super::super::model::kurulu() {
            panic!("model kurulu değil: Çeviri ekranından indirin");
        }
        let basla = std::time::Instant::now();
        let mut c = Cevirici::yukle().expect("model yüklenemedi");
        println!("model yüklendi: {} ms", basla.elapsed().as_millis());

        // Karar #29'un külliyatından, üç zaafın da göründüğü satırlar.
        let ornekler = [
            "You should speak with the innkeeper before nightfall.",
            "Press F to pick up the ancient key.",
            "Keep your guard up.",
            "This one bites back.",
            "Mission failed - return to checkpoint",
        ];
        for o in ornekler {
            let t = std::time::Instant::now();
            let cikti = c.cevir(o).expect("çeviri başarısız");
            println!("[{} ms]\n  EN: {o}\n  TR: {cikti}", t.elapsed().as_millis());
            assert!(!cikti.trim().is_empty(), "boş çeviri: {o}");
        }
    }

    /// Akışın tamamı gerçek modelle: ön işleme + bölme + sözlük + bellek.
    #[test]
    #[ignore = "yarım gigabaytlık model dosyalarına ihtiyaç duyuyor"]
    fn gercek_modelle_akis() {
        if !super::super::model::kurulu() {
            panic!("model kurulu değil: Çeviri ekranından indirin");
        }
        let mut c = Cevirici::yukle().expect("model yüklenemedi");
        let mut bellek = super::super::CeviriBellegi::yeni("deneme.exe", "en", "tr");
        bellek.terim_ekle("Longsword", "Uzun Kılıç");

        let ham = "MISSION FAILED - RETURN TO CHECKPOINT\n                   Keep your guard up. This one bites back.\n                   Take the Longsword.";
        let sonuc = super::super::akis::cevir(ham, &mut bellek, &mut c).expect("akış başarısız");

        for u in &sonuc.uyarilar {
            println!("uyarı: {u}");
        }
        for b in &sonuc.birimler {
            println!(
                "  EN: {}\n  TR: {}{}{}",
                b.kaynak,
                b.ceviri,
                if b.bos { "   [BOŞ]" } else { "" },
                if b.kayip_terimler.is_empty() {
                    String::new()
                } else {
                    format!("   [KAYIP: {}]", b.kayip_terimler.join(", "))
                }
            );
        }

        // Karar #29 zaaf 3: iki cümle iki birim olmalı ve ikisi de dolu.
        assert!(sonuc.birimler.len() >= 4, "birimler: {:?}", sonuc.birimler);
        assert!(
            sonuc.birimler.iter().all(|b| !b.bos),
            "bir birim boş döndü — zaaf 3 hâlâ açık"
        );
        // Karar #30'un ölçülmemiş varsayımı burada sınanıyor: işaret
        // SentencePiece'ten ve greedy çözümlemeden sağ çıkıyor mu?
        let kayip: Vec<&String> = sonuc
            .birimler
            .iter()
            .flat_map(|b| b.kayip_terimler.iter())
            .collect();
        println!("kayıp terimler: {kayip:?}");
    }


    /// Hangi terim işareti modelden sağ çıkıyor? (karar #30'un açık sorusu)
    ///
    /// `sozluk` terimleri çeviriden önce bir işaretle değiştiriyor ve o
    /// işaretin SentencePiece parçalamasından ve greedy çözümlemeden
    /// bozulmadan geçeceği karar #30'da **varsayım** olarak yazılmıştı.
    /// İlk ölçümde varsayım çürüdü: `[[0]]` → `[0]`.
    ///
    /// Bu test aday biçimleri sırayla deniyor. Kalıcı, çünkü model ya da
    /// tokenizer değişirse aynı soru yeniden sorulacak ve cevabı tahminle
    /// değil ölçümle verilmeli:
    ///
    /// ```text
    /// cargo test ceviri::cevirici::testler::isaret_adaylari -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "yarım gigabaytlık model dosyalarına ihtiyaç duyuyor"]
    fn isaret_adaylari() {
        if !super::super::model::kurulu() {
            panic!("model kurulu değil: Çeviri ekranından indirin");
        }
        let mut c = Cevirici::yukle().expect("model yüklenemedi");

        let adaylar = [
            "[[0]]", "[0]", "{0}", "<0>", "#0#", "@0@", "%0%", "((0))", "|0|",
            "Zqx0", "XX0XX", "\u{27e6}0\u{27e7}", "\u{ab}0\u{bb}", "__0__", "~0~",
        ];
        let kaliplar = [
            "Take the {} now.",
            "Your {} is low.",
            "The {} of Dawn is heavy.",
            "{} restored.",
        ];

        println!("{:<10} {:>8}  ornek cikti", "isaret", "sagkalim");
        println!("{}", "-".repeat(70));
        for isaret in adaylar {
            let mut sagkalan = 0;
            let mut ilk_cikti = String::new();
            for kalip in kaliplar {
                let girdi = kalip.replace("{}", isaret);
                let cikti = match c.cevir(&girdi) {
                    Ok(x) => x,
                    Err(e) => format!("<hata: {e}>"),
                };
                if cikti.contains(isaret) {
                    sagkalan += 1;
                }
                if ilk_cikti.is_empty() {
                    ilk_cikti = cikti;
                }
            }
            println!(
                "{:<10} {:>5}/{}  {}",
                isaret,
                sagkalan,
                kaliplar.len(),
                ilk_cikti
            );
        }
    }

    #[test]
    fn model_kurulu_degilken_anlasilir_hata() {
        // Bu makinede model kurulu OLABİLİR; o zaman test bir şey iddia
        // etmiyor. Kurulu değilse hata metni kullanıcıya ne yapacağını
        // söylemek zorunda.
        if super::super::model::kurulu() {
            return;
        }
        let hata = match Cevirici::yukle() {
            Ok(_) => panic!("model kurulu değilken yüklendi"),
            Err(e) => e.to_string(),
        };
        assert!(hata.contains("indirilebilir"), "hata yol göstermiyor: {hata}");
    }
}
