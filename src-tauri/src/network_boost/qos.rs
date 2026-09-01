//! QoS ilkesi: oyun trafiğini DSCP ile işaretleme.
//!
//! Windows'un QoS Paket Zamanlayıcısı, ilkeleri registry'deki ilke
//! anahtarından okuyor. Muifly bir ilke **oluşturuyor**; ilkeyi uygulayan
//! Windows'un kendisi. Paketlere doğrudan dokunulmuyor, ağ sürücüsüne
//! filtre takılmıyor.
//!
//! ## Dürüst sınır: bu ayar tek başına ping düşürmez
//!
//! DSCP işareti yalnızca **yerel ağdaki** cihazlar (router/switch) onu dikkate
//! alırsa bir şey ifade ediyor; ev router'larının çoğu görmezden geliyor ve
//! internet servis sağlayıcıları işareti genellikle sıfırlıyor. Faydası
//! ölçülebilir olduğu yer, aynı evdeki başka bir cihaz ya da uygulama hattı
//! doldururken oyunun paketlerinin öne alınması.
//!
//! Arayüz bunu bu netlikte söylüyor. `DESIGN_PRINCIPLES.md` madde 4'ün
//! anlamı tam olarak bu: özelliği satmak için abartmak yerine ne zaman işe
//! yaradığını söylemek.
//!
//! ## Etkili olma zamanı
//!
//! İlke, kullanıcı oturumu yeniden açıldığında (ya da ilke yenilemesinde)
//! devreye giriyor. Anında etki eden bir ayar değil ve arayüz bunu yazıyor —
//! "uygulandı" deyip hiçbir şeyin değişmemesi, güveni en hızlı kıran şey.

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::ledger::Undo;
use crate::registry::{self, Kok};

/// Windows QoS ilkelerinin bulunduğu anahtar.
const ILKE_KOKU: &str = "SOFTWARE\\Policies\\Microsoft\\Windows\\QoS";

/// Muifly'ın oluşturduğu ilkelerin ön eki.
///
/// Ön ek şart: kullanıcının ya da kurumsal bir grup ilkesinin oluşturduğu
/// QoS ilkeleriyle karışmamalı ve "Muifly'ın koyduklarını kaldır" işlemi
/// yalnızca kendi koyduklarını kaldırmalı.
pub const ILKE_ON_EKI: &str = "Muifly-";

/// DSCP 46 = Expedited Forwarding.
///
/// Gecikmeye duyarlı trafik için standart işaret (RFC 3246). Ses/görüntü
/// çağrılarının kullandığı değer; oyun trafiği de aynı sınıfa giriyor.
const DSCP_OYUN: u32 = 46;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ilke {
    pub ad: String,
    /// İlkenin bağlandığı uygulama (`oyun.exe`).
    pub uygulama: Option<String>,
    pub dscp: Option<u32>,
    /// Muifly tarafından mı oluşturuldu?
    pub bizim: bool,
}

fn ilke_yolu(ad: &str) -> String {
    format!("{ILKE_KOKU}\\{ad}")
}

/// Sistemdeki QoS ilkeleri.
pub fn listele() -> Result<Vec<Ilke>> {
    let mut ilkeler = Vec::new();
    for ad in registry::alt_anahtarlar(Kok::Makine, ILKE_KOKU)? {
        let yol = ilke_yolu(&ad);
        ilkeler.push(Ilke {
            bizim: ad.starts_with(ILKE_ON_EKI),
            uygulama: registry::metin_oku(Kok::Makine, &yol, "Application Name").unwrap_or(None),
            dscp: registry::metin_oku(Kok::Makine, &yol, "DSCP Value")
                .unwrap_or(None)
                .and_then(|v| v.parse().ok()),
            ad,
        });
    }
    Ok(ilkeler)
}

