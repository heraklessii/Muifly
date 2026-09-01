# Çalışma Günlüğü

> En yeni girdi en üstte. Yeni bir oturum önce buraya bakmalı.

## Oturum 5 — 1 Eylül 2026

Arayüz baştan yazıldı. Backend'e, komut yüzeyine ve `lib/` katmanına
dokunulmadı: değişen şey yerleşim, tasarım sistemi ve ekranların bilgi
sıralaması. Arayüz testleri 32 → 34, ikisi de yeni davranışı koruyor.

### Neden baştan

Eski arayüz beş ekranın hepsinde aynı şeyi yapıyordu: eşit ağırlıkta kartları
alt alta dizmek. Bunun iki sonucu vardı. Kullanıcının en çok baktığı bilgi
(hangi moddayız, ne yapılabilir) bir panel başlığının içinde, ölçüm
kutularının üstünde küçük bir rozet olarak duruyordu; ve Durum ekranı yedi
kartlık düz bir liste olduğu için hiçbir şey öne çıkmıyordu.

### Kabuk (`App.tsx`)

- Üst çubuk + yan menü yerine **tam boy kenar çubuğu + bağlamsal başlık
  çubuğu**. Başlık çubuğu her ekranın ne olduğunu bir cümleyle söylüyor.
- Kenar çubuğunun dibinde **canlı mod kartı**: mod, öndeki süreç, CPU eğrisi
  ve son ölçümler. Hangi sekmede olursan ol görünüyor — "şu an ne oluyor"
  sorusu için Durum sekmesine dönmek gerekmiyor.
- **Mod rengi kabuktan yayılıyor.** `data-mod` kabukta duruyor, `--mod`
  jetonunu oradan alan her parça (nabız, kahraman kartı, durum çubuğu) aynı
  anda değişiyor. Renk dekorasyon değil, bilgi.
- **Ctrl+1..5 ile sekme değiştirme.** Numaralar GÖRÜNEN sekmelere göre:
  demoda ağ sekmesi çizilmediği için Ctrl+3 Günlük'ü açıyor. İkisi de testle
  korunuyor.
- Uzun süren işlem için tek bir üst çubuk göstergesi; her düğmenin kendi
  dönen simgesi olsaydı aynı anda iki iş varmış gibi görünürdü.

### Durum ekranı

- **Kahraman kartı**: mod adı, süreç, durumu anlatan cümle ve asıl eylem tek
  kutuda, mod rengiyle. Ekranın ilk kutusuna ölçüm koyup eylemi aşağı itmek,
  aracı "izlenen bir gösterge paneli" yapardı; oysa asıl iş uygulamak ve geri
  almak.
- Ölçüm kutularına **mini eğri** eklendi (`Sparkline.tsx`, yeni). Yalnızca
  gerçek zaman serisi olanlara: CPU, bellek, gecikme. Jitter ve paket kaybı
  türetilmiş değerler, onlara eğri uydurmak gösterilen her şeyin ölçülmüş
  olduğu kuralını bozardı.
- Grafik 120 → 168 px, alan dolgusu, %25/%50/%75 kılavuzları ve **imleç
  okuması** (saat, CPU, bellek, gecikme) kazandı. Eksen etiketleri ve imleç
  noktaları SVG'nin dışında: `preserveAspectRatio="none"` yatay gerdiriyor,
  içerideki bir `<text>` gerilir, bir `<circle>` elips olurdu.
- Karşılaştırmada her satıra **yön işareti** geldi. Oran hâlâ üretilmiyor
  (`format.ts` → `yon` değişmedi); değişen tek şey yönün ikonla da
  gösterilmesi.

### Diğer ekranlar

- **Profiller**: satır yerine kart ızgarası. Bir profilde okunacak dört ayrı
  şey var (ad, eşleşen dosyalar, ne yapacağı, eylemler); tek satırda üçü
  kısalıyordu. Profilin ne içerdiği artık rozetlerle görünüyor.
- **Ağ**: DNS sonuçları ve yol testi atlamaları **göreli çubuklarla**.
  Çubuk ölçeği o turdaki en yavaş sonuca göre, mutlak bir eşiğe göre değil —
  "iyi ping" diye evrensel bir eşik yok ve öyle bir eşik çizmek sayısal vaat
  yasağının arka kapıdan ihlali olurdu. Uzun çubuğun "daha iyi" diye
  okunmaması için altına bir cümle yazıldı.
- **Günlük**: süzgeç açılır listeden bölümlenmiş seçiciye geçti (hangisinin
  açık olduğu listeye bakarken de görünmeli). Satırlar düzeye göre renkli sol
  kenar aldı. İki kural değişmedi: geri alınan satır silinmiyor, süzgeç
  açıkken kaç satırın gizlendiği yazıyor.
