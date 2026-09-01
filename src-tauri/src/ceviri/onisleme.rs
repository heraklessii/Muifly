//! OCR çıktısının çeviriye verilmeden önce hazırlanması.
//!
//! İki ayrı iş yapıyor ve ikisini karıştırmamak önemli:
//!
//! 1. **Düzeltir**: TAMAMI BÜYÜK HARF metni cümle düzenine indirir. Bu bir
//!    seçenek değil, zorunluluk (karar #29, zaaf 1).
//! 2. **İşaretler**: OCR'ın bozduğundan şüphelenilen yerleri uyarı olarak
//!    döndürür — ama **düzeltmeye kalkmaz** (karar #28).
//!
//! ## Neden şüpheli yerler düzeltilmiyor
//!
//! `QUITTODESKTOP`'u `QUIT TO DESKTOP`'a bölmek bir tahmindir ve tahmin
//! tutmadığında kullanıcı bunu göremez. Karar #29'un üçüncü zaafı tam olarak
//! bu: akıcı görünen, kendinden emin, yanlış çıktı. Buradaki duruş, o zaafı
//! bir tane daha üretmemek — program bildiğini düzeltir, tahminini söyler.
//!
//! ## Uyarılar yanlış alarm verebilir, bu bilinçli
//!
//! Sezgisel kuralların hepsinin yanlış pozitifi var ve her birinin yanında
//! yazılı. Denge kasıtlı: yanlış alarmın bedeli gereksiz bir not, kaçırmanın
//! bedeli kullanıcının güvendiği yanlış bir çeviri.

/// Hazırlanmış metin ve hazırlarken fark edilenler.
#[derive(Debug, Clone, PartialEq)]
pub struct Hazirlik {
    /// Çeviriye verilecek hali.
    pub metin: String,
    /// OCR'ın verdiği hali, hiç dokunulmamış.
    ///
    /// Saklanıyor çünkü karar #29 zaaf 3 gereği çeviri **her zaman kaynak
    /// metinle birlikte** gösterilecek. Gösterilecek kaynak, çeviriye giren
    /// (düzeltilmiş) metin değil kullanıcının ekranda gördüğüdür.
    pub ozgun: String,
    pub uyarilar: Vec<Uyari>,
}

impl Hazirlik {
    /// Çevrilecek bir şey var mı?
    pub fn bos(&self) -> bool {
        self.metin.trim().is_empty()
    }
}

/// Hazırlık sırasında fark edilenler. Hepsi kullanıcıya gösterilmek için.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Uyari {
    /// Tamamı büyük harf bir satır cümle düzenine indirildi.
    ///
    /// Kullanıcıya söyleniyor çünkü çeviriye giren metin ekranda gördüğünden
    /// farklı; bunu saklamak küçük ama gereksiz bir kara kutu olurdu.
    BuyukHarfKucultuldu { satir: String },

    /// Kelimelerin bitişik okunmuş olabileceği bir parça.
    ///
    /// Karar #28'in en zararlı hata sınıfı: `QUIT TO DESKTOP` →
    /// `QUITTODESKTOP`. Sessizce yanlış değil, tanınmaz bir girdi üretiyor;
    /// yani çeviri de tanınmaz bir şey döndürecek.
    BitisikKelimeSuphesi { parca: String },

    /// Kelimenin ortasına ya da sonuna düşmüş beklenmedik noktalama.
    ///
    /// Karar #28'in ikinci hata sınıfı: `We need` → `We.peed`,
    /// `collapsed.` → `collapsed]`. Karar #29 bu girdinin sonucunu ölçtü:
    /// model akıcı ama anlamı bambaşka bir cümle üretiyor.
    NoktalamaSuphesi { parca: String },
}

/// Bir satırdaki büyük/küçük harf oranına bakarken en az kaç harf aranıyor.
///
/// Tek harflik ("I") ya da iki harflik ("OK", "HP") parçalar oyunlarda
/// meşru şekilde büyük harf; onları küçültmek bir şey kazandırmıyor,
/// bozma ihtimali ise var.
const EN_AZ_HARF: usize = 4;

/// Bitişik kelime şüphesi için eşik.
///
/// **Yanlış pozitifi var ve kabul ediliyor**: `CONGRATULATIONS` (15 harf)
/// gerçek bir kelime ama bu eşiği aşıyor. Daha yükseğe çekmek `QUITTODESKTOP`
/// (13) gibi asıl yakalanmak isteneni kaçırırdı. Uyarı bir not, bir ret
/// değil — bu yüzden yanlış alarm tarafında hata yapmak tercih edildi.
const BITISIK_ESIK: usize = 12;

