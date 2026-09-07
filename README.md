# Muifly

**Windows için oyun performans aracı — ne yaptığını gösteren, her adımı geri
alınabilen türden.**

Sistem optimizasyonu ve ağ ölçümü tek uygulamada.
Abonelik yok, reklam yok, telemetri yok.

> Muifly açık kaynak ve ücretsizdir — [Apache License 2.0](LICENSE).

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

**Ölçüm**
- Oyun çalışırken kare süresini ölçer; ortalama, %1 en kötü ve takılma sayısı
  ayrı ayrı gösterilir
- Optimizasyon öncesi ve sonrası ölçümler yan yana durur

**Kütüphane**
- Kurulu Steam ve Epic oyunlarını kapak görselleriyle listeler; profili
  oyuna tıklayarak açarsın, exe adını bilmen gerekmez
- Bulamadığı oyunlar için `.exe` dosyasını doğrudan seçebilirsin
- Bunların hepsi **senin diskinden** okunur: hiçbir servise sorulmaz, hangi
  oyunlara sahip olduğun hiçbir yere gönderilmez

**Şeffaflık**
- Yapılan her değişiklik günlüğe yazılır: ne, ne zaman, hangi değerden hangi değere
- Her değişiklik tek tıkla geri alınabilir
- Biten oturumlar geçmişte durur; dosya bu bilgisayardan çıkmaz

## Ne yapmaz

Bunlar eksik özellik değil, bilinçli sınırlar:

| Yapmaz | Neden |
|---|---|
| Oyun sürecine kod enjekte etmez | DLL injection ve bellek hook'lama anti-cheat riski taşır. Yalnızca resmi Windows API'leri kullanılır. |
| Ağ trafiğini kendi sunucularına yönlendirmez | VPN tüneli çoğu zaman ping'i kötüleştirir, abonelik modeline zorlar ve trafiğini görebileceğimiz bir konuma geçmemizi gerektirir. |
| DNS ayarını kendiliğinden değiştirmez | Adaptör DNS'ini programın değiştirmesi, yanlış gittiğinde seni internetsiz bırakır. Ölçüp öneriyoruz. |
| Süreçleri kapatmaz | Dondurma tersine çevrilebilir, kapatma değil. |
| Bellek "temizlemez" | Standby list temizlemenin ölçülebilir bir faydası gösterilemiyor. |
| Görüntü ölçekleme ve kare üretimi yapmaz | Denendi ve çıkarıldı: sonuç yeterince iyi değildi ve bakımı, aracın asıl işinden çalıyordu. Bu iş için ayrı araçlar var. |
| Oyun kütüphaneni dışarı bildirmez | Oyun adları ve kapaklar Steam ve Epic'in kendi disk dosyalarından okunur. Kütüphanen için tek bir ağ isteği yapılmaz. |
| Telemetri toplamaz | Kullanım istatistiği, çökme raporu, analytics — hiçbiri gönderilmiyor. |
| Sayısal vaat vermez | "Ping'i 20 ms düşürür" gibi bir iddia dürüst olamaz. Gösterilen her sayı senin makinende ölçülür. |

## Kurulum

Hazır kurulum dosyaları [Releases](../../releases) bölümünde olacak.

Sistem gereksinimi: Windows 10 sürüm 1809 veya üzeri, x64.

## Kaynaktan derleme

Gerekenler: [Rust](https://rustup.rs) (1.77.2+), [Node.js](https://nodejs.org)
(20+) ve Visual Studio Build Tools (C++ iş yükü).

```bash
npm install
node arac/olcum-yardimcisi-hazirla.mjs   # kare ölçümü yardımcısı — build öncesi şart
npm run tauri dev                        # geliştirme
npm run tauri build                      # kurulum paketi
```

Testler:

```bash
npm test                                  # arayüz (vitest)
cargo test --manifest-path src-tauri/Cargo.toml
```

## Sık sorulanlar

**Anti-cheat sorunu çıkarır mı?**
Muifly oyun sürecine hiçbir şey enjekte etmez, oyun belleğini okumaz, dosyalarını
değiştirmez. Yaptığı her şey Windows'un kendi API'leri üzerinden dışarıdan
yapılır. Yine de hiçbir üçüncü taraf anti-cheat sistemi için garanti verilemez;
bunu net söylemeyi tercih ediyoruz.

**Yönetici yetkisi neden isteniyor?**
Sürekli istenmiyor. Arka plan izleme yükseltilmiş yetkiyle çalışmaz. Yalnızca
sistem geneli ağ ayarları (Nagle, QoS) ve kare ölçümü için yönetici gerekir ve
ne için istendiği ekranda yazar.

**Program çökerse sistemim değişmiş halde mi kalır?**
Hayır. Geri alma defteri diske yazılır; bir sonraki açılışta bekleyen
değişiklikler otomatik geri alınır. Dondurulmuş süreçler devam ettirilir, güç
planı eski haline döner.

**Profillerimi paylaşabilir miyim?**
Evet. Profiller `%APPDATA%\Muifly\profiller` altında okunabilir JSON dosyaları
olarak durur. Format belgelidir.

## Katkı

Hata bildirimi ve özellik isteği için [Issues](../../issues), kod için pull
request. Katkı gönderdiğinde, katkının Apache License 2.0 altında
lisanslanmasını kabul etmiş olursun (lisans metni, madde 5).

Kod yazmadan önce [`docs/DESIGN_PRINCIPLES.md`](docs/DESIGN_PRINCIPLES.md)
okunmalı: beş ilkenin çoğu testle korunuyor ve ihlal eden bir yama CI'da
kırmızıya döner.

## Mui ailesi

Muifly; Muitoon, Muita, Muiget ve Muivly ile aynı tasarım dilini paylaşır.
Hepsi açık kaynak ve ücretsizdir.

---

Copyright 2026 Muifly · [Apache License 2.0](LICENSE)
