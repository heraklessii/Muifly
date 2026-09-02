# Mod Sistemi ve Profiller

## Modlar

| Mod | Tetiklenme | Kapsam |
|---|---|---|
| **Sistem Açılışı** | Windows başlangıcı | Güç planı ayarı, startup servis gecikmesi (kapatma değil geciktirme), DNS/route ön-testi |
| **Boşta** | Oyun algılanmadığında (varsayılan durum) | Hafif izleme, agresif optimizasyon yok, güncelleme kontrolü |
| **Oyun Algılandı — Genel** | Whitelist'te olmayan bir .exe foreground'a geçtiğinde | Varsayılan hafif profil: process priority + temel suspend + QoS |
| **Oyun Bazlı Profil** | Bilinen oyun (kullanıcı tanımlı veya paylaşılan profil) | Oyuna özel tam profil: suspend listesi, network ayarı, scaling ayarı (varsa) |
| **Rekabetçi Mod** | Kullanıcı tarafından manuel seçilir/işaretlenir | Frame generation ve agresif scaling otomatik kapalı (gecikme riski), sadece sistem+network optimizasyonu ve varsa düşük gecikmeli spatial upscaling |

Mod geçişleri `profile_engine` tarafından yönetilir ve her geçiş `monitor` modülüne
log event'i olarak bildirilir (şeffaflık ilkesi, bkz. `DESIGN_PRINCIPLES.md`).

## Profil JSON Şeması (taslak)

```json
{
  "profile_id": "example_game_v1",
  "display_name": "Örnek Oyun",
  "executable_names": ["examplegame.exe"],
  "competitive": false,
  "system": {
    "priority_class": "high",
    "cpu_affinity": "p_cores_only",
    "suspend_process_list": ["discord.exe", "spotify.exe"],
    "suspend_whitelist_exempt": ["nvidia_overlay.exe"],
    "power_plan": "ultimate_performance"
  },
  "network": {
    "preferred_dns": "auto_test",
    "qos_priority": true,
    "tcp_nodelay": true
  },
  "scaling": {
    "enabled": false,
    "algorithm": null,
    "frame_generation": false
  },
  "ceviri": {
    "enabled": false,
    "region": { "left": 0.2, "top": 0.72, "width": 0.6, "height": 0.22 },
    "source_language": null
  },
  "created_by": "user",
  "shared": false
}
```

Notlar:
- `suspend_process_list`: kullanıcı tarafından düzenlenebilir, varsayılan boş liste
  ile başlar (güvenli varsayılan).
- `scaling.algorithm`: `"tam_sayi"` | `"bilinear"` | `"lanczos"` | `"xbr"` ya da
  `null` (varsayılan: tam sayı katı — kaynakta olmayan renk üretmeyen tek yol).
  Tanınmayan bir ad `dogrula` içinde düşürülüyor ve kullanıcıya söyleniyor;
  sessizce başka bir algoritma çalıştırmak, seçtiğini sandığından başkasını
  vermek olurdu.
