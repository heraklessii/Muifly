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

### 3. Demo ikilisinin elle doğrulanması

Kısıtlar kodda ve testte var (`cargo test --features demo`), ama demo ikilisi
henüz elle çalıştırılıp gözden geçirilmedi:

- [ ] Ağ sekmesi hiç görünmüyor mu
- [ ] İkinci profil eklenmeye çalışılınca ne yazıyor
- [ ] Windows ile başlat anahtarı kapalı ve gerekçesi okunuyor mu
- [ ] Tam sürümde açılmış otomatik başlatma, demo ikilisinde **kapatılabiliyor**
      mu (geri alma hiçbir sürümde kilitli değil)

### 4. Kütüphane ekranının gerçek pencerede denenmesi

Mantık ve CSS ayrı ayrı doğrulandı (Rust testleri gerçek Steam/Epic
kurulumuyla koştu, stiller tarayıcıda tek dosya olarak gözden geçirildi) ama
ekran `npm run tauri dev` ile açılmış bir pencerede bir kez denenmedi:

- [ ] Çok oyunlu bir kütüphanede tarama ne kadar sürüyor
- [ ] Görseller kaydırdıkça mı yükleniyor (IntersectionObserver yolu)
- [ ] `.exe dosyası seç` penceresi süzgeci ve iptali
- [ ] Kapağı olmayan oyunlarda ikon mu, baş harf mi çıkıyor

## Değerlendirilecek

- **ETW tabanlı FPS ölçümü** (karar #14). Faz 2'nin son büyük parçası.
  PresentMon'un yaklaşımı araştırılmalı; hook gerektirmiyorsa tasarım ilkesi
  3'e uygun.
- **Ekran çevirisi (Faz 5)** — karar #22, `ROADMAP.md` → Faz 5. Kapsam ve
  sınırlar yazıldı, kod yazılmadı. Başlamadan önce iki fizibilite sorusu
  cevaplanmalı (OCR gerçek oyun yazı tiplerini okuyor mu, yerel EN→TR kalitesi
  yeterli mi) ve Faz 1 sahada doğrulanmış olmalı. "Muifly modülü mü ayrı ürün
  mü" sorusu bilinçli olarak açık bırakıldı.
- **Tarayıcıda arayüz önizlemesi.** Oturum 5'te arayüzü gözle doğrulamak için
  geçici bir sahte backend yazıldı (`window.__TAURI_INTERNALS__.invoke`
  taklidi) ve iş bitince silindi. Kalıcı hale getirilirse `npm run dev` Rust
  derlemeden çalışan bir arayüz verir; bedeli, komut yüzeyiyle senkron
  tutulması gereken ikinci bir dosya. Arayüzde çok çalışılacaksa değer,
  yoksa borç.
- **Katalogun büyütülmesi.** Şu an 53 oyun (`src-tauri/katalog.json`).
  Kapsam arttıkça değeri artıyor ve riski yok: eşleşmeyen satır sessizce
  atlanıyor. Hazır ayar EKLENMEMELİ — karar #26.
- **Xbox / Microsoft Store oyunları.** Paketli uygulamalar `WindowsApps`
  altında ve ACL korumalı; manifest okumak Steam/Epic kadar basit değil.
  Şimdilik çıkış yolu `.exe dosyası seç`.
- **İçe aktarma önizlemesinin elle denenmesi**: akış testlerle korunuyor ama
  gerçek bir dosya seçme penceresiyle bir kez denenmedi (dosya süzgeci, iptal,
  yazma izni olmayan klasör).

- **GitHub Actions Node 20 uyarısı.** `actions/checkout@v4`,
  `setup-node@v4`, `configure-pages@v5`, `deploy-pages@v4` ve
  `upload-artifact@v4` Node 20 hedefliyor; runner onları zorla Node 24'te
  koşturuyor ve her çalışmada uyarı basıyor. Şimdilik çalışıyor; v5
  sürümleri çıktıkça yükseltilecek.
- **Site'taki "Demoyu indir" düğmesi boş Releases'e gidiyor.** Demo
  ikilisi M3'ü (kod imzalama) bekliyor. Ya demo yayınlanacak ya düğme
  "yakında" diline çevrilecek — diğer iki düğme zaten öyle.

## Ertelendi (gerekçesiyle)

- **Servis geciktirme** — karar #12
- **DNS otomatik uygulama** — karar #6
- **Statik route ekleme** — karar #7
- **Bellek/standby list temizleme** — karar #16
- **Faz 3 (ölçekleme) ve Faz 4 (kare üretimi)** — faz disiplini gereği Faz 1
  ve 2 sahada doğrulanmadan başlanmıyor

## Tamamlandı

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
