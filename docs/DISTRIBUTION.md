# Dağıtım, Lisans ve Ticari Model

> Bu dosya "Muifly nasıl satılıyor ve ne açık, ne kapalı" sorusunun tek
> otoritesidir. Bir yerde (README, site, mağaza metni, UI) bununla çelişen bir
> ifade görülürse, doğru olan burasıdır.

## Temel Karar

**Muifly kapalı kaynak, tek seferlik ücretli bir üründür.**

- Kaynak kodu yayınlanmaz. Repo özeldir (private).
- Abonelik yok, reklam yok, oyun içi satın alma yok, telemetri yok.
- Ücretsiz ve fonksiyon olarak sınırlı bir **demo** vardır (aşağıda kapsamı).

Bu, Mui ailesindeki diğer projelerden bilinçli bir ayrımdır: Muiget ve Muivly
açık kaynak/ücretsizdir, Muifly değildir. Yeni bir oturum "Mui projeleri açık
kaynak olur" varsayımıyla hareket etmemelidir.

## "Şeffaflık" ne demek, ne demek değil

`DESIGN_PRINCIPLES.md` madde 2'deki şeffaflık ilkesi **çalışma zamanı
şeffaflığıdır**, kaynak kodu şeffaflığı değil:

| Şeffaflık = | Şeffaflık ≠ |
|---|---|
| Hangi process durduruldu, listede görünür | Kaynak kodun okunabilir olması |
| Hangi ayar hangi değerden hangi değere geçti, loglanır | Derleme adımlarının yayınlanması |
| Her değişiklik tek tıkla geri alınabilir | Lisansın izin verici olması |
| Ne zaman ağa çıkıldığı kullanıcıya söylenir | Repo'nun public olması |

Bu ayrım pazarlama metninde de korunur: "açık kaynak" ya da "open source"
kelimeleri Muifly için **hiçbir yerde kullanılmaz**. Kullanılacak dil:
"ne yaptığını gösteren", "geri alınabilir", "kara kutu değil".

## Kanallar

| Kanal | Rol | Ne var |
|---|---|---|
| **Steam** | Birincil satış kanalı | Tam sürüm (ücretli), demo (Steam demo uygulaması), duyurular, kullanıcı yorumları |
| **itch.io** | İkincil satış kanalı | Aynı tam sürüm, DRM'siz indirme, Steam'in %30'unu istemeyen alıcı için |
| **GitHub** | Yalnızca tanıtım + demo dağıtımı | Public bir "site" reposu: GitHub Pages tanıtım sayfası, demo binary'si Releases'te, issue takibi (hata bildirimi). **Kaynak kod yok.** |

### GitHub reposunun kapsamı (net sınır)

İki depo var ve adları sabit:

| Depo | Görünürlük | İçerik |
|---|---|---|
| `heraklessii/Muifly` | **public** | Vitrin: tanıtım sayfası, README, EULA, Releases (demo), Issues |
| `heraklessii/Muifly-dev` | **private** | Geliştirme: kaynak, belgeler, araçlar, CI |

Site içindeki bağlantılar (`site/index.html` → Releases, Issues, EULA)
public depoyu gösteriyor; bu yüzden public deponun adı `Muifly`.

Public repo **sadece** şunları içerir:

- `site/` — GitHub Pages tanıtım sayfası (bu depodaki `site/` klasörü)
- `README.md` — ürün tanıtımı, mağaza linkleri, demo indirme linki
- `LICENSE.md` — **EULA** (son kullanıcı lisans sözleşmesi), açık kaynak lisansı değil
- Releases — yalnızca **demo** binary'si (imzalı kurulum dosyası)
- Issues — hata bildirimi ve özellik isteği

Public repoya **asla** girmeyecekler: `src/`, `src-tauri/`, `docs/` (bu iç
belgeler), profil üretim araçları, imzalama anahtarları.

Bu ayrım fiziksel olarak korunur: geliştirme deposu ayrı ve private'tır.
Yayın süreci, private depodan public depoya **derlenmiş çıktı ve site
klasörünü kopyalamaktır**, kaynak kodu değil.

Kopyalama elle yapılmıyor — `arac/vitrin-hazirla.mjs` bir **izin listesine**
göre kopyalıyor ve hedefte kaynak kod bulursa duruyor:

```bash
node arac/vitrin-hazirla.mjs ../Muifly-vitrin
```

Betik git komutu çalıştırmıyor: dosyaları hazırlıyor, `git push` insana
kalıyor. Kaynak kodun public depoya sızması geri alınamaz bir olay (git
geçmişi ve GitHub önbelleği kalır), o yüzden iki kapı da kapalı — izin
listesi ve çıkıştaki yasaklı yol kontrolü.

## Demo Kapsamı

Demo, ürünü "kısıtlanmış" değil "eksiksiz ama dar" gösterir — kullanıcı
gerçekten çalıştığını kendi makinesinde görmeli, çünkü bu kategoride en büyük
satın alma engeli "acaba placebo mu" şüphesidir.

