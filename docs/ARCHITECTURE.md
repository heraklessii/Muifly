# Mimari

## Genel Görünüm

```
┌─────────────────────────────────────────────────────┐
│                  Tauri UI (React)                     │
│  Dashboard / Profil yönetimi / Canlı FPS-Ping grafiği  │
│  Mod seçici / Öncesi-sonrası karşılaştırma raporu       │
└───────────────────────┬───────────────────────────────┘
                         │ Tauri commands (invoke) + events (emit/listen)
┌───────────────────────▼───────────────────────────────┐
│                    Rust Core                            │
│                                                          │
│  ┌────────────────┐  ┌────────────────┐  ┌───────────┐ │
│  │ system_boost    │  │ network_boost   │  │ monitor   │ │
│  │                 │  │                 │  │           │ │
│  │ - process mgmt  │  │ - DNS test      │  │ - FPS okuma│ │
│  │ - priority/     │  │ - route test    │  │ - ping/    │ │
│  │   affinity      │  │ - QoS           │  │   jitter   │ │
│  │ - power plan    │  │ - TCP tuning    │  │ - overlay  │ │
│  │ - memory mgmt   │  │                 │  │   (DXGI)   │ │
│  └────────────────┘  └────────────────┘  └───────────┘ │
│                                                          │
│  ┌────────────────┐  ┌────────────────┐  ┌───────────┐ │
│  │ scaling (Faz 3+)│  │ profile_engine  │  │ library   │ │
│  │ - screen capture│  │ - JSON profil   │  │ - Steam/  │ │
│  │ - upscale algo  │  │   yükleme       │  │   Epic     │ │
│  │ - (Faz 4: ML     │  │ - oyun algılama │  │ - kapak/  │ │
│  │   frame gen)     │  │ - mod state     │  │   ikon     │ │
│  │                  │  │ - katalog       │  │ (yerel)    │ │
│  └────────────────┘  └────────────────┘  └───────────┘ │
└──────────────────────────────────────────────────────┘
```

## Süreç Modeli

- **Ana uygulama**: Tauri process, sistem tepsisinde (tray) çalışır, UI gerektiğinde açılır.
- **Arka plan servisi**: Windows açılışında başlayan, düşük öncelikli bir izleme
  süreci (oyun algılama, mod state). Tam admin gerektiren işlemler (process suspend,
  priority değiştirme, power plan) için UAC elevation sadece o an istenir — servis
  sürekli admin olarak çalışmaz.
- **Neden sürekli admin değil**: Güven ve güvenlik ilkesi (`DESIGN_PRINCIPLES.md`).
  Kullanıcı "bu programın sürekli tam yetkiyle arka planda çalıştığını" hissetmemeli.

## IPC / Event Akışı

1. UI, `set_mode(mode: GameMode)` gibi Tauri command'leri çağırır.
2. Rust core, ilgili modülleri (system_boost, network_boost) tetikler.
3. Core, `monitor` modülünden gelen canlı verileri (FPS, ping, jitter, CPU/GPU
   kullanımı) event olarak UI'a yayınlar (`emit("stats_update", payload)`).
4. UI, öncesi/sonrası karşılaştırmayı bu event akışından iki snapshot alarak üretir.

## Profil Sistemi (özet — detay `PROFILES.md`)

Her oyun bir JSON profiline bağlanır: hangi process'ler suspend edilecek, hangi
network ayarları uygulanacak, hangi scaling algoritması (varsa) kullanılacak.
Profiller kullanıcı tarafından düzenlenebilir ve ileride community'de paylaşılabilir
hale getirilebilir (henüz karar verilmedi, Faz 1 kapsamı dışında).

## Ekran Yakalama / Scaling Mimarisi (Faz 3+ — ön tasarım)

Lossless Scaling referans alınarak: motor entegrasyonu YOK, bunun yerine
Windows Desktop Duplication API (veya DXGI) ile pencereli/kenarlıksız modda
çalışan oyunun render edilmiş frame'i doğrudan yakalanır, upscale edilip tekrar
gösterilir. Bu, `DESIGN_PRINCIPLES.md`'deki "injection yok" ilkesiyle uyumludur
çünkü oyun sürecine hiçbir şey enjekte edilmez — sadece ekran çıktısı okunur.

