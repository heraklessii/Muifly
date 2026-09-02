//! Oyuna özel terim sözlüğünün çeviriye uygulanması.
//!
//! ## Neden gerekli
//!
//! Karar #29'un ikinci zaafı ölçülmüş bir gerçek: genel amaçlı bir çeviri
//! modeli oyun terimlerini bilmiyor. "Longsword" → "Uzunsöz", "party" →
//! "Partiniz", "innkeeper" → "handacı". Model ince ayarı karar #22'de üç
//! gerekçeyle reddedildi; yerine konan şey bu sözlük.
//!
//! ## Nasıl çalışıyor
//!
//! Terimler çeviriden ÖNCE metinden çıkarılıp yerlerine bir işaret konuyor,
//! çeviriden SONRA işaretler karşılıklarıyla değiştiriliyor. Yani model
//! terimi hiç görmüyor, dolayısıyla bozamıyor.
//!
//! Aynı mekanizma iki farklı ihtiyacı karşılıyor:
//!
//! - `Stamina` → `Dayanıklılık` — çevrilmesi gereken ama modelin yanlış
//!   çevirdiği terim
//! - `Whiterun` → `Whiterun` — hiç çevrilmemesi gereken özel ad
//!
//! İkisi de aynı eşleme: kaynak terim → hedef terim.
//!
//! ## İşaretin biçimi ÖLÇÜLDÜ — ve ilk seçim yanlıştı
//!
//! Bu modül yazıldığında model henüz bağlı değildi ve işaretin
//! SentencePiece parçalamasından + greedy çözümlemeden bozulmadan geçeceği
//! **varsayım** olarak yazılmıştı (karar #30 bunu açıkça "ölçülmemiş" diye
//! işaretlemişti). Model bağlanınca varsayım **çürüdü**: ilk biçim `[[0]]`
//! çıktıda `[0]` oluyordu, yani her terim kayıp sayılıyordu.
//!
//! On beş aday ölçüldü (`cevirici::testler::isaret_adaylari`, karar #37).
//! Dört kalıpta dört kere sağ çıkanlar: `#0#`, `@0@`, `XX0XX`, `Zqx0`.
//! Hiç çıkmayanlar arasında köşeli/süslü/açılı parantezler ve tek yönlü
//! tırnaklar var; çoğu `<unk>`e düşüyor. Seçilen biçim en kısası:
//! [`isaret`].
//!
//! Ölçüm testi silinmedi, `--ignored` olarak duruyor: model ya da tokenizer
//! değişirse aynı soru yeniden sorulmalı ve cevabı yine tahminle değil
//! ölçümle verilmeli.
//!
//! ## Kayıp yine de raporlanıyor
//!
//! İşaret artık sağ çıkıyor ama [`geri_koy`] kaybolanları **döndürmeye
//! devam ediyor** ve arayüz onları göstermek zorunda. Sebep değişmedi:
//! karar #29'un üçüncü zaafı sessiz cümle atlama ve model bir cümleyi
//! düşürdüyse o cümledeki işaret de düşer. Kaybı sessizce yutan bir
//! tasarım, ölçülmüş bir zaafı görünmez kılardı.

use std::collections::BTreeMap;

/// Çeviri sırasında bir terimin yerine konan işaret ve ne olduğu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Yerlesim {
    /// Metne konan işaret, örn. `[[0]]`.
    pub isaret: String,
    /// Kaynak metinde geçtiği hali (kullanıcıya "şu terim korundu" demek için).
    pub terim: String,
    /// Sözlükteki karşılığı; çeviriden sonra buraya konacak.
    pub karsilik: String,
}

/// Terimin yerine konan işaretin biçimi.
///
/// `#0#`, `#1#`… Biçim **ölçülerek** seçildi (modül belgesi): köşeli
/// parantezli ilk biçim modelden `[0]` olarak çıkıyordu ve her terim kayıp
/// sayılıyordu. Sağ çıkan dört adaydan en kısası bu.
///
/// Oyun metninde `#` ile çevrili bir sayının geçmesi beklenmiyor ama
/// "beklenmiyor" garanti değil. [`koru`] metinde zaten böyle bir kalıp olup
/// olmadığına **bakmıyor**: baksa bile sonucu değiştirmezdi, çünkü geri
/// koyma birebir eşleşme arıyor ve tutmayan her şey rapor ediliyor.
fn isaret(sira: usize) -> String {
    format!("#{sira}#")
}

