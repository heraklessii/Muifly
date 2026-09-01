# Muifly

**Windows için oyun performans aracı — ne yaptığını gösteren, her adımı geri
alınabilen türden.**

Sistem optimizasyonu, ağ ölçümü ve (ileride) görüntü ölçekleme tek uygulamada.
Abonelik yok, reklam yok, telemetri yok.

> Bu depo tanıtım sayfası ve demo dağıtımı içindir. Muifly kapalı kaynaklı,
> tek seferlik ücretli bir üründür — kaynak kod burada yayınlanmaz.
> Bkz. [LICENSE.md](LICENSE.md).

## Ne yapar

**Sistem**
- Öndeki oyunu algılar, süreç önceliğini yükseltir
- Seçtiğin arka plan uygulamalarını **dondurur** (kapatmaz — oyundan çıkınca
  kaldıkları yerden devam ederler)
- Güç planını değiştirir, oyun kapanınca eskisine döner
- Hibrit CPU'larda oyunu performans çekirdeklerine sabitleyebilir (isteğe bağlı,
  varsayılan kapalı)

**Ağ**
- DNS çözümleyicilerini gerçek sorgularla karşılaştırır ve en hızlısını gösterir
- Gecikme, jitter ve paket kaybını sürekli ölçer
- Yol testiyle gecikmenin nerede biriktiğini gösterir
- Nagle birleştirmesini ve Windows'un çokluortam ağ kısıtlamasını kaldırabilir
- Oyun trafiğine QoS önceliği tanımlayabilir

**Kütüphane**
- Kurulu Steam ve Epic oyunlarını kapak görselleriyle listeler; profili
  oyuna tıklayarak açarsın, exe adını bilmen gerekmez
- Bulamadığı oyunlar için `.exe` dosyasını doğrudan seçebilirsin
- Bunların hepsi **senin diskinden** okunur: hiçbir servise sorulmaz, hangi
  oyunlara sahip olduğun hiçbir yere gönderilmez

**Şeffaflık**
- Yapılan her değişiklik günlüğe yazılır: ne, ne zaman, hangi değerden hangi değere
- Her değişiklik tek tıkla geri alınabilir
- Optimizasyon öncesi ve sonrası ölçümler yan yana gösterilir

## Ne yapmaz

Bunlar eksik özellik değil, bilinçli sınırlar:

| Yapmaz | Neden |
|---|---|
| Oyun sürecine kod enjekte etmez | DLL injection ve bellek hook'lama anti-cheat riski taşır. Yalnızca resmi Windows API'leri kullanılır. |
| Ağ trafiğini kendi sunucularına yönlendirmez | VPN tüneli çoğu zaman ping'i kötüleştirir, abonelik modeline zorlar ve trafiğini görebileceğimiz bir konuma geçmemizi gerektirir. |
| DNS ayarını kendiliğinden değiştirmez | Adaptör DNS'ini programın değiştirmesi, yanlış gittiğinde seni internetsiz bırakır. Ölçüp öneriyoruz. |
| Süreçleri kapatmaz | Dondurma tersine çevrilebilir, kapatma değil. |
| Bellek "temizlemez" | Standby list temizlemenin ölçülebilir bir faydası gösterilemiyor. |
| Oyun kütüphaneni dışarı bildirmez | Oyun adları ve kapaklar Steam ve Epic'in kendi disk dosyalarından okunur. Kütüphanen için tek bir ağ isteği yapılmaz. |
| Telemetri toplamaz | Kullanım istatistiği, çökme raporu, analytics — hiçbiri gönderilmiyor. |
| Sayısal vaat vermez | "Ping'i 20 ms düşürür" gibi bir iddia dürüst olamaz. Gösterilen her sayı senin makinende ölçülür. |

## Kurulum

- **Steam** — *(mağaza sayfası hazırlanıyor)*
- **itch.io** — *(hazırlanıyor)*
- **Demo** — *(hazırlanıyor; yayımlandığında bu deponun
  [Releases](../../releases) bölümünde olacak)*

Sistem gereksinimi: Windows 10 sürüm 1809 veya üzeri, x64.

Demo süresiz olacak; zaman sınırı, nag ekranı veya kapanma sayacı içermeyecek. Sınır
özellik seviyesinde: System Boost'un tamamı ve öncesi/sonrası ölçüm demoda
olacak, Network Boost ve çoklu profil tam sürümde.

## Sık sorulanlar

**Anti-cheat sorunu çıkarır mı?**
Muifly oyun sürecine hiçbir şey enjekte etmez, oyun belleğini okumaz, dosyalarını
değiştirmez. Yaptığı her şey Windows'un kendi API'leri üzerinden dışarıdan
yapılır. Yine de hiçbir üçüncü taraf anti-cheat sistemi için garanti verilemez;
bunu net söylemeyi tercih ediyoruz.

**Yönetici yetkisi neden isteniyor?**
Sürekli istenmiyor. Arka plan izleme yükseltilmiş yetkiyle çalışmaz. Yalnızca
sistem geneli ağ ayarları (Nagle, QoS) için yönetici gerekir ve ne için
istendiği ekranda yazar.

**Program çökerse sistemim değişmiş halde mi kalır?**
Hayır. Geri alma defteri diske yazılır; bir sonraki açılışta bekleyen
değişiklikler otomatik geri alınır. Dondurulmuş süreçler devam ettirilir, güç
planı eski haline döner.

**Profillerimi paylaşabilir miyim?**
Evet. Profiller `%APPDATA%\Muifly\profiller` altında okunabilir JSON dosyaları
olarak durur. Format belgelidir ve profil dosyaları sana aittir.

## Hata bildirimi

[Issues](../../issues) bölümünü kullan. Kaynak kod paylaşılmıyor ama hata
raporları ve özellik istekleri buradan takip ediliyor.

## Mui ailesi

Muifly; Muitoon, Muita, Muiget ve Muivly ile aynı tasarım dilini paylaşır.
Muiget ve Muivly açık kaynak ve ücretsizdir; Muifly ticari bir üründür.

---

© 2026 Muifly. Tüm hakları saklıdır. Kullanım koşulları: [LICENSE.md](LICENSE.md)
