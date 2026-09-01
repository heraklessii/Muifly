# Roadmap

> Fazlar sırayla ilerler. Bir faz "kabul kriterleri" karşılanmadan bir sonrakine
> geçilmez (bkz. `DESIGN_PRINCIPLES.md` → Faz Disiplini).

## Faz 1 — Sistem Optimizasyonu (temel)

**Kapsam**:
- Oyun process algılama (foreground + whitelist)
- Process priority/affinity ayarlama
- Arka plan process suspend/resume (whitelist tabanlı, kullanıcı onaylı liste)
- Güç planı otomatik geçiş ve geri yükleme
- Açılış modu (startup servis gecikmesi)
- Temel profil sistemi (JSON, manuel düzenleme)
- Şeffaflık log ekranı (ne yapıldı, ne zaman, geri alındı mı)

**Kabul kriterleri**:
- Her aksiyon geri alınabilir ve test edilmiş olmalı (oyun kapatıldığında sistem
  tam olarak önceki duruma dönüyor mu?)
- En az 5-10 farklı oyunla manuel test edilmiş olmalı
- Hiçbir process injection/hooking yok
- Log ekranı tüm aksiyonları doğru şekilde gösteriyor

**Neden önce bu**: En düşük risk, en yüksek güven inşası, ML/görüntü işleme
gerektirmiyor, mevcut Rust/Windows sistem deneyimiyle doğrudan örtüşüyor.

## Faz 2 — Network Optimizasyonu

**Kapsam**:
- DNS test ve öneri (Cloudflare/Google/ISP karşılaştırması)
- Route/path testi (oyun sunucusuna en düşük gecikmeli yol tespiti)
- QoS önceliklendirme (oyun trafiğine öncelik, arka plan indirmelerini throttle)
- TCP tuning (Nagle kapatma, ACK frequency) — geri alınabilir registry değişiklikleri
- Jitter/packet loss canlı izleme, `monitor` modülüne entegre

**Kabul kriterleri**:
- Tüm network değişiklikleri geri alınabilir ve varsayılana dönüş butonu çalışıyor
- Sayısal vaat içeren hiçbir UI metni yok (bkz. `DESIGN_PRINCIPLES.md`)
- Jitter/packet loss grafiği öncesi/sonrası karşılaştırma gösterebiliyor

## Faz 3 — Spatial Upscaling (ML değil) — 🟡 kod tamam, saha doğrulaması bekliyor

**Kapsam**:
- ✅ Desktop Duplication API ile ekran yakalama (pencereli/kenarlıksız mod)
- ✅ Klasik upscaling algoritmaları: Lanczos, xBR, integer scaling, bilinear
- ✅ Rekabetçi Mod'da bu modülün otomatik **kapalı** olması

**Kabul kriterleri**:
- 🟡 Görüntü kalitesi kabul edilebilir seviyede (görsel karşılaştırma testleri)
  — sentetik görüntülerle ölçülüyor (`scaling::algoritma::testler`); gerçek
  bir oyun karesiyle **gözle karşılaştırma yapılmadı**
- ✅ Gecikme artışı ölçülmüş ve kullanıcıya gösterilebilir durumda
  (`scaling::gecikme`, Ölçekleme sekmesi)
- ✅ Anti-cheat riski taşımıyor (sadece ekran okuma, injection yok)

**Not**: Bu faza, Faz 1'in saha doğrulaması yapılmadan başlandı — gerekçe ve
taşınan risk `decisions.md` #33'te. Kalan işler `tasks.md` → Sıradaki 7.

## Faz 4 — ML Tabanlı Frame Generation (uzun vadeli)

**Kapsam**: Lossless Scaling'in LSFG'sine benzer, iki ardışık frame'i analiz edip
aralarına yapay bir frame üreten özel eğitilmiş model.

