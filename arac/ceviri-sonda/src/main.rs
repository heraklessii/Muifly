//! ATILACAK SONDA — Muifly Faz 5 fizibilitesi, soru 2.
//!
//! "Yerel EN->TR ceviri kalitesi gercek oyun diyalogunda kabul edilebilir mi?"
//! (`docs/ROADMAP.md` -> Faz 5, karar #22)
//!
//! Model: onnx-community/opus-mt-tc-big-en-tr, int8 nicelenmis ONNX.
//! Cozumleme: greedy, KV onbellegi YOK — sure olculeri bu yuzden kotumser.

mod sozluk;

use ort::session::Session;
use ort::value::Tensor;
use sozluk::Sozluk;
use std::time::Instant;
use tokenizers::Tokenizer;

const BASLANGIC: i64 = 57059; // decoder_start_token_id (= pad)
const SON: i64 = 43741; // eos_token_id
const EN_FAZLA_TOKEN: usize = 96;

/// Kaynak cumle + elle yazilmis referans ceviri.
///
/// Referanslar tek kisinin cevirisi; BLEU gibi tek referansli bir skor bu
/// buyuklukte anlamli olmaz. Bu yuzden sonda skor uretmiyor, ciktilari yan
/// yana basiyor ve degerlendirme nitel yapiliyor.
const KULLIYAT: &[(&str, &str, &str)] = &[
    (
        "duz diyalog",
        "You should speak with the innkeeper before nightfall.",
        "Gece olmadan hanciyla konusmalisin.",
    ),
    (
        "duz diyalog",
        "The bridge collapsed. We need another way across the river.",
        "Kopru cokmus. Nehri gecmek icin baska bir yol lazim.",
    ),
    (
        "tus istemi",
        "Press F to pick up the ancient key.",
        "Kadim anahtari almak icin F'ye bas.",
    ),
    (
        "duygusal replik",
        "My father left this blade to me, and now I leave it to you.",
        "Babam bu kilici bana birakti, ben de simdi sana birakiyorum.",
    ),
    (
        "esya kutusu",
        "Iron Longsword - Damage 42, Weight 6.5, Value 120 gold",
        "Demir Uzun Kilic - Hasar 42, Agirlik 6,5, Deger 120 altin",
    ),
    (
        "menu",
        "LOAD GAME    SETTINGS    QUIT TO DESKTOP",
        "OYUN YUKLE   AYARLAR   MASAUSTUNE CIK",
    ),
    (
        "sistem mesaji",
        "Autosaving. Do not turn off your console.",
        "Otomatik kaydediliyor. Konsolunuzu kapatmayin.",
    ),
    (
        "basarisizlik ekrani",
        "MISSION FAILED - RETURN TO CHECKPOINT",
        "GOREV BASARISIZ - KONTROL NOKTASINA DON",
    ),
    (
        "cok cumleli",
        "We have been walking for three days without water. If the well is dry, we turn back at dawn.",
        "Uc gundur susuz yuruyoruz. Kuyu kuruysa safakta geri donuyoruz.",
    ),
    (
        "oyun jargonu",
        "Your party gains 250 experience and a rare crafting material.",
        "Grubunuz 250 deneyim ve nadir bir uretim malzemesi kazanir.",
    ),
    (
        "deyim",
        "Keep your guard up. This one bites back.",
        "Tetikte ol. Bu, karsilik verir.",
    ),
    (
        "ogretici",
        "Hold the left trigger to aim, then release to throw.",
        "Nisan almak icin sol tetigi basili tut, atmak icin birak.",
    ),
    // OCR'in gercekten urettigi bozuk girdiler (karar #28). Boru hattinin
    // birlesik davranisini gormek icin burada.
    (
        "OCR bozugu",
        "The bridge collapsed] We.peed another way across the river.",
        "(kaynak zaten bozuk — cevirinin ne yaptigina bakiliyor)",
    ),
    (
        "OCR bozugu",
        "LOAD GAME SETTINGS QUITTODESKTOP",
        "(kaynak zaten bozuk — cevirinin ne yaptigina bakiliyor)",
    ),
    // BUYUK HARF AZALTMASI: yukaridaki iki menu ornegi buyuk harfli haliyle
    // cokuyordu. Model duz metinle egitilmis; TAMAMI BUYUK girdi dagilim
    // disinda kaliyor. Ucuz bir on isleme bunu duzeltiyor mu?
    (
        "buyuk harf duzeltilmis",
        "Mission failed - return to checkpoint",
        "GOREV BASARISIZ - KONTROL NOKTASINA DON",
    ),
    (
        "buyuk harf duzeltilmis",
        "Load game. Settings. Quit to desktop.",
        "Oyunu yukle. Ayarlar. Masaustune cik.",
    ),
];

