//! Profillerin diskte saklanması.
//!
//! Her profil ayrı bir JSON dosyası, tek bir veritabanı değil. Sebep:
//! kullanıcının bir profili bir metin editöründe açıp okuyabilmesi ve bir
//! arkadaşına gönderebilmesi (`docs/DISTRIBUTION.md`: kaynak kapalı, profil
//! formatı açık). Tek dosyalık bir veritabanı bunu imkânsız kılardı.
//!
//! Dosya adı profil kimliğinden üretiliyor ve dosya sistemi için
//! temizleniyor; kimliğin kendisi dosyanın içinde de duruyor, yani ad
//! değişse bile kimlik kaybolmuyor.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::profile_engine::schema::Profil;

/// Dosya adı için güvenli hale getirir.
///
/// Kullanıcı profil kimliğini elle verebiliyor; `..\..\` gibi bir değerin
/// profil klasörünün dışına yazmasına izin verilmemeli.
pub fn dosya_adi(kimlik: &str) -> String {
    let temiz: String = kimlik
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let temiz = temiz.trim_matches('_').to_string();
    // Uzantı her iki dalda da eklenmek zorunda: `hepsini_yukle` yalnızca
    // `.json` uzantılı dosyaları okuyor, uzantısız yazılan bir profil bir
    // daha hiç görünmezdi.
    let govde = if temiz.is_empty() { "profil" } else { &temiz };
    format!("{govde}.json")
}

/// Profil klasöründeki tüm profiller.
///
/// Bozuk bir dosya diğerlerini engellemiyor: okunamayan dosya atlanıyor ve
/// hata listesine yazılıyor. Tek bozuk profil yüzünden kullanıcının bütün
/// profillerini kaybetmesi kabul edilemez.
pub fn hepsini_yukle(dizin: &Path) -> (Vec<Profil>, Vec<String>) {
    let mut profiller = Vec::new();
    let mut hatalar = Vec::new();

    let girdiler = match std::fs::read_dir(dizin) {
        Ok(g) => g,
        // Klasör henüz yoksa profil de yok; bu bir hata değil.
        Err(_) => return (profiller, hatalar),
    };

    for girdi in girdiler.flatten() {
        let yol = girdi.path();
        if yol.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        match yukle(&yol) {
            Ok((profil, duzeltmeler)) => {
                for d in duzeltmeler {
                    hatalar.push(format!("{}: {d}", profil.display_name));
                }
                profiller.push(profil);
            }
            Err(e) => hatalar.push(format!(
                "{}: {}",
                yol.file_name().unwrap_or_default().to_string_lossy(),
                crate::error::tek_satir(&e)
            )),
        }
    }

    profiller.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
    });
    (profiller, hatalar)
}

/// Tek profili okur ve doğrular.
pub fn yukle(yol: &Path) -> Result<(Profil, Vec<String>)> {
    let icerik = std::fs::read_to_string(yol)?;
    let profil: Profil = serde_json::from_str(&icerik)?;
    profil.dogrula()
}

/// Profili diske yazar (doğrulanmış haliyle).
///
/// Doğrulama kaydetmeden önce yapılıyor: diskte hiçbir zaman geçersiz bir
/// profil durmuyor, yani bir sonraki açılışta düzeltme mesajları tekrar
/// gösterilmiyor.
pub fn kaydet(dizin: &Path, profil: Profil) -> Result<(Profil, Vec<String>)> {
    let (profil, duzeltmeler) = profil.dogrula()?;
    std::fs::create_dir_all(dizin)?;
    let yol = dizin.join(dosya_adi(&profil.profile_id));
    std::fs::write(&yol, serde_json::to_string_pretty(&profil)?)?;
    Ok((profil, duzeltmeler))
}

pub fn sil(dizin: &Path, kimlik: &str) -> Result<()> {
    let yol = dizin.join(dosya_adi(kimlik));
    if !yol.exists() {
        return Err(Error::ProfileNotFound(kimlik.to_string()));
    }
    std::fs::remove_file(yol)?;
    Ok(())
}

pub fn yolu(dizin: &Path, kimlik: &str) -> PathBuf {
    dizin.join(dosya_adi(kimlik))
}

/// Süreç adına uyan profili bulur.
///
/// Birden fazla profil aynı `.exe` adını listeliyorsa ilki kullanılıyor ve
/// bu durum çağıran tarafa bildiriliyor (arayüz uyarı gösteriyor) — sessizce
/// birini seçip diğerini yok saymak, kullanıcının "profilim çalışmıyor"
/// demesine yol açardı.
pub fn eslesen<'a>(profiller: &'a [Profil], surec_adi: &str) -> (Option<&'a Profil>, usize) {
    let eslesenler: Vec<&Profil> = profiller
        .iter()
        .filter(|p| p.eslesiyor(surec_adi))
        .collect();
    (eslesenler.first().copied(), eslesenler.len())
}