**Not**: Bu faz, Faz 1-3'ten çok daha büyük bir mühendislik yatırımı gerektirir
(ML model eğitimi, veri toplama, GPU inference optimizasyonu). Solo geliştirici
için bu, referans aldığımız Lossless Scaling'in kendisinin de yedi yıllık bir
evrimle ulaştığı bir nokta. İlk sürümlerde bu faza girilmesi ZORUNLU DEĞİL —
ürün Faz 1-3 ile de bağımsız bir değer önerisi sunabilir.

**Kabul kriterleri**: Bu faza başlanmadan önce ayrı bir fizibilite değerlendirmesi
yapılmalı (gerekli veri seti, eğitim maliyeti, inference hızı hedefleri).

## Faz 5 — Ekran Çevirisi (değerlendiriliyor, kapsamı karara bağlı)

**Kapsam**: Tuşa basınca, oyun profiline kaydedilmiş bir ekran alanındaki
yazıyı okuyup (OCR) seçilen dil çiftinde çeviren, sonucu ayrı bir üst pencerede
gösteren modül. Sürekli/otomatik çeviri **yok** — gerekçe karar #22'de.

**Öğrenme**: Model ince ayarı değil, çeviri belleği + oyuna özel terim sözlüğü.
Kullanıcının onayladığı/düzelttiği çeviri oyun başına bir JSON'da birikiyor,
sonraki seferde aynen kullanılıyor. Karar #22.

**Yakalamaya dokunan her şey Faz 3'ten önce başlamaz.** Ekran yakalama katmanı
orada geliyor; daha erken başlanırsa aynı iş ikinci kez yazılır. Faz 1'in saha
doğrulaması da hâlâ önkoşul (faz disiplini).

