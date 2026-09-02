//! Metni çeviri birimlerine ayırır.
//!
//! ## Neden ayrılıyor — ölçülmüş bir sebep
//!
//! Karar #29'un üçüncü zaafı, sondanın en tehlikeli bulgusuydu:
//!
//! ```text
//! "Keep your guard up. This one bites back." → "Bu seferki ısırıyor."
//! ```
//!
//! Birinci cümle **tamamen düştü**. Hata yok, uyarı yok, kısalma işareti yok
//! — çıktı akıcı ve doğru görünüyor. Model bağlanırken aynı külliyat tekrar
//! koşturuldu ve aynı cümle yine düştü ("Bu seferki geri ısırıyor."), yani
//! bu rastlantı değil, greedy çözümlemenin bilinen davranışı.
//!
//! Her cümleyi ayrı çevirmek bu zaafı **kaynağında** kapatıyor: tek cümlelik
//! bir girdide düşecek ikinci cümle yok. Kaybolan bir birim de artık
//! görünür, çünkü birim sayısı girişte ve çıkışta aynı olmak zorunda
//! (`akis` boş çeviriyi ayrıca işaretliyor).
//!
//! Yan faydası çeviri belleği: önbellek birim başına tutuluyor. Oyunlar aynı
//! cümleyi farklı kombinasyonlarda tekrarlıyor; paragraf düzeyinde bir
//! önbellek çok daha az isabet ederdi (karar #22).
//!
//! ## Ne YAPMIYOR
//!
//! Dilbilgisi çözümlemesi yok. Bu, noktalama ve satır sonuna bakan ucuz bir
//! bölücü; OCR çıktısının zaten bozuk olabileceği bir yerde daha akıllısına
//! güvenmek için sebep yok (karar #28 hata sınıfları).

/// Bir çeviri biriminin taşıyabileceği en fazla karakter.
///
/// Model 96 token'da kesiyor (`cevirici::EN_FAZLA_TOKEN`) ve token sayısı
/// karakterden küçük. Noktalaması hiç olmayan uzun bir OCR çıktısı — harf
/// aralıklı menü metni böyle geliyor (karar #28) — bu sınır olmasa tek
/// parça hâlinde modele gider ve sessizce kırpılırdı.
pub const EN_UZUN_BIRIM: usize = 220;

/// Cümle sonu sayılan karakterler.
const BITIRENLER: [char; 5] = ['.', '!', '?', '…', ':'];

/// Sonundaki noktası cümle sonu OLMAYAN kısaltmalar.
///
/// Liste kısa ve bilerek öyle: oyun diyaloğunda geçen kısaltmalar bunlar.
/// Uzun bir kısaltma sözlüğü tutmak, kazanılacak doğruluğun çok üstünde bir
/// bakım yükü olurdu — yanlış bölünen bir cümle çeviriyi bozmuyor, sadece
/// ikiye ayırıyor.
const KISALTMALAR: [&str; 10] = ["mr", "mrs", "ms", "dr", "st", "vs", "lt", "sgt", "capt", "prof"];

/// Metni çeviri birimlerine ayırır.
///
/// Satır sonu her zaman bir sınır: OCR'da iki ayrı arayüz öğesi (başlık ve
/// gövde, iki menü satırı) ayrı satırlar hâlinde geliyor ve bunları
/// birleştirmek modele olmayan bir cümle vermek olurdu.
pub fn bol(metin: &str) -> Vec<String> {
    let mut birimler = Vec::new();
    for satir in metin.split('\n') {
        for cumle in satiri_bol(satir) {
            for parca in uzunu_bol(&cumle) {
                let p = parca.trim();
                if !p.is_empty() {
                    birimler.push(p.to_string());
                }
            }
        }
    }
    birimler
}

