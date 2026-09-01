//! Şeffaflık günlüğü.
//!
//! Tasarım ilkesi 2: program ne yaptığını her zaman göstermeli. Bu dosya o
//! ilkenin veri yapısı. Sistemde bir şey değiştiren her modül buraya bir satır
//! yazıyor; arayüz de bu satırları olduğu gibi gösteriyor.
//!
//! Günlük **halka tampon**: bellekte sabit sayıda satır tutuluyor. Sınırsız
//! büyüyen bir liste, günlerce açık kalan bir arka plan aracında sessizce
//! bellek yiyen tek yapı olurdu ve bu, aracın kendi vaadiyle çelişirdi.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

/// Bellekte tutulan satır sayısı.
///
/// 500, "bir oyun oturumunun tamamı rahat sığar" ölçüsünde seçildi: tipik bir
/// oturumda 20-60 satır üretiliyor.
pub const KAPASITE: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Duzey {
    /// Sistemde bir şey DEĞİŞTİ. Kullanıcının görmesi gereken asıl satır.
    Aksiyon,
    /// Bir değişiklik geri alındı.
    GeriAlma,
    /// Yalnızca bilgi: mod değişti, ölçüm başladı.
    Bilgi,
    /// Denendi ama olmadı. Program çalışmaya devam ediyor.
    Uyari,
    /// İşlem başarısız. Kullanıcının bir şey yapması gerekebilir.
    Hata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Kategori {
    Sistem,
    Ag,
    Profil,
    Olcum,
    Uygulama,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Satir {
    pub id: u64,
    /// Unix milisaniye.
    pub zaman: i64,
    pub duzey: Duzey,
    pub kategori: Kategori,
    /// Kullanıcıya gösterilen cümle. Türkçe, sayısal vaat içermez
    /// (tasarım ilkesi 4).
    pub mesaj: String,
    /// Varsa, bu satırı geri alan defter kaydının kimliği. Arayüz bunu görünce
    /// satırın yanına "geri al" düğmesi koyuyor.
    pub geri_alma_id: Option<u64>,
}

#[derive(Debug)]
pub struct Gunluk {
    satirlar: VecDeque<Satir>,
    sonraki_id: u64,
}

impl Default for Gunluk {
    fn default() -> Self {
        Self {
            satirlar: VecDeque::with_capacity(KAPASITE),
            sonraki_id: 1,
        }
    }
}

impl Gunluk {
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Satır ekler ve eklenen satırı döner.
    ///
    /// Dönen değer kopya: çağıran taraf bunu doğrudan arayüze event olarak
    /// yayınlıyor, günlüğün tamamını yeniden okumaya gerek kalmıyor.
    pub fn yaz(
        &mut self,
        duzey: Duzey,
        kategori: Kategori,
        mesaj: impl Into<String>,
        geri_alma_id: Option<u64>,
    ) -> Satir {
        let satir = Satir {
            id: self.sonraki_id,
            zaman: chrono::Utc::now().timestamp_millis(),
            duzey,
            kategori,
            mesaj: mesaj.into(),
            geri_alma_id,
        };
        self.sonraki_id += 1;

        if self.satirlar.len() == KAPASITE {
            self.satirlar.pop_front();
        }
        self.satirlar.push_back(satir.clone());
        satir
    }

    /// Kısayol: sistemde bir şey değiştiren satır.
    pub fn aksiyon(&mut self, kategori: Kategori, mesaj: impl Into<String>, undo: u64) -> Satir {
        self.yaz(Duzey::Aksiyon, kategori, mesaj, Some(undo))
    }

    pub fn bilgi(&mut self, kategori: Kategori, mesaj: impl Into<String>) -> Satir {
        self.yaz(Duzey::Bilgi, kategori, mesaj, None)
    }

    pub fn uyari(&mut self, kategori: Kategori, mesaj: impl Into<String>) -> Satir {
        self.yaz(Duzey::Uyari, kategori, mesaj, None)
    }

    pub fn hata(&mut self, kategori: Kategori, mesaj: impl Into<String>) -> Satir {
        self.yaz(Duzey::Hata, kategori, mesaj, None)
    }

    /// En yeniden en eskiye, en fazla `adet` satır.
    pub fn son(&self, adet: usize) -> Vec<Satir> {
        self.satirlar.iter().rev().take(adet).cloned().collect()
    }

    pub fn uzunluk(&self) -> usize {
        self.satirlar.len()
    }

    /// Bir defter kaydı geri alındığında, o kayda bağlı satırların "geri al"
    /// düğmesini düşürür.
    ///
    /// Satır SİLİNMİYOR: geçmiş, geri alındıktan sonra da geçmiş. Kullanıcı
    /// "ne oldu" sorusunun cevabını, işlem geri alınmış olsa da görebilmeli.
    pub fn geri_alma_isaretle(&mut self, undo_id: u64) {
        for satir in self.satirlar.iter_mut() {
            if satir.geri_alma_id == Some(undo_id) {
                satir.geri_alma_id = None;
            }
        }
    }

    /// Kullanıcının günlüğü temizlemesi.
    pub fn temizle(&mut self) {
        self.satirlar.clear();
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn halka_tampon_kapasiteyi_asmiyor() {
        let mut g = Gunluk::yeni();
        for i in 0..(KAPASITE + 50) {
            g.bilgi(Kategori::Sistem, format!("satir {i}"));
        }
        assert_eq!(g.uzunluk(), KAPASITE);
        // En eskiler düşmüş olmalı: ilk satır artık "satir 0" değil.
        let hepsi = g.son(KAPASITE);
        assert_eq!(hepsi.last().unwrap().mesaj, format!("satir {}", 50));
    }

    #[test]
    fn son_en_yeniden_geliyor() {
        let mut g = Gunluk::yeni();
        g.bilgi(Kategori::Sistem, "eski");
        g.bilgi(Kategori::Sistem, "yeni");
        let s = g.son(10);
        assert_eq!(s[0].mesaj, "yeni");
        assert_eq!(s[1].mesaj, "eski");
    }

    #[test]
    fn geri_alinan_satir_silinmiyor_dugmesi_dusuyor() {
        let mut g = Gunluk::yeni();
        g.aksiyon(Kategori::Sistem, "discord.exe donduruldu", 7);
        assert_eq!(g.uzunluk(), 1);

        g.geri_alma_isaretle(7);
        let s = g.son(1);
        assert_eq!(s.len(), 1, "satır geçmişte kalmalı");
        assert!(s[0].geri_alma_id.is_none(), "düğme düşmeli");
    }

    #[test]
    fn kimlikler_benzersiz() {
        let mut g = Gunluk::yeni();
        let a = g.bilgi(Kategori::Ag, "a");
        let b = g.bilgi(Kategori::Ag, "b");
        assert_ne!(a.id, b.id);
    }
}