fn imza(baslik: &str, oturum: &Session) {
    println!("--- {baslik} ---");
    print!("  girdi :");
    for g in oturum.inputs() {
        print!(" {}", g.name());
    }
    println!();
    print!("  cikti :");
    for c in oturum.outputs() {
        print!(" {}", c.name());
    }
    println!();
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let kok = std::path::Path::new("model");

    let tokenlestirici = Tokenizer::from_file(kok.join("tokenizer.yamali.json"))?;
    // Parcalama tokenizer.json'un, sayiya cevirme vocab.json'un isi. Gerekcesi
    // `sozluk.rs`'in basinda.
    let sozluk = Sozluk::yukle(&kok.join("vocab.json"))?;

    let kodlayici_yolu = std::env::args().nth(1).unwrap_or("encoder.onnx".into());
    let cozucu_yolu = std::env::args().nth(2).unwrap_or("decoder.onnx".into());
    println!("model: {kodlayici_yolu} + {cozucu_yolu}");

    let yuklendi = Instant::now();
    let mut kodlayici = Session::builder()?.commit_from_file(kok.join(kodlayici_yolu))?;
    let mut cozucu = Session::builder()?.commit_from_file(kok.join(cozucu_yolu))?;
    println!("Model yuklendi: {} ms", yuklendi.elapsed().as_millis());
    println!();

    imza("kodlayici", &kodlayici);
    imza("cozucu", &cozucu);
    println!();

    let mut toplam_ms = 0u128;

    for (tur, kaynak, referans) in KULLIYAT {
        let basla = Instant::now();

        // `false`: ozel token'i tokenizer eklemesin, id'si yanlis uzayda olur.
        let kodlama = tokenlestirici.encode(*kaynak, false)?;
        let mut girdi: Vec<i64> = kodlama.get_tokens().iter().map(|p| sozluk.id(p)).collect();
        girdi.push(SON);
        let maske: Vec<i64> = vec![1; girdi.len()];
        let n = girdi.len() as i64;

        let cikti = kodlayici.run(ort::inputs![
            "input_ids" => Tensor::from_array((vec![1i64, n], girdi.clone()))?,
            "attention_mask" => Tensor::from_array((vec![1i64, n], maske.clone()))?,
        ])?;
        let (gizli_sekil, gizli) = cikti["last_hidden_state"].try_extract_tensor::<f32>()?;
        let gizli_sekil: Vec<i64> = gizli_sekil.iter().map(|d| *d as i64).collect();
        let gizli: Vec<f32> = gizli.to_vec();

        let mut uretilen: Vec<i64> = vec![BASLANGIC];
        loop {
            let t = uretilen.len() as i64;
            let c = cozucu.run(ort::inputs![
                "input_ids" => Tensor::from_array((vec![1i64, t], uretilen.clone()))?,
                "encoder_attention_mask" => Tensor::from_array((vec![1i64, n], maske.clone()))?,
                "encoder_hidden_states" => Tensor::from_array((gizli_sekil.clone(), gizli.clone()))?,
            ])?;
            let (sekil, logit) = c["logits"].try_extract_tensor::<f32>()?;
            let kelime = *sekil.last().unwrap() as usize;
            let son_adim = &logit[logit.len() - kelime..];

            let mut en_iyi = 0usize;
            let mut en_iyi_deger = f32::NEG_INFINITY;
            for (i, v) in son_adim.iter().enumerate() {
                // bad_words_ids: <pad> uretilmesi yasak.
                if i as i64 == BASLANGIC {
                    continue;
                }
                if *v > en_iyi_deger {
                    en_iyi_deger = *v;
                    en_iyi = i;
                }
            }
            uretilen.push(en_iyi as i64);
            if en_iyi as i64 == SON || uretilen.len() >= EN_FAZLA_TOKEN {
                break;
            }
        }

        let cikti_idler: Vec<i64> = uretilen
            .iter()
            .skip(1)
            .filter(|t| **t != SON && **t != BASLANGIC)
            .copied()
            .collect();
        let ceviri = sozluk.metin(&cikti_idler);
        let ms = basla.elapsed().as_millis();
        toplam_ms += ms;

        println!("[{tur}]  ({ms} ms, {} token)", cikti_idler.len());
        println!("  EN  : {kaynak}");
        println!("  TR  : {ceviri}");
        println!("  ref : {referans}");
        println!();
    }

    println!(
        "Ortalama: {} ms/cumle  (KV onbellegi yok — gercek uygulamada daha hizli)",
        toplam_ms / KULLIYAT.len() as u128
    );
    Ok(())
}