/// Ham OCR çıktısını çeviriye hazırlar.
pub fn hazirla(ham: &str) -> Hazirlik {
    let mut uyarilar = Vec::new();

    // Şüphe taraması ÖZGÜN metinde yapılıyor: küçültme büyük harf ipucunu
    // yok ediyor, `QUITTODESKTOP` küçüldükten sonra sıradan bir uzun kelime
    // gibi görünürdü.
    supheleri_tara(ham, &mut uyarilar);

    // Karar verme satır satır. Oyun metni çoğu zaman büyük harf bir başlıkla
    // düz bir gövdeyi aynı bölgede taşıyor ("YOU DIED\nPress any key..."):
    // metnin tamamına tek karar vermek o başlığı büyük harf bırakırdı.
    let mut satirlar = Vec::new();
    for satir in ham.lines() {
        if tamami_buyuk_harf(satir) {
            uyarilar.push(Uyari::BuyukHarfKucultuldu {
                satir: satir.trim().to_string(),
            });
            satirlar.push(cumle_duzenine_indir(satir));
        } else {
            satirlar.push(satir.to_string());
        }
    }

    Hazirlik {
        metin: satirlar.join("\n"),
        ozgun: ham.to_string(),
        uyarilar,
    }
}

/// Satırdaki harflerin hepsi büyük mü?
///
/// "Hiç küçük harf yok" değil, "yeterince harf var ve hepsi büyük" diye
/// soruluyor: rakam ve noktalamadan ibaret bir satırın küçültülecek bir yanı
/// yok, ama küçük harfi de olmadığı için ilk soruya "evet" derdi.
pub fn tamami_buyuk_harf(satir: &str) -> bool {
    let harfler: Vec<char> = satir.chars().filter(|c| c.is_alphabetic()).collect();
    harfler.len() >= EN_AZ_HARF && harfler.iter().all(|c| c.is_uppercase())
}

/// `MISSION FAILED - RETURN TO CHECKPOINT` → `Mission failed - return to checkpoint`
///
/// Düz küçültme değil cümle düzeni: karar #29'un ölçtüğü çıktı bu biçimde
/// alındı. Cümle başları büyük kalıyor, gerisi küçülüyor.
pub fn cumle_duzenine_indir(satir: &str) -> String {
    let mut sonuc = String::with_capacity(satir.len());
    // Satırın ilk harfi de bir cümle başıdır.
    let mut cumle_basi = true;

    for c in satir.chars() {
        if c.is_alphabetic() {
            if cumle_basi {
                sonuc.extend(c.to_uppercase());
                cumle_basi = false;
            } else {
                sonuc.extend(c.to_lowercase());
            }
        } else {
            if matches!(c, '.' | '!' | '?') {
                cumle_basi = true;
            }
            sonuc.push(c);
        }
    }
    sonuc
}

/// Bitişik kelime ve noktalama şüphelerini toplar.
fn supheleri_tara(ham: &str, uyarilar: &mut Vec<Uyari>) {
    for parca in ham.split_whitespace() {
        if bitisik_kelime_suphesi(parca) {
            uyarilar.push(Uyari::BitisikKelimeSuphesi {
                parca: parca.to_string(),
            });
        }
        if noktalama_suphesi(parca) {
            uyarilar.push(Uyari::NoktalamaSuphesi {
                parca: parca.to_string(),
            });
        }
    }
}

/// Harf aralıklı büyük harf menü metninin kelime sınırını kaybetmiş hali.
///
/// Yalnızca büyük harf parçalara bakılıyor: karar #28'de bu hata harf
/// aralıklı (letter-spaced) menü metninde görüldü ve o metinler büyük harf.
/// Düz cümlelerde uzun kelime meşru, orada aramak yanlış alarm üretirdi.
fn bitisik_kelime_suphesi(parca: &str) -> bool {
    let harfler: Vec<char> = parca.chars().filter(|c| c.is_alphabetic()).collect();
    harfler.len() >= BITISIK_ESIK
        && harfler.iter().all(|c| c.is_uppercase())
        && !parca.chars().any(|c| c.is_numeric())
}