**Yakalamadan bağımsız katman yazıldı** (karar #30, `src-tauri/src/ceviri/`).
Uygulanan ayrım testi: *"Faz 3'in yakalama katmanı geldiğinde bu kod yeniden
yazılır mı?"* — cevabı hayır olan dört parça yazıldı, evet olan hiçbir şey
yazılmadı.

- ✅ `onisleme` — BÜYÜK HARF küçültme (aşağıdaki bağlayıcı gereğin ilki) +
  OCR hata sınıflarının işaretlenmesi
- ✅ `sozluk` — terim koruma/geri koyma
- ✅ `bellek` — oyun başına JSON: çeviri belleği + terim sözlüğü
- ✅ `ocr_dil` — kaynak dilin OCR paketi kontrolü
- ⬜ yakalama, overlay, `RegisterHotKey`, model indirme ve çıkarım — Faz 3'ün
  arkasında
- ⬜ arayüz — özellik gerçekten çevirmeye başlayınca (çalışmayan bir özelliğin
  ekranı karşılanmamış vaattir, ilke 4)

**Kabul kriterlerinden önce cevaplanacak iki soru** (fizibilite, ayrı ve
atılacak bir denemeyle):
- ✅ `Windows.Media.Ocr` hedeflenen oyunların yazı tiplerini gerçekten okuyor
  mu? — **Cevaplandı, koşullu evet** (karar #28). Sentetik külliyatta karakter
  benzerliği %98,3, bölge başına 14-19 ms. Sınırları ve iki ciddi hata sınıfı
  kararda yazılı; gerçek ekran görüntüleriyle tekrar ölçülmesi gerekiyor.
- ✅ Yerel EN→TR çeviri kalitesi gerçek oyun diyaloğunda kabul edilebilir mi?
  — **Cevaplandı, koşullu evet** (karar #29). Düz diyalogda çıktı
  kullanılabilir, int8'de cümle başına ortalama 140 ms, indirme ~512 MB. Üç
  zaaf adlı adınca kararda: büyük harf menü metni (ucuz ön işlemeyle
  çözülüyor), oyun sözlüğü (karar #22'nin terim sözlüğünü doğruluyor) ve
  sessiz cümle atlama / bozuk girdide kendinden emin uydurma.

**İki soru da olumlu: faz açılabilir.** Açılma önkoşulları değişmedi — Faz 1'in
saha doğrulaması ve Faz 3'ün yakalama katmanı hâlâ önde.

**Fizibiliteden gelen iki bağlayıcı ürün gereği**:
- Çeviriden önce TAMAMI BÜYÜK HARF metin küçültülecek (karar #29, zaaf 1).
- Çeviri **her zaman kaynak metinle birlikte** gösterilecek; kaynağı gizleyen
  bir overlay tasarımı tercih edilemez (karar #29, zaaf 3).

**Kabul kriterleri**:
- ⬜ Çeviri isteği oyunun akışını kesmiyor (duraklamış diyalog kutusu senaryosu)
- ⬜ Overlay'in exclusive fullscreen'de çalışmadığı kullanıcıya baştan söyleniyor
- ⬜ Model ikiliye gömülü değil, isteğe bağlı indiriliyor (karar #1 ile tutarlılık)
- ✅ Çeviri belleği kullanıcı tarafından okunabiliyor, düzenlenebiliyor,
  silinebiliyor — oyun başına bir JSON, `ceviri::bellek` (karar #30)
- ✅ Özellik sıfır geri bildirimle de tam çalışıyor — makine çevirileri de
  belleğe giriyor, yalnızca kökenleri farklı (`ceviri::bellek::Koken`)

**Açık soru**: Muifly modülü mü, ayrı bir Mui ürünü mü? Karar #22'de iki tarafın
gerekçeleri duruyor; Faz 3'ün yakalama katmanı gerçekleştikten sonra bakılacak.

## Faz Sonrası / Sürekli

- İkinci GPU'ya (iGPU+dGPU) hesaplama offload desteği
- Overlay geliştirmeleri (DXGI hook, dikkatli risk değerlendirmesiyle)
- Community profil paylaşımı (güvenlik incelemesiyle)
- Linux/Steam Deck desteği (ayrı büyük mühendislik kapsamı, henüz planlanmadı)

## Yayın Kilometre Taşları (faz planından bağımsız, paralel yürür)

Ürün ticari (bkz. `DISTRIBUTION.md`). Kod fazlarıyla mağaza işleri paralel
ilerler; ikincisi ilkini beklerse yayın tarihi kayar.

| Kilometre taşı | Ne zaman | Kapsam |
|---|---|---|
| **M1 — Tanıtım sayfası** | Faz 1 kodu çalışır çalışmaz | Public GitHub deposu + Pages sitesi, "yakında" durumu, e-posta/wishlist yönlendirmesi yok (henüz Steam sayfası yok) |
| **M2 — Steam sayfası** | Faz 1 kabul kriterleri karşılandığında | Steam Direct ücreti, mağaza sayfası, ekran görüntüleri, wishlist açılır. Yayından **en az 2 ay önce** açılmalı — wishlist sayısı Steam algoritmasını doğrudan etkiliyor |
| **M3 — Demo** | Faz 1 + kod imzalama | Demo kapsamı `DISTRIBUTION.md`'de. Hem Steam demo hem GitHub Releases'te imzalı kurulum |
| **M4 — 1.0 yayını** | Faz 1 + Faz 2 stabil | Steam + itch.io eşzamanlı. Faz 3 yokken de bağımsız bir değer önerisi var |
| **M5 — Scaling güncellemesi** | Faz 3 | Ücretsiz güncelleme, fiyat artışı yapılabilir (mevcut sahipler etkilenmez) |

**M3'ün kapsamı büyüdü**: kare ölçümü ayrı bir yükseltilmiş yardımcı ikilide
yapılıyor (karar #27) ve o ikili kurulumla birlikte gidiyor. İmzasız bir
yardımcı, ana ikili imzalı olsa bile SmartScreen uyarısını geri getirir —
üstelik tam da UAC istenen anda. **İki ikili de imzalanmalı.**

**Kod imzalama sertifikası M3'ün önkoşuludur.** İmzasız bir kurulum dosyasında
SmartScreen uyarısı çıkıyor; performans aracı kategorisinde bu uyarı doğrudan
"virüs mü" algısı yaratıyor ve indirmelerin çoğu orada duruyor. Bu kodla
çözülmüyor, satın alınması gerekiyor.
