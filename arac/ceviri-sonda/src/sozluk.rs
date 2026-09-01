//! Kelime dagarcigi eslemesi.
//!
//! # Neden tokenizer.json'un id'leri kullanilmiyor
//!
//! `onnx-community/opus-mt-tc-big-en-tr` deposundaki `tokenizer.json`, ONNX
//! modelinin gomme matrisiyle **ayni sirada degil**. Olculdu:
//!
//! | token | vocab.json (modelin uzayi) | tokenizer.json |
//! |---|---|---|
//! | `▁The` | 50392 | 23901 |
//! | `</s>` | 43741 | 1072 |
//! | `▁Köprü` | 30358 | 18301 |
//!
//! `tokenizer.json` kendi icinde tutarli, ama id'leri modele verilince model
//! bambaska gomme satirlarini okuyor. Belirtisi sinsi: model cokmuyor,
//! NaN uretmiyor, istatistikleri saglikli goruluyor — sadece anlamsiz ama
//! dilbilgisel olarak makul Turkce uretiyor. Bir fizibilite denemesinde bu
//! "model kotu" diye yanlis okunabilecek bir hata.
//!
//! Cozum: parcalara ayirmayi (Unigram segmentasyonu) tokenizer.json yapiyor
//! — parca **dizeleri** dogru. Sayiya cevirmeyi `vocab.json` yapiyor.

use std::collections::HashMap;

pub struct Sozluk {
    ileri: HashMap<String, i64>,
    geri: Vec<String>,
}

impl Sozluk {
    pub fn yukle(yol: &std::path::Path) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let ham = std::fs::read_to_string(yol)?;
        let ileri: HashMap<String, i64> = serde_json::from_str(&ham)?;
        let en_buyuk = ileri.values().copied().max().unwrap_or(0) as usize;
        let mut geri = vec![String::new(); en_buyuk + 1];
        for (k, v) in &ileri {
            geri[*v as usize] = k.clone();
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

    pub fn parca(&self, id: i64) -> &str {
        self.geri.get(id as usize).map(|s| s.as_str()).unwrap_or("")
    }

    /// Parca dizisini okunabilir metne cevirir. SentencePiece'te `▁` kelime
    /// basi demek.
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