**Demoda VAR:**
- System Boost'un tamamı, tek bir oyun profili sınırıyla (kullanıcı 1 profil
  oluşturabilir)
- Öncesi/sonrası ölçüm ve karşılaştırma raporu (satın alma kararını veren şey bu)
- Şeffaflık log ekranı, tam haliyle
- Geri alma (revert) mekanizmasının tamamı

**Demoda YOK:**
- Network Boost (Faz 2)
- Scaling (Faz 3+)
- Sınırsız profil, profil içe/dışa aktarma
- Sistem açılışı modu (otomatik başlatma)

**Demoda asla olmayacak**: zaman sınırı (7 gün deneme), kapanma sayacı, nag
ekranı. Demo süresiz çalışır. Sınır özellik seviyesindedir, zaman seviyesinde
değil — çünkü zaman sınırı, ürünün "geri alınabilirlik" vaadiyle çelişen bir
baskı aracıdır.

### Nasıl derleniyor

```bash
cargo build --release --features demo    # src-tauri/ içinde
```

Kısıtlar `src-tauri/src/surum.rs` içindeki `Kisitlar` yapısında, tek yerde
(karar #20). Çalışma zamanı lisans kontrolü, anahtar doğrulama ya da sunucuya
sorma yok — demo ayrı bir ikili.

**Demoda da tamamen açık olan iki yol**: geri alma ve temizlik. Otomatik
başlatmayı *kapatmak* ve QoS ilkelerini kaldırmak demoda da çalışıyor. Sürüm
farkı, kullanıcının sistemine bırakılan izi hiçbir koşulda artırmamalı.

## Fiyatlandırma

- Hedef aralık: **Lossless Scaling'in ($6.99) biraz üzerinde**, çünkü Muifly üç
  kategoriyi birleştiriyor. Faz 1+2 yayındayken $7.99–9.99 bandı makul.
- Faz 3 (scaling) eklenince fiyat artışı yapılabilir; **önceden satın alanlar
  ücretsiz alır** (Steam'de standart davranış, ayrıca duyurulur).
- İndirim politikası: Steam sezon indirimleri normal, ama %75+ derin indirim
  yapılmaz — bu kategoride derin indirim "değersiz araç" sinyali veriyor.
- itch.io fiyatı Steam ile aynı tutulur. Fiyat farkı kanal çatışması yaratır.

## Lisans Modeli (teknik)

- **DRM yok.** Steam'in kendi sahiplik kontrolü (Steamworks) dışında ek bir
  aktivasyon/lisans sunucusu yok. itch.io sürümü tamamen DRM'siz.
- Program çalışmak için **hiçbir zaman** ağa çıkmak zorunda değildir. Ağ
  erişimi yalnızca kullanıcının açıkça başlattığı işlemlerde olur (DNS/route
  testi, sürüm kontrolü) ve bu her seferinde loglanır.
- Lisans anahtarı doğrulama sunucusu kurulmaz. Solo geliştirici için bakım
  yükü, engellediği korsanlıktan büyük.

## Mağaza Sayfası Gereksinimleri (yayın öncesi kontrol listesi)

`DESIGN_PRINCIPLES.md` madde 4 (sayısal vaat yok) mağaza metni için de
geçerlidir — hatta orada daha kritiktir, çünkü Steam yorumlarında "vaat edilen
FPS artışı gelmedi" en hızlı puan düşüren şikayettir.

- [ ] Steam mağaza açıklaması: sayısal iddia içermiyor
- [ ] Ekran görüntüleri: gerçek uygulamadan, düzenlenmemiş
- [ ] Tanıtım videosu: öncesi/sonrası ölçümü gerçek bir oturumdan
- [ ] Anti-cheat bölümü: "injection yapmıyoruz" net şekilde yazılı, garanti
      verilmeden (bkz. `RISKS.md`)
- [ ] Sistem gereksinimleri: Windows 10 1809+ / Windows 11, x64
- [ ] "Ne yapmaz" bölümü var: ping'i garanti etmez, oyun motoruna dokunmaz,
      internet hızını artırmaz
- [ ] İade politikası Steam varsayılanı (2 saat / 14 gün) — özel kısıt yok
- [ ] Kod imzalama sertifikası alındı (SmartScreen uyarısı satışı doğrudan
      öldürür; bu kodla çözülmüyor)
- [x] Üçüncü taraf lisans bildirimleri uygulama içinde (EULA madde 8'in vaadi;
      Ayarlar → Yasal, karar #21). Yayın öncesi `node arac/ucuncu-taraf-uret.mjs`
      son bir kez çalıştırılmalı ki listede yayınlanan sürümün bağımlılıkları
      dursun

## Telemetri

**Yok.** Kullanım istatistiği, çökme raporu, analytics toplanmaz. Bir çökme
raporu sistemi ileride eklenirse: varsayılan kapalı, açık rıza ile, ne
gönderildiği kullanıcıya gösterilerek.

Bu, ücretli bir üründe rakiplerden ayrışmanın en ucuz yolu ve mağaza
sayfasında doğrudan söylenebilecek bir şey.