/// Kelimenin içine ya da sonuna düşmüş, orada olmaması gereken noktalama.
///
/// İki ayrı belirti:
///
/// - Harf + `.,;:` + harf, arada boşluk yok (`We.peed`). Solda **en az iki
///   harf** aranıyor; `e.g.` ve `U.S.` böyle eleniyor. Yanlış pozitif olarak
///   `example.com` kalıyor — oyun diyaloğunda alan adı beklemiyoruz.
/// - Prozada hiç işi olmayan bir karakter (`] [ | } {`) harfe yapışmış
///   (`collapsed]`). Bunlar oyun metninde neredeyse hiç geçmiyor, o yüzden
///   yanlış pozitifi düşük.
fn noktalama_suphesi(parca: &str) -> bool {
    let ch: Vec<char> = parca.chars().collect();

    for i in 0..ch.len() {
        if matches!(ch[i], ']' | '[' | '|' | '}' | '{') && ch.iter().any(|c| c.is_alphabetic()) {
            return true;
        }
        if matches!(ch[i], '.' | ',' | ';' | ':') {
            let solda_iki_harf = i >= 2 && ch[i - 1].is_alphabetic() && ch[i - 2].is_alphabetic();
            let sagda_harf = ch.get(i + 1).is_some_and(|c| c.is_alphabetic());
            if solda_iki_harf && sagda_harf {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod testler {
    use super::*;

    // --- Karar #29 zaaf 1: bu davranış zorunluluk, seçenek değil ---

    #[test]
    fn buyuk_harf_menu_metni_cumle_duzenine_iniyor() {
        // Kararın tablosundaki tam örnek. Küçültülmeden çevrilirse model
        // "MİSYONU KAYBETTİ - TÜKETMEYE DÖNÜŞ" üretiyor.
        let h = hazirla("MISSION FAILED - RETURN TO CHECKPOINT");
        assert_eq!(h.metin, "Mission failed - return to checkpoint");
    }

    #[test]
    fn kucultme_kullanicidan_saklanmiyor() {
        let h = hazirla("GAME OVER");
        assert!(h
            .uyarilar
            .iter()
            .any(|u| matches!(u, Uyari::BuyukHarfKucultuldu { .. })));
    }

    #[test]
    fn duz_metne_dokunulmuyor() {
        let metin = "Press F to pick up the ancient key.";
        assert_eq!(hazirla(metin).metin, metin);
    }

    #[test]
    fn karisik_satirda_yalnizca_buyuk_harf_olani_iniyor() {
        // Başlık büyük, gövde düz — ikisi aynı bölgede sık görülüyor.
        let h = hazirla("YOU DIED\nPress any key to continue.");
        assert_eq!(h.metin, "You died\nPress any key to continue.");
    }

    #[test]
    fn cumle_basi_buyuk_kaliyor() {
        assert_eq!(
            cumle_duzenine_indir("HOLD ON. THIS IS A TRAP!"),
            "Hold on. This is a trap!"
        );
    }

    #[test]
    fn kisa_basliklar_bozulmuyor() {
        // "HP" ve "OK" oyunlarda meşru şekilde büyük harf.
        assert_eq!(hazirla("HP").metin, "HP");
        assert_eq!(hazirla("OK").metin, "OK");
    }

    #[test]
    fn rakam_ve_noktalamadan_ibaret_satir_kucultulmuyor() {
        assert!(!tamami_buyuk_harf("12/34 -- 56%"));
    }

    // --- Karar #28: OCR hata sınıfları işaretleniyor, düzeltilmiyor ---

    #[test]
    fn bitisik_okunmus_menu_metni_isaretleniyor() {
        let h = hazirla("QUITTODESKTOP");
        assert!(h
            .uyarilar
            .iter()
            .any(|u| matches!(u, Uyari::BitisikKelimeSuphesi { .. })));
    }

    #[test]
    fn bitisik_kelime_suphesi_duzeltilmeye_calisilmiyor() {
        // Ürün duruşu: tahmin edilen bölme yapılmıyor. Metin küçülüyor
        // (büyük harf kuralı) ama kelimelere AYRILMIYOR.
        let h = hazirla("QUITTODESKTOP");
        assert!(!h.metin.contains(' '));
    }

    #[test]
    fn kelime_ortasindaki_nokta_isaretleniyor() {
        // Karar #28'de ölçülen gerçek hata: "We need" -> "We.peed".
        let h = hazirla("We.peed another way across the river.");
        assert!(h
            .uyarilar
            .iter()
            .any(|u| matches!(u, Uyari::NoktalamaSuphesi { .. })));
    }

    #[test]
    fn kelimeye_yapismis_parantez_isaretleniyor() {
        // "collapsed." -> "collapsed]"
        let h = hazirla("The bridge collapsed]");
        assert!(h
            .uyarilar
            .iter()
            .any(|u| matches!(u, Uyari::NoktalamaSuphesi { .. })));
    }

    #[test]
    fn kisaltmalar_yanlis_alarm_uretmiyor() {
        // "e.g." solunda iki harf olmadığı için eleniyor.
        let h = hazirla("Bring supplies, e.g. rope and torches.");
        assert!(h.uyarilar.is_empty(), "beklenmedik uyarı: {:?}", h.uyarilar);
    }

    #[test]
    fn saglam_cumle_hic_uyari_uretmiyor() {
        let h = hazirla("My father left this blade to me, and now I leave it to you.");
        assert!(h.uyarilar.is_empty(), "beklenmedik uyarı: {:?}", h.uyarilar);
    }

    // --- Karar #29 zaaf 3: kaynak metin her zaman elde kalıyor ---

    #[test]
    fn ozgun_metin_korunuyor() {
        let ham = "MISSION FAILED";
        let h = hazirla(ham);
        assert_eq!(h.ozgun, ham, "kaynak metin çeviriyle birlikte gösterilecek");
    }

    #[test]
    fn bos_metin_bos_diyor() {
        assert!(hazirla("   \n  ").bos());
        assert!(!hazirla("Hello").bos());
    }
}
