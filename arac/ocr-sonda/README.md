# ocr-sonda — atılacak fizibilite denemesi

Bu **ürün kodu değil**. `ROADMAP.md` → Faz 5'in şart koştuğu iki sorudan
birincisini ölçülebilir hale getiriyor:

> `Windows.Media.Ocr` hedeflenen oyunların yazı tiplerini gerçekten okuyor mu?

Cevap ve gerekçesi karar #28'de. Soru kapandıktan sonra bu dizin silinebilir.

## Çalıştırma

```
powershell -ExecutionPolicy Bypass -File uret-ornekler.ps1
cargo run --release
```

Birincisi `ornekler/` altına PNG + aynı adlı doğru metin dosyası üretiyor
(sürüm kontrolüne girmiyor, tohumlu rastgelelikle tekrar üretilebiliyor).
İkincisi her görüntüyü OCR'dan geçirip karakter benzerliği, kelime düzeyinde
düzenleme mesafesi ve süre veriyor.

## Külliyatın sınırı — sonuçlar okunurken hatırlanmalı

Görüntüler **sentetik**. Gerçek oyun ekran görüntüsü değiller; bu makinede
hiç yoktu. Oyun metninin bilinen zorluklarını taklit ediyorlar:

| Örnek | Neyi zorluyor |
|---|---|
| 01 | taban çizgisi: yüksek kontrast düz altyazı |
| 02 | konturlu beyaz metin, kalabalık renkli zemin |
| 03 | gölgeli metin, degrade zemin |
| 04 | süslü italik serif, parşömen |
| 05 | küçük punto tooltip, yarı saydam panel |
| 06 | büyük harf + harf aralığı (menü) |
| 07 | düşük kontrast, gri üstüne gri |
| 08 | stilize kalın başlık (Impact), kalın kontur |
| 09 | çok satırlı diyalog kutusu |
| 10 | tam kare 1920×1080 — maliyet eğrisinin üst ucu |

**Taklit edemedikleri**: oyunun kendi ölçekleme/keskinleştirme boru hattı,
video sıkıştırma gürültüsü, hareket bulanıklığı ve oyuna özel bitmap
fontlar. Sonuçlar bu yüzden **iyimser taraftan** okunmalı — "en iyi durumda
şu kadar" diye, "her oyunda şu kadar" diye değil.

## Ölçümün kendi iki hatası (düzeltildi, not düşülüyor)

1. Doğru metin dosyaları BOM'lu UTF-8 yazılıyordu; görünmez karakter ilk
   kelimeye yapışıp her örnekte kelime hizasını kaydırıyordu.
2. Kelimeler konumsal karşılaştırılıyordu. OCR iki kelimeyi birleştirince
   ("We need" → "We.peed") sonraki her kelime kayıyor ve hepsi yanlış
   sayılıyordu; 10 kelimelik bir cümle tek hatayla 2/10 görünüyordu. Artık
   kelime dizileri arasında düzenleme mesafesi kullanılıyor.

İkisi de OCR'ın değil ölçüm aracının hatasıydı. Bir fizibilite denemesinde
metriğe önce güvenmemek gerekiyor.