/// Sözlükteki terimleri metinden çıkarır, yerlerine işaret koyar.
///
/// Uzun terim önce deneniyor: `Longsword of Dawn` sözlükte varsa
/// `Longsword`'e düşmemeli. Eşleşme büyük/küçük harf duyarsız ama **kelime
/// sınırında**: `art` terimi `start` içinde eşleşmez.
pub fn koru(metin: &str, terimler: &BTreeMap<String, String>) -> (String, Vec<Yerlesim>) {
    if terimler.is_empty() {
        return (metin.to_string(), Vec::new());
    }

    // Uzunluğa göre azalan; eşitlikte alfabetik, sonuç makineden makineye
    // değişmesin diye.
    let mut sirali: Vec<(&String, &String)> = terimler.iter().collect();
    sirali.sort_by(|a, b| {
        b.0.chars()
            .count()
            .cmp(&a.0.chars().count())
            .then(a.0.cmp(b.0))
    });

    let ch: Vec<char> = metin.chars().collect();
    let mut sonuc = String::with_capacity(metin.len());
    let mut yerlesimler: Vec<Yerlesim> = Vec::new();
    let mut i = 0;

    while i < ch.len() {
        let mut eslesti = false;

        for (terim, karsilik) in &sirali {
            let t: Vec<char> = terim.chars().collect();
            if t.is_empty() || i + t.len() > ch.len() {
                continue;
            }
            if !kelime_sinirinda(&ch, i, t.len()) {
                continue;
            }
            let ayni = ch[i..i + t.len()]
                .iter()
                .zip(t.iter())
                .all(|(a, b)| a.to_lowercase().eq(b.to_lowercase()));
            if !ayni {
                continue;
            }

            let im = isaret(yerlesimler.len());
            sonuc.push_str(&im);
            yerlesimler.push(Yerlesim {
                isaret: im,
                terim: ch[i..i + t.len()].iter().collect(),
                karsilik: (*karsilik).clone(),
            });
            i += t.len();
            eslesti = true;
            break;
        }

        if !eslesti {
            sonuc.push(ch[i]);
            i += 1;
        }
    }

    (sonuc, yerlesimler)
}

/// `[baslangic, baslangic+uzunluk)` aralığının iki yanı da kelime sınırı mı?
fn kelime_sinirinda(ch: &[char], baslangic: usize, uzunluk: usize) -> bool {
    let sol_uygun = baslangic == 0 || !ch[baslangic - 1].is_alphanumeric();
    let bitis = baslangic + uzunluk;
    let sag_uygun = bitis >= ch.len() || !ch[bitis].is_alphanumeric();
    sol_uygun && sag_uygun
}

/// Çeviri çıktısındaki işaretleri karşılıklarıyla değiştirir.
///
/// Dönen ikinci değer **kaybolan terimler**: çıktıda işareti bulunamayanlar.
/// Boş olmayan bir liste "çeviri eksik" demektir ve kullanıcıdan
/// saklanmamalı (karar #29 zaaf 3).
///
/// Eşleşme bilerek katı: model `[[0]]` yerine `[[ 0 ]]` ya da `[0]` üretmişse
/// bu **kayıp** sayılıyor. Toleranslı bir eşleşme sorunu görünmez kılardı;
/// oysa bu modülün ölçülmemiş varsayımı (işaret modelden sağ çıkar) tam da
/// burada sınanacak.
pub fn geri_koy(cikti: &str, yerlesimler: &[Yerlesim]) -> (String, Vec<String>) {
    let mut metin = cikti.to_string();
    let mut kayip = Vec::new();

    for y in yerlesimler {
        if metin.contains(&y.isaret) {
            metin = metin.replace(&y.isaret, &y.karsilik);
        } else {
            kayip.push(y.terim.clone());
        }
    }

    (metin, kayip)
}

#[cfg(test)]
mod testler {
    use super::*;

    fn sozluk(ciftler: &[(&str, &str)]) -> BTreeMap<String, String> {
        ciftler
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    }

    // --- Karar #29 zaaf 2: modelin bozduğu terimler korunuyor ---

    #[test]
    fn model_terimi_hic_gormuyor() {
        let s = sozluk(&[("Longsword", "Uzun kılıç")]);
        let (korunmus, y) = koru("Take the Longsword.", &s);
        assert!(
            !korunmus.contains("Longsword"),
            "terim modele gidiyor: {korunmus}"
        );
        assert_eq!(y.len(), 1);
    }

    #[test]
    fn ceviriden_sonra_karsiligi_konuyor() {
        let s = sozluk(&[("Stamina", "Dayanıklılık")]);
        let (korunmus, y) = koru("Your Stamina is low.", &s);
        // Modelin yapacağı iş taklit ediliyor: işaret olduğu gibi kalıyor.
        let cikti = korunmus
            .replace("Your", "Senin")
            .replace("is low.", "azaldı.");
        let (metin, kayip) = geri_koy(&cikti, &y);
        assert_eq!(metin, "Senin Dayanıklılık azaldı.");
        assert!(kayip.is_empty());
    }