- `ceviri.region`: çevrilecek ekran parçası, **oran** olarak (0..1). Piksel
  değil, çünkü profil paylaşılabilir bir dosya: 1080p'de seçilen bir alan
  1440p bir makinede yanlış yere düşerdi ve bu sessizce olurdu, çünkü hâlâ
  geçerli bir dikdörtgen olurdu. `null` = ekranın tamamı okunur — alan
  seçmek şart değil, kaliteyi artıran bir tercih (karar #37).
- `ceviri.source_language`: OCR'ın okuyacağı dilin BCP-47 etiketi. Çeviri
  **yönünü değiştirmiyor** (model tek yönlü, EN→TR); yalnızca Windows'un
  hangi metin tanıma paketiyle okuyacağını söylüyor. `en-GB` paketi kurulu
  bir makinede `en-US` istemek boşuna hata olurdu.
- `ceviri` bölümü rekabetçi modda **kapatılmıyor** — `scaling`den farkı bu.
  Ölçekleme her karede gecikme ekliyor; çeviri kullanıcı tuşa bastığında bir
  kez çalışıyor. Açıkça istenen bir işi reddetmek, kapının koruduğu şeyi
  korumazdı (karar #37).
- `competitive: true` olan profillerde `scaling.frame_generation` zorla `false`
  olmalı — UI seviyesinde de engellenmeli, sadece config'e güvenilmemeli.
- `competitive: true` olan profillerde `scaling.enabled` de zorla `false`:
  ölçekleme her karede ölçülebilir bir gecikme ekliyor, rekabetçi mod tam
  olarak o gecikmeyi en aza indirmek için var (karar #32). Kısıtlı değil,
  kapalı.
- `scaling.frame_generation` artık **çalışıyor** (Faz 4a, karar #35). Faz 4
  gelene kadar geçerli olan "her koşulda `false`" kuralı kaldırıldı;
  rekabetçi profildeki kapı ise kalıcı. Rekabetçi olmayan bir profilde ayar
  artık yok sayılmıyor — sessizce yok sayılsaydı kullanıcı, profilinde
  açtığı özelliğin neden çalışmadığını bulamazdı.
  Bedeli ölçeklemeninkinden farklı: kare üretimi ikinci gerçek kareyi elde
  tutuyor ve bu bekleme daha hızlı donanımla azalmıyor.
- `shared`: ileride community profil paylaşımı için ayrılmış alan, Faz 1 kapsamında
  kullanılmıyor.

## Oyun Algılama Mantığı

1. Foreground process adı whitelist/profil veritabanıyla eşleştirilir.
2. Eşleşme varsa ilgili profil yüklenir, mod `Oyun Bazlı Profil`'e geçer.
3. Eşleşme yoksa ama process tam ekran/borderless bir pencere ise, sezgisel olarak
   "oyun olabilir" varsayımıyla `Oyun Algılandı — Genel` moduna geçilir (hafif
   varsayılan profil).
4. Foreground'dan çıkıldığında (kullanıcı Alt+Tab yaptığında veya oyun kapandığında)
   belirli bir gecikme sonrası `Boşta` moduna dönülür ve tüm suspend edilen
   process'ler otomatik devam ettirilir.

## Profil Paylaşımı

**Durum: arayüzde var** (Profiller ekranı → "İçe aktar" ve satır başına dışa
aktarma). Uygulaması `src-tauri/src/profile_engine/aktarim.rs`, kararı
`decisions.md` #23.

Güvenlik notu — bu bölümün asıl kuralı: paylaşılan profiller
`suspend_process_list` gibi alanlar içerdiği için, içe aktarma öncesi kullanıcıya
içerik gösterilmeli, kör güven olmamalı. Bu yüzden akış iki adım:

1. `profil_onizle` — dosyayı okur, doğrular, "bu profil uygulanınca ne olacak"
   listesini ve uyarıları üretir. **Diske hiçbir şey yazmaz.**
2. `profil_ice_aktar` — kullanıcı önizlemeyi gördükten sonra kaydeder.

Kimlik çakışmasında varsayılan davranış yeni bir kimlikle eklemek
(`kimlik-2`); üzerine yazmak kullanıcının açması gereken bir anahtar.

Merkezi bir profil deposu **yok**: paylaşım dosya alışverişi olarak kalıyor
(aşağıdaki ticari model notu).

Ticari model notu (`DISTRIBUTION.md`): profil dosyalarının **formatı** açıktır
ve belgelenir — kullanıcı kendi profilini bir metin editöründe okuyabilmeli ve
paylaşabilmeli. Bu, kaynak kodun kapalı olmasıyla çelişmez: kullanıcının kendi
makinesinde ne çalıştığını görebilmesi şeffaflık ilkesinin gereğidir. Profil
paylaşımı için merkezi bir sunucu/hesap sistemi kurulmaz (bakım yükü ve
telemetri yokluğu vaadi); paylaşım dosya alışverişi olarak kalır.

Profil içe aktarma **demoda kapalıdır** — bkz. `DISTRIBUTION.md` → Demo Kapsamı.
