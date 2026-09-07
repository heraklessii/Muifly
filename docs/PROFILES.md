# Mod Sistemi ve Profiller

## Modlar

| Mod | Tetiklenme | Kapsam |
|---|---|---|
| **Sistem Açılışı** | Windows başlangıcı | Güç planı ayarı, startup servis gecikmesi (kapatma değil geciktirme), DNS/route ön-testi |
| **Boşta** | Oyun algılanmadığında (varsayılan durum) | Hafif izleme, agresif optimizasyon yok, güncelleme kontrolü |
| **Oyun Algılandı — Genel** | Whitelist'te olmayan bir .exe foreground'a geçtiğinde | Varsayılan hafif profil: process priority + temel suspend + QoS |
| **Oyun Bazlı Profil** | Bilinen oyun (kullanıcı tanımlı veya paylaşılan profil) | Oyuna özel tam profil: suspend listesi, network ayarı |
| **Rekabetçi Mod** | Kullanıcı işaretler, ya da katalog oyunu böyle tanır | Sistem + ağ optimizasyonu. **Şu an davranışsal bir farkı yok** — koruduğu şeyler (kare üretimi, agresif ölçekleme) karar #39'la kaldırıldı; mod bir etiket olarak duruyor |

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
  "created_by": "user",
  "shared": false
}
```

Notlar:
- `suspend_process_list`: kullanıcı tarafından düzenlenebilir, varsayılan boş liste
  ile başlar (güvenli varsayılan).
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

Profil dosyalarının formatı belgelidir — kullanıcı kendi profilini bir metin
editöründe okuyabilmeli ve paylaşabilmeli. Profil paylaşımı için merkezi bir
sunucu/hesap sistemi kurulmaz (bakım yükü ve telemetri yokluğu vaadi);
paylaşım dosya alışverişi olarak kalır.
