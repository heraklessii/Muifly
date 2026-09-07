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

## Faz 3, 4 ve 5 — KALDIRILDI

Görüntü ölçekleme (Faz 3), kare üretimi (Faz 4a/4b) ve ekran çevirisi
(Faz 5) yazıldı, denendi ve **kaldırıldı** — karar #39.

Kısaca: üçü de kod olarak bitmişti; ölçekleme ve kare üretimi gerçek
kullanımda yeterince iyi çalışmadı, üçünün toplam bakım yükü ise aracın
asıl işinden (sistem + ağ) çalıyordu. Ürün "üç kategoriyi tek araçta
birleştirmek"ten "sistem ve ağ tarafını doğru yapmak"a daraldı.

Kaldırılanların yerine bir vaat konmadı: görüntü ölçekleme isteyen
kullanıcı için Muifly bir seçenek değil ve README bunu açıkça söylüyor.

## Faz Sonrası / Sürekli

- Faz 1 ve 2'nin **saha doğrulaması** (en öncelikli iş, `tasks.md`)
- Community profil paylaşımı (güvenlik incelemesiyle)
- Linux/Steam Deck desteği (ayrı büyük mühendislik kapsamı, henüz planlanmadı)

## Yayın Kilometre Taşları

Ürün ücretsiz ve açık kaynak (bkz. `DISTRIBUTION.md`).

| Kilometre taşı | Ne zaman | Kapsam |
|---|---|---|
| **M1 — Depo ve tanıtım sayfası** | ✅ yapıldı | Public GitHub deposu, kaynak kod, Pages sitesi |
| **M2 — Saha doğrulaması** | Faz 1 kabul kriterleri | En az 5-10 oyunda elle test; `tasks.md` → Sıradaki 1 |
| **M3 — 1.0 yayını** | Faz 1 + Faz 2 stabil ve doğrulanmış | GitHub Releases'te imzalı kurulum |

**M3'ün kapsamı iki ikili.** Kare ölçümü ayrı bir yükseltilmiş yardımcı
ikilide yapılıyor (karar #27) ve o ikili kurulumla birlikte gidiyor.
İmzasız bir yardımcı, ana ikili imzalı olsa bile SmartScreen uyarısını geri
getirir — üstelik tam da UAC istenen anda. **İki ikili de imzalanmalı.**

**Kod imzalama sertifikası M3'ün önkoşuludur.** İmzasız bir kurulum
dosyasında SmartScreen uyarısı çıkıyor; performans aracı kategorisinde bu
uyarı doğrudan "virüs mü" algısı yaratıyor ve indirmelerin çoğu orada
duruyor. Bu kodla çözülmüyor, satın alınması gerekiyor.
