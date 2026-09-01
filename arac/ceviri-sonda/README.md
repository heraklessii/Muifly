# ceviri-sonda — atılacak fizibilite denemesi

Bu **ürün kodu değil**. `ROADMAP.md` → Faz 5'in şart koştuğu iki sorudan
ikincisini ölçülebilir hale getiriyor:

> Yerel EN→TR çeviri kalitesi gerçek oyun diyaloğunda kabul edilebilir mi?

Cevap ve gerekçesi karar #29'da. Soru kapandıktan sonra bu dizin silinebilir.
Modelin üründe hangi motorla koşacağı **ayrı bir karar** — bu sonda yalnızca
kaliteyi ölçtü.

## Model dosyaları sürüm kontrolünde değil

`model/` `.gitignore`'da: yarım gigabaytlık ağırlıkları depoya koymak
anlamsız. İndirmek için:

```bash
mkdir -p model && cd model
B=https://huggingface.co/onnx-community/opus-mt-tc-big-en-tr/resolve/main
curl -sLO $B/vocab.json
curl -sL $B/tokenizer.json -o tokenizer.json
curl -sL $B/onnx/encoder_model_quantized.onnx -o encoder.onnx
curl -sL $B/onnx/decoder_model_quantized.onnx -o decoder.onnx
```

Sonra `tokenizer.json`'un `normalizer` alanı `null`lanmalı (aşağıda neden) ve
`tokenizer.yamali.json` olarak kaydedilmeli.

```bash
node -e "const f=require('fs');const j=JSON.parse(f.readFileSync('tokenizer.json','utf8'));j.normalizer=null;f.writeFileSync('tokenizer.yamali.json',JSON.stringify(j))"
```

Çalıştırma: `cargo run --release` (int8) ya da
`cargo run --release -- encoder_fp32.onnx decoder_fp32.onnx`.

## İki tuzak — ikisi de saatler yedi, ikisi de sessiz

**1. `tokenizer.json`'un id uzayı modelin kelime dağarcığıyla aynı değil.**
Ayrıntı ve ölçülen tablo `src/sozluk.rs`'in başında. Belirtisi sinsiydi:
model çökmedi, NaN üretmedi, gizli katman istatistikleri sağlıklı göründü —
sadece **akıcı ama anlamsız Türkçe** üretti. "Model kötü" diye yanlış
okunabilirdi. Çözüm: parçalama `tokenizer.json`'un, sayıya çevirme
`vocab.json`'un işi.

**2. `Precompiled` normalleştiricinin `precompiled_charsmap` alanı `null`**
ve `tokenizers` kasası bunu çözemiyor (panik). Anlamca "uygulanacak eşleme
yok" demek, o yüzden `normalizer` `null`lanıyor.

Bir fizibilite denemesinde ilk kötü sonucu modele yormamak gerekiyor.
Buradaki iki hatanın ikisi de araçtaydı.

## Külliyatın sınırı

12 temiz cümle + OCR'ın gerçekten ürettiği 2 bozuk girdi + 2 büyük harf
düzeltmesi. Referans çeviriler tek kişinin; bu büyüklükte BLEU gibi tek
referanslı bir skor anlamlı olmaz, o yüzden sonda **skor üretmiyor** —
çıktıları yan yana basıyor, değerlendirme nitel yapılıyor.
