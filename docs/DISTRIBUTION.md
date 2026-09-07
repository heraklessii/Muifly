# Dağıtım ve Lisans

> Bu dosya "Muifly nasıl dağıtılıyor ve hangi lisansla" sorusunun tek
> otoritesidir. Bir yerde (README, site, UI) bununla çelişen bir ifade
> görülürse, doğru olan burasıdır.

## Temel Karar

**Muifly ücretsiz ve açık kaynaktır — Apache License 2.0.**

- Kaynak kod GitHub'da yayınlanır, depo public'tir.
- Ücret yok, abonelik yok, reklam yok, oyun içi satın alma yok, telemetri yok.
- Demo/tam sürüm ayrımı **yok**: tek bir ikili, bütün özellikleriyle.

Bu, karar #39'la değişti; öncesinde Muifly kapalı kaynak ve tek seferlik
ücretli olarak planlanıyordu. Artık Mui ailesindeki diğer projelerle
(Muiget, Muivly) aynı modelde.

## Neden Apache 2.0

- Patent hükmü var (madde 3): katkı gönderen, katkısındaki patentler için
  kullanıcılara lisans vermiş oluyor. MIT'te bu boşluk açık.
- İzin verici: kullanıcıyı da, kurumsal bir kullanımı da kısıtlamıyor.
- Değişiklik bildirimi (madde 4b) istiyor: fork'lanmış bir sürümün
  değiştirildiği görünür kalıyor.
- Mui ailesinin geri kalanıyla aynı lisans — bir kullanıcının iki proje için
  iki farklı lisans metni okuması gerekmiyor.

Katkı gönderen, Apache 2.0 madde 5 uyarınca katkısını aynı lisansla
lisanslamış sayılıyor. Ayrı bir CLA yok: solo bir projede CLA, gönderilen
yamaların önündeki en gereksiz eşik.

## "Şeffaflık" ne demek

`DESIGN_PRINCIPLES.md` madde 2'deki şeffaflık ilkesi iki katmanlı:

| Katman | Ne demek |
|---|---|
| **Çalışma zamanı** | Hangi süreç durduruldu, hangi ayar hangi değerden hangi değere geçti, ne zaman geri alındı, ne zaman ağa çıkıldı — hepsi ekranda |
| **Kaynak** | Kod okunabilir, derlenebilir, çatallanabilir |

İkisi ayrı işler ve ikisi de gerekli. Kaynak açık diye çalışma zamanı
günlüğünden vazgeçilmiyor: kimse bir aracı her çalıştırmadan önce kaynağını
okumuyor.

## Kanallar

| Kanal | Rol | Ne var |
|---|---|---|
| **GitHub** | Birincil | Kaynak kod, Releases'te imzalı kurulum paketi, Issues'ta hata takibi, Pages'te tanıtım sayfası |

Tek depo: `heraklessii/Muifly`, public. Eskiden var olan "vitrin deposu"
ayrımı (`arac/vitrin-hazirla.mjs`) kaldırıldı — dayanağı kaynak kodun kapalı
olmasıydı.

Steam ve itch.io yayını şu an planda değil. İleride yapılırsa lisans
değişmez: Apache 2.0 ücretli dağıtımı yasaklamıyor, ama kaynak açık olduğu
sürece o kanalın satacağı şey kolaylıktır (otomatik güncelleme, kurulum),
ürünün kendisi değil.

## Derleme

```bash
npm install
node arac/olcum-yardimcisi-hazirla.mjs   # kare ölçümü sidecar'ı — build ÖNCESİ şart
npm run tauri build
```

Gerekenler: Rust 1.77.2+, Node.js 20+, Visual Studio Build Tools (C++ iş yükü).

## Sürüm ve güncelleme

- **DRM yok, lisans anahtarı yok, aktivasyon sunucusu yok.**
- Program çalışmak için **hiçbir zaman** ağa çıkmak zorunda değildir. Ağ
  erişimi yalnızca kullanıcının açıkça başlattığı işlemlerde olur (DNS/yol
  testi, gecikme ölçümü) ve her seferinde günlüğe yazılır.
- Otomatik güncelleme yok. Yeni sürüm Releases'e konuyor.

## Yayın kontrol listesi

`DESIGN_PRINCIPLES.md` madde 4 (sayısal vaat yok) yayın metinleri için de
geçerlidir.

- [ ] README ve site: sayısal iddia içermiyor
- [ ] Ekran görüntüleri: gerçek uygulamadan, düzenlenmemiş
- [ ] Anti-cheat bölümü: "injection yapmıyoruz" net yazılı, garanti
      verilmeden (bkz. `RISKS.md`)
- [ ] Sistem gereksinimleri: Windows 10 1809+ / Windows 11, x64
- [ ] "Ne yapmaz" bölümü var: ping'i garanti etmez, oyun motoruna dokunmaz,
      internet hızını artırmaz, görüntü ölçekleme yapmaz
- [ ] `node arac/olcum-yardimcisi-hazirla.mjs` çalıştırıldı ve üretilen
      sidecar ikilisi de imzalandı (imzalanmazsa kare ölçümü yayın
      sürümünde sessizce ölür)
- [ ] Kod imzalama sertifikası (SmartScreen uyarısı ilk kurulumu zorlaştırır;
      bu kodla çözülmüyor)
- [x] Üçüncü taraf lisans bildirimleri uygulama içinde (Ayarlar → Yasal,
      karar #21). Yayın öncesi `node arac/ucuncu-taraf-uret.mjs` son bir kez
      çalıştırılmalı ki listede yayınlanan sürümün bağımlılıkları dursun
- [ ] `LICENSE` dosyası pakete dahil

## Telemetri

**Yok.** Kullanım istatistiği, çökme raporu, analytics toplanmaz. Bir çökme
raporu sistemi ileride eklenirse: varsayılan kapalı, açık rıza ile, ne
gönderildiği kullanıcıya gösterilerek.
