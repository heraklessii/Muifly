# Kare Üretimi — Uygulanan Yol ve ML Fizibilitesi

> `ROADMAP.md` Faz 4'ün kabul kriteri: "Bu faza başlanmadan önce ayrı bir
> fizibilite değerlendirmesi yapılmalı (gerekli veri seti, eğitim maliyeti,
> inference hızı hedefleri)." Bu belge o değerlendirme.
>
> Sonuç önden: **klasik yol yazıldı ve çalışıyor** (karar #35). **ML yolu
> şu an açılmıyor** ve gerekçesi aşağıda sayılarla duruyor.

## 1. Yol haritası Faz 4'ü yanlış çerçevelemiş

`ROADMAP.md` Faz 4'ü "özel eğitilmiş model" olarak tanımlıyor ve şöyle
diyor: solo geliştirici için bu, Lossless Scaling'in yedi yılda ulaştığı
nokta.

İkinci cümle doğru, birincisi eksik. **Kare üretimi ML gerektirmiyor.**
Referans aldığımız aracın ilk kare üreteci (LSFG 1.0) klasik bir
algoritmaydı; ML sürümü sonradan, kalite yükseltmesi olarak geldi. Yani
Faz 4 tek parça bir dağ değil, iki basamak:

| Basamak | Ne | Durum |
|---|---|---|
| 4a | Klasik hareket tahmini + warp | ✅ yazıldı (karar #35) |
| 4b | ML tabanlı ara kare | ⬜ bu belgenin konusu |

Bu ayrım yapılmasaydı, ulaşılabilir olan bir özellik ulaşılmaz olanın
arkasında bekleyecekti.

## 2. Uygulanan yol (4a)

```
yakalanan kare N-1, N
   │
   ├── parlaklık piramidi            luma, 1/2, 1/4        (uretim.hlsl)
   ├── blok eşleme, kabadan inceye   16 px blok, 3 seviye
   ├── 3×3 ortanca                   aykırı vektörleri atar
   └── çift yönlü warp + karışım     örtüşmede tek kareye düşer
```

Dosyalar: `src-tauri/src/scaling/hareket.rs` (CPU referansı ve testler),
`uretim.hlsl` (gerçek zamanlı yol), `uretim.rs` (D3D11 boru hattı).

**Neden bu tasarım**

- **Blok eşleme, piksel başına optik akış değil.** Izgara çözünürlüğü
  1920×1080'de 120×68'e düşüyor; arama maliyetinin katlanabilir olmasının
  tek sebebi bu. Piksel başına akış (Horn-Schunck, Lucas-Kanade) daha iyi
  bir alan üretirdi ama gerçek zamanlı bütçeye sığmıyor.
- **Üç seviyeli piramit.** İki seviye denendi, yetmedi: kaba seviye
  vektörü dört pikselin katına yuvarlıyor ve dar bir düzeltme penceresi o
  yuvarlamayı kapatamıyor. `bilinen_kaydirma_bulunuyor` testi bunu
  yakaladı — 5 piksellik kaydırma 3 olarak ölçülüyordu.
- **Ortanca, ortalama değil.** Ortalama, tek bir yanlış vektörü
  komşularına bulaştırıyor.
- **Örtüşmede karışım yok.** İki yön birbirini tutmuyorsa orada bir
  nesnenin arkasından çıkan piksel var ve o piksel önceki karede **yok**.
  Ortalaması hayalet bir görüntü üretir; onun yerine zaman olarak yakın
  kare seçiliyor. Sonuç "yanlış" değil, "üretilmemiş".

**Doğruluğu nasıl biliniyor**

Gözle değil, sentetik gerçek-referansla. `hareket.rs`'teki testler bilinen
bir kaydırma uygulanmış iki kare veriyor ve çıkan vektörün o kaydırma
olmasını bekliyor; ayrıca üretilen ara karenin, "hiç üretmeyip önceki
kareyi tekrarlamak"tan belirgin olarak daha yakın olmasını arıyor.
Gölgelendirici sabitleri CPU referansıyla test ile bağlı
(`sabitler_referansla_ayni`) ve HLSL'in derlendiği ayrı bir testte
doğrulanıyor.

**Bunların hiçbiri görüntünün iyi göründüğünü söylemiyor.** Onu ancak göz
söyler; `tasks.md` → Sıradaki 8.

## 3. Bedel — ve neden algoritma hızlanınca azalmıyor

Ara kare, iki **gerçek** kare arasına giriyor. İkinci gerçek kare
üretilmeden ara kare hesaplanamaz; dolayısıyla ikinci kare elde tutuluyor
ve bir sunum turu geç gösteriliyor.

```
üretimsiz:   N-1 ────────► göster N-1      N ────────► göster N
üretimli:    N-1 ────────► göster N-1      N ──► [tut] ──► göster ara ──► göster N
                                                 └── bu bekleme algoritmanın hızıyla AZALMIYOR
```

Kaynak 60 kare/s ise bu ~17 ms. Sonsuz hızlı bir GPU'da bile duruyor,
çünkü beklenen şey hesap değil **bilgi**: henüz üretilmemiş bir karenin
içeriği.

Bunun iki doğrudan sonucu var ve ikisi de koda girmiş durumda:

1. **Rekabetçi modda kapalı** — profil şeması ve `Motor` seviyesinde iki
   ayrı kapı. Rekabetçi mod tam olarak bu beklemeyi en aza indirmek için
   var.
2. **Ekran yenileme hızı şart** — her gerçek kare için iki sunum turu
   harcanıyor. Ekran kaynaktan belirgin olarak hızlı değilse (`mod.rs`:
   `UYUMLU_YENILEME_HZ`) üretilen kare, gerçek karelerin sırasını
   bekletmekten başka bir işe yaramıyor. Döngü yenileme hızını okuyup
   kullanıcıya söylüyor.

## 4. ML yolu (4b) — fizibilite

### 4.1 Veri seti

İhtiyaç: ardışık kare üçlüleri (N-1, N, N+1), oyun görüntüsü, çeşitli
tür ve çözünürlükte. Model N-1 ve N+1'den N'i tahmin etmeyi öğreniyor;
yani etiketleme **gerekmiyor** — kendi kendini denetleyen (self-supervised)
bir problem. Bu, veri tarafını sanılandan ucuz yapıyor.

Ama kolay olan kısım burada bitiyor:

- Genel amaçlı video veri setleri (Vimeo-90K, X4K1000FPS) film ve gerçek
  görüntü içeriyor. Oyun görüntüsü farklı: keskin arayüz katmanları,
  yazı, saydam efektler, kamera kesmeleri, tekrarlayan doku desenleri.
  Film üstünde eğitilmiş bir modelin oyun arayüzünde ne yapacağı ayrı bir
  soru.
- Oyun görüntüsü toplamak **hukuki bir soru** açıyor: yakalanan kareler
  başkasının telifli eseri. Kendi makinemizde eğitim için kullanmak ile
  bir veri setini dağıtmak aynı şey değil; ikincisi yapılmayacak.
- Gerçekçi bir ilk hedef: 5-10 oyundan, 1080p, ~50-100 saat kayıt.
  Sıkıştırılmamış bu boyut ~10 TB mertebesinde; makul bir kodekle ~200-400 GB.

### 4.2 Eğitim maliyeti

Bu sınıftaki modeller (RIFE, IFRNet ve türevleri) tek bir üst seviye GPU'da
günler mertebesinde eğitiliyor. Kiralık bir A100/H100 ile birkaç yüz
dolarlık bir tur, birkaç turluk deneme-yanılma ile birkaç bin dolarlık bir
kalem. **Bu, projenin duramayacağı bir rakam değil.**

Asıl maliyet para değil: her tur sonunda "daha mı iyi oldu" sorusunun
cevabı gözle veriliyor ve tek kişilik bir projede o döngü haftalar sürüyor.

### 4.3 Inference hızı — asıl engel

Kritik kısıt bu. Bütçe:

| Ekran | Kare aralığı | Boru hattının tamamına düşen |
|---|---|---|
| 144 Hz | 6,9 ms | üretim + ölçekleme + sunum |
| 240 Hz | 4,2 ms | aynısı |

Yani ara karenin **birkaç milisaniyede** üretilmesi gerekiyor, 1080p'de,
üstelik oyunun kendisi aynı GPU'yu kullanırken. Bu bütçede çalışan bir
model mümkün ama:

- ONNX Runtime ya da DirectML bağımlılığı geliyor — şu an ikilide
  **hiçbir** ML çalışma zamanı yok ve kurulum 2,2 MB. Bir inference
  motoru bunu bir mertebe büyütür.
- Model ağırlıkları ikiliye gömülecekse boyut, indirilecekse "indirilen
  model yok" duruşu (karar #28) bozulur.
- Oyunla **aynı** GPU'da çalışan bir model, oyundan hesap gücü çalıyor.
  Kare üretiminin kazandırdığı akıcılık, oyunun kendi kare hızından
  düşerse net sonuç eksiye geçebilir. Bunu ancak ölçerek bilebiliriz.

### 4.4 Klasik yol ile ML yolu arasındaki gerçek fark

ML'in kazandıracağı şey hız değil **kalite**: örtüşen bölgeler, hızlı
kamera hareketi, saydam efektler ve arayüz katmanları. Klasik yolun
bilinen zayıflıkları tam olarak bunlar ve kodda dürüstçe işaretli
(örtüşmede "üretilmemiş" kareye düşmek, ±24 piksellik hareket sınırı).

Yani 4b, 4a'nın yerine geçen bir şey değil, onun zayıf olduğu yerleri
kapatan bir yükseltme.

## 5. Karar

**4b şu an açılmıyor.** Gerekçe tek tek maliyetler değil, sıralama:

1. 4a **sahada doğrulanmadı**. Klasik yolun gerçek oyunlarda ne kadar iyi
   olduğu bilinmeden, onu iyileştirecek bir modele yatırım yapmak,
   çözülmemiş bir problemi optimize etmek olur.
2. Faz 1'in saha doğrulaması hâlâ bekliyor (`tasks.md` → Sıradaki 1).
   Faz 3 bu kurala rağmen yazıldı (karar #33) ve 4a da öyle (karar #35);
   bu **iki kez taşınan bir risk** ve üçüncüye çıkarılmayacak.
3. ML çalışma zamanı, ikili boyutu ve model dağıtımı üçü birden ürünün
   dağıtım biçimini değiştiriyor (`DISTRIBUTION.md`). Bu, kod kararı
   değil ürün kararı.

**Yeniden değerlendirme koşulu**: 4a en az 5 oyunda gözle denendikten
sonra, kusurların **hangisinin** ML ile kapanacağı listelenmiş olarak.
O liste olmadan 4b'nin neyi çözeceği bilinmiyor demektir.

## 6. Bu belgeyi ne geçersiz kılar

- 4a'nın saha denemesinde kabul edilemez bulunması (o zaman soru "ML mi"
  değil, "kare üretimi bu üründe olmalı mı").
- Windows'un kendi sağladığı, ikiliye ağırlık gömmeyi gerektirmeyen bir
  kare enterpolasyon API'si çıkması (OCR'de olduğu gibi — karar #28).
- Inference bütçesinin ölçülmesi: 1080p'de birkaç ms'de koşan bir modelin
  bu makinede **gerçekten** koştuğunun gösterilmesi.
