# Ürün Vizyonu

## Problem

Oyuncular performans için genelde iki kategoriye ayrı ayrı para/emek harcıyor:

1. **Sistem optimizasyonu**: Razer Cortex, gereksiz "PC temizleyici" araçları —
   çoğu reklam dolu, şişirilmiş, bazıları placebo etkisi ötesine geçmiyor.
2. **Network optimizasyonu**: WTFast, ExitLag, Outfox — abonelik modeli, sabit VPN
   tüneli mantığı, her zaman garanti etmiyor, bazen ping'i kötüleştirebiliyor.

Hiçbiri şeffaf değil (ne yaptığını göstermiyor), çoğu tersine çevrilemez
değişiklikler yapıyor veya abonelik modeliyle şişirilmiş.

## Konumlandırma

Muifly: **hafif, ücretsiz, açık kaynak, şeffaf, tersine çevrilebilir**
performans aracı. Lisans Apache 2.0 (bkz. `DISTRIBUTION.md`).

Mui portföyünün "gereksiz şişirilmiş rakiplere karşı sade ve dürüst
alternatif" kimliği burada da sürüyor: abonelik yok, reklam yok, telemetri
yok, ücret yok. Muiget ve Muivly ile aynı iş modelinde.

Şeffaflık ilkesi (`DESIGN_PRINCIPLES.md` madde 2) iki katmanda çalışıyor:
kaynak kodun okunabilir olması ve **programın çalışırken ne yaptığını
göstermesi** — hangi süreç durduruldu, hangi ayar değişti, ne zaman geri
alındı. İkincisi olmadan birincisi yetmez: kimse bir aracın kaynağını her
çalıştırmadan önce okumaz.

## Kapsam kararı: üç değil, iki kategori

Proje bir dönem üç kategoriyi (sistem + ağ + görüntü ölçekleme) tek araçta
birleştirmeyi hedefledi; ekran çevirisi de dördüncü bir ek olarak yazıldı.
Ölçekleme, kare üretimi ve çeviri **kaldırıldı** (karar #39): denendiler,
sonuç yeterince iyi değildi ve üçünün bakımı aracın asıl işinden çalıyordu.

Bunun bir sonucu var ve saklanmıyor: görüntü ölçekleme isteyen kullanıcı için
Muifly bir seçenek değil. Lossless Scaling bu işi yapıyor ve iyi yapıyor.

## Farklılaşma Noktaları

1. Sistem ve ağ tarafını tek araçta, tek tutarlı arayüzde toplama
2. Optimizasyon öncesi/sonrası ölçülebilir kanıt gösterme (rakiplerin çoğu bunu yapmıyor)
3. Her değişikliğin şeffaf loglanması ve tersine çevrilebilirliği
4. Sayısal vaat yerine dürüst, doğrulanabilir dil kullanımı
5. Anti-cheat güvenliğini mimari seviyede garanti altına alma (injection/hooking yok)
6. Ücretsiz ve açık kaynak (WTFast/ExitLag'in aylık aboneliğine karşı net duruş)

## Hedef Kullanıcı

İlk faz: orta-düşük donanımlı sistemlerde oynayan, tek tık kolaylık isteyen
oyuncular. Rekabetçi oyuncular ikinci öncelik.

## Dağıtım

Ücretsiz ve açık kaynak; Apache License 2.0. GitHub birincil kanal: kaynak
kod, Releases'te kurulum paketi, Issues'ta hata takibi.

Detay: `DISTRIBUTION.md`.