Detaylı modül arayüzleri ve fonksiyon imzaları: `MODULES.md`

---

# Uygulanmış Akış (1 Eylül 2026)

Yukarıdaki şema hedefi, aşağısı kurulmuş olanı anlatıyor.

## Arka plan döngüsü

`lib.rs::arka_plan_dongusu` ayrı bir iş parçacığında saniyede bir uyanıyor:

1. **Mod kontrolü (her saniye)** — `Motor::mod_guncelle` öndeki pencereyi
   okuyup `profile_engine::mod_sec` ile karar veriyor. Mod değişmediyse
   hiçbir olay yayınlanmıyor; arayüze her saniye aynı olayı göndermek
   gereksiz iş. Değiştiyse dönen `ModDegisimi`, yeni modun yanında geçişte
   geri alınan kayıt sayısını da taşıyor — bildirim metni "geri alındı"
   diyecekse gerçekten sayılmış bir şeye dayanmak zorunda (karar #24).

2. **Oyundan çıkış gecikmesi** — Alt+Tab yapan kullanıcı için tampon.
   Kullanıcının seçtiği süre dolmadan oturum kapatılmıyor; her sekmede
   optimizasyonun kalkıp geri gelmesi hem gereksiz hem gürültülü.

3. **Ölçüm (kullanıcının seçtiği aralıkta)** — CPU/bellek okuması + (açıksa)
   bir ICMP ölçümü. Mod kontrolünden ayrı bir ritimde: algılamanın hızlı
   olması gerekiyor, ölçümün sık olması gerekmiyor ve her ölçüm bir paket.

## Olaylar

| Olay | Ne zaman | Yük |
|---|---|---|
| `muifly://durum` | Mod değiştiğinde, ayar yazıldığında | Tam `Durum` |
| `muifly://gunluk` | Mod değiştiğinde | Son 20 satır (arayüz tam listeyi ayrıca çekiyor) |
| `muifly://ornek` | Her ölçümde | Tek `Ornek` |

Arayüz kendiliğinden yoklama (polling) yapmıyor.

## Uygula → deftere yaz → günlüğe yaz

`Motor::uygula_ve_yaz` bu üçlüyü tek yerde tutuyor ve sırası sabit. İşlem
başarısızsa deftere **hiçbir şey yazılmıyor**: yapılmamış bir değişikliğin
geri alma kaydı, ileride bir geri alma turunu boşuna hataya düşürürdü.

Modüller doğrudan deftere yazmıyor — `system_boost::power::plani_uygula` bir
`Undo` döndürüyor, onu deftere koyan motor.

## Çökme sonrası temizlik

`Motor::baslat` → `revert::acilista_temizle`:

- **Oturum kapsamındaki** kayıtlar geri alınıyor (dondurulan süreçler devam
  ettiriliyor, güç planı geri yükleniyor)
- **Kalıcı kayıtlara dokunulmuyor** — TCP ayarı gibi bilinçli değişiklikler,
  program çöktü diye kendiliğinden kalkmamalı
- PID yeniden kullanımına karşı süreç adı doğrulanıyor; ad tutmuyorsa işlem
  yapılmıyor ve kayıt defterde kalıyor

## Normal kapanış

Açılış temizliği **çökme için son savunma**; normal çıkışta ona ihtiyaç
duyulmuyor. `lib::run` içindeki `RunEvent::Exit` üzerinde oturumluk kayıtlar
geri alınıyor: dondurulan süreçler devam ediyor, güç planı geri dönüyor
(karar #19).

Pencerenin kapatılması ise kapanış değil: `tepsiye_kucult` ayarı açıkken
(varsayılan) pencere gizleniyor, program tepside mod izlemeye devam ediyor.
Gerçekten çıkmanın yolu tepsi menüsündeki "Çıkış" (`tray.rs`).

## Kilitleme

Motor `parking_lot::Mutex` içinde (karar #18). Komutlar kilidi alıp hemen
bırakıyor; uzun süren ağ işleri (DNS karşılaştırması, yol testi) kilidi hiç
almıyor ve `spawn_blocking` üzerinde koşuyor — o sırada arayüzün durum
sorgusu bloke olmuyor.