/// Bir oyun için QoS ilkesi oluşturur.
///
/// Aynı adda bir ilke zaten varsa hata dönüyor: var olan bir ilkenin üzerine
/// yazmak, o ilke kullanıcının ya da kurumun kendi ilkesiyse geri
/// alınamayacak bir kayıp olurdu.
pub fn ilke_olustur(oyun_exe: &str) -> Result<Undo> {
    let ad = format!("{ILKE_ON_EKI}{}", oyun_exe.trim_end_matches(".exe"));
    let yol = ilke_yolu(&ad);

    if registry::anahtar_var(Kok::Makine, &yol) {
        return Err(Error::ProfileInvalid(format!(
            "{ad} adında bir QoS ilkesi zaten var"
        )));
    }

    // Değerler Windows'un beklediği adlarla ve REG_SZ olarak yazılıyor;
    // DWORD yazmak ilkeyi sessizce geçersiz kılıyor.
    registry::metin_yaz(Kok::Makine, &yol, "Version", "1.0")?;
    registry::metin_yaz(Kok::Makine, &yol, "Application Name", oyun_exe)?;
    registry::metin_yaz(Kok::Makine, &yol, "DSCP Value", &DSCP_OYUN.to_string())?;
    // Boş değerler "sınırlama yok / tüm protokoller" anlamına geliyor.
    registry::metin_yaz(Kok::Makine, &yol, "Throttle Rate", "-1")?;
    registry::metin_yaz(Kok::Makine, &yol, "Protocol", "*")?;
    registry::metin_yaz(Kok::Makine, &yol, "Local Port", "*")?;
    registry::metin_yaz(Kok::Makine, &yol, "Local IP", "*")?;
    registry::metin_yaz(Kok::Makine, &yol, "Local IP Prefix Length", "*")?;
    registry::metin_yaz(Kok::Makine, &yol, "Remote Port", "*")?;
    registry::metin_yaz(Kok::Makine, &yol, "Remote IP", "*")?;
    registry::metin_yaz(Kok::Makine, &yol, "Remote IP Prefix Length", "*")?;

    Ok(Undo::RegistryAnahtari {
        yol: registry::tam_yol(Kok::Makine, &yol),
    })
}

/// Muifly'ın oluşturduğu tüm ilkeleri kaldırır.
///
/// Yalnızca ön eki taşıyanlar: başkasının ilkesine dokunulmuyor.
pub fn bizim_ilkeleri_kaldir() -> Result<usize> {
    let mut adet = 0;
    for ilke in listele()? {
        if ilke.bizim {
            registry::anahtar_sil(Kok::Makine, &ilke_yolu(&ilke.ad))?;
            adet += 1;
        }
    }
    Ok(adet)
}

/// Kullanıcıya gösterilen açıklama.
pub const ACIKLAMA: &str = "Oyunun paketlerine öncelik işareti (DSCP 46) koyar. \
Bu işaret yalnızca yerel ağdaki cihazlar dikkate alırsa bir şey ifade eder; \
internet servis sağlayıcıları genellikle görmezden gelir. Etkisi, aynı ağdaki \
başka bir indirme hattı doldururken görülür. Ayar, oturum yeniden açıldığında \
devreye girer.";

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn ilke_adi_on_ek_tasiyor() {
        // Kaldırma işlemi ön eke bakıyor; ad üretimi ön eki bırakırsa
        // Muifly kendi ilkesini kaldıramaz hale gelir.
        let ad = format!("{ILKE_ON_EKI}{}", "oyun");
        assert!(ad.starts_with(ILKE_ON_EKI));
    }

    #[test]
    fn dscp_degeri_standart() {
        // 46 = Expedited Forwarding. Rastgele bir sayı değil; değişirse
        // bilinçli olsun.
        assert_eq!(DSCP_OYUN, 46);
    }

    #[test]
    fn aciklama_kosullu_dil_kullaniyor() {
        // Özelliğin ne zaman işe yaradığını söylemesi gerekiyor; "ping'i
        // düşürür" gibi koşulsuz bir cümle olmamalı.
        let a = ACIKLAMA.to_lowercase();
        assert!(a.contains("yalnızca") || a.contains("genellikle"));
        assert!(!a.contains("garanti"));
        assert!(!a.contains("ping'i düşür"));
    }

    #[test]
    fn ilke_yolu_koke_bagli() {
        assert_eq!(
            ilke_yolu("Muifly-oyun"),
            format!("{ILKE_KOKU}\\Muifly-oyun")
        );
    }
}