/// Tek bir satırı cümlelere ayırır.
fn satiri_bol(satir: &str) -> Vec<String> {
    let ch: Vec<char> = satir.chars().collect();
    let mut cumleler = Vec::new();
    let mut bas = 0usize;

    let mut i = 0usize;
    while i < ch.len() {
        if !BITIRENLER.contains(&ch[i]) {
            i += 1;
            continue;
        }
        // Peş peşe noktalama tek sınır sayılıyor ("...", "?!").
        let mut son = i;
        while son + 1 < ch.len() && BITIRENLER.contains(&ch[son + 1]) {
            son += 1;
        }
        // Kapatan tırnak noktalamanın sağında kalıyorsa birimin içinde kalsın.
        while son + 1 < ch.len() && matches!(ch[son + 1], '"' | '\'' | '”' | '’' | ')') {
            son += 1;
        }

        if sinir_mi(&ch, bas, i, son) {
            cumleler.push(ch[bas..=son].iter().collect::<String>());
            bas = son + 1;
        }
        i = son + 1;
    }

    if bas < ch.len() {
        cumleler.push(ch[bas..].iter().collect::<String>());
    }
    cumleler
}

/// `son` konumundaki noktalama gerçekten cümle sonu mu?
fn sinir_mi(ch: &[char], bas: usize, ilk: usize, son: usize) -> bool {
    // Satır sonundaysa sınırdır.
    if son + 1 >= ch.len() {
        return true;
    }
    // Ardından boşluk gelmiyorsa sınır değil: "6.5", "example.com".
    if !ch[son + 1].is_whitespace() {
        return false;
    }
    // Kısaltma mı? ("Mr. Smith")
    if ch[ilk] == '.' {
        let mut j = ilk;
        while j > bas && ch[j - 1].is_alphabetic() {
            j -= 1;
        }
        let kelime: String = ch[j..ilk].iter().collect::<String>().to_lowercase();
        if KISALTMALAR.contains(&kelime.as_str()) {
            return false;
        }
        // Tek harf + nokta: baş harf ("J. Smith") ya da harf aralıklı metin.
        if kelime.chars().count() == 1 {
            return false;
        }
    }
    true
}

/// Sınırı aşan bir birimi daha küçük parçalara böler.
///
/// Önce virgül/noktalı virgül, sonra boşluk deneniyor: bölme noktası ne
/// kadar doğal olursa çeviri o kadar az bozuluyor. Hiçbiri yoksa (boşluksuz
/// uzun bir dizi — `QUITTODESKTOP` gibi) parça olduğu gibi bırakılıyor:
/// bir kelimeyi ortadan kesmek, modele kesinlikle anlamsız bir girdi vermek
/// olurdu.
fn uzunu_bol(birim: &str) -> Vec<String> {
    if birim.chars().count() <= EN_UZUN_BIRIM {
        return vec![birim.to_string()];
    }

    let ch: Vec<char> = birim.chars().collect();
    let mut parcalar = Vec::new();
    let mut bas = 0usize;

    while ch.len() - bas > EN_UZUN_BIRIM {
        let tavan = bas + EN_UZUN_BIRIM;
        let kes = son_uygun(&ch, bas, tavan, |c| c == ',' || c == ';')
            .or_else(|| son_uygun(&ch, bas, tavan, |c| c.is_whitespace()));
        match kes {
            Some(k) => {
                parcalar.push(ch[bas..=k].iter().collect::<String>());
                bas = k + 1;
            }
            None => break,
        }
    }
    if bas < ch.len() {
        parcalar.push(ch[bas..].iter().collect::<String>());
    }
    parcalar
}

/// `[bas, tavan)` aralığındaki son uygun kesme noktası.
fn son_uygun(ch: &[char], bas: usize, tavan: usize, uygun: impl Fn(char) -> bool) -> Option<usize> {
    let tavan = tavan.min(ch.len());
    (bas..tavan).rev().find(|i| uygun(ch[*i]))
}

#[cfg(test)]
mod testler {
    use super::*;

