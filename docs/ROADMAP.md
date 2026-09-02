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

## Faz 4 — Frame Generation

> Bu faz **ikiye ayrıldı** (karar #35). Önceki hali tek parça "ML tabanlı
> frame generation" idi ve fazı yanlış çerçeveliyordu: kare üretimi ML
> gerektirmiyor. Referans aldığımız Lossless Scaling'in ilk kare üreteci de
> klasik bir algoritmaydı; ML sonradan, kalite yükseltmesi olarak geldi.

### Faz 4a — Klasik kare üretimi ✅

**Kapsam**: İki ardışık kare arasında hareket tahmini (piramitli blok
eşleme) yapıp aralarına bir kare koyan boru hattı. ML yok, model yok,
indirilen ağırlık yok.

**Durum**: Kod, gölgelendirici, arayüz ve testler tamam (karar #35).
Doğruluk sentetik gerçek-referansla ölçülüyor — bilinen bir kaydırma
uygulanmış iki kare veriliyor ve çıkan vektörün o kaydırma olması
bekleniyor.

**Kabul kriterleri**:
- [x] Hareket tahmini bilinen kaydırmayı buluyor
- [x] Eklenen bedel ölçülüyor ve kullanıcıya gösteriliyor
- [x] Rekabetçi modda kapalı (profil şeması + çalışma zamanı, iki kapı)
- [x] Ekran yenileme hızı okunup uygunsuzluk söyleniyor
- [ ] **En az 5 oyunda gözle denenmiş** — `tasks.md` → Sıradaki 8

### Faz 4b — ML tabanlı kare üretimi ⬜

**Kapsam**: 4a'nın zayıf olduğu yerleri (örtüşme, hızlı kamera hareketi,
saydam efektler, arayüz katmanları) kapatan eğitilmiş model. 4a'nın yerine
geçmiyor, üstüne biniyor.

**Fizibilite değerlendirmesi yapıldı**: `docs/FRAME_GENERATION.md`.
Veri seti, eğitim maliyeti, inference bütçesi ve dağıtım etkisi orada.

**Sonuç: şu an açılmıyor.** Gerekçe maliyet değil sıralama — 4a sahada
doğrulanmadan onu iyileştirecek bir modele yatırım yapmak, çözülmemiş bir
problemi optimize etmek olur.

**Açılma koşulu**: 4a en az 5 oyunda denenmiş ve görülen kusurların
**hangisinin** ML ile kapanacağı listelenmiş olmalı. O liste olmadan 4b'nin
neyi çözeceği bilinmiyor demektir.

## Faz 5 — Ekran Çevirisi — 🟡 kod tamam, saha doğrulaması bekliyor

**Kapsam**: Tuşa basınca, oyun profiline kaydedilmiş bir ekran alanındaki
yazıyı okuyup (OCR) İngilizceden Türkçeye çeviren, sonucu ayrı bir üst
pencerede gösteren modül. Sürekli/otomatik çeviri **yok** — gerekçe karar
#22'de.

**Öğrenme**: Model ince ayarı değil, çeviri belleği + oyuna özel terim
sözlüğü. Kullanıcının onayladığı/düzelttiği çeviri oyun başına bir JSON'da
birikiyor, sonraki seferde aynen kullanılıyor. Karar #22.

**Faz iki turda yazıldı.** Karar #30 yakalamaya dokunmayan dört parçayı Faz
3'ten önce yazdı; karar #37 kalanını yazdı ve **açık soruyu kapattı**:
ekran çevirisi ayrı bir ürün değil, Muifly'ın varsayılan kapalı bir modülü.

- ✅ `onisleme` — BÜYÜK HARF küçültme + OCR hata sınıflarının işaretlenmesi
- ✅ `sozluk` — terim koruma/geri koyma
- ✅ `bellek` — oyun başına JSON: çeviri belleği + terim sözlüğü
- ✅ `ocr_dil` — kaynak dilin OCR paketi kontrolü
- ✅ `ocr` — `Windows.Media.Ocr` ile yakalanan kareden yazı çıkarma
- ✅ `alan` — çevrilecek bölge, **oran** olarak; profile yazılıyor
- ✅ `cumle` — çeviri birimlerine ayırma (karar #29 zaaf 3'ü kaynağında kapatıyor)
- ✅ `cevirici` — ONNX Runtime üzerinde greedy çözümleme
- ✅ `model` + `indirme` + `sha256` — isteğe bağlı indirme ve doğrulama
- ✅ `kisayol` — `RegisterHotKey`; kaydedilemezse özellik açılmıyor
- ✅ `denetleyici` — iş parçacığı, model boşta düşürme, sonuç yayınlama
- ✅ arayüz — Çeviri sekmesi, overlay penceresi, alan seçici

**Faz sırası**: Faz 1'in saha doğrulaması hâlâ yapılmadı ve bu **üçüncü
kez** taşınan bir borç (karar #33, #35, #37). Faz 5'in yazılması onu
ertelemiş oldu; `tasks.md` → Sıradaki 1 yerinde duruyor.

**Fizibilite soruları — ikisi de cevaplandı ve ikisi de ürüne bağlandı**:
- ✅ `Windows.Media.Ocr` hedeflenen oyunların yazı tiplerini gerçekten okuyor
  mu? — **Koşullu evet** (karar #28). Sentetik külliyatta karakter benzerliği
  %98,3, bölge başına 14-19 ms. Külliyatın sentetik olması hâlâ bir sınır;
  gerçek ekran görüntüleriyle tekrar ölçülmesi gerekiyor.
- ✅ Yerel EN→TR çeviri kalitesi gerçek oyun diyaloğunda kabul edilebilir mi?
  — **Koşullu evet** (karar #29). Üç zaaf da adlı adınca ölçülmüştü; üçünün
  de kodda bir karşılığı var ve karar #37'de sonuçları yazılı.

**Fizibiliteden gelen iki bağlayıcı ürün gereği — ikisi de karşılandı**:
- ✅ Çeviriden önce TAMAMI BÜYÜK HARF metin küçültülüyor (`ceviri::onisleme`).
- ✅ Çeviri **her zaman kaynak metinle birlikte** gösteriliyor — hem Çeviri
  sekmesinde hem overlay'de, testle korunuyor
  (`CeviriPaneli.test.tsx` → "kaynak metin çevirinin yanında duruyor").

**Kabul kriterleri**:
- 🟡 Çeviri isteği oyunun akışını kesmiyor — bu makinede model yükleme 1,7 s,
  cümle başına 78-243 ms ölçüldü ve çıkarım iki çekirdekle sınırlı; **gerçek
  bir oyunda doğrulanmadı**
- ✅ Overlay'in münhasır tam ekranda çalışmadığı kullanıcıya baştan söyleniyor
  (Çeviri sekmesi, anahtarın altındaki açıklama)
- ✅ Model ikiliye gömülü değil, isteğe bağlı indiriliyor — testle korunuyor
  (`ceviri::model::testler::model_ikiliye_gomulmuyor`)
- ✅ Çeviri belleği kullanıcı tarafından okunabiliyor, düzenlenebiliyor,
  silinebiliyor — oyun başına bir JSON, `ceviri::bellek`
- ✅ Özellik sıfır geri bildirimle de tam çalışıyor — makine çevirileri de
  belleğe giriyor, yalnızca kökenleri farklı (`ceviri::bellek::Koken`)

**Açık soru KAPANDI**: Muifly modülü mü, ayrı bir Mui ürünü mü? — Muifly'ın
modülü (karar #37). Gerekçe: Faz 3 geldikten sonra çeviri modülünün ihtiyaç
duyduğu her altyapı parçası (yakalama, ekran listesi, kısayol disiplini,
profil eşleştirme, günlük, kurulum) zaten yazılmıştı; ayrı bir üründe
hepsi ikinci kez yazılırdı. PRODUCT_VISION'daki "üç kategori" cümlesi
korunuyor: çeviri o üçün yanına konan bir eş değil, varsayılan kapalı bir ek.

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
| **M6 — Çeviri güncellemesi** | Faz 5 | Ücretsiz güncelleme. Mağaza sayfasında **model indirmesinin boyutu** (~507 MiB) ve İngilizce→Türkçe tek yönlü olduğu açıkça yazılmalı; ikisi de satın alma kararını etkileyen sınırlar |

**M3'ün kapsamı büyüdü**: kare ölçümü ayrı bir yükseltilmiş yardımcı ikilide
yapılıyor (karar #27) ve o ikili kurulumla birlikte gidiyor. İmzasız bir
yardımcı, ana ikili imzalı olsa bile SmartScreen uyarısını geri getirir —
üstelik tam da UAC istenen anda. **İki ikili de imzalanmalı.**

**Kod imzalama sertifikası M3'ün önkoşuludur.** İmzasız bir kurulum dosyasında
SmartScreen uyarısı çıkıyor; performans aracı kategorisinde bu uyarı doğrudan
"virüs mü" algısı yaratıyor ve indirmelerin çoğu orada duruyor. Bu kodla
çözülmüyor, satın alınması gerekiyor.