#[cfg(test)]
mod testler {
    use super::*;

    fn ornek(kimlik: &str, exe: &str) -> Profil {
        Profil::yeni(kimlik, kimlik, exe)
    }

    #[test]
    fn dosya_adi_yol_kacisini_engelliyor() {
        // Bu test bir güvenlik sınırı: kimlik alanı kullanıcı girdisi.
        let ad = dosya_adi("..\\..\\Windows\\System32\\evil");
        assert!(!ad.contains('\\'));
        assert!(!ad.contains(".."));
        assert!(ad.ends_with(".json"));
    }

    #[test]
    fn dosya_adi_bos_kimlikte_varsayilana_dusuyor() {
        assert_eq!(dosya_adi("///"), "profil.json");
        assert_eq!(dosya_adi(""), "profil.json");
    }

    #[test]
    fn dosya_adi_normal_kimligi_koruyor() {
        assert_eq!(dosya_adi("valorant_v1"), "valorant_v1.json");
        assert_eq!(dosya_adi("cs2-rekabetci"), "cs2-rekabetci.json");
    }

    #[test]
    fn kaydet_yukle_gidis_donusu() {
        let dizin = tempfile::tempdir().unwrap();
        let mut p = ornek("test_v1", "oyun.exe");
        p.system.suspend_process_list = vec!["discord.exe".into()];

        let (kaydedilen, _) = kaydet(dizin.path(), p).unwrap();
        let (okunan, duzeltmeler) = yukle(&yolu(dizin.path(), "test_v1")).unwrap();

        assert_eq!(kaydedilen, okunan);
        assert!(
            duzeltmeler.is_empty(),
            "diske doğrulanmış hali yazıldığı için okumada düzeltme olmamalı"
        );
    }

    #[test]
    fn bozuk_profil_digerlerini_engellemiyor() {
        let dizin = tempfile::tempdir().unwrap();
        kaddet_yardimci(dizin.path(), "saglam", "oyun.exe");
        std::fs::write(dizin.path().join("bozuk.json"), "{ bu json degil").unwrap();

        let (profiller, hatalar) = hepsini_yukle(dizin.path());
        assert_eq!(profiller.len(), 1, "sağlam profil yüklenmeli");
        assert_eq!(hatalar.len(), 1, "bozuk dosya rapor edilmeli");
        assert!(hatalar[0].contains("bozuk.json"));
    }

    fn kaddet_yardimci(dizin: &Path, kimlik: &str, exe: &str) {
        kaydet(dizin, ornek(kimlik, exe)).unwrap();
    }

    #[test]
    fn olmayan_klasor_hata_vermiyor() {
        let (profiller, hatalar) = hepsini_yukle(Path::new("C:\\bo\\yle\\bir\\yol\\yok"));
        assert!(profiller.is_empty());
        assert!(hatalar.is_empty(), "klasörün olmaması hata değil");
    }

    #[test]
    fn cakisan_profiller_sayiliyor() {
        let (a, _) = ornek("a", "oyun.exe").dogrula().unwrap();
        let (b, _) = ornek("b", "oyun.exe").dogrula().unwrap();
        let (c, _) = ornek("c", "baska.exe").dogrula().unwrap();

        let liste = [a, b, c];
        let (bulunan, adet) = eslesen(&liste, "oyun.exe");
        assert_eq!(adet, 2, "çakışma görünür olmalı");
        assert_eq!(bulunan.unwrap().profile_id, "a");
    }

    #[test]
    fn eslesme_yoksa_none() {
        let (a, _) = ornek("a", "oyun.exe").dogrula().unwrap();
        let liste = [a];
        let (bulunan, adet) = eslesen(&liste, "baska.exe");
        assert!(bulunan.is_none());
        assert_eq!(adet, 0);
    }

    #[test]
    fn silme_olmayan_profilde_hata() {
        let dizin = tempfile::tempdir().unwrap();
        assert!(sil(dizin.path(), "yok").is_err());
    }

    #[test]
    fn profiller_ada_gore_sirali() {
        let dizin = tempfile::tempdir().unwrap();
        kaydet(dizin.path(), Profil::yeni("z", "Zelda", "z.exe")).unwrap();
        kaydet(dizin.path(), Profil::yeni("a", "Anno", "a.exe")).unwrap();
        let (profiller, _) = hepsini_yukle(dizin.path());
        assert_eq!(profiller[0].display_name, "Anno");
        assert_eq!(profiller[1].display_name, "Zelda");
    }
}
