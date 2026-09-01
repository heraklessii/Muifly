# Muifly — CLAUDE.md

> Bu dosya Claude Code için giriş noktasıdır. Token bütçesini korumak için burada
> sadece özet ve yönlendirme var; detaylar `docs/` altında. Her dosyayı gerektiğinde oku,
> hepsini baştan yükleme.

## Proje Nedir

Muifly, Windows için oyun performans optimizasyon aracı. Üç ana modülü tek çatı
altında birleştirir:

1. **System Boost** — process önceliklendirme, arka plan servis yönetimi, güç planı
2. **Network Boost** — DNS/route ölçümü, QoS, jitter/packet loss izleme
3. **Scaling** — post-process upscaling (spatial), ileride frame generation (ML, faz 4)

Rakip/ilham: Lossless Scaling (Steam), Razer Cortex, WTFast/ExitLag. Boşluk: bu üçünü
ayrı ayrı satın alıyorlar, biz tek + şeffaf + tersine çevrilebilir bir araçta topluyoruz.

Detaylı ürün vizyonu ve faz planı: `docs/PRODUCT_VISION.md`

## Ticari Model — ÖNCE BUNU OKU

**Muifly kapalı kaynaklı, tek seferlik ücretli bir üründür.** Steam birincil
kanal, itch.io ikincil, GitHub yalnızca tanıtım sayfası + demo dağıtımı.

Bu, Mui ailesindeki diğer projelerden bilinçli bir ayrım: Muiget ve Muivly
açık kaynak/ücretsizdir, Muifly değildir. "Mui projeleri açık kaynak olur"
varsayımıyla hareket etme.

"Açık kaynak" / "open source" ifadeleri Muifly için **hiçbir metinde**
kullanılmaz. Şeffaflık ilkesi burada **çalışma zamanı şeffaflığı** demek:
programın ne yaptığını göstermesi, kaynak kodun yayınlanması değil.

Detay (demo kapsamı, fiyat, lisans, mağaza kontrol listesi):
`docs/DISTRIBUTION.md`

## Teknoloji Yığını

- **Core**: Rust (Windows API çağrıları, process/network yönetimi)
- **UI**: Tauri v2 + React/Vite/TypeScript
- **Görüntü işleme (Faz 3+)**: DXGI/Desktop Duplication API tabanlı ekran yakalama

Diğer Mui projeleriyle (Muiget, Muivly) ortak konvansiyonlar: JSON tabanlı config,
documentation-first yapı, token-optimized CLAUDE.md, aynı tasarım jetonları.

## Mimari Özet

```
commands.rs        ← arayüzün tek girişi (Tauri komutları)
   │
state.rs (Motor)   ← akış: oyun algılandı → profil uygula → kapanınca geri al
   ├── profile_engine/   mod seçimi, profil şeması, disk deposu, aktarım, katalog
   ├── library/          kurulu oyunlar: Steam/Epic manifestleri, kapak, exe adayları
   ├── system_boost/     öncelik, affinite, dondurma, güç planı, açılış
   ├── network_boost/    DNS ölçümü, gecikme/jitter, TCP, QoS
   ├── monitor/          şeffaflık günlüğü + ölçüm
   ├── ledger.rs         geri alma defteri (veri)
   └── revert.rs         geri alma uygulayıcısı (davranış)
```

**Değişmez kural**: Sistemde bir şey değiştiren her yol `state::Motor`
üzerinden geçer ve **hem deftere hem günlüğe** yazar. Üçünden biri eksik
kalırsa ya geri alma kaybolur ya kullanıcı ne olduğunu göremez.

Detaylı mimari: `docs/ARCHITECTURE.md` · Modül detayı: `docs/MODULES.md`

## Kritik Tasarım İlkeleri (asla ihlal edilmez)

1. **Tersine çevrilebilirlik** — her optimizasyon geri alınabilir. Process
   suspend → kapatma değil dondurma. Defter diske yazılır, çökme sonrası
   açılışta bekleyenler geri alınır.