    // --- Karar #29 zaaf 3: düşen cümle ---

    #[test]
    fn sondanin_dusen_cumlesi_ayri_birim() {
        // Sondada tek parça verildiğinde birinci cümle tamamen düşmüştü.
        let b = bol("Keep your guard up. This one bites back.");
        assert_eq!(b.len(), 2, "iki cümle tek birim kaldı: {b:?}");
        assert_eq!(b[0], "Keep your guard up.");
        assert_eq!(b[1], "This one bites back.");
    }

    #[test]
    fn uc_cumle_uc_birim() {
        let b = bol("We have been walking for three days. The well is dry. We turn back at dawn.");
        assert_eq!(b.len(), 3);
    }

    // --- Bölünmemesi gerekenler ---

    #[test]
    fn ondalik_sayi_bolunmuyor() {
        // "Weight 6.5" — nokta cümle sonu değil.
        let b = bol("Iron Longsword - Damage 42, Weight 6.5, Value 120 gold");
        assert_eq!(b.len(), 1, "ondalık ayırıcı cümle sonu sayıldı: {b:?}");
    }

    #[test]
    fn kisaltma_bolunmuyor() {
        let b = bol("Dr. Vahlen wants to see you.");
        assert_eq!(b.len(), 1, "kısaltma cümle sonu sayıldı: {b:?}");
    }

    #[test]
    fn bas_harf_bolunmuyor() {
        let b = bol("Ask J. Walker about the key.");
        assert_eq!(b.len(), 1);
    }

    // --- Satır sonu ---

    #[test]
    fn satir_sonu_her_zaman_sinir() {
        // İki ayrı arayüz öğesi; birleştirilirse modele olmayan bir cümle gider.
        let b = bol("LOAD GAME\nSETTINGS\nQUIT");
        assert_eq!(b, vec!["LOAD GAME", "SETTINGS", "QUIT"]);
    }

    #[test]
    fn bos_satirlar_dusuyor() {
        assert!(bol("\n\n   \n").is_empty());
    }

    // --- Noktalama kümeleri ---

    #[test]
    fn ucnokta_tek_sinir() {
        let b = bol("Wait... Something moved.");
        assert_eq!(b.len(), 2);
        assert_eq!(b[0], "Wait...");
    }

    #[test]
    fn kapatan_tirnak_birimde_kaliyor() {
        let b = bol("\"Run!\" she shouted.");
        assert_eq!(b[0], "\"Run!\"");
    }

    #[test]
    fn son_noktalamasiz_metin_de_birim() {
        assert_eq!(bol("Press F to open"), vec!["Press F to open"]);
    }

    // --- Uzunluk sınırı ---

    #[test]
    fn cok_uzun_birim_bolunuyor() {
        let uzun = "word ".repeat(80); // 400 karakter, noktalama yok
        let b = bol(&uzun);
        assert!(b.len() > 1, "sınır uygulanmadı");
        assert!(b.iter().all(|p| p.chars().count() <= EN_UZUN_BIRIM));
    }

    #[test]
    fn boslugu_olmayan_uzun_dizi_ortadan_kesilmiyor() {
        // Harf aralıklı menü metninin OCR'da bitişmiş hali (karar #28).
        let dizi = "A".repeat(EN_UZUN_BIRIM + 50);
        let b = bol(&dizi);
        assert_eq!(b, vec![dizi], "kelime ortadan kesildi");
    }

    #[test]
    fn virgul_bosluga_tercih_ediliyor() {
        // Sınırı gerçekten aşmak zorunda: aşmayan bir girdi hiç bölünmez ve
        // test "virgül seçilmedi" değil "bölme çalışmadı" derdi.
        let uzun = format!("{}, {}", "a".repeat(EN_UZUN_BIRIM - 10), "b".repeat(60));
        let b = bol(&uzun);
        assert!(b[0].ends_with(','), "virgülden bölünmedi: {b:?}");
    }
}