    #[test]
    fn ozel_ad_cevrilmeden_geciyor() {
        // Aynı mekanizma: karşılığı kendisi olan terim hiç çevrilmiyor.
        let s = sozluk(&[("Whiterun", "Whiterun")]);
        let (korunmus, y) = koru("The road to Whiterun is long.", &s);
        let (metin, _) = geri_koy(&korunmus, &y);
        assert!(metin.contains("Whiterun"));
    }

    // --- Eşleşme kuralları ---

    #[test]
    fn uzun_terim_kisasina_yenilmiyor() {
        let s = sozluk(&[
            ("Longsword", "Uzun kılıç"),
            ("Longsword of Dawn", "Şafak Kılıcı"),
        ]);
        let (_, y) = koru("Wield the Longsword of Dawn now.", &s);
        assert_eq!(y.len(), 1);
        assert_eq!(y[0].karsilik, "Şafak Kılıcı");
    }

    #[test]
    fn kelime_ortasinda_eslesmiyor() {
        // "art" terimi "start" içinde yakalanmamalı.
        let s = sozluk(&[("art", "sanat")]);
        let (korunmus, y) = koru("Press start to begin.", &s);
        assert!(y.is_empty(), "kelime ortasında eşleşti: {korunmus}");
    }

    #[test]
    fn buyuk_kucuk_harf_farki_eslesmeyi_bozmuyor() {
        let s = sozluk(&[("stamina", "Dayanıklılık")]);
        let (_, y) = koru("STAMINA depleted.", &s);
        assert_eq!(y.len(), 1);
        assert_eq!(y[0].terim, "STAMINA", "terimin özgün hali korunmalı");
    }

    #[test]
    fn ayni_terim_iki_kez_ayri_isaret_aliyor() {
        // Ayrı işaret şart: model bir tanesini düşürürse fark edilebilsin.
        let s = sozluk(&[("Stamina", "Dayanıklılık")]);
        let (_, y) = koru("Stamina low. Stamina empty.", &s);
        assert_eq!(y.len(), 2);
        assert_ne!(y[0].isaret, y[1].isaret);
    }


    /// İşaret biçimi ölçümle seçildi; kazara değişmesin.
    ///
    /// Bu testin koruduğu şey bir davranış değil bir **ölçüm sonucu**:
    /// köşeli/süslü/açılı parantezli biçimler modelden sağ çıkmıyor
    /// (`cevirici::testler::isaret_adaylari`). Biçim değiştirilecekse önce
    /// o test yeniden koşturulmalı.
    #[test]
    fn isaret_bicimi_olculmus_olan() {
        assert_eq!(isaret(0), "#0#");
        assert_eq!(isaret(12), "#12#");
        for kotu in ["[", "]", "{", "}", "<", ">", "«", "⟦"] {
            assert!(
                !isaret(0).contains(kotu),
                "modelden sağ çıkmayan bir karakter işarete girdi: {kotu}"
            );
        }
    }

    #[test]
    fn bos_sozluk_metne_dokunmuyor() {
        let metin = "Nothing to protect here.";
        let (korunmus, y) = koru(metin, &BTreeMap::new());
        assert_eq!(korunmus, metin);
        assert!(y.is_empty());
    }

    // --- Karar #29 zaaf 3: kayıp sessiz kalmıyor ---

    #[test]
    fn dusen_isaret_kayip_olarak_bildiriliyor() {
        let s = sozluk(&[("Stamina", "Dayanıklılık")]);
        let (_, y) = koru("Your Stamina is low.", &s);
        // Modelin cümleyi tamamen düşürdüğü durum (karar #29 zaaf 3).
        let (_, kayip) = geri_koy("Bir şey oldu.", &y);
        assert_eq!(kayip, vec!["Stamina".to_string()]);
    }

    #[test]
    fn bozulmus_isaret_sessizce_kabul_edilmiyor() {
        let s = sozluk(&[("Stamina", "Dayanıklılık")]);
        let (_, y) = koru("Stamina", &s);
        // Model işareti bozdu; toleranslı eşleşme bunu görünmez kılardı.
        // Bu tam olarak ilk ölçümde yaşanan şey: `[[0]]` → `[0]` (karar #37).
        let (_, kayip) = geri_koy("# 0 #", &y);
        assert_eq!(kayip.len(), 1);
    }
}