- **Ayarlar**: dört panele bölündü, her panelin başında ne işe yaradığı
  yazıyor. Ayar satırları arasına ayraç kondu — hangi açıklamanın hangi
  anahtara ait olduğu okunmuyordu.

### Tasarım sistemi (`styles.css`)

Jeton isimleri kardeş projelerle hizalı kaldı; teal vurgu ve Outfit yerinde.
Değişen üç şey: düz zemin yerine çok sönük bir renk yıkaması (`body::before`,
statik — animasyonlu bir arka plan boştayken bile GPU çalıştırırdı), gölge
yerine ışık farkıyla katman (`--yuzey-0..3` + saç teli kenarlık + üst iç
ışık), ve moda bağlı `--mod` jetonu.

Nabız animasyonu `box-shadow` yerine `transform`/`opacity` ile çalışıyor:
ikisi de derleyici katmanında, yeniden boyama yok. Bir performans aracının
arayüzü açık durduğu her saniye CPU harcamamalı.

### Yol üstünde çıkan iki hata

- `overflow: hidden` taşıyan kahraman kartı `min-height: auto` korumasını
  kaybedip dikey flex kutusunda içeriğinin altına eziliyordu; `.icerik > *`
  artık `flex: 0 0 auto`.
- Dar pencerede mod kartındaki mod adı kartın dışına taşıyordu: flex
  satırında `min-width: 0` olmadan kısaltma çalışmıyor. 880 px altında ad
  zaten durum çubuğunda yazdığı için gizleniyor.

### Doğrulama

- `tsc` temiz, `vite build` temiz (JS 257 KB / gzip 78 KB, CSS 32 KB /
  gzip 6.8 KB), 34 arayüz testi geçiyor.
- Beş ekran, iki tema, profil diyaloğu ve dar pencere yerleşimi tarayıcıda
  gerçek veriyle açılıp gözle doğrulandı (geçici bir sahte backend'le;
  doğrulama bitince silindi).
- Rust tarafına dokunulmadı.

## Oturum 4 — 1 Eylül 2026

"Sıradaki" listesinde kod işi kalmamıştı; bu oturum "Değerlendirilecek"
listesinden iki maddeyi kapattı ve arayüzde birkaç yer düzeltti. Rust testleri
160 → 180, arayüz testleri 19 → 32.

### Profil içe/dışa aktarma (karar #23)

Profil dosyaları başından beri paylaşılabilirdi (okunabilir JSON, tek dosya);
eksik olan arayüzden yapabilmekti. Akış bilinçli olarak iki adım:

- `profile_engine/aktarim.rs` (yeni): `onizle` dosyayı okuyup doğruluyor,
  "bu profil uygulanınca ne olacak" listesini ve uyarıları üretiyor —
  **diske dokunmadan**. `ice_aktar` ancak kullanıcı onaylayınca çağrılıyor.
- Kimlik çakışmasında varsayılan **ezmemek**: gelen profil `kimlik-2` olarak
  ekleniyor, üzerine yazmak ayrı bir anahtar. Bir profilin üzerine yazmak,
  geri alma defterinin kapsamadığı tek yıkıcı işlem olurdu (defter sistemdeki
  değişiklikleri geri alıyor, silinen dosyayı değil).
- Boş kimlik araması **dosya adı seviyesinde**: `a/b` ve `a_b` aynı dosyaya
  yazıyor (`store::dosya_adi`), ikisini farklı kimlik saymak bir profili
  ezerdi.
