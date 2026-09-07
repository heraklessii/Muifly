# Muifly — CLAUDE.md

> Bu dosya Claude Code için giriş noktasıdır. Token bütçesini korumak için burada
> sadece özet ve yönlendirme var; detaylar `docs/` altında. Her dosyayı gerektiğinde oku,
> hepsini baştan yükleme.

## Proje Nedir

Muifly, Windows için oyun performans optimizasyon aracı. İki ana modülü tek
çatı altında birleştirir:

1. **System Boost** — process önceliklendirme, arka plan servis yönetimi, güç planı
2. **Network Boost** — DNS/route ölçümü, QoS, jitter/packet loss izleme

Yanlarında bir üçüncü ayak var ama o bir modül değil bir kural: **şeffaflık
günlüğü + ölçüm + geri alma**, ikisinin de üstünden geçiyor.

Proje bir dönem görüntü ölçekleme (Faz 3), kare üretimi (Faz 4a) ve ekran
çevirisi (Faz 5) da içeriyordu. Üçü de yazıldı, denendi ve **kaldırıldı**
(karar #39). Geri getirme önerisi gelirse önce o kararı oku: silinme sebebi
kod kalitesi değil, kapsam.

Rakip/ilham: Razer Cortex, WTFast/ExitLag. Boşluk: sistem ve ağ tarafını
şeffaf ve tersine çevrilebilir yapan bir araç yok.

Detaylı ürün vizyonu ve faz planı: `docs/PRODUCT_VISION.md`

## Lisans ve Dağıtım — ÖNCE BUNU OKU

**Muifly açık kaynak ve ücretsizdir — Apache License 2.0.** Tek kanal GitHub:
kaynak kod, Releases'te kurulum paketi, Issues'ta hata takibi, Pages'te
tanıtım sayfası.

Demo/tam sürüm ayrımı **yok** (karar #39'da kaldırıldı). `surum.rs`,
`Kisitlar` ya da `--features demo` diye bir şey aramaya çalışma; yoklar.

Şeffaflık ilkesi iki katmanlı: kaynağın okunabilir olması **ve** programın
çalışırken ne yaptığını göstermesi. İkincisi birincisinin yerine geçmiyor.

Detay: `docs/DISTRIBUTION.md`

## Teknoloji Yığını

- **Core**: Rust (Windows API çağrıları, process/network yönetimi)
- **UI**: Tauri v2 + React/Vite/TypeScript

Diğer Mui projeleriyle (Muiget, Muivly) ortak konvansiyonlar: JSON tabanlı config,
documentation-first yapı, token-optimized CLAUDE.md, aynı tasarım jetonları,
aynı lisans.

## Mimari Özet

```
commands.rs        ← arayüzün tek girişi (Tauri komutları)
   │
state.rs (Motor)   ← akış: oyun algılandı → profil uygula → kapanınca geri al
   ├── profile_engine/   mod seçimi, profil şeması, disk deposu, aktarım, katalog
   ├── library/          kurulu oyunlar: Steam/Epic manifestleri, kapak, exe adayları
   ├── system_boost/     öncelik, affinite, dondurma, güç planı, açılış
   ├── network_boost/    DNS ölçümü, gecikme/jitter, TCP, QoS
   ├── monitor/          şeffaflık günlüğü + ölçüm + oturum geçmişi
   ├── ledger.rs         geri alma defteri (veri)
   └── revert.rs         geri alma uygulayıcısı (davranış)
```

**Değişmez kural**: Sistemde bir şey değiştiren her yol `state::Motor`
üzerinden geçer ve **hem deftere hem günlüğe** yazar. Üçünden biri eksik
kalırsa ya geri alma kaybolur ya kullanıcı ne olduğunu göremez.

Bu kuralın eskiden iki istisnası vardı (`scaling/` ve `ceviri/`, deftere
yazmıyorlardı). İkisi de karar #39'la kalktı; artık istisna yok.

Detaylı mimari: `docs/ARCHITECTURE.md` · Modül detayı: `docs/MODULES.md`

## Kritik Tasarım İlkeleri (asla ihlal edilmez)

1. **Tersine çevrilebilirlik** — her optimizasyon geri alınabilir. Process
   suspend → kapatma değil dondurma. Defter diske yazılır, çökme sonrası
   açılışta bekleyenler geri alınır.
2. **Şeffaflık** — ne değişti, kullanıcıya log olarak gösterilir. Kara kutu yok.
3. **Anti-cheat güvenliği** — process/DLL injection, memory hooking YOK.
   Sadece resmi Windows API'leri.
4. **Sayısal vaat yok** — "ping'i X ms düşürür" gibi iddialar kullanılmaz.
   Bu bir stil tercihi değil, testle korunan bir kural
   (`network_boost::tcp::testler::aciklamalarda_sayisal_vaat_yok`).
5. **Admin yetkisi minimum ve açık** — arka plan izleme yükseltilmiş
   çalışmaz; UAC sadece gerektiğinde ve neden istendiği söylenerek.

Detaylı gerekçeler: `docs/DESIGN_PRINCIPLES.md`

## Geliştirme Komutları

```bash
npm run dev        # sadece frontend (Vite, localhost:1420)
npm run build      # tsc + vite build → dist/
npm run tauri dev  # tam uygulama (Rust + pencere)
npm test           # arayüz testleri (vitest + jsdom) — 51 test
```

```bash
cargo test         # src-tauri/ içinde — 286 test

# Ölçüm yardımcısı `required-features` arkasında (paketleme çakışması,
# tasks.md → Tamamlandı). `cargo test` onu DERLEMİYOR; derleme hatasının
# yayın gününe kalmaması için:
cargo clippy --all-targets --features olcum-yardimcisi -- -D warnings
```

```bash
node arac/ucuncu-taraf-uret.mjs               # üçüncü taraf bildirimleri
node arac/olcum-yardimcisi-hazirla.mjs        # ölçüm yardımcısı — `tauri build` ÖNCESİ
npx tauri icon src-tauri/icons/kaynak.svg     # ikon seti
```

> İkinci komut `src-tauri/binaries/` altına sidecar üretiyor ve **çalıştırılmazsa
> `tauri build` kırılır**. Kare ölçümü ayrı bir yükseltilmiş ikilide yapılıyor
> (karar #27); o ikili kurulumla birlikte gitmezse özellik yayın sürümünde
> sessizce ölür. Kod imzalama yapılırken **bu ikili de imzalanmalı**.

> Birinci komut ÜRETİLEN bir dosya yazıyor, elle düzenlenmez. Bağımlılık
> eklendiğinde ya da yükseltildiğinde çalıştırılmalı; unutulursa
> `ucuncu_taraf::testler` CI'da kırmızıya döner (karar #21).

> Arayüzü denerken: `cargo run` ile açılan debug binary arayüzü `dist/` yerine
> `devUrl`den (localhost:1420) yüklüyor. Vite çalışmıyorken pencere boş kalır.
> Rust tarafını böyle denemek geçerli; arayüz için `npm run tauri dev` şart.

## Dizin Yapısı

```
Muifly/
├── CLAUDE.md                bu dosya
├── README.md                kullanıcıya dönük tanıtım
├── LICENSE                  Apache License 2.0
├── docs/
│   ├── PRODUCT_VISION.md    konumlandırma, rakip analizi
│   ├── DISTRIBUTION.md      lisans, dağıtım, yayın kontrol listesi
│   ├── ARCHITECTURE.md
│   ├── MODULES.md           modül bazlı teknik detay
│   ├── DESIGN_PRINCIPLES.md beş ilke ve gerekçeleri
│   ├── PROFILES.md          mod sistemi, profil JSON şeması
│   ├── ROADMAP.md           fazlar + yayın kilometre taşları
│   ├── RISKS.md             bilinen riskler ve azaltmaları
│   ├── decisions.md         ADR tarzı kararlar (kod buraya numarayla atıf yapıyor)
│   ├── worklog.md           oturum günlüğü
│   └── tasks.md             yapılacaklar
├── arac/                    geliştirme betikleri
│   ├── ucuncu-taraf-uret.mjs        ÜRETEÇ — bağımlılık değişince çalıştır
│   ├── olcum-yardimcisi-hazirla.mjs sidecar — `tauri build` öncesi
│   └── etw-sonda/                   ATILACAK fizibilite denemesi (karar #27)
├── site/                    GitHub Pages tanıtım sayfası
├── src/                     React arayüzü
│   ├── App.tsx              kabuk: kenar çubuğu, başlık çubuğu, altı ekran
│   ├── styles.css           Mui tasarım sistemi (teal, Outfit, koyu zemin)
│   ├── components/          6 panel + 3 diyalog + grafik + ikon + toast
│   ├── assets/fonts/        Outfit + LICENSE-OFL.txt (gömülü, CDN yok)
│   └── lib/                 api.ts (invoke sarmalayıcıları), types.ts, format.ts
└── src-tauri/               Rust çekirdeği
    ├── icons/kaynak.svg     ikon setinin tek kaynağı
    ├── ucuncu-taraf.json    ÜRETİLEN — üçüncü taraf bildirimleri
    ├── katalog.json         ELLE bakılan — oyun katalogu (karar #26)
    └── src/
        ├── lib.rs           Tauri kurulumu + arka plan döngüsü
        ├── commands.rs      arayüze açılan komutlar
        ├── state.rs         Motor: akışın kurulduğu yer
        ├── ledger.rs        geri alma defteri (diske yazılıyor)
        ├── revert.rs        geri alma uygulayıcısı
        ├── registry.rs      her yazma bir Undo döndürüyor
        ├── settings.rs      ayarlar + veri yolları
        ├── tray.rs          sistem tepsisi simgesi ve menüsü
        ├── ucuncu_taraf.rs  gömülü lisans bildirimleri
        ├── error.rs         tek hata tipi
        ├── winutil.rs       HANDLE RAII sarmalayıcı
        ├── monitor/         log.rs (günlük), gecmis.rs (oturum geçmişi, karar #31),
        │                    metrics.rs (jitter/özet), frames.rs (kare
        │                    istatistiği), etw.rs + olcum.rs (karar #27)
        ├── system_boost/    detect, priority, suspend, power, startup
        ├── network_boost/   dns, latency, tcp, qos
        ├── profile_engine/  schema, store, aktarım (içe/dışa), mod seçimi, katalog
        └── library/         steam, epic, exe adayları, vdf, ikon, png — hepsi yerel
```

## Faz Durumu

- **Faz 1** (sistem) — kod tamam, saha testi bekliyor (5-10 oyun).
  Bu, projenin en uzun süredir bekleyen işi ve üç faz boyunca ertelendi
  (kararlar #33, #35, #37). Karar #39 o üç fazı silerek borcu kapattı;
  saha testi hâlâ yapılmadı — `tasks.md` → Sıradaki 1.
- **Faz 2** (network + kare ölçümü) — kod tamam (DNS otomatik uygulama
  bilinçli olarak yok, karar #6). Kare ölçümü uçtan uca bağlandı: ETW
  oturumu, yükseltilmiş yardımcı ikili, arayüz (kararlar #14, #27).
  Gerçek bir oyunda doğrulama bekliyor — `tasks.md` → Sıradaki 4.
- **Faz 3, 4, 5** — **kaldırıldı** (karar #39). Ölçekleme, kare üretimi ve
  ekran çevirisi yazıldı, denendi, silindi. Kapsam dışı; geri getirme
  önerisi gelirse önce kararı oku.

## Claude Code için notlar

- Kod yazmadan önce **`docs/worklog.md`'nin son girdisini** ve
  **`docs/tasks.md` → "Sıradaki"** oku.
- "Neden böyle yapılmış" sorusunun cevabı `docs/decisions.md`'de, numaralı.
  Kod yorumları oraya atıf yapıyor; numaraları değiştirme.
- Tasarım ilkelerinden sapan hiçbir implementasyon (özellikle
  injection/hooking) yapılmamalı — önce kullanıcıya sor.
- Faz sırasını atlama. Bu kural bir kez çiğnendi ve bedeli ödendi: Faz 1
  saha testi yapılmadan yazılan üç faz, sonunda silindi (karar #39).
- Yeni bir "yapmıyoruz" kararı verilirse, onu koruyan bir test yaz. Ürün
  duruşlarının çoğu şu an testle korunuyor (`tunel_destegi`,
  `bellek_temizleme_destegi`, `varsayilan_liste_bos`, realtime öncelik).
- Kullanıcıya gösterilen metin yazarken `DESIGN_PRINCIPLES.md` madde 4'ü
  hatırla: sayısal vaat yok, koşullu ve doğrulanabilir dil.
