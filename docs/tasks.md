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

### 5. Kare ölçümünün gerçek oyunda doğrulanması

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

### 6. Oturum geçmişinin elle denenmesi

Kod ve testler tamam (karar #31); ekran gerçek bir pencerede bir kez
görülmedi ve asıl sınavı zamanla ortaya çıkanlar:

- [ ] Sekme `npm run tauri dev` ile açılmış bir pencerede bir kez gezildi mi
- [ ] Bir oyun açılıp kapandığında kayıt gerçekten düşüyor mu, süre doğru mu
- [ ] Kapasite (200) dolduğunda en eski kayıt düşüyor mu — dosyayı elle
      şişirerek denenebilir
- [ ] Dışa aktarma penceresi ve yazılan dosyanın okunabilirliği
- [ ] Ayar kapatılınca yeni kayıt yazılmıyor, var olanlar duruyor mu

### 7. Ölçeklemenin gerçek bir oyunda denenmesi (Faz 3)

Kod, testler ve arayüz tamam (karar #32). Boru hattı bu makinede bir kez
uçtan uca koştu (`cargo test gercek_ekranda_bir_tur -- --ignored
--nocapture`): yakalama açıldı, gölgelendirici derlendi, kareler çizildi.
Ama **görüntünün doğru göründüğünü ancak göz söyler**. Sırayla:

- [x] Pencere açılıyor, gölgelendirici derleniyor, kare ölçülüyor
      (`gercek_ekranda_bir_tur`)
- [ ] **Kaçış kısayolu gerçekten çalışıyor mu** (karar #34). Ölçekleme
      açıkken `Ctrl+Alt+Shift+S` — pencere kapanmalı, günlükte "ölçekleme
      durduruldu (kaçış kısayolu)" satırı görünmeli. Oyun fareyi
      yakalamışken de denenmeli: bu kısayolun asıl işi, başka hiçbir şey
      çalışmadığı andır.
- [ ] **Gizlenme kuralı** (karar #34). Masaüstündeyken "Başlat" → ekranda
      hiçbir şey değişmemeli, panelde "ekranda henüz bir şey yok" şeridi
      çıkmalı. Oyuna geçilince pencere gelmeli; Muifly'a dönülünce
      kaybolup arayüz görünmeli; oyuna dönülünce kaldığı yerden sürmeli.
- [ ] **Görüntü doğru mu.** `npm run tauri dev` → Ölçekleme sekmesi →
      "Yakalamayı dene" → "Başlat". Düşük çözünürlüklü, **pencereli** bir
      oyun/uygulama açıkken denenmeli: ölçeklenmiş görüntü ekranı kaplamalı
      ve kaynağından büyük görünmeli.
- [ ] **Kendini yakalama gerçekten kesildi mi.** `WDA_EXCLUDEFROMCAPTURE`
      bu makinede hata vermedi ama etkisi gözle doğrulanmadı: ekranda
      birbirinin içine giren bir tünel görünüyorsa çalışmıyor demektir.
- [ ] **Odak ve tıklama geçişi.** Ölçekleme açıkken öndeki pencere hâlâ oyun
      mu (`WS_EX_NOACTIVATE`), tıklamalar oyuna gidiyor mu
      (`WS_EX_TRANSPARENT`), Alt+Tab listesinde bizim pencere görünüyor mu
      (görünmemeli).
- [ ] **Dört algoritmanın görsel karşılaştırması.** Aynı sahnede sırayla
      denenip ekran görüntüsü alınmalı. CPU referansıyla aynı çıktıyı
      verdikleri **varsayım**; sabit testleri satır satır eşitliği
      kanıtlamıyor (karar #32).
- [ ] **Münhasır tam ekran yolu.** Oyun exclusive tam ekrandayken hata
      metni gerçekten çıkıyor mu ve kenarlıksız moda geçince düzeliyor mu.
- [ ] **Ekran modu değişimi.** Oyun açılırken çözünürlük değişiyor;
      `ErisimKesildi` sonrası yeniden açma yolu bir kez görülmedi.
      Yol karar #36'da yeniden yazıldı (eskisi ekranı siyah bırakıyordu):
      çözünürlük değişince görüntü kaldığı yerden sürmeli, ölçekleme
      durmamalı, günlükte "boru hattı yeniden kuruldu" satırı görünmeli.
- [ ] **Çok ekranlı kurulum.** İkinci ekran seçilince pencere doğru ekranda
      mı açılıyor (masaüstü kökeni (0,0) değil).
- [ ] **Ölçülen gecikmenin makul olup olmadığı.** "Sunum" satırı dikey
      eşitleme beklemesini içeriyor; sayı ekran yenileme aralığına
      yakınsa beklenen, çok üstündeyse bakılmalı.
- [ ] **Oyun kapanınca pencere kapanıyor mu.** `oturumu_kapat` ölçeklemeyi
      durduruyor; ekranda kalan siyah bir pencere en görünür hata olurdu.

### 8. Kare üretiminin gerçek bir oyunda denenmesi (Faz 4a)

Kod ve testler tamam (karar #35). Hareket tahmini sentetik gerçek-referansa
karşı doğrulandı ve gölgelendirici derleniyor. Ama **görüntünün doğru
göründüğünü ancak göz söyler** — birim testleri yanlış bir vektörün ekranda
nasıl durduğunu göremez. Sırayla:

- [x] Hareket tahmini bilinen kaydırmayı buluyor (`hareket::testler`)
- [x] Gölgelendirici derleniyor (`golgelendirici_derleniyor`)
- [x] Gölgelendirici sabitleri CPU referansıyla aynı
      (`sabitler_referansla_ayni`)
- [ ] **Ara kare makul görünüyor mu.** Ölçekleme çalışırken "Kare
      üretimini aç". Hareketli bir sahnede, üretilen karede yırtılma,
      hayalet iz ya da 16 pikselde bir basamak **olmamalı**. Basamak
      görünüyorsa ızgara örneklemesi, hayalet iz görünüyorsa örtüşme eşiği
      bakılacak.
- [ ] **Açıp kapatmak ekranı karartmıyor mu.** Anahtar çalışırken
      değiştirilebilmeli; farkın aynı sahnede görülebilmesinin tek yolu bu.
      Karar #36'dan sonra ilk ara kare bir tur gecikiyor (ısınma):
      anahtarı açtığın anda **tek karelik bir hayalet iz olmamalı**.
- [ ] **Hızlı kamera hareketi.** ±24 pikselden hızlı hareket sınırın
      dışında; orada üretilen kare "karışım yerine en yakın gerçek kare"ye
      düşmeli, bozulmamalı.
- [ ] **Arayüz katmanı ve yazı.** Oyun arayüzü (HUD, menü) sabit durur
      ama arkası hareket eder; kare üretiminin klasik zayıf noktası burası.
      Yazının titreyip titremediğine bakılacak.
- [ ] **Ölçülen bedel makul mü.** "Kare üretimi (hesap süresi)" satırı.
      Ekran yenileme aralığına yakınsa boru hattı sığmıyor demektir.
      "Kaynağı bekleme" satırı ayrı okunmalı: o süre bedele dahil değil
      (karar #36) ve kaynağın kare hızını gösteriyor.
- [ ] **Yenileme hızı uyarısı.** 60 Hz bir ekranda uyarı çıkıyor mu;
      yüksek yenilemeli ekranda çıkmıyor mu.
- [ ] **Rekabetçi mod kapısı.** Rekabetçi moda geçilince kare üretimi
      gerçekten kapanıyor mu (iki kapı da: profil ve çalışma zamanı).
- [ ] **Kusur listesi.** Görülen her kusur yazılacak — `FRAME_GENERATION.md`
      §5'e göre Faz 4b'nin (ML) açılma koşulu bu listenin varlığı.

### 9. Ekran çevirisinin gerçek bir oyunda denenmesi (Faz 5)

Kod, testler ve arayüz tamam (karar #37). Model bu makinede yüklendi,
çeviri kalitesi ölçüldü ve üç zaafın üçü de beklenen davranışı gösterdi.
Ama **hiçbiri gerçek bir oyunda denenmedi** ve bu özelliğin asıl sınavı
orada. Sırayla:

- [x] Model yükleniyor ve makul Türkçe üretiyor
      (`ceviri::cevirici::testler::gercek_modelle_uctan_uca`)
- [x] Boru hattının tamamı sentetik metinle koşuyor
      (`gercek_modelle_akis`) — ön işleme, bölme, sözlük, bellek
- [x] Terim işaretinin modelden sağ çıktığı ölçüldü (`isaret_adaylari`)
- [ ] **Kısayol gerçekten çalışıyor mu.** Çeviri açıkken
      `Ctrl+Alt+T`. Asıl sınav oyun fareyi yakalamışken: kısayolun işi tam
      da başka hiçbir şeyin çalışmadığı andır. Kombinasyon başka bir
      uygulamada kayıtlıysa sıradaki adaya düşmeli ve arayüzde **hangisinin**
      alındığı yazmalı.
- [ ] **Overlay gerçekten görünüyor mu.** Kenarlıksız pencere modunda bir
      oyun açıkken kısayola bas: pencere ekranın altında belirmeli, oyunun
      üstünde durmalı, kapatma düğmesi tıklanabilmeli. Münhasır tam ekranda
      **görünmemesi beklenen** davranış — arayüz bunu baştan söylüyor,
      doğrulanmalı.
- [ ] **Alan seçici.** Profiller ekranından bir profil için "Alanı seç":
      donmuş görüntü gelmeli, dikdörtgen çizilebilmeli, Esc kapatmalı.
      Kaydedilen alan profil JSON'ında oran olarak görünmeli.
- [ ] **Gerçek oyun metniyle OCR.** Karar #28'in külliyatı sentetikti ve bu
      bilinen bir sınır. Gerçek bir oyunun altyazısında okuma ne kadar
      doğru, `onisleme`nin uyarıları ne sıklıkta yanlış alarm veriyor?
      Eşikler sentetik örneklere göre seçildi.
- [ ] **Duran ekran sorunu.** Masaüstü çoğaltması yalnızca DEĞİŞİKLİK
      veriyor. Oyunlar sürekli çizdiği için sorun beklenmiyor ama duraklatılmış
      bir oyunda "ekrandan yeni bir kare gelmedi" hatası çıkabilir; çıkarsa
      metin kullanıcıya ne yapacağını söylüyor mu?
- [ ] **Model indirmesi uçtan uca.** Bu oturumda dosyalar `curl` ile
      indirildi; ürünün kendi WinHTTP yolu (`ceviri::indirme`) **hiç
      çalıştırılmadı**. İlerleme çubuğu, iptal ve yarım kalan indirmenin
      `.yarim` dosyası bırakmadığı görülmeli.
- [ ] **Vekil sunucu / kurumsal ağ.** WinHTTP otomatik vekil ayarını
      kullanıyor ama bu makinede vekil yok; denenmedi.
- [ ] **Çeviri sırasında oyunun takılıp takılmadığı.** Çıkarım iki
      çekirdekle sınırlı ve istek tek seferlik; yine de kare süresine etkisi
      `monitor::olcum` ile ölçülebilir. Kabul kriteri "çeviri isteği oyunun
      akışını kesmiyor" ancak böyle doğrulanır.
- [ ] **Kusur listesi.** Görülen her kusur yazılacak — hem OCR hem çeviri
      tarafında. Karar #29'un üç zaafı bu listeye göre yeniden okunmalı.

## Değerlendirilecek

- **Ölçekleme demo ikilisinde açık kalsın mı?** Şu an açık: `surum.rs`'teki
  kısıtlar listesine eklenmedi, çünkü demo kapsamı bir ürün kararı ve
  `DISTRIBUTION.md`'de yazılı. Soru gerçek: ölçekleme, ürünün üç ana
  modülünden biri ve demoda tam açık olması "eksiksiz ama dar" dengesini
  değiştirebilir. Karar verilirse hem `surum.rs`'e hem `DISTRIBUTION.md`'ye
  hem de o kararı koruyan bir teste yazılmalı.
- **Ölçeklemenin profil dosyasından otomatik açılması denenmedi.**
  `scaling.enabled: true` olan bir profil uygulandığında ölçekleme
  başlıyor (`state::profil_uygula` 7. adım) ve bu yol testlerle değil
  yalnızca kodla duruyor: gerçek bir profille bir kez koşturulmalı.
- **İkinci GPU'ya boşaltma (iGPU offload).** `MODULES.md`'de "nice to have"
  olarak duruyor. Yakalama artık ekranı süren adaptörü buluyor; ölçeklemeyi
  başka bir adaptöre taşımak ayrı bir paylaşımlı doku işi ve ölçülmeden
  girilmemeli.
- ~~Ekran çevirisi (Faz 5)~~ **Yazıldı** (karar #37). "Muifly modülü mü ayrı
  ürün mü" sorusu da kapandı: Muifly'ın varsayılan kapalı bir modülü. Kalan
  iş kod değil, saha denemesi — Sıradaki 9.
- **Çeviri demo ikilisinde açık kalsın mı?** Ölçeklemeyle aynı soru, aynı
  durum: `surum.rs`teki kısıt listesine eklenmedi. Çevirinin lehine bir
  ayrıntı var — modeli indirmek zaten kullanıcının açık bir adımı, yani
  demoda "açık" olması otomatik bir bedel getirmiyor. Karar verilirse hem
  `surum.rs`e hem `DISTRIBUTION.md`ye hem de o kararı koruyan bir teste
  yazılmalı.
- **ONNX Runtime statik bağlı ve ikili 8,69 → 31,81 MiB büyüdü** (karar #37).
  Kabul edilen bir bedel ama küçük değil ve `Cargo.toml`'daki "ikili boyutu
  önemli" cümlesi hâlâ duruyor. Alternatif `load-dynamic`ti: DLL de model
  gibi çalışma zamanında inerdi. Reddedilme gerekçesi kararda (imzalama,
  arşiv açma, indirilen şeyin veri değil kod olması). Yeniden bakılacaksa
  ölçülecek şey şu: kurulum boyutunun satın almaya etkisi mi büyük, imzasız
  bir DLL'in SmartScreen riski mi?
- ~~`sozluk`'ün işaretinin modelden sağ çıktığının ÖLÇÜLMESİ.~~ **Ölçüldü ve
  varsayım çürüdü** (karar #37): `[[0]]` çıktıda `[0]` oluyordu, yani her
  terim kayıp sayılıyordu. On beş aday sınandı, biçim `#0#` oldu. Ölçüm
  testi `--ignored` olarak duruyor (`isaret_adaylari`) — model ya da
  tokenizer değişirse aynı soru yeniden sorulmalı.
- **OCR'ın gerçek ekran görüntüleriyle tekrar ölçülmesi.** Karar #28'in
  külliyatı sentetik; bu makinede hiç oyun ekran görüntüsü yoktu. Yakalama
  tarafı artık açık (karar #37), yani bu artık bir engel değil bir iş —
  Sıradaki 9'a bağlı. `onisleme`'nin sezgisel uyarıları
  (bitişik kelime, noktalama şüphesi) da o külliyatta yanlış alarm oranıyla
  birlikte ölçülmeli — eşikler şu an sentetik örneklere göre seçildi.
- **Tarayıcıda arayüz önizlemesi.** Oturum 5'te arayüzü gözle doğrulamak için
  geçici bir sahte backend yazıldı (`window.__TAURI_INTERNALS__.invoke`
  taklidi) ve iş bitince silindi. Kalıcı hale getirilirse `npm run dev` Rust
  derlemeden çalışan bir arayüz verir; bedeli, komut yüzeyiyle senkron
  tutulması gereken ikinci bir dosya. Arayüzde çok çalışılacaksa değer,
  yoksa borç.
- **OCR'a giden kesitin büyütülmesi ölçülmedi.** Küçük punto yazıda OCR'ı
  büyütülmüş bir görüntüyle beslemek doğruluğu artırabilir. Şu an kesit
  olduğu gibi gidiyor ve bu **bilinçli**: karar #28 ölçümünü 1:1 görüntüyle
  yaptı, ölçülmemiş bir dönüşüm eklemek doğruluğu artırdığı kadar
  azaltabilir. Önce gerçek ekran görüntüleriyle ölçülmeli.
- **Model boştayken düşüyor ama ne kadar bellek bıraktığı ölçülmedi.**
  Varsayılan beş dakika (`ceviri_bosta_dusur_sn`). Yüklüyken tutulan
  bellek bu makinede ölçülmedi; `RISKS.md`'deki "yüzlerce MB" tahmini hâlâ
  tahmin.

- **`site/` tanıtım sayfası iki fazdır geride.** Sayfa hâlâ "Üç şey yapar"
  diyor ve Sistem / Ağ / Şeffaflık listeliyor; ölçekleme (Faz 3), kare
  üretimi (Faz 4a) ve ekran çevirisi (Faz 5) hiç geçmiyor. Bu **şimdilik
  doğru bir eksiklik**: üçü de sahada denenmedi ve tanıtım sayfasına
  yazılan her satır bir vaattir (ilke 4). Saha denemeleri bitince üçü
  birden eklenmeli — ekran çevirisi için üç sınırıyla birlikte (indirme
  boyutu, tek yönlü dil çifti, münhasır tam ekranda görünmemesi).
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
- **Site'taki demo düğmesi "Demo yakında" oldu** (1 Eylül 2026). Demo
  ikilisi M3'ü (kod imzalama) beklediği için düğme boş bir Releases
  sayfasına gidiyordu. Demo yayımlandığında düğme ile README ve site'taki
  gelecek zamanlı cümleler geri çevrilmeli.

## Ertelendi (gerekçesiyle)

- **Servis geciktirme** — karar #12
- **DNS otomatik uygulama** — karar #6
- **Statik route ekleme** — karar #7
- **Bellek/standby list temizleme** — karar #16
- **Faz 3 (ölçekleme) ve Faz 4 (kare üretimi)** — faz disiplini gereği Faz 1
  ve 2 sahada doğrulanmadan başlanmıyor

## Tamamlandı

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