2. **Şeffaflık** — ne değişti, kullanıcıya log olarak gösterilir. Kara kutu yok.
   (Kaynak kodu açmak DEĞİL — bkz. Ticari Model.)
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
npm test           # arayüz testleri (vitest + jsdom) — 41 test
```

```bash
cargo test                    # src-tauri/ içinde — 230 test
cargo test --features demo    # demo ikilisinin kısıtlarıyla
cargo build --features demo   # demo ikilisi (bkz. docs/decisions.md #20)
```

```bash
node arac/ucuncu-taraf-uret.mjs               # üçüncü taraf bildirimleri
node arac/vitrin-hazirla.mjs ../Muifly-vitrin  # public depo içeriği (kaynak kod HARİÇ)
npx tauri icon src-tauri/icons/kaynak.svg     # ikon seti
```

> Birinci ve üçüncü komut ÜRETİLEN dosyalar yazıyor, elle düzenlenmez.
> Bağımlılık eklendiğinde ya da yükseltildiğinde birincisi çalıştırılmalı;
> unutulursa `ucuncu_taraf::testler` CI'da kırmızıya döner (karar #21).
> İkincisi yalnızca yayın günü çalışır ve public depoya **kaynak kod
> kopyalamaz** — izin listeli (`docs/DISTRIBUTION.md`).

> Arayüzü denerken: `cargo run` ile açılan debug binary arayüzü `dist/` yerine
> `devUrl`den (localhost:1420) yüklüyor. Vite çalışmıyorken pencere boş kalır.
> Rust tarafını böyle denemek geçerli; arayüz için `npm run tauri dev` şart.

## Dizin Yapısı

`✅` = var ve derleniyor, `⬜` = henüz yok.

```
Muifly/
├── CLAUDE.md                ✅ bu dosya
├── README.md                ✅ kullanıcıya dönük tanıtım (kapalı kaynak dilinde)
├── LICENSE.md               ✅ EULA — açık kaynak lisansı DEĞİL
├── docs/
│   ├── PRODUCT_VISION.md    ✅ konumlandırma, rakip analizi
│   ├── DISTRIBUTION.md      ✅ ticari model, demo kapsamı, mağaza listesi
│   ├── ARCHITECTURE.md      ✅
│   ├── MODULES.md           ✅ modül bazlı teknik detay
│   ├── DESIGN_PRINCIPLES.md ✅ beş ilke ve gerekçeleri
│   ├── PROFILES.md          ✅ mod sistemi, profil JSON şeması
│   ├── ROADMAP.md           ✅ fazlar + yayın kilometre taşları
│   ├── RISKS.md             ✅ bilinen riskler ve azaltmaları
│   ├── decisions.md         ✅ ADR tarzı kararlar (kod buraya numarayla atıf yapıyor)
│   ├── worklog.md           ✅ oturum günlüğü
│   └── tasks.md             ✅ yapılacaklar
├── arac/                    ✅ geliştirme betikleri
│   ├── ucuncu-taraf-uret.mjs   ✅ ÜRETEÇ — bağımlılık değişince çalıştır
│   └── vitrin-hazirla.mjs      ✅ public depo içeriği (izin listeli)
├── site/                    ✅ GitHub Pages tanıtım sayfası
├── src/                     ✅ React arayüzü
│   ├── App.tsx              ✅ kabuk: kenar çubuğu, başlık çubuğu, beş ekran
│   ├── styles.css           ✅ Mui tasarım sistemi (teal, Outfit, koyu zemin)
│   ├── components/          ✅ 5 panel + 3 diyalog + grafik + mini eğri + ikon + toast
│   ├── assets/fonts/        ✅ Outfit + LICENSE-OFL.txt (gömülü, CDN yok)
│   └── lib/                 ✅ api.ts (invoke sarmalayıcıları), types.ts, format.ts
└── src-tauri/               ✅ Rust çekirdeği
    ├── icons/kaynak.svg     ✅ ikon setinin tek kaynağı
    ├── ucuncu-taraf.json    ✅ ÜRETİLEN — üçüncü taraf bildirimleri
    ├── katalog.json         ✅ ELLE bakılan — oyun katalogu (karar #26)
    └── src/
        ├── lib.rs           ✅ Tauri kurulumu + arka plan döngüsü
        ├── commands.rs      ✅ arayüze açılan komutlar
        ├── state.rs         ✅ Motor: akışın kurulduğu yer
        ├── ledger.rs        ✅ geri alma defteri (diske yazılıyor)
        ├── revert.rs        ✅ geri alma uygulayıcısı
        ├── registry.rs      ✅ her yazma bir Undo döndürüyor
        ├── settings.rs      ✅ ayarlar + veri yolları
        ├── surum.rs         ✅ demo/tam sürüm kısıtları (derleme bayrağı)
        ├── tray.rs          ✅ sistem tepsisi simgesi ve menüsü
        ├── ucuncu_taraf.rs  ✅ gömülü lisans bildirimleri (EULA md. 8)
        ├── error.rs         ✅ tek hata tipi
        ├── winutil.rs       ✅ HANDLE RAII sarmalayıcı
        ├── monitor/         ✅ log.rs (şeffaflık günlüğü), metrics.rs (jitter/özet)
        ├── system_boost/    ✅ detect, priority, suspend, power, startup
        ├── network_boost/   ✅ dns, latency, tcp, qos
        ├── profile_engine/  ✅ schema, store, aktarım (içe/dışa), mod seçimi, katalog
        ├── library/         ✅ steam, epic, exe adayları, vdf, ikon, png — hepsi yerel
        └── scaling/         ⬜ Faz 3
```

## Faz Durumu

- **Faz 1** (sistem) — ✅ kod tamam, ⬜ saha testi bekliyor (5-10 oyun)
- **Faz 2** (network) — ✅ 4/5 (DNS otomatik uygulama bilinçli olarak yok, karar #6)
- **Faz 3** (spatial upscaling) — ⬜ Faz 1-2 sahada doğrulanmadan başlanmıyor
- **Faz 4** (ML frame generation) — ⬜ ayrı fizibilite gerekiyor
- **Faz 5** (ekran çevirisi) — ⬜ kapsamı yazıldı, kodu yok. Faz 3'ün yakalama
  katmanından önce başlamaz; iki fizibilite sorusu önkoşul (karar #22)

## Claude Code için notlar

- Kod yazmadan önce **`docs/worklog.md`'nin son girdisini** ve
  **`docs/tasks.md` → "Sıradaki"** oku.
- "Neden böyle yapılmış" sorusunun cevabı `docs/decisions.md`'de, numaralı.
  Kod yorumları oraya atıf yapıyor; numaraları değiştirme.
- Tasarım ilkelerinden sapan hiçbir implementasyon (özellikle
  injection/hooking) yapılmamalı — önce kullanıcıya sor.
- Faz sırasını atlama: Faz 1 sahada doğrulanmadan Faz 3/4'e geçilmez.
- Yeni bir "yapmıyoruz" kararı verilirse, onu koruyan bir test yaz. Ürün
  duruşlarının çoğu şu an testle korunuyor (`tunel_destegi`,
  `bellek_temizleme_destegi`, `varsayilan_liste_bos`, realtime öncelik).
- Kullanıcıya gösterilen metin yazarken `DESIGN_PRINCIPLES.md` madde 4'ü
  hatırla: sayısal vaat yok, koşullu ve doğrulanabilir dil.
