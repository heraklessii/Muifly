# Yapılacaklar

> `Sıradaki` bölümü her oturumun başında okunur. Bitenler `Tamamlandı`ya
> taşınır, silinmez — neyin ne zaman yapıldığı görünür kalsın.

## Sıradaki

### 1. Gerçek dünya doğrulaması (kod işi değil)

`ROADMAP.md` Faz 1 kabul kriteri: **en az 5-10 farklı oyunla manuel test**.
Bunu yapmadan "Faz 1 tamam" denemez. Her oyunda kontrol edilecekler:

- [ ] Oyun algılandı mı (tam ekran ve kenarlıksız ayrı ayrı)
- [ ] Öncelik gerçekten değişti mi (Görev Yöneticisi'nden doğrula)
- [ ] Dondurulan uygulamalar oyundan çıkınca kaldığı yerden devam etti mi
- [ ] Güç planı eski haline döndü mü
- [ ] Program zorla sonlandırıldığında, bir sonraki açılışta temizlik çalıştı mı

Son madde en önemlisi: geri alma defterinin (karar #3) asıl sınavı bu.

### 2. Kod imzalama sertifikası (kod işi değil)

`ROADMAP.md` M3'ün önkoşulu. İmzasız kurulumda SmartScreen uyarısı çıkıyor ve
performans aracı kategorisinde bu doğrudan "virüs mü" algısı yaratıyor.
Kodla çözülmüyor.

### 3. Kütüphane ekranının gerçek pencerede denenmesi

Mantık ve CSS ayrı ayrı doğrulandı (Rust testleri gerçek Steam/Epic
kurulumuyla koştu, stiller tarayıcıda tek dosya olarak gözden geçirildi) ama
ekran `npm run tauri dev` ile açılmış bir pencerede bir kez denenmedi:

- [ ] Çok oyunlu bir kütüphanede tarama ne kadar sürüyor
- [ ] Görseller kaydırdıkça mı yükleniyor (IntersectionObserver yolu)
- [ ] `.exe dosyası seç` penceresi süzgeci ve iptali
- [ ] Kapağı olmayan oyunlarda ikon mu, baş harf mi çıkıyor

### 4. Kare ölçümünün gerçek oyunda doğrulanması

Kod bitti — ETW oturumu, yükseltilmiş yardımcı (`muifly-olcum.exe`),
komutlar, sidecar paketlemesi ve arayüz ekranı. Kalanların hepsi **elle
deneme**, kod işi değil:

- [ ] **Gerçek bir oyunda doğrulama.** Sonda çalıştığında açık oyun yoktu.
      Ölçülen değer oyunun kendi FPS sayacıyla karşılaştırılmalı; tam ekran
      (exclusive) ve kenarlıksız ayrı ayrı denenmeli.
- [ ] **UAC akışının elle denenmesi.** Reddetme yolu ("hiçbir şey
      değişmedi") ve zaman aşımı yolu testlerle korunuyor ama gerçek bir
      istemle bir kez görülmedi.
- [x] **Paketlenmiş kurulumda yardımcının yanına düştüğü** — `tauri build`
      2 Eylül 2026'da koştu, üretilen `installer.nsi` yardımcıyı
      `$INSTDIR\muifly-olcum.exe` olarak ana ikilinin yanına yazıyor.
      Kurulum dosyası çalıştırılıp **kurulmadı**: dosyanın gerçekten oraya
      düştüğü ve yükseltilmiş olarak açıldığı hâlâ görülmedi.
- [ ] **Yardımcının imzalanması** — M3'ün kapsamı büyüdü, `ROADMAP.md`'de
      not düşüldü.

### 5. Oturum geçmişinin elle denenmesi

Kod ve testler tamam (karar #31); ekran gerçek bir pencerede bir kez
görülmedi ve asıl sınavı zamanla ortaya çıkanlar:

- [ ] Sekme `npm run tauri dev` ile açılmış bir pencerede bir kez gezildi mi
- [ ] Bir oyun açılıp kapandığında kayıt gerçekten düşüyor mu, süre doğru mu
- [ ] Kapasite (200) dolduğunda en eski kayıt düşüyor mu — dosyayı elle
      şişirerek denenebilir
- [ ] Dışa aktarma penceresi ve yazılan dosyanın okunabilirliği
- [ ] Ayar kapatılınca yeni kayıt yazılmıyor, var olanlar duruyor mu

## Değerlendirilecek

- **Tarayıcıda arayüz önizlemesi.** Oturum 5'te arayüzü gözle doğrulamak için
  geçici bir sahte backend yazıldı (`window.__TAURI_INTERNALS__.invoke`
  taklidi) ve iş bitince silindi. Kalıcı hale getirilirse `npm run dev` Rust
  derlemeden çalışan bir arayüz verir; bedeli, komut yüzeyiyle senkron
  tutulması gereken ikinci bir dosya. Arayüzde çok çalışılacaksa değer,
  yoksa borç.

- **"Rekabetçi Mod" artık hiçbir şey yapmıyor.** Koruduğu iki şey (kare
  üretimi, agresif ölçekleme) karar #39'la kaldırıldı; geriye mod adı,
  profildeki `competitive` bayrağı ve `katalog.json`daki etiket kaldı.
  Kullanıcıya davranışı değişmeyen bir mod göstermek şeffaflık ilkesiyle
  çelişiyor. İki yol var: (a) tamamen kaldırmak — `katalog.json`, mod
  numaralandırması ve oturum geçmişi kayıtlarına dokunur; (b) moda gerçek
  bir içerik vermek (örneğin daha dar bir dondurma listesi ya da ölçüm
  aralığını seyreltmek). Karar verilmeden önce hangisinin kullanıcıya ne
  söylediğine bakılmalı.
- **Katalogun büyütülmesi.** Şu an 142 oyun (`src-tauri/katalog.json`).
  Kapsam arttıkça değeri artıyor ve riski düşük: eşleşmeyen satır sessizce
  atlanıyor. Hazır ayar EKLENMEMELİ — karar #26. Yeni satır eklerken tek
  gerçek tehlike **fazla genel bir exe adı** (`game.exe`, `launcher.exe`):
  alakasız bir süreci oyun diye etiketler. `cok_genel_exe_adi_yok` testi
  bilinen genel adları tutuyor ama liste kapsamlı değil — eklerken exe adının
  o oyuna özel olduğundan emin ol.
- **Xbox / Microsoft Store oyunları.** Paketli uygulamalar `WindowsApps`
  altında ve ACL korumalı; manifest okumak Steam/Epic kadar basit değil.
  Şimdilik çıkış yolu `.exe dosyası seç`.
- **İçe aktarma önizlemesinin elle denenmesi**: akış testlerle korunuyor ama
  gerçek bir dosya seçme penceresiyle bir kez denenmedi (dosya süzgeci, iptal,
  yazma izni olmayan klasör).

- ~~Kurulum ölçüm yardımcısının iki kopyasını taşıyor.~~ **Çözüldü**
  (2 Eylül 2026) — ayrıntı `Tamamlandı` bölümünde.

- **GitHub Actions Node 20 uyarısı.** `actions/checkout@v4`,
  `setup-node@v4`, `configure-pages@v5`, `deploy-pages@v4` ve
  `upload-artifact@v4` Node 20 hedefliyor; runner onları zorla Node 24'te
  koşturuyor ve her çalışmada uyarı basıyor. Şimdilik çalışıyor; v5
  sürümleri çıktıkça yükseltilecek.
- **Kurulum paketi henüz yayınlanmadı.** README ve site Releases'e
  yönlendiriyor ama orada henüz bir şey yok; M3 (kod imzalama) bekliyor.

## Ertelendi (gerekçesiyle)

- **Servis geciktirme** — karar #12
- **DNS otomatik uygulama** — karar #6
- **Statik route ekleme** — karar #7
- **Bellek/standby list temizleme** — karar #16
- **Faz 3 (ölçekleme), Faz 4 (kare üretimi), Faz 5 (ekran çevirisi)** —
  yazıldılar ve **kaldırıldılar** (karar #39). Ertelenmiş değil, kapsam
  dışı: geri getirilecekse önce Faz 1 ve 2 sahada doğrulanmış olmalı

## Tamamlandı

- [x] **Ölçekleme, kare üretimi ve çeviri kaldırıldı; ürün Apache 2.0
      oldu** (7 Eylül 2026, karar #39). `scaling/` ve `ceviri/` modülleri,
      `surum.rs`, dört arayüz bileşeni, iki fizibilite sondası, vitrin
      betiği ve `FRAME_GENERATION.md` silindi; `ort` + `tokenizers`
      bağımlılıkları ve 12 `windows` özelliği düştü. EULA yerine Apache
      2.0, demo/tam sürüm ayrımı kalktı. `cargo test` 487 → 286,
      `npm test` 79 → 51; hiçbiri düşmedi.
- [x] **Denetim turu: bu makinede görünmeyen sekiz yol** (3 Eylül 2026,
      karar #38). Tur başlarken `cargo test` yeşil, clippy sıfır uyarıydı;
      sekizinin hiçbiri o iki aracın baktığı yerde değildi:
      (1) kare ölçümü, kullanıcı adında boşluk varsa hiç çalışmıyordu —
      yükseltilmiş yardımcının komut satırı kaçırılmıyordu;
      (2) o yardımcının yazdığı dosyanın adı tahmin edilebilirdi (yönetici
      yetkisiyle yazma yönlendirilebilirdi);
      (3) geri alma defteri yarım yazılabiliyordu — karar #3'ün mekanizması
      tam da en gerekli anda kendini kaybediyordu;
      (4) farklı kimlikli iki profil aynı dosyaya yazabiliyordu;
      (5) biriken çeviri istekleri sıraya giriyordu;
      (6) `GetMessageW`in hata dönüşü mesaj sayılıyor, bir çekirdek sonsuza
      kadar dönebiliyordu;
      (7) ilk CPU örneği "açılıştan bu yana ortalama"ydı;
      (8) iki model dosyası aynı geçici ada inebilirdi.
      Test 473 → 487. Hiçbiri saha denemesinin yerine geçmiyor.
- [x] **Kararlılık turu: sessiz bozulan dört yol** (2 Eylül 2026, karar #36).
      Hiçbiri kullanıcıya hata göstermiyordu:
      (1) ekran modu değişince ölçekleme siyah kalıyordu — `yeniden_ac`
      yeni bir D3D11 cihazı kuruyor, sunum penceresi eskisiyle çiziyordu;
      (2) kare üretimi anahtarı açıldığında ilk ara kare bayat bir
      piramitle hesaplanıyordu;
      (3) gecikme ölçümü kaynağın kare hızını bedele katıyordu — aynı
      sahnede 7,26 ms yerine 0,40 ms;
      (4) arka plan ICMP ölçümü Motor kilidini bir saniyeye kadar
      tutuyordu.
      Beş yeni test duruşu koruyor. Dördü de yalnızca gerçek bir oyunla
      görülebilecek yollar — Sıradaki 7 ve 8'e madde eklendi.
- [x] **MSI paketlemesini kıran yardımcı ikili çakışması** (2 Eylül 2026).
      `tauri build` paketin bütün cargo `bin` hedeflerini kuruluma koyuyordu;
      `externalBin` sidecar'ı da aynı dosyayı koyunca `muifly-olcum.exe` iki
      kez yazılıyordu. NSIS üstüne yazıp geçiyordu, WiX `ICE30` ile
      reddediyordu — `targets` listesinde `"msi"` yazdığı halde 0.3.0'a
      kadar **hiç MSI üretilmemişti**.

      Çözüm: yardımcı ikili `required-features = ["olcum-yardimcisi"]`
      arkasına alındı. Normal `cargo build` onu üretmiyor, dolayısıyla tauri
      de paketlemeye almıyor; kuruluma giren tek kopya sidecar'ın kendisi —
      yani imzalanması gereken dosya. Desteklenen yol (`externalBin`)
      korundu. `olcum::testler::yardimci_ikili_ozellik_arkasinda` düzeltmenin
      sessizce geri alınmasını engelliyor: sorun derleme zamanında değil,
      yalnızca paketleme gününde görünürdü.

      Yan etki: `cargo test` artık yardımcıyı derlemiyor. Derleme hatasının
      yayın gününe kalmaması için `cargo clippy --all-targets --features
      olcum-yardimcisi` komutu CLAUDE.md'ye eklendi.
- [x] Ticari model kararı ve belgelerin düzeltilmesi (31 Ağustos 2026)
- [x] Proje iskeleti: Tauri v2 + React + Rust
- [x] Faz 1: sistem optimizasyonu, geri alma defteri, profil motoru
- [x] Faz 2: DNS ölçümü, gecikme/jitter, yol testi, TCP, QoS (DNS uygulama
      hariç — karar #6)
- [x] Arayüz: beş ekran, Mui tasarım dili
- [x] `site/` tanıtım sayfası, `README.md`, `LICENSE.md` (EULA)
- [x] Sistem tepsisi: simge, menü (Göster / Öndekine uygula / Varsayılana dön /
      Çıkış), sol tık pencereyi getiriyor, ipucu modu gösteriyor
      (1 Eylül 2026)
- [x] "Kapatınca tepsiye in" ayarının gerçekten çalışması — ayar vardı,
      davranışı yoktu (1 Eylül 2026)
- [x] Çıkışta oturumluk değişikliklerin geri alınması — karar #19
      (1 Eylül 2026)
- [x] Demo/tam sürüm ayrımı: `--features demo`, `surum.rs`, komut ve arayüz
      kontrolleri — karar #20 (1 Eylül 2026)
- [x] Çok işlemci gruplu makinede yanlış P-core affinite maskesi düzeltildi
      (1 Eylül 2026)
- [x] Arayüz baştan yazıldı: kenar çubuğu + canlı mod kartı, kahraman kartı,
      mini eğriler, imleç okumalı grafik, profil kartları, ağ ölçüm çubukları,
      Ctrl+1..5 kısayolları (1 Eylül 2026, oturum 5)
- [x] Arayüz testleri: vitest + jsdom + Testing Library, 10 test; CI'da
      `npm test` adımı (1 Eylül 2026)
- [x] Muifly'a ait uygulama ikonu: `icons/kaynak.svg` tek kaynak, ikon seti
      ondan üretiliyor; Muiget'ten kopyalanan indirme oku gitti (1 Eylül 2026)
- [x] Profil içe/dışa aktarma: iki adımlı önizleme, çakışmada ezmeme,
      demo kısıtı, arayüz diyaloğu — karar #23 (1 Eylül 2026)
- [x] Mod değişiminde masaüstü bildirimi, varsayılanı kapalı ayarla —
      karar #24 (1 Eylül 2026)
- [x] Günlükte arama + kaç satırın gösterildiğinin yazılması; profil
      listesinde arama; profil satırında "Ne yapacak?" özeti
      (1 Eylül 2026)
- [x] GitHub: private geliştirme deposu (`Muifly-dev`) + public vitrin
      (`Muifly`), izin listeli `arac/vitrin-hazirla.mjs`, Pages canlı
      (1 Eylül 2026)
- [x] Oyun kütüphanesi: Steam/Epic manifestleri, kapak görselleri, exe
      adayları, gömülü katalog, kütüphane diyaloğu — kararlar #25 ve #26
      (1 Eylül 2026)
- [x] Üçüncü taraf lisans ekranı — EULA madde 8'in vaadi. `arac/` altındaki
      üreteç, ikiliye gömülen `ucuncu-taraf.json`, Ayarlar → Yasal ekranı;
      yazı tipinin eksik OFL metni de eklendi — karar #21 (1 Eylül 2026)