- Etki metinleri Rust tarafında (karar #17 ile aynı gerekçe): sayısal vaat
  yasağı tek yerde test ediliyor.
- Demoda kapalı. `DISTRIBUTION.md` zaten "profil içe/dışa aktarma" diyordu;
  `Kisitlar::profil_aktarimi` bunu koda taşıdı. Dışa aktarma da kapsamda:
  demo ikilisi paylaşılabilir dosya üretmiyor.
- Arayüz: `IceAktarmaDiyalogu.tsx` (yeni) dosya adını, kaydedilecek kimliği,
  uyarıları ve etki listesini gösteriyor; dosya seçme/kaydetme pencereleri
  `lib/api.ts` üzerinden (izinler zaten `capabilities/default.json`'daydı).

### Mod değişimi bildirimi (karar #24)

Tepsi menüsünden yapılan işlemler bildirim gösteriyordu, mod değişimi
göstermiyordu — pencere kapalıyken (otomatik başlatmanın normal hâli) ne
olduğunu görmenin yolu yoktu.

- `Ayarlar::mod_bildirimi`, **varsayılanı kapalı**: her Alt+Tab'da bildirim
  atan bir araç gürültü olurdu.
- `tray::mod_bildirim_metni` saf fonksiyon, metni bildirim altyapısından
  bağımsız test ediliyor.
- **Olmayan iş bildirilmiyor**: oyundan çıkışta hiçbir şey geri alınmadıysa
  bildirim de yok. Bunun için `Motor::mod_guncelle` artık `ModDegisimi`
  döndürüyor (yeni mod + geri alınan kayıt sayısı) — "3 değişiklik geri
  alındı" cümlesi sayılmış bir şeye dayanmak zorunda.

### Arayüz

- **Profil satırında "Ne yapacak?"**: açılır özet, `profil_etkileri`
  komutundan içe aktarma önizlemesiyle **aynı** cümleleri çekiyor. Kullanıcının
  kendi profili için de aynı soruya aynı cevabın verilmesi gerekiyor; metin
  Rust tarafında olduğu için sayısal vaat yasağı tek yerde test ediliyor.
  Komut açılınca çağrılıyor, liste kurulurken değil.
- **Günlükte arama**: mesaja göre süzme + "1 / 3 satır gösteriliyor" sayacı.
  Sayaç şeffaflık için: kullanıcı eksik bir listeye baktığını bilmeli.
  Küçültme yerelli (`toLocaleLowerCase('tr')`) — `toLowerCase()` "İ"yi noktalı
  bir "i"ye çeviriyor ve "İndirme" araması "indirme" ile eşleşmiyordu.
- **Profil listesinde arama**: beş profilden sonra görünüyor.
- Durum ekranındaki "Bekleyen değişiklikler" panelinden **Günlüğe geçiş**
  düğmesi: tek tek geri alma orada ve "ayrıntılar günlükte" cümlesi bir yol
  göstermeliydi.
- Yan menü düğmelerine `aria-current`.

### Doğrulama

- `cargo test` ve `cargo test --features demo`: 180 test geçiyor
- `cargo clippy --all-targets -D warnings` iki yapılandırmada da temiz,
  `cargo fmt` uygulandı, `cargo build` temiz
- `npm test`: 32 arayüz testi; `tsc` ve `vite build` temiz (245 KB JS,
  gzip ~75 KB)
- **Elle doğrulanmadı**: dosya seçme/kaydetme pencerelerinin gerçek
  görünümü ve bildirimlerin masaüstünde çıkması. `tasks.md` →
  "Değerlendirilecek".

## Oturum 3 — 1 Eylül 2026

İki kod işi — uygulama ikonu ("Sıradaki" listesindeki tek kod maddesi) ve
üçüncü taraf lisans ekranı — bir de kod yazılmadan karara bağlanan bir öneri:
ekran çevirisi (karar #22). "Sıradaki" listesinde artık kod işi kalmadı — üç madde
de sahada ya da mağaza tarafında yapılacak işler (saha testi, imzalama
sertifikası, demo ikilisinin elle gözden geçirilmesi).

Rust testleri 149 → 160, arayüz testleri 10 → 19.

### Uygulama ikonu

Depoda Muiget'in indirme oku duruyordu — yanlış uygulamanın ikonu. Yerine
Muifly'ın kendi işareti kondu: yükselen çizgi + ok ucu, yani arayüzdeki
`IconMuifly` ve site favicon'uyla aynı yol.

- `src-tauri/icons/kaynak.svg` (yeni) **tek kaynak**. Kare/gradyan iskeleti
  Mui ailesinden: aynı köşe yarıçapı, aynı koyu zemin, aynı teal vurgu.
  Glyph, 24'lük ızgaradan `X = 32u + 128, Y = 32v + 128` ile ölçeklendi;
  dosyanın başındaki yorum bu eşlemeyi ve üretim komutunu yazıyor.
- Set `npx tauri icon src-tauri/icons/kaynak.svg` ile üretildi. Komut
  `android/`, `ios/` ve `icon.icns` de üretiyor; Muifly yalnızca nsis+msi
  paketlediği için bunlar silindi.
- Eski `kaynak-1024.png` de silindi: Muiget'in PNG'siydi ve artık kaynak
  değil. Kaynak tek yerde durmalı, yoksa bir sonraki düzenleme hangisinin
  gerçek olduğunu bilemez.
- Tepsi simgesi `default_window_icon()` üzerinden geldiği için
  (`tray.rs:85`) kendiliğinden yeni ikona döndü.

16/24/32/48 px'e küçültülüp gözden geçirildi: 24 ve üstünde yükselen ok net,
16'da beklendiği gibi yumuşuyor ama hâlâ seçiliyor.

### Üçüncü taraf lisans ekranı (karar #21)

EULA madde 8 uygulama içinde bir "Üçüncü taraf lisanslar" bölümü vaat
ediyordu; ekran yoktu. Ürünün tutmadığı tek yazılı söz buydu, kapatıldı.

- `arac/ucuncu-taraf-uret.mjs` (yeni) listeyi `cargo tree -e normal`,
  `cargo metadata`, `npm ls --omit=dev` ve kayıt önbelleğindeki LICENSE
  dosyalarından çevrimdışı üretiyor. `cargo-about` kurulu değil ve kurmak
  ağ + uzun bir derleme isterdi; aynı sonucu ek araç olmadan veriyorsa
  bağımlılık taşımıyoruz.
- Çıktı `src-tauri/ucuncu-taraf.json`: 258 bileşen, 118 benzersiz lisans
  metni, ~1,1 MB. Aynı MIT/Apache metni yüzlerce crate'te tekrarladığı için
  metinler bir kez saklanıp indeksle gösteriliyor.
- `src-tauri/src/ucuncu_taraf.rs` (yeni) dosyayı `include_str!` ile gömüyor
  ve **tembel** ayrıştırıyor. Arayüz önce yalnızca listeyi çekiyor (~60 KB),
  metni ancak bir bileşen açıldığında istiyor — iki komut:
  `ucuncu_taraf_listesi`, `ucuncu_taraf_metni`.
- `src/components/LisanslarDiyalogu.tsx` (yeni): arama, açılır satırlar,
  kaynak adresi (`opener` eklentisiyle tarayıcıda). Giriş noktası
  Ayarlar → Yasal.

**Metni olmayan 10 bileşen gizlenmedi.** `webview2-com`, `unic-*`,
`selectors`, `alloc-stdlib` yayımlanan sürümlerinde LICENSE dosyası
taşımıyor. Listeden çıkarmak listeyi temiz gösterirdi ama eksik yapardı;
SPDX kimliği ve kaynak adresiyle duruyorlar, eksik olan da ekranda yazıyor.

**Yazı tipinin OFL metni depoda yoktu.** Outfit'i dağıtıp lisansını
dağıtmamak OFL-1.1'in kendi şartına aykırıydı; metin upstream'den alınıp
`src/assets/fonts/` ve `site/fonts/` altına kondu.

**Tazelik testle korunuyor.** `listedeki_surumler_cargo_lock_ile_ayni`
listedeki her crate'i `Cargo.lock` ile karşılaştırıyor. Test bilerek
bozularak denendi: bir sürüm elle değiştirildiğinde kırmızıya dönüyor ve
mesajı üreteci çalıştırmayı söylüyor.

### Yol üstünde çıkan iki şey

1. **Vitest çağrı geçmişini testler arasında temizlemiyordu.** Yapılandırmada
   `restoreMocks: true` vardı; o gerçeklemeyi geri alıyor ama `mock.calls`
   birikmeye devam ediyor. "Kaç kez çağrıldı" iddiaları bu yüzden sızıyordu.
   `clearMocks: true` eklendi (`vite.config.ts`). Var olan testler bunu fark
   etmemişti çünkü hiçbiri çağrı sayısı iddia etmiyordu.
2. **Üretilen tarih UTC'ydi.** Geceyarısından sonra çalıştırıldığında dosyaya
   dünün tarihi yazılıyordu; yerel tarihe çevrildi.

### Ekran çevirisi tartışıldı, karara bağlandı, yazılmadı (karar #22)

Gerçek zamanlı ekran çevirisi önerildi: seçili alan + kısayol + yerel OCR ve
çeviri, üstüne kullanıcı geri bildirimiyle oyuna özel öğrenen bir katman.
**Kod yazılmadı**; tartışmadan çıkan sınırlar `decisions.md` #22, `ROADMAP.md`
Faz 5 ve `RISKS.md`'ye işlendi.

Çıkan başlıca kararlar:

- **Sürekli çeviri yok, isteğe bağlı var** — gerekçe performans değil kalite.
  Hareket eden ekranda OCR çöp üretir; güvenilir olduğu tek rejim duran diyalog
  kutusu.
- **Öğrenme = çeviri belleği + oyun sözlüğü, model ince ayarı değil.**
  Onayla/reddet ikili sinyali seq2seq için çok zayıf, ürünün içinde eğitim hattı
  gerektirir ve zamanla kayan bir model ilke 2'nin yasakladığı kara kutudur.
  Bellek yaklaşımı daha şeffaf (günlük "bu çeviri senin onayladığın kayıttan
  geldi" diyebilir), daha hızlı (önbellek isabeti model çalıştırmaz) ve kabaca
  onda bir iş.
- **Geri bildirimde asıl olan "Düzelt", "Reddet" değil**; ve özellik sıfır geri
  bildirimle tam çalışmak zorunda — kullanıcıların çoğu hiçbir şeyi puanlamaz.
- **Kısayol `RegisterHotKey`, `WH_KEYBOARD_LL` değil.** İkisi dışarıdan aynı
  görünüyor ama ikincisi kanca ve ilke 3'ü ihlal ediyor. Bu ayrım yazılmasa
  sonraki oturumda gözden kaçabilirdi.
- **Model ikiliye girmez.** Karar #1 Electron'u "~120 MB RAM tabanı hafiflik
  iddiasıyla çelişir" diye elemişti; yerel NMT modeli aynı argümanı bize karşı
  çalıştırıyor. OCR'da bu bedel yok: `Windows.Media.Ocr` işletim sisteminde
  hazır ve `windows` crate'i zaten bağımlılığımız.
- **Overlay ayrı üst pencere** → exclusive fullscreen'de çalışmaz. `RISKS.md`
  bunu zaten karara bağlamıştı; sonucu açıkça kabul edip yazdık.

**Bilinçli olarak açık bırakılan soru**: Muifly modülü mü, ayrı bir Mui ürünü
mü? `PRODUCT_VISION.md` farklılaşma maddesi #1 ("üç kategori tek araçta") bir
sınır; dördüncü kategori onu bozuyor. Alıcı ve rakip kümesi de farklı
(Translumo, LunaTranslator — çoğu ücretsiz). Karşı tarafta Faz 3 ile ortak
yakalama altyapısı var. Şu an karar verecek veri yok; Faz 3'ün yakalama
katmanından sonra bakılacak.

**Neden şimdi yazılmadı**: Faz 1 hâlâ tek bir gerçek oyunda denenmedi. Mevcut
modüllerin hepsinden büyük bir modülü çekirdek ürün doğrulanmadan açmak, faz
disiplininin engellemek için var olduğu şey.

### Doğrulama

`cargo test --all-features` 160 test, `npm test` 19 test, clippy
`-D warnings` altında temiz, `cargo fmt --check` temiz, `npm run build`
temiz (238 KB JS / 73 KB gzip).

Lisans ekranı gerçek veriyle **gözle de doğrulandı**: geçici bir önizleme
sayfası Tauri köprüsünü taklit edip diyaloğu gerçek `ucuncu-taraf.json` ile
çizdirdi. Liste, arama (3/258 süzme), metni olan bir bileşenin açılması ve
metni olmayan bir bileşenin uyarısı koyu ve açık temada tek tek görüldü;
önizleme dosyaları sonra silindi. Birim testleri CSS'i denetlemiyor, bu adım
onun içindi.

## Oturum 2 — 1 Eylül 2026

`tasks.md` → "Sıradaki" listesindeki iki kod maddesi (sistem tepsisi, demo
ayrımı) kapatıldı, yol üstünde iki hata düzeltildi. Rust testleri 133 → 149.

### Sistem tepsisi (`src-tauri/src/tray.rs`, yeni)

Simge + menü: Göster / Öndekine uygula / Varsayılana dön / (ayraç) / Çıkış.
Sol tık pencereyi getiriyor, menü sağ tıkta. İpucu mod değiştikçe
güncelleniyor ("Muifly — Oyun: x.exe").

Menü işlemleri `state::Motor` üzerinden geçiyor; `ondekine_uygula` mantığı
`commands.rs`'ten motora taşındı, iki çağıran da aynı yolu kullanıyor
(değişmez kural: uygula → deftere yaz → günlüğe yaz). Pencere kapalıyken
sonucu görmenin yolu olmadığı için menü işlemleri kısa bir OS bildirimi
gösteriyor.

Tepsi kurulumu başarısız olursa `--tepside` pencereyi **gizlemiyor**:
gizlenmiş ama tepsi simgesi olmayan bir program, kullanıcının erişemediği bir
programdır.

### Düzeltilen iki hata

1. **"Kapatınca tepsiye in" ayarı bir şey yapmıyordu.** Ayar `settings.rs`'te
   ve arayüzde vardı, davranışı yoktu; pencere kapatma programı kapatıyordu.
   Artık `WindowEvent::CloseRequested` yakalanıyor.
2. **Çıkışta oturumluk değişiklikler geri alınmıyordu.** Dondurulmuş bir
   uygulama, Muifly bir daha açılana kadar dondurulmuş kalıyordu. Artık
   `RunEvent::Exit` üzerinde oturumluk kayıtlar geri alınıyor (karar #19).
   Açılıştaki temizlik çökme için son savunma olarak duruyor.
3. **Çok işlemci gruplu makinede yanlış P-core maskesi.** 64'ten fazla
   mantıksal çekirdekli sistemlerde farklı grupların maskeleri VEYA'lanıyordu;
   sonuç, başka bir grubun çekirdeğini işaret eden anlamsız bir affinite
   maskesiydi. Hesap `topolojiyi_hesapla` saf fonksiyonuna çıkarıldı (dört
   yeni test) ve çok gruplu makinede maske verilmiyor.

### Demo/tam sürüm ayrımı (`src-tauri/src/surum.rs`, yeni)

`cargo build --features demo` ile ayrı ikili (karar #20). Kısıtlar tek yerde:
profil sınırı 1, ağ modülü kapalı, otomatik başlatma kapalı.

Kontrol iki katmanda: arayüz kapalı özelliğin sekmesini hiç göstermiyor,
komutlar ayrıca `Error::DemoKisiti` dönüyor. Profil uygulanırken ağ adımları
atlanıyor ve **atlandığı kullanıcıya söyleniyor** — sessiz atlama, profilde
yazanla gerçeği ayırırdı.

Kilitlenmeyen iki yol: QoS temizliği ve otomatik başlatmayı **kapatma**. Geri
alma hiçbir sürümde kısıtlanmıyor; sürüm farkı kullanıcının sistemine bırakılan
izi artırmamalı.

`surum::testler::kisitlarda_zaman_alani_yok` demoya zaman sınırı/sayaç
girmesini engelliyor. CI zaten `cargo test --all-features` koşuyor, demo
yapılandırması da test ediliyor.

### Arayüz testleri (vitest)

`tasks.md`'de "bağımlılık kararı verilmedi" diye bekleyen madde kapandı:
vitest + jsdom + Testing Library eklendi, yapılandırma `vite.config.ts`
içindeki `test` bloğunda (ayrı dosya değil — tek yapılandırma, tek yer).
CI'ya `npm test` adımı girdi.

10 test, iki dosya:

- `src/components/GunlukPaneli.test.tsx` — "geri al" düğmesi yalnızca defterde
  hâlâ duran kayıtlar için görünüyor; geri alınmış satır listeden silinmiyor
- `src/App.test.tsx` — profil düzeltmelerinin kullanıcıya gösterilmesi, demo
  ikilisinde ağ sekmesinin hiç görünmemesi, kısıtlar okunamazsa tam sürüm gibi
  davranma, demo profil sınırında düğmenin gerekçesiyle kapanması

Testler backend'e dokunmuyor: `src/lib/api.ts` mock'lanıyor, örnek veriler
`src/test/ornekler.ts`'de.

### Doğrulama

- `cargo test` ve `cargo test --features demo`: 149 test geçiyor
- `npm test`: 10 arayüz testi geçiyor
- `cargo clippy --all-targets -D warnings` iki yapılandırmada da temiz,
  `cargo fmt` uygulandı
- `npm run build` temiz (233 KB JS, gzip 72 KB)
- İkili gerçekten çalıştırıldı: `--tepside` ile açıldığında ana pencere gizli
  (Win32 `IsWindowVisible` ile doğrulandı), program tepside ayakta kalıyor
- **Elle doğrulanmadı**: tepsi menüsünün görünümü ve öğelerin tıklanması,
  bildirimlerin çıkması, demo ikilisinin arayüzü. `tasks.md` → "Sıradaki" 5.

## Oturum 1 — 31 Ağustos 2026

Proje sıfırdan kuruldu. Belge klasörü dışında hiçbir şey yoktu; oturum
sonunda derlenen, testleri geçen ve arayüzü çalışan bir uygulama var.

### Ticari model düzeltmesi

`PRODUCT_VISION.md` "ücretsiz/açık kaynak" seçeneğini açık bırakıyordu. Karar
verildi ve belgelere işlendi: **kapalı kaynak, tek seferlik ücretli**, Steam
birincil kanal, itch.io ikincil, GitHub yalnızca tanıtım + demo.

- `docs/DISTRIBUTION.md` yazıldı (yeni): kanallar, demo kapsamı, fiyat aralığı,
  lisans modeli, mağaza sayfası kontrol listesi, telemetri duruşu
- `PRODUCT_VISION.md` konumlandırma ve fiyatlandırma bölümleri yeniden yazıldı
- `DESIGN_PRINCIPLES.md` madde 2'ye "şeffaflık ≠ açık kaynak" ayrımı eklendi
- `PROFILES.md` profil formatının açık kalacağı notuyla güncellendi
- `ROADMAP.md`'ye M1–M5 yayın kilometre taşları eklendi
- `LICENSE.md` (EULA) ve `README.md` yazıldı — ikisi de kapalı kaynak dilinde

### Faz 1 — Sistem optimizasyonu (tamamlandı)

- `system_boost/detect.rs` — öndeki pencere, süreç listesi, tam ekran sezgisi,
  dokunulmaz süreç listesi
- `system_boost/priority.rs` — öncelik + affinite, hibrit CPU topolojisi
  (P-core maskesi). `REALTIME` tip seviyesinde imkânsız (karar #4)
- `system_boost/suspend.rs` — `NtSuspendProcess`/`NtResumeProcess`, çalışma
  anında aranıyor; bulunamazsa özellik kapanıyor, program açılıyor
- `system_boost/power.rs` — güç planı oku/değiştir/geri yükle
- `system_boost/startup.rs` — Windows ile başlatma (HKCU, yönetici gerekmiyor)
- `ledger.rs` + `revert.rs` — diske yazılan geri alma defteri, açılışta
  bekleyenlerin temizlenmesi (karar #3)
- `profile_engine/` — profil şeması, doğrulama, disk deposu, mod seçimi

### Faz 2 — Ağ optimizasyonu (4/5 tamamlandı)

- `network_boost/dns.rs` — elle kurulan DNS paketiyle çözümleyici
  karşılaştırması, sistemin kendi DNS'i dahil
- `network_boost/latency.rs` — ICMP gecikme/jitter ölçümü, TTL tabanlı yol
  testi, en büyük sıçrama tespiti
- `network_boost/tcp.rs` — Nagle kapatma (arayüz başına), ağ kısıtlaması
  kaldırma; hepsi geri alınabilir
- `network_boost/qos.rs` — DSCP 46 ilkesi, yalnızca `Muifly-` ön ekli
  ilkelere dokunuyor

**Eksik kalan**: DNS'in otomatik uygulanması. Ölçüm ve öneri var, değiştirme
yok — gerekçe karar #6'da. Bu bilinçli bir sınır, yarım kalmış iş değil.

### Arayüz

Mui tasarım dili (Outfit, teal #2dd4bf, koyu zemin #0f1115) `src/styles.css`'e
taşındı; jeton isimleri Muiget/Muivly ile hizalı. Beş ekran: Durum, Profiller,
Ağ, Günlük, Ayarlar. Grafik bağımlılıksız SVG.

### Sayılar

- Rust: 133 birim testi geçiyor, `cargo check` ve `cargo build` temiz
- Frontend: `tsc` temiz, `vite build` 232 KB JS (gzip 72 KB)
- Toplam yeni dosya: 19 Rust, 13 TypeScript/CSS, 5 belge

### Sıradaki oturum için

`docs/tasks.md` → "Sıradaki" bölümü. En kritik iki madde kod işi değil:
gerçek oyunlarla saha testi ve kod imzalama sertifikası.

### Oturum sonu düzeltmeleri

- `tauri-plugin-notification` Rust (2.3.3) ve npm (2.4.0) sürümleri
  uyuşmuyordu; `cargo update` ile eşitlendi. Tauri bunu açılışta uyarı olarak
  bildiriyordu.
- Clippy `-D warnings` altında temiz; `cargo fmt` uygulandı. CI iş akışı
  (`.github/workflows/ci.yml`) ikisini de kontrol ediyor.
- `site/index.html` içindeki depo bağlantıları tam URL'ye çevrildi
  (`heraklessii/Muifly` varsayıldı — depo adı farklıysa düzeltilmeli).
- Uygulama gerçek pencerede açıldı: `Motor::baslat` çalıştı ve
  `%APPDATA%\Muifly\geri-alma-defteri.json` oluştu.

---

## Oturum 7 — 1 Eylül 2026 · Oyun kütüphanesi

**Soru**: "Oyunlara özel ayar için ücretsiz bir API bulabilir miyiz? En azından
oyun adlarını ve görsellerini çeksek? Exe adlarını elle yazmak yerine seçsek?"

**Cevap: API'ye gerek yok.** Aranan üç şey de zaten diskte duruyor. Bu
makinede doğrulandı:

- `appmanifest_431960.acf` → `"name" "Wallpaper Engine"`, `"installdir"`
- `appcache/librarycache/431960/library_600x900.jpg` → kapak görseli, 43 KB
- Epic manifesti → `DisplayName`, `InstallLocation`, `LaunchExecutable`

IGDB / RAWG / SteamGridDB elendi: hepsi anahtar istiyor ve dağıtılan bir
ikiliye gömülen anahtar çıkarılabilir bir sır. Gerekçenin tamamı karar #25'te.

### Eklenenler

- `library/vdf.rs` — Steam'in VDF/ACF biçimi için hoşgörülü çözümleyici
- `library/steam.rs` — `libraryfolders.vdf` → `appmanifest_*.acf` → kapak
- `library/epic.rs` — `Manifests/*.item`, `AppCategories` ile eleme
- `library/exe.rs` — kurulum klasöründen aday exe çıkarma: gürültü elemesi +
  puanlama, genişlik öncelikli yürüyüş, 800 klasörlük bütçe
- `library/ikon.rs` — `PrivateExtractIconsW` → GDI → RGBA
- `library/png.rs` — sıkıştırmasız PNG yazıcı (zlib "stored" blokları)
- `profile_engine/katalog.rs` + `katalog.json` — 53 oyun; exe → ad + rekabetçi
- `KutuphaneDiyalogu.tsx` — kapak ızgarası + iki adımlı onay
- Profil penceresinde exe alanı: tanınan çalışan oyunlar, çalışan süreç
  listesi, `.exe` dosya seçici

### Gerçek makinede doğrulandı

Tarama üç oyun buldu: Wallpaper Engine (Steam, gerçek kapak görseliyle),
League of Legends ve NTE (Epic, exe ikonuyla). İki bulgu koda döndü:

1. Epic'in `LaunchExecutable` alanı NTE'de `NTEGlobalLauncher.exe` diyor ama
   oyunun kendisi `NTEGlobalGame.exe`. Başlatıcıyı listenin başına koymak
   yanlıştı — mod algılaması öndeki tam ekran pencerenin sahibine bakıyor.
   Epic'in bildirdiği exe artık listede eksikse **sona** ekleniyor.
2. `crashclientreporter.exe` gürültü elemesinden kaçıyordu; eleme kalıbı
   `crashreport` idi. `reporter` eklendi. `crash` tek başına eklenmedi:
   adında geçen gerçek oyunlar var.

PNG boru hattı `notepad.exe` ikonuyla uçtan uca denendi ve görsel olarak
doğrulandı (saydamlık, kanal sırası, satır yönü doğru).

### Sayılar

- Rust: 230 test (önceki 180), clippy `-D warnings` temiz, `cargo fmt` uygulandı
- Arayüz: 41 test (önceki 34), `tsc` temiz, 265 KB JS (gzip 80 KB)
- Yeni bağımlılık: **sıfır**

### Kararlar

- **#25** — Oyun kütüphanesi yerelden okunuyor, hiçbir API'ye gidilmiyor
- **#26** — Katalog oyunun adını söylüyor, ayarını değil

### GitHub kurulumu (aynı oturum)

Klasör bu noktaya kadar sürüm kontrolü altında değildi. Kurulan düzen
`DISTRIBUTION.md`'nin tarif ettiği ayrım:

| Depo | Görünürlük | İçerik | Durum |
|---|---|---|---|
| `heraklessii/Muifly-dev` | private | kaynak, belgeler, araçlar | CI yeşil |
| `heraklessii/Muifly` | public | site, README, EULA | Pages canlı |

Public deponun ağacı push sonrası **uzaktan** doğrulandı: `src/`,
`src-tauri/`, `docs/`, `arac/` ve `CLAUDE.md` yok. Sekiz dosya var, hepsi
izin listesinden geliyor.

- `heraklessii.github.io/Muifly` yayında. Pages "GitHub Actions" kaynağıyla
  açıldı; `pages.yml` ilk denemede kırmızı dönmüştü çünkü Pages henüz
  açılmamıştı (`configure-pages` "Get Pages site failed" veriyor).
- Sayfadaki **"Demoyu indir" düğmesi şu an boş bir Releases sayfasına
  gidiyor** — demo ikilisi M3'ü (kod imzalama) bekliyor. Bilinerek böyle
  bırakıldı; "Steam'de yakında" ve "itch.io'da yakında" düğmeleri zaten
  doğru dili kullanıyor.
- CI (`ci.yml`) Windows runner'da tam koştu: arayüz derlemesi, arayüz
  testleri, Rust testleri, clippy, biçim — hepsi geçti.

**Açık kalan iki bakım maddesi**, `tasks.md`'de.
