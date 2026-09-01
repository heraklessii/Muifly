# Ürün Vizyonu

## Problem

Oyuncular performans için genelde 3 farklı kategoriye ayrı ayrı para/emek harcıyor:

1. **Sistem optimizasyonu**: Razer Cortex, gereksiz "PC temizleyici" araçları —
   çoğu reklam dolu, şişirilmiş, bazıları placebo etkisi ötesine geçmiyor.
2. **Network optimizasyonu**: WTFast, ExitLag, Outfox — abonelik modeli, sabit VPN
   tüneli mantığı, her zaman garanti etmiyor, bazen ping'i kötüleştirebiliyor.
3. **Görüntü ölçekleme / frame generation**: Lossless Scaling — bu alanda tek başına
   güçlü ama sadece bunu yapıyor, sistem/network tarafına dokunmuyor.

Hiçbiri üçünü birleştirmiyor, hiçbiri şeffaf değil (ne yaptığını göstermiyor),
çoğu tersine çevrilemez değişiklikler yapıyor veya abonelik modeliyle şişirilmiş.

## Konumlandırma

Muifly: **hafif, tek seferlik satın alınan, şeffaf, tersine çevrilebilir**
performans aracı. Kapalı kaynak ve ticari bir üründür (bkz. `DISTRIBUTION.md`).

Mui portföyünün diğer ürünlerindeki "gereksiz şişirilmiş rakiplere karşı sade ve
dürüst alternatif" kimliği burada da sürüyor — ama bu kimliğin karşılığı
"bedava" değil, **abonelik yok, reklam yok, telemetri yok, tek seferlik ödeme**.
Muiget açık kaynak/ücretsiz bir üründü; Muifly değil. Aynı ailedeler, aynı iş
modelinde değiller.

Şeffaflık ilkesi (`DESIGN_PRINCIPLES.md` madde 2) burada **kaynak kodu açmak**
anlamına gelmiyor. Şeffaflık = programın çalışırken ne yaptığını kullanıcıya
göstermesi: hangi process durduruldu, hangi ayar değişti, ne zaman geri alındı.
Bu, kapalı kaynak bir üründe de tam olarak sağlanabilir ve rakiplerin
yapmadığı şey de tam olarak budur.

## Referans / İlham Alınan Ürün: Lossless Scaling

- Steam'de $6.99, solo geliştirici (THS), 2-5 milyon civarı sahip, %93 pozitif
  36.000+ yorum (Ağustos 2026 itibarıyla).
- **Kapalı kaynak ve ücretli** — kanıtlıyor ki bu kategoride tek seferlik ücretli
  ve kapalı kaynak bir araç, güven kaybı yaşamadan çok geniş bir kitleye ulaşabilir.
  Güveni sağlayan şey kaynak kodu değil, ürünün davranışının öngörülebilir olması.
- Yedi yıllık evrim: basit bir upscaling aracından, motor entegrasyonu olmadan
  ekran yakalama + kendi ML modeliyle (LSFG) frame generation yapan bir araca dönüştü.
- **Nasıl çalışıyor (bizim için kritik referans)**: Motor bilgisine (motion vector,
  derinlik buffer) erişimi yok. Bunun yerine iki ardışık render edilmiş frame'in
  görüntüsünü analiz edip aralarına yeni bir frame yerleştiriyor — saf görüntü
  işleme + ML, motor entegrasyonu değil.
- **Zayıf noktaları (bizim fırsat alanımız)**: Native DLSS/FSR'ye göre daha çok
  artefakt riski (motion vector yok), gecikme artışı (rekabetçi oyunlarda önerilmiyor),
  network tarafına hiç dokunmuyor, sistem optimizasyonu yapmıyor, resmi Linux/Steam
  Deck desteği yok (üçüncü parti proje ile sağlanıyor).

## Farklılaşma Noktaları

1. Üç kategoriyi (sistem + network + scaling) tek araçta birleştirme
2. Optimizasyon öncesi/sonrası ölçülebilir kanıt gösterme (rakiplerin çoğu bunu yapmıyor)
3. Her değişikliğin şeffaf loglanması ve tersine çevrilebilirliği
4. Sayısal vaat yerine dürüst, doğrulanabilir dil kullanımı
5. Anti-cheat güvenliğini mimari seviyede garanti altına alma (injection/hooking yok)
6. Abonelik değil tek seferlik ödeme (WTFast/ExitLag'in aylık modeline karşı net duruş)

> **Madde 1 bir sınırdır, sadece bir vaat değil.** "Üç kategori" cümlesi ürünün
> tek cümlelik tanımı; dördüncü ve alakasız bir kategori eklemek onu bozar.
> Ekran çevirisi (Faz 5) tam olarak bu gerilimi yaratıyor ve bu yüzden "Muifly
> modülü mü, ayrı bir Mui ürünü mü" sorusu bilinçli olarak açık bırakıldı —
> gerekçelerin ikisi de `decisions.md` #22'de.

## Hedef Kullanıcı

İlk faz: orta-düşük donanımlı sistemlerde oynayan, tek tık kolaylık isteyen
oyuncular. Rekabetçi oyuncular ikinci öncelik (onlar için gecikme hassasiyeti
daha yüksek, scaling/frame-gen'i bilinçli kapatan bir "Rekabetçi Mod" gerekiyor).

## Fiyatlandırma / Dağıtım

**Karar verildi**: kapalı kaynak, tek seferlik ücretli. Steam birincil kanal,
itch.io ikincil kanal, GitHub yalnızca tanıtım sayfası + demo dağıtımı.

Detay (fiyat aralığı, demo kapsamı, mağaza sayfası gereksinimleri, lisans
modeli): `DISTRIBUTION.md`.
