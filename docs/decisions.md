# Mimari Kararlar

> ADR tarzı, kronolojik. Bir karar değişirse **silinmez**, altına "revize edildi"
> notu düşülür — kodda "neden böyle" sorusunun cevabı burada aranıyor ve
> silinmiş bir kararın izi kalmazsa aynı tartışma altı ay sonra baştan yapılır.
>
> Kod yorumları buraya numarayla atıf yapıyor (`docs/decisions.md #6` gibi).
> Numaraları değiştirme.

---

## #1 — Tauri v2 + React, Electron değil

**Karar**: Masaüstü kabuk Tauri v2, arayüz React + Vite + TypeScript.

**Neden**: Muifly, sürekli arka planda duran bir araç. Electron'un ~120 MB RAM
tabanı, "sistemini hafifleten araç" iddiasıyla doğrudan çelişirdi. Tauri
sistemin WebView'ini kullanıyor ve Rust çekirdeği zaten gerekli (Windows API
çağrıları için).

Ailedeki Muiget ve Muivly de aynı yığında; birikmiş deneyim aktarılıyor.

---

## #2 — Windows'a özel, çapraz platform soyutlama katmanı yok

**Karar**: Sistem çağrıları doğrudan `windows` crate'i üzerinden. Windows dışı
hedefler için yalnızca `Error::Unsupported` dönen ince stub'lar var.

**Neden**: Ürünün tamamı platform API'lerine dayanıyor —
`SetPriorityClass`, `NtSuspendProcess`, `PowerSetActiveScheme`, QoS Paket
Zamanlayıcı, Desktop Duplication. Bunların Linux/macOS karşılığı ya yok ya
tamamen farklı. Genel bir "PlatformBackend" trait'i yazmak, tek bir
implementasyonu olan bir soyutlama olurdu.

Stub'lar yine de duruyor: saf mantık (jitter hesabı, profil doğrulama, defter
serileştirme) Windows dışında da derlenip test edilebilsin diye.

---

## #3 — Geri alma defteri diske yazılıyor

**Karar**: Her sistem değişikliğinin eski değeri `%APPDATA%\Muifly\geri-alma-defteri.json`
dosyasına yazılıyor; program açılışında bekleyen oturum kayıtları geri alınıyor.

**Neden**: Tasarım ilkesi 1 (tersine çevrilebilirlik) yalnızca bellekte tutulan
bir listeyle sağlanamıyor. Program çökerse ya da kullanıcı Görev
Yöneticisi'nden sonlandırırsa, dondurulmuş süreçler donmuş, güç planı
değişmiş halde kalırdı — üstelik kullanıcı bunu geri alacak arayüze de
ulaşamazdı.

**Sonuç**: `ledger.rs` veri, `revert.rs` uygulayıcı. Ayrım şart: defter, geri
alma kodunu tanımadan serileştirilebiliyor ve bambaşka bir program oturumunda
okunabiliyor.

**Ek karar**: Bozuk bir defter dosyası programı açılmaz hale getirmiyor; dosya
`.bozuk` uzantısıyla saklanıp boş defterle devam ediliyor.

---

## #4 — `REALTIME_PRIORITY_CLASS` tip seviyesinde imkânsız

**Karar**: `Oncelik` enum'ında gerçek zamanlı varyantı **yok**. Profil dosyası
`"realtime"` yazsa bile ayrıştırma hata veriyor.

**Neden**: `docs/RISKS.md` bu sınıfın sistem servislerini (fare/klavye
sürücüleri dahil) aç bırakıp makineyi kilitleyebildiğini not ediyor. Bunu bir
`if` kontrolüyle engellemek yerine temsil edilemez kılmak, ileride yazılacak
yeni bir kod yolunun kontrolü atlamasını da imkânsız kılıyor.

`priority.rs` içinde bir test hiçbir varyantın `0x00000100` üretmediğini
doğruluyor.

---

## #5 — Sistem süreçleri sabit kodlanmış bir listeyle korunuyor

**Karar**: `detect::DOKUNULMAZ` listesindeki süreçler dondurulamıyor. Liste
kullanıcı tarafından düzenlenemiyor.

**Neden**: Düzenlenebilir bir "güvenli liste" olsaydı, paylaşılan bir profil onu
boşaltarak `csrss.exe` ya da `explorer.exe`'yi dondurabilir ve makineyi
kilitleyebilirdi. Profil formatı açık ve paylaşılabilir (#9); bu yüzden
güvenlik kontrolü profilin dışında olmak zorunda.

Kontrol iki katmanlı: profil doğrulaması listeyi temizliyor
(`schema::dogrula`), dondurma anında ayrıca kontrol ediliyor
(`suspend::dondurma_engeli`). Oyunun kendi PID'i de dondurulamıyor ve bu,
fonksiyon imzasında zorunlu bir parametre.

---

## #6 — DNS ölçülüyor ama değiştirilmiyor

**Karar**: Muifly çözümleyicileri gerçek DNS sorgularıyla karşılaştırıp en
hızlısını gösteriyor; sistem DNS ayarına dokunmuyor.

**Neden**: Adaptör seviyesinde DNS değiştirmenin güvenilir yolu `netsh`. Geri
alması ise adaptörün önceki durumunun (statik liste mi DHCP mi) doğru
okunmasına bağlı. Bu okuma yanlış olursa kullanıcı internetsiz kalıyor — ve
programın geri alma vaadi tam da en kötü anda tutmuyor oluyor.

Ölçüp önermek, yanlış uygulamaktan iyi. Kullanıcı sonucu görüp Windows'un
kendi ayarından değiştirebiliyor.

**Yeniden değerlendirme koşulu**: Registry'den (`Tcpip\Parameters\Interfaces\{GUID}\NameServer`)
okunan önceki durumun `netsh` sonucuyla birebir eşleştiği bir test yazılabilirse
bu karar tekrar açılabilir.

---

## #7 — Yol testi var, yol değiştirme yok

**Karar**: Traceroute benzeri ölçüm yapılıyor ve gecikmenin en çok arttığı
atlama gösteriliyor. Statik route eklenmiyor.

**Neden**: `route add` sistemin yönlendirme tablosunu değiştiriyor; yanlış bir
kayıt kullanıcıyı internetsiz bırakıyor. Ayrıca "daha iyi bir yol" seçmek
programın elinde değil — yolu ISS'nin BGP kararları belirliyor.

Ölçümün değeri şu: kullanıcı gecikmenin kendi modeminde mi yoksa ISS'nin
omurgasında mı biriktiğini görebiliyor. Bu, servis sağlayıcısıyla konuşurken
elindeki tek somut veri.

---

## #8 — Ölçüm ICMP `IcmpSendEcho` ile, ham soketle değil

**Karar**: Gecikme ölçümü `IcmpSendEcho` API'si üzerinden.

**Neden**: Ham soket (`SOCK_RAW`) Windows'ta yönetici yetkisi istiyor. Sürekli
açık duran ölçüm tarafının yükseltilmiş yetki gerektirmemesi tasarım ilkesi 5
(minimum ve açık admin yetkisi). `IcmpSendEcho` yetki istemiyor.

---

## #9 — Profiller ayrı JSON dosyaları, veritabanı değil

**Karar**: Her profil `%APPDATA%\Muifly\profiller\<kimlik>.json`.

**Neden**: Ürün kapalı kaynak (`DISTRIBUTION.md`) ama profil formatı açık.
Kullanıcı profilini metin editöründe açıp okuyabilmeli ve bir arkadaşına
gönderebilmeli. Tek dosyalık bir veritabanı bunu imkânsız kılardı.

Bozuk bir profil diğerlerini engellemiyor: okunamayan dosya atlanıp hata
listesine yazılıyor.

**Güvenlik notu**: Profil kimliği kullanıcı girdisi ve dosya adına dönüşüyor;
`store::dosya_adi` yol kaçışını (`..\..\`) engelliyor ve bunun bir testi var.

---

## #10 — Otomatik uygulama varsayılan KAPALI

**Karar**: `Ayarlar::otomatik_uygula` varsayılan `false`. Oyun algılansa bile
optimizasyon kendiliğinden uygulanmıyor.

**Neden**: Bir performans aracı, kurulduğu anda sistemi değiştirmeye
başlamamalı. Kullanıcı önce ne olduğunu görmeli, sonra istemeli. Rakiplerin
"kur ve unut" yaklaşımı, tam da güven kaybeden davranış.

Aynı mantıkla dondurma listesi de varsayılan boş: program tanımadığı bir
oyunda kullanıcının uygulamalarına dokunmuyor. Öneri listesi var, otomatik
seçim yok.

---

## #11 — Güç planı oluşturulmuyor, yalnızca değiştiriliyor

**Karar**: "Üstün performans" planı sistemde yoksa yüksek performansa
düşülüyor. `powercfg -duplicatescheme` ile plan üretilmiyor.

**Neden**: Plan üretmek sistemde kalıcı bir kayıt bırakıyor ve tam geri alma
(planı silmek), kullanıcının kendi oluşturduğu bir planı silme riskini
taşıyor. Geri alınamayacak bir değişikliği yapmaktansa hiç yapmamak doğru.

Düşüş **sessiz değil**: hangi planın gerçekten uygulandığı günlüğe yazılıyor.

---

## #12 — Servis geciktirme özelliği ertelendi

**Karar**: `ROADMAP.md` Faz 1'de geçen "startup servis gecikmesi" bu sürümde
yok. Arayüzde gri bir düğme de yok — özellik hiç görünmüyor.

**Neden**: Üç sorun var: (a) yanlış servisi geciktirmek (antivirüs, sürücü,
VPN) makineyi açılışta savunmasız bırakabilir; (b) hangi servisin "gereksiz"
olduğuna program karar veremez, kullanıcı da çoğunu tanımaz; (c) kullanıcı
arada servisi elle değiştirirse defterdeki eski değer artık doğru değil.

Sistem Açılışı modunun kalan parçaları (Muifly'ın kendisinin Windows ile
başlaması, açılışta güç planı) var.

---

## #13 — QoS ilkesi kalıcı kapsamda, oturumluk değil

**Karar**: QoS ilkesi oluşturulunca defterde `Kalici` kapsamda duruyor; oyun
kapanınca kaldırılmıyor.

**Neden**: İlke, Windows'un ilke yenilemesinde/oturum açılışında devreye
giriyor. Her oyun kapanışında silinip her açılışta yeniden yazılsa hiçbir
zaman etkin olmazdı. Kullanıcı isterse tek tıkla kaldırıyor.

**Ön ek kuralı**: Muifly yalnızca `Muifly-` ön ekli ilkeleri kaldırıyor;
kullanıcının ya da kurumsal bir grup ilkesinin oluşturduğu QoS ilkelerine
dokunmuyor.

---

## #14 — FPS ölçümü Faz 2'ye ertelendi, sahte değer gösterilmiyor

**Karar**: `monitor::fps_destegi_var()` `false` dönüyor ve arayüz FPS alanını
"henüz ölçülmüyor" olarak gösteriyor.

**Neden**: Hook'suz FPS ölçümünün doğru yolu ETW (Event Tracing for Windows) —
PresentMon'un kullandığı yaklaşım. Hook'lu alternatif tasarım ilkesi 3'e
takılıyor. ETW yolu araştırılana kadar boş bırakmak, tahmini bir sayı
göstermekten dürüst.

> **Revize edildi (karar #27).** ETW yolu araştırıldı ve açıldı, ama ölçülen
> bir kısıtla: gerçek zamanlı ETW oturumu yükseltilmiş yetki istiyor. Ölçüm
> bu yüzden sürekli değil, kullanıcının başlattığı süreli bir pencere.
> "Sahte değer gösterilmiyor" ilkesi aynen duruyor.

---

## #15 — Öncesi/sonrası tek bir "iyileşme oranı" üretmiyor

**Karar**: Karşılaştırma iki özeti yan yana gösteriyor; "%12 iyileşme" gibi bir
oran hesaplanmıyor.

**Neden**: Böyle bir oran, ölçüm koşullarının iki pencerede aynı olduğunu
varsayar — oyun içi yük asla aynı değil. Tek sayıya indirgemek, tasarım ilkesi
4'ün (sayısal vaat yok) arka kapıdan ihlali olurdu: kullanıcı onu bir vaat
gibi okur ve tutmadığında güven kaybeder.

Jitter için yalnızca **yön** söyleniyor ("düştü" / "arttı" / "değişmedi") ve
cümle bunun o oturuma özel bir ölçüm olduğunu açıkça yazıyor.

---

## #16 — Bellek/standby list temizleme yok

**Karar**: `system_boost::bellek_temizleme_destegi()` `false` ve bir testle
korunuyor.

**Neden**: "Bellek temizlendi, X MB boşaldı" ekranı rakiplerin en çok
kullandığı placebo göstergesi. Standby list temizlemenin ölçülebilir bir
faydası gösterilemiyor; temizleme anında yeniden yükleme maliyeti yüzünden
kısa bir yavaşlama üretiyor. Muifly'ın konumlandırması tam olarak buna karşı
(`PRODUCT_VISION.md`).

---

## #17 — Arayüz metinleri Rust tarafında

**Karar**: Ağ ayarlarının açıklamaları, "ne yapmaz" listesi ve benzeri ürün
metinleri Rust'ta sabit ve komutla arayüze veriliyor.

**Neden**: Tasarım ilkesi 4'e (sayısal vaat yok) karşı gözden geçirme tek bir
yerde yapılabiliyor ve **testle korunabiliyor** —
`network_boost::tcp::testler::aciklamalarda_sayisal_vaat_yok` metinlerde
yasaklı kalıpları arıyor. Metin TypeScript'e dağılsaydı bu kontrol mümkün
olmazdı.

---

## #18 — `parking_lot::Mutex`, standart `Mutex` değil

**Karar**: Motor kilidi `parking_lot::Mutex`.

**Neden**: Standart `Mutex`'in zehirlenme (poisoning) davranışı burada zarar
veriyor: bir panik sonrası motor erişilemez hale gelirdi ve kullanıcı **geri
alma arayüzüne de** ulaşamazdı. Panik anında en çok ihtiyaç duyulan şey tam
olarak o arayüz.

Aynı sebeple `Cargo.toml`'da `panic = "abort"` yok: bir optimizasyon panic
ederse defterin çalışabilmesi için unwind gerekiyor.

---

## #19 — Kapanış iki aşamalı: pencere kapanır, program oturumu geri alarak çıkar

**Karar**: Pencerenin kapatılması (varsayılan ayarla) programı sonlandırmıyor,
tepsiye indiriyor. Program gerçekten çıkarken — tepsi menüsünden "Çıkış",
ayar kapalıyken pencere kapatma ya da oturum kapanması — `RunEvent::Exit`
üzerinde **oturumluk** kayıtlar geri alınıyor. Kalıcı kayıtlar defterde
kalıyor.

**Neden**: Çıkışta geri alma yapılmadığında dondurulmuş bir uygulama, Muifly
bir daha açılıp `revert::acilista_temizle` çalışana kadar dondurulmuş kalıyordu
(karar #3'teki defter bunu ancak *sonraki* açılışta düzeltir). Açılış
temizliği çökme için son savunma; normal çıkışta ona ihtiyaç duyulmamalı.

Kalıcı kayıtların çıkışta geri alınmaması bilinçli: kullanıcı onları açıkça
istedi ve arayüzdeki "varsayılana dön" ile duruyorlar. Program her
kapandığında geri alınsalardı "kalıcı" sözü anlamsızlaşırdı.

---

## #20 — Demo/tam sürüm ayrımı derleme bayrağıyla, çalışma zamanı lisansıyla değil

**Karar**: Demo ayrı bir ikili: `cargo build --features demo`. Kapsam tek bir
yerde (`src-tauri/src/surum.rs` → `Kisitlar`), komutlar bu kısıtları kendi
kontrol ediyor; arayüz kapalı özelliğin sekmesini hiç göstermiyor.

**Neden**: Çalışma zamanı lisans kontrolü (anahtar doğrulama, sunucuya sorma)
üç şey getirirdi: ağa çıkan bir yol, saklanacak bir sır ve kullanıcının
göremediği bir davranış. Üçü de ürünün duruşuyla çelişiyor — telemetri yok
(`DISTRIBUTION.md`) ve şeffaflık ilkesi programın ne yaptığını gösteriyor
olmasını istiyor.

**Sınır özellik seviyesinde, zaman seviyesinde değil.** Demoda kalan gün,
deneme sayacı, kapanma ekranı yok; `surum::testler::kisitlarda_zaman_alani_yok`
bunu koruyor. Geri alma ve temizlik yolları demoda da tamamen açık: sürüm
farkı, kullanıcının sistemine bırakılan izi asla artırmamalı.

---

## #21 — Üçüncü taraf bildirimleri ikiliye gömülü, üreteci depoda

**Karar**: Bileşen listesi ve lisans metinleri `src-tauri/ucuncu-taraf.json`
dosyasında duruyor, `include_str!` ile ikiliye gömülüyor ve arayüzde
Ayarlar → Yasal altından okunuyor. Dosyayı `arac/ucuncu-taraf-uret.mjs`
üretiyor; `cargo-about` kullanılmıyor.

**Neden gömülü**: EULA madde 8 "uygulama içindeki bölüm"den söz ediyor.
Kurulum dizinine ayrı bir dosya konsa kullanıcı onu silebilir ya da bozabilir
ve uygulama yazılı bir sözü tutamaz hâle gelirdi. Gömülü olan silinemez.

**Neden kendi üretecimiz**: `cargo-about` doğru aracı ama kurulum + derleme
istiyor; buradaki veri zaten çevrimdışı elde edilebiliyor (`cargo tree`,
`cargo metadata`, `npm ls` ve kayıt önbelleğindeki LICENSE dosyaları). Aynı
sonucu ek bir araç bağımlılığı olmadan veriyorsa, bağımlılık taşımıyoruz.

**Kapsam ikiliye gireni içeriyor**: Rust tarafında `-e normal`, npm tarafında
`--omit=dev`. Test ya da derleme sırasında kullanılıp dağıtılmayan bir
kütüphaneyi listelemek, listeyi okunmaz yapmaktan başka işe yaramaz.

**Metni olmayan bileşen gizlenmiyor.** Bazı paketler (`webview2-com`,
`unic-*`, `selectors`, `alloc-stdlib`) yayımlanan sürümlerinde LICENSE
dosyası taşımıyor. Onları listeden çıkarmak listeyi temiz gösterirdi ama
eksik yapardı; SPDX kimliği ve kaynak adresiyle duruyorlar, eksik olan da
ekranda açıkça yazıyor. `cargo-about` da bu duvara çarpardı — sorun araçta
değil, paketlerde.

**Tazelik testle korunuyor**: `ucuncu_taraf::testler::listedeki_surumler_cargo_lock_ile_ayni`
listedeki her crate'i `Cargo.lock` ile karşılaştırıyor. Bir bağımlılık
yükseltilip üreteç çalıştırılmazsa CI kırmızıya döner — kullanıcıya yanlış
sürüm gösterilmesi bir belge hatası değil, lisans uyumu sorunudur.

**Yan etki**: Yazı tipinin OFL-1.1 metni depoda yoktu; fontu dağıtıp
lisansını dağıtmamak OFL'in kendi şartına aykırıydı.
`src/assets/fonts/LICENSE-OFL.txt` ve `site/fonts/LICENSE-OFL.txt` eklendi.

---

## #22 — Ekran çevirisi: isteğe bağlı, seçili alanda, çeviri belleğiyle

**Karar**: Gerçek zamanlı ekran çevirisi **Faz 5** olarak planlandı ve şimdilik
yazılmıyor. Yazıldığında aşağıdaki sınırlarla yazılacak. Bu maddeler tartışma
sonucu çıktı; kod yazılmadan önce kaybolmasınlar diye buraya kondu.

**Sürekli çeviri YOK, tuşa basınca seçili alan var.** Gerekçe performans değil,
**kalite**: hareket eden ekranda yazı yarı render edilmiş, fade animasyonu
içinde ya da motion blur altında yakalanır ve OCR çöp üretir. OCR'ın güvenilir
olduğu tek rejim duran bir diyalog kutusudur. Alan seçimi oyun profiline
kaydedilir (profil sistemi zaten var, karar #9).

**Kısayol `RegisterHotKey` ile, `WH_KEYBOARD_LL` ile değil.** İkisi dışarıdan
aynı görünür ama ikincisi bir kanca (hook) ve tasarım ilkesi 3'ü ihlal eder.
Yakalama `Windows.Graphics.Capture` / Desktop Duplication — Faz 3'ün risk
profilinin aynısı, yeni bir kategori açmıyor (`RISKS.md`).

**Overlay ayrı bir her-zaman-üstte pencere.** `RISKS.md` → "Overlay için
DirectX Hook İhtiyacı" bunu zaten karara bağlamıştı. Sonucu kabul ediyoruz:
kenarlıksız modda çalışır, **exclusive fullscreen'de çalışmaz**. Bu, mağaza
sayfasında ve özelliğin kendi ekranında baştan yazılacak bir sınır.

**Çeviri modeli ikiliye GİRMEZ.** Karar #1 Electron'u "~120 MB RAM tabanı,
'sistemini hafifleten araç' iddiasıyla çelişir" diye elemişti; yerel bir NMT
modeli (kabaca 100–300 MB disk, yüklüyken yüzlerce MB RAM) aynı argümanı bu
sefer bize karşı çalıştırır. Model isteğe bağlı indirilir, boştayken bellekten
düşer. OCR tarafında böyle bir bedel yok: `Windows.Media.Ocr` Windows 10+'ta
hazır ve `windows` crate'i zaten bağımlılığımız — ikiliye sıfır bayt ekliyor.

**Öğrenme = çeviri belleği + oyun sözlüğü. Model ince ayarı DEĞİL.**

İnce ayar üç sebeple reddedildi: (1) onayla/reddet ikili sinyali seq2seq
eğitimi için çok zayıf, doğrusunun ne olduğunu söylemiyor; (2) ürünün içinde
bir eğitim hattı taşımak gerekirdi ve gerilemeyi ölçmenin yolu olmazdı;
(3) zamanla kayan bir model, tasarım ilkesi 2'nin yasakladığı kara kutunun ta
kendisidir — kullanıcı çevirinin neden değiştiğini göremez.

Yerine: birebir eşleşme önbelleği (aynı metin daha önce onaylandıysa aynen
kullanılır) + oyuna özel terim sözlüğü (`Stamina → Dayanıklılık`,
`Whiterun → Whiterun`). Oyun başına bir JSON, profil dosyalarıyla aynı şekilde
okunabilir/düzenlenebilir/silinebilir. Bu daha az şeffaf değil **daha**
şeffaf: günlük "bu çeviri senin onayladığın kayıttan geldi" diyebilir. Yan
faydası: önbellek isabeti anında döner, model hiç çalışmaz — oyunlar metni
çok tekrarladığı için isabet oranı yüksek olur.

**Geri bildirimde asıl olan "Düzelt", "Reddet" değil.** Reddetme yalnızca
yanlış olduğunu söyler, doğrusunu söylemez. Ve özellik **sıfır geri bildirimle
tam çalışmak zorunda**: kullanıcıların çoğu hiçbir şeyi puanlamaz. Puanlamaya
bağımlı bir tasarım, kullanılmayacak bir tasarımdır.

**Açık kalan soru: Muifly modülü mü, ayrı bir Mui ürünü mü?**

Karara bağlanmadı, çünkü şu an yeterli veri yok. Faz 3'ün yakalama katmanı
gerçekleştikten sonra tekrar bakılacak. Tartışmanın iki tarafı:

- *Ayrı ürün lehine*: `PRODUCT_VISION.md` farklılaşma maddesi #1 "üç kategoriyi
  tek araçta birleştirme" — dördüncü ve alakasız bir kategori bu cümleyi
  bozar. Alıcı da farklı (JRPG/VN oynayan kitle), rakipler de farklı
  (Translumo, LunaTranslator, Textractor — çoğu ücretsiz ve açık kaynak, zor
  bir fiyat çıpası). Üstelik model çıkarımı çalışırken frame hitch'i yaratır;
  kimliği "oyununu daha iyi çalıştırır" olan bir araçta bu ironik. Özellik
  zaten duraklamış diyalog kutusuna, yani metin ağırlıklı oyunlara daralıyor.
- *Modül lehine*: Faz 3 ile ortak altyapı (ekran yakalama, sunum penceresi),
  tek satın alma, kabuk zaten kurulu.

**Neden şimdi yazılmıyor**: Faz 1 hâlâ tek bir gerçek oyunda denenmedi
(`tasks.md` madde 1). Mevcut modüllerin hepsinden büyük bir modülü, çekirdek
ürün sahada doğrulanmadan açmak faz disiplininin engellemek için var olduğu
şeydir. Ayrıca yakalama katmanı Faz 3'te geliyor; daha önce başlanırsa aynı iş
ikinci kez yazılır.

---

## #23 — Profil içe aktarma iki adımlı ve var olanı ezmiyor

**Karar**: Bir profil dosyasını içe aktarmak iki komut: `profil_onizle`
dosyayı okuyup doğruluyor ve "bu profil uygulanınca ne olacak" listesini
üretiyor — **diske hiçbir şey yazmadan**; `profil_ice_aktar` ancak kullanıcı
önizlemeyi gördükten sonra çağrılıyor. Kimlik çakışırsa varsayılan davranış
**yeni bir kimlikle eklemek** (`kimlik-2`); üzerine yazmak ayrı bir anahtar ve
kullanıcının açması gerekiyor.

**Neden iki adım**: `PROFILES.md` güvenlik notu, paylaşılan profillerin
`suspend_process_list` gibi alanlar taşıdığını ve içe aktarma öncesi içeriğin
gösterilmesi gerektiğini söylüyor. Tek adımlı bir "dosyayı seç, uygulandı"
akışı, başkasının dosyasının senin makinende hangi uygulamaları donduracağını
görmeden kabul etmek demekti.

**Neden ezmiyor**: Bir profilin üzerine yazmak, geri alma defterinin
kapsamadığı tek yıkıcı işlem olurdu — defter sistemdeki değişiklikleri geri
alıyor, silinen bir dosyayı değil. Tasarım ilkesi 1 ("her şey geri
alınabilir") burada ancak varsayılanı güvenli tarafa koyarak korunuyor.
Üzerine yazma seçeneği duruyor, ama bilinçli bir tıklama istiyor.

**Etki metinleri Rust tarafında** (`profile_engine::aktarim::etkiler`), karar
#17 ile aynı gerekçe: sayısal vaat yasağı (`DESIGN_PRINCIPLES.md` madde 4) tek
yerde test edilebiliyor (`etkilerde_sayisal_vaat_yok`).

**Demoda kapalı** — `DISTRIBUTION.md` → Demo Kapsamı zaten "profil içe/dışa
aktarma" diyordu; `Kisitlar::profil_aktarimi` bunu koda taşıyor. Dışa aktarma
da kapsamda: demo ikilisi paylaşılabilir dosya üretmiyor.

---

## #24 — Mod değişimi bildirimi var, varsayılanı kapalı

**Karar**: Oyun algılandığında ve oyundan çıkışta kısa bir masaüstü bildirimi
gösteriliyor; ayar `settings::Ayarlar::mod_bildirimi` ve **varsayılanı
kapalı**.

**Neden var**: Program tepside çalışırken (otomatik başlatmanın normal hâli)
ne olduğunu görmenin başka yolu yok. Tepsi menüsünden yapılan işlemler zaten
bildirim gösteriyordu; mod değişimi göstermiyordu.

**Neden varsayılan kapalı**: Alt+Tab yapan kullanıcı dakikada birkaç mod
geçişi üretebiliyor. Kurulduğu anda bildirim yağdıran bir araç, ilk izlenimini
gürültüyle veriyor — ve varsayılanların "en az müdahale" yönünde olması
`settings.rs` modül belgesinin kuralı.

**Bildirim olmayan işi bildirmiyor**: Oyundan çıkışta hiçbir şey geri
alınmadıysa (profil hiç uygulanmamıştı) bildirim de çıkmıyor. Bunun için
`Motor::mod_guncelle` artık geri alınan kayıt sayısını da döndürüyor
(`ModDegisimi`): "3 değişiklik geri alındı" cümlesi, gerçekten sayılmış bir
şeye dayanmak zorunda (tasarım ilkesi 2).

---

## #25 — Oyun kütüphanesi yerelden okunuyor, hiçbir API'ye gidilmiyor

**Karar**: Kurulu oyunların adı, kapak görseli ve çalıştırılabilir dosyaları
**diskten** okunuyor. IGDB, RAWG, SteamGridDB, Steam Web API — hiçbiri
kullanılmıyor ve programda oyun kütüphanesi için tek bir ağ isteği yok.

Kaynaklar:

| Kaynak | Ne veriyor | Nereden |
|---|---|---|
| Steam | ad, appid, kurulum klasörü | `steamapps/appmanifest_*.acf` |
| Steam | kapak/hero/logo görselleri | `appcache/librarycache/<appid>/` |
| Epic | ad, kurulum, başlatma exe'si | `ProgramData\Epic\...\Manifests\*.item` |
| Her ikisi | aday exe listesi | kurulum klasörü taraması |
| Görselsizler | ikon | exe'nin kendi kaynağından (`PrivateExtractIconsW`) |

**Neden API değil**: Sayılan servislerin hepsi anahtar istiyor. Muifly kapalı
kaynaklı ama **dağıtılan bir ikili**: içine gömülen anahtar çıkarılabilir bir
sırdır. Sonuç üç ayrı sorun olurdu — kotanın yakılması, servisin kullanım
koşullarının çiğnenmesi, ve kullanıcının hangi oyunlara sahip olduğunun bir
sunucuya bildirilmesi. Sonuncusu `commands::yapilmayanlar` içindeki "telemetri
toplamaz" vaadiyle doğrudan çelişirdi.

Kullanıcıya kendi anahtarını girdirmek de değerlendirildi ve elendi: hesap
açıp anahtar yapıştırmayı isteyen bir akış, "exe adını elle yazma" sorununu
çözmek için kurulan özelliğin kendisinden daha zahmetli.

**Neden yerel yeterli**: Aranan üç şey zaten diskte duruyor. Steam istemcisi
kapakları kendi önbelleğine indiriyor; Epic manifesti `LaunchExecutable`
alanını doğrudan veriyor. Kalan boşluk (görseli olmayan oyunlar) exe ikonuyla
kapanıyor. Bedeli: **anahtar yok, kota yok, çevrimdışı çalışıyor, gizlilik
sorusu yok.**

**Telif**: Kapak görselleri yayıncılara ait. Muifly onları kullanıcının kendi
diskinden **yalnızca görüntülüyor** — kopyalamıyor, ikiliye gömmüyor, hiçbir
yere göndermiyor. Dağıtılan pakette tek bir oyun görseli yok.

**Bağımlılık eklenmedi**: VDF/ACF çözümleyicisi (`library::vdf`), PNG yazıcısı
(`library::png`) ve base64 kodlayıcı elle yazıldı. Üçü de dar kapsamlı ve
testli; bir kasa eklemek karar #21 gereği üçüncü taraf bildirimlerinin de
yeniden üretilmesi demekti. Görseller arayüze `data:` adresi olarak gidiyor:
`asset:` protokolünü açmak `capabilities/default.json`'a dosya sistemi izni
eklemeyi gerektirirdi.

**Arayüz dosya yolu göndermiyor**: Görsel ve taslak komutları oyunu `kimlik`
ile buluyor, yol ile değil. Son taramanın sonucu `library::SON_TARAMA`
önbelleğinde duruyor. Yol kabul eden bir komut, webview'e "istediğin dosyayı
okut" yüzeyi açardı. Tek istisna `oyun_elle_ekle` ve oradaki yol da
kullanıcının kendi açtığı dosya penceresinden geliyor.

---

## #26 — Katalog oyunun adını söylüyor, ayarını değil

**Karar**: `src-tauri/katalog.json` ikiliye gömülü ve her girdisi yalnızca üç
şey taşıyor: exe adı, oyunun okunabilir adı, rekabetçi olup olmadığı. Hazır
öncelik sınıfı, güç planı, CPU affinitesi ya da dondurma listesi **yok**.

**Neden yok**: Rakiplerin "oyuna özel optimizasyon veritabanı" pazarlaması
büyük ölçüde ölçülmemiş tavsiyeden ibaret. "Cyberpunk'ta güç planını şuna al"
demek için o karşılaştırmayı yapmış olmak gerekir; yapmadık. Tasarım ilkesi 4
(`DESIGN_PRINCIPLES.md`) sayısal vaadi yasaklıyor ve ölçülmemiş bir per-oyun
ayarı, sayı yazmasa da aynı türden bir vaat.

`rekabetci` bayrağı istisna değil, tam tersi: bir şey **açmıyor**, kapatıyor.
Şema rekabetçi profilde kare üretimini ve agresif ölçeklemeyi zorla kapatıyor
(`profile_engine::schema`). Kapatan bir varsayılan için ölçüme ihtiyaç yok.

**Katalog otomatik uygulanmıyor**: Ürettiği şey bir profil **taslağı**
(`katalog::taslak`) ve taslak kullanıcıya gerekçeleriyle gösteriliyor
(`aciklamalar`). Kaydetme ve uygulama her zamanki kapılardan geçiyor —
karar #10 ve #23'ün aynı gerekçesi.

**Elle bakımı yapılıyor**: `katalog.json` üretilmiş bir dosya DEĞİL
(`ucuncu-taraf.json` ile karıştırılmamalı, karar #21). Yanlış ya da eskimiş
bir satırın bedeli düşük: eşleşme olmaz, kullanıcı adı kendi yazar. Hiçbir
satır tek başına sistemde bir değişikliğe yol açmıyor.

**Neden güncelleme kanalı yok**: Katalogun GitHub'dan güncellenmesi
değerlendirildi ve şimdilik yapılmadı — programa ilk ağ bağımlılığını, dosya
imzalama sorusunu ve "indirilen içerik ne yapıyor" sorusunu birlikte
getiriyordu. Katalog sürümle birlikte gidiyor.

---

## #27 — Kare ölçümü yükseltilmiş yetki istiyor; bu yüzden sürekli değil, süreli

**Ölçülen kısıt**: Gerçek zamanlı bir ETW oturumu açmak (`StartTraceW`)
yükseltilmemiş bir süreçte **`ERROR_ACCESS_DENIED` (5)** dönüyor. Bu
varsayılmadı, bu makinede atılacak bir sondayla ölçüldü: yönetici olmayan ve
`Performance Log Users` üyesi olmayan bir kullanıcı hesabında oturum
açılamıyor. Oturumu **tüketmek** de (`OpenTraceW` / `ProcessTrace`) aynı
yetkiyi istiyor, yani "oturumu başkası açsın biz sadece dinleyelim" diye bir
kaçış yok.

**Neden bu bir tasarım sorusu**: Tasarım ilkesi 5 arka plan izlemesinin
yükseltilmiş çalışmasını yasaklıyor. Kare ölçümünü sürekli açık tutmak,
programın tamamının sürekli yönetici koşması demekti — aracın en temel
duruşundan vazgeçmek.

**Karar**: Kare ölçümü arka plan döngüsünün parçası **değil**. Kullanıcının
başlattığı, başladığını ve bittiğini gördüğü, kendi kendine kapanan bir
ölçüm penceresi olarak kurgulanıyor. `monitor::etw::Olcum` bu yüzden kendi
başına UAC istemiyor ve kendini otomatik başlatmıyor; yetki yoksa
`Engel::YetkiYok` dönüyor ve ne yapılacağına çağıran karar veriyor.

**Elenen alternatifler**:

- **Programı baştan yönetici çalıştırmak.** İlke 5'in doğrudan ihlali. Ayrıca
  yükseltilmiş bir süreçte açılan pencereye sürükle-bırak çalışmıyor ve
  webview yüzeyi yönetici bağlamına taşınıyordu.
- **Kullanıcıyı `Performance Log Users` grubuna eklemek.** Bir kez admin
  isteyip sonra hiç istememesi cazip, ama bu kalıcı bir güvenlik ayarı
  değişikliği: kullanıcıya sistem genelinde izleme yetkisi veriyor ve
  programın kaldırılmasıyla geri gitmiyor. "Geri alınabilirlik" ilkesinin
  defterle çözdüğü problemi, defterin kapsayamayacağı bir yere taşırdı.
- **Kalıcı bir Windows servisi** (PresentMon'un yaptığı). Kurulumda bir kez
  admin, sonrasında UAC yok. Bedeli sürekli yükseltilmiş bir yüzey, kurulum/
  kaldırma yükü ve kod imzalama zorunluluğunun büyümesi. "Admin yetkisi
  minimum" duruşuyla çelişiyor.
- **Hook'la ölçmek.** Tasarım ilkesi 3. Tartışılmadı bile.

**`fps_destegi_var()` kaldırıldı**, `false` dönmesi düzeltilmedi. Sebep:
"destek var mı" artık evet/hayır bir soru değil. Yerine `kare_olcum_durumu`
komutu geçti — yardımcının yanımızda olup olmadığını, yetkinin gerektiğini
ve **neden gerektiğini** birlikte söylüyor. Arayüzün söylemesi gereken şey
"ölçemiyoruz" değil, "ölçmek için şunu yapman gerekiyor" ve niçin.
`Durum.fps_olcumu` alanı da bu yüzden düştü.

### Yükseltilmiş sondanın sonucu (ölçüldü)

Yönetici terminalinde koşturulan sonda üç şeyi birden doğruladı:

```
[1] StartTraceW -> 0
[2] EnableTraceEx2(DXGI) -> 0
[2] EnableTraceEx2(D3D9) -> 0

surec                        present   ort FPS    ort ms  %1 kotu ms
wallpaper64.exe                  122      15.0     66.65       71.96
WindowsTerminal.exe               24       1.9    526.01     1348.22
claude.exe                        20      10.3     96.76      658.28
```

Yani: oturum yükseltilmişken açılıyor, sağlayıcılar **başka süreçlerin**
Present olaylarını veriyor, ve aralıklardan anlamlı kare süresi çıkıyor.
Hook yok, injection yok — tasarım ilkesi 3 korunuyor.

**Yan bulgu, ürünü ilgilendiriyor**: masaüstü uygulamaları yalnızca yeniden
çizerken sunum yapıyor (`WindowsTerminal.exe` 1,9 "FPS"). Oyun dışı bir
süreçte "FPS" diye bir sayı göstermek anlamsız olurdu. Ürün kodu bu yüzden
sağlayıcıları çekirdek tarafında **PID süzgeciyle** açıyor
(`EVENT_FILTER_TYPE_PID`) ve ölçüm her zaman belirli bir oyuna bağlı.

**Hâlâ ölçülmedi**: gerçek bir oyunda, oyunun kendi FPS sayacıyla
karşılaştırma. Sonda çalıştırılırken açık bir oyun yoktu. Bu, faz kapısı
değil doğrulama işi; `tasks.md`'de duruyor.

**Karar #14 revize edildi**: "ETW yolu araştırılana kadar boş bırakmak"
maddesi kapandı; yol araştırıldı ve yukarıdaki kısıtla açık.

---

## #28 — Faz 5 fizibilitesi, soru 1: OCR yeterli, ama sınırları adlı adınca

**Soru** (`ROADMAP.md` → Faz 5): `Windows.Media.Ocr` hedeflenen oyunların
yazı tiplerini gerçekten okuyor mu?

**Cevap: evet, koşullu evet.** Ölçüm `arac/ocr-sonda/` ile yapıldı; oyun
metninin bilinen zorluklarını taklit eden 10 örnekte:

- 6/10 örnek **birebir** okundu
- karakter düzeyinde ortalama benzerlik **%98,3**
- kelime düzeyinde 99 kelimede 9 hata
- süre: bölge boyutundaki bir şeritte **14-19 ms**, tam 1920×1080 karede
  **74 ms** (ilk çağrı motor ısınmasıyla ~140 ms)

Süre tarafı rahat: karar #22 çeviriyi tuşa basınca ve seçili bir alanda
yapıyor, sürekli değil. "Çeviri isteği oyunun akışını kesmiyor" kabul
kriteri bu ölçümlerle karşılanabilir görünüyor.

**Hata sınıfları** — hepsi tanınabilir, ikisi ciddi:

- **Harf aralıklı büyük harf menü metni kelime sınırını kaybediyor**:
  `QUIT TO DESKTOP` → `QUITTODESKTOP`. Çeviri açısından en zararlı hata,
  çünkü sessizce yanlış değil, tanınmaz bir girdi üretiyor.
- **Kalabalık zemin üstünde konturlu metinde noktalama/glif karışması**:
  `collapsed.` → `collapsed]`, `We need` → `We.peed`.
- Stilize başlıkta uydurma sondaki tire, tam karede satır başında
  `I have` → `alhave`.

**Külliyat sentetik ve bu bir sınır**: Bu makinede hiç gerçek oyun ekran
görüntüsü yoktu. Örnekler kontur, gölge, düşük kontrast, küçük punto, süslü
font ve kalabalık zemini taklit ediyor; **taklit edemedikleri** oyunun kendi
ölçekleme boru hattı, sıkıştırma gürültüsü, hareket bulanıklığı ve oyuna
özel bitmap fontlar. Rakamlar "en iyi durumda şu kadar" diye okunmalı.
Faz 5 açılırsa gerçek ekran görüntüleriyle tekrar ölçülmeli.

**Ürün gereği çıktı — dil paketi çalışma zamanında kontrol edilecek**:
`OcrEngine::AvailableRecognizerLanguages` bu makinede `en-US` ve `tr`
verdi, ama bu Windows'un kurulu dil paketlerine bağlı ve garanti değil.
Kaynak dilin OCR paketi yoksa özellik sessizce yanlış çalışmamalı; ne
eksik olduğunu ve nasıl kurulacağını söylemeli.

**Karar #1 ile tutarlılık**: OCR tarafı için ikiliye gömülecek ya da
indirilecek bir model **yok** — Windows'un kendi bileşeni. Model sorusu
yalnızca çeviri tarafında duruyor (soru 2, henüz açık).

---

## #29 — Faz 5 fizibilitesi, soru 2: çeviri yeterli, üç adlı zaafla

**Soru** (`ROADMAP.md` → Faz 5): Yerel EN→TR çeviri kalitesi gerçek oyun
diyaloğunda kabul edilebilir mi?

**Cevap: evet, koşullu evet.** Ölçüm `arac/ceviri-sonda/` ile yapıldı;
model `onnx-community/opus-mt-tc-big-en-tr`, ONNX Runtime üzerinde greedy
çözümleme (KV önbelleği yok, yani süreler kötümser).

Düz oyun diyaloğunda çıktı gerçekten kullanılabilir:

- "Press F to pick up the ancient key." → "Eski anahtarı almak için F tuşuna
  basın."
- "My father left this blade to me, and now I leave it to you." → "Babam bu
  bıçağı bana bıraktı ve şimdi ben de sana bırakıyorum."
- "We have been walking for three days without water. If the well is dry, we
  turn back at dawn." → "Üç gündür su olmadan yürüyoruz. Eğer kuyu kurursa
  şafak vakti geri dönüyoruz."

**Niceleme kaliteyi bozmuyor, hızlandırıyor**: int8 çıktısı fp32 ile
pratikte aynı, süre yarısı (cümle başına 78-270 ms yerine 151-506 ms).
Yani indirme fp32'nin ~1,1 GB'ı değil, **~512 MB** (encoder 129 MB +
decoder 377 MB + sözlük/tokenizer ~7 MB). Karar #1 modeli ikiliye gömmüyor,
isteğe bağlı indiriyor — bu boyut o kararla tutarlı ama küçük değil ve
mağaza sayfasında açıkça yazılmalı.

### Üç zaaf

**1. TAMAMI BÜYÜK HARF girdi çöküyor — ama ucuz bir ön işleme çözüyor.**

| girdi | çıktı |
|---|---|
| `MISSION FAILED - RETURN TO CHECKPOINT` | "MİSYONU KAYBETTİ - TÜKETMEYE DÖNÜŞ" |
| `Mission failed - return to checkpoint` | "Görev başarısız - kontrol noktasına geri dön" |

Model düz metinle eğitilmiş; büyük harf dağılım dışında kalıyor. Menü ve
başlık metinleri oyunlarda çoğunlukla büyük harf, yani bu nadir bir durum
değil — ama çevirmeden önce büyük harf tespiti + küçültme sorunu **tamamen**
kapatıyor. Faz 5 açılırsa bu bir zorunluluk, seçenek değil.

**2. Oyun sözlüğü yanlış.** "Longsword" → "Uzunsöz", "party" → "Partiniz",
"blade" → "bıçak", "innkeeper" → "handacı". Genel amaçlı bir çeviri modeli
oyun terimlerini bilmiyor.

Bu, karar #22'nin tasarımını **çürütmüyor, doğruluyor**: orada zaten model
ince ayarı değil, oyuna özel terim sözlüğü + çeviri belleği öngörülmüştü.
Ölçüm o kararın hangi somut boşluğu doldurduğunu gösteriyor.

**3. En tehlikelisi: sessiz atlama ve kendinden emin uydurma.**

- "Keep your guard up. This one bites back." → "Bu seferki ısırıyor."
  Birinci cümle **tamamen düştü**. Hata mesajı yok, kısalma uyarısı yok.
- OCR'ın bozduğu girdi (`The bridge collapsed] We.peed another way across
  the river.`) → "Nehrin diğer tarafına doğru gittik." Akıcı, doğru
  görünen, anlamı bambaşka bir cümle.

Bu ikisi bozuk görünmüyor, bu yüzden tehlikeli. OCR hatası (karar #28) ile
birleşince kullanıcı yanlış bir çeviriye güvenebilir. **Ürün gereği**:
çeviri çıktısı her zaman kaynak metinle birlikte gösterilmeli ve arayüz
bunun makine çevirisi olduğunu saklamamalı. Kaynağı gizleyen bir overlay
tasarımı bu ölçümden sonra tercih edilemez.

### Süre

int8'de cümle başına 78-270 ms, ortalama 140 ms — hem de KV önbelleği
olmadan. Karar #22 çeviriyi tuşa basınca yapıyor, sürekli değil; "çeviri
isteği oyunun akışını kesmiyor" kabul kriteri karşılanabilir görünüyor.

### Not: ilk kötü sonucu modele yorma

Sondanın ilk çalıştırmaları akıcı ama anlamsız Türkçe üretti ve bu kolayca
"model yetersiz" diye okunabilirdi. İki hata da araçtaydı: deponun
`tokenizer.json`'unun id uzayı modelin kelime dağarcığıyla aynı değil
(`▁The` → tokenizer'da 23901, modelde 50392), ve `Precompiled`
normalleştiricinin eşlemesi boş. Ayrıntı `arac/ceviri-sonda/README.md` ve
`src/sozluk.rs`'te. Belirti sinsiydi: çökme yok, NaN yok, tensör
istatistikleri sağlıklı.

---

## #30 — Faz 5 kısmen açıldı: yakalamaya dokunmayan katman önce yazıldı

**Karar**: Faz 5'in **yalnızca ekran yakalamadan bağımsız** parçası yazıldı
(`src-tauri/src/ceviri/`). Yakalama, overlay, kısayol ve çeviri modelinin
kendisi hâlâ yazılmadı ve Faz 3'ün arkasında bekliyor.

**Faz disiplini bozulmadı, ayrıştırıldı.** Karar #22 ve `ROADMAP.md` Faz 5'i
Faz 3'ün arkasına koyarken tek bir somut gerekçe veriyor: *"yakalama katmanı
Faz 3'te geliyor; daha önce başlanırsa aynı iş ikinci kez yazılır."* Bu
gerekçe Faz 5'in tamamı için değil, yakalamaya dokunan parçası için geçerli.
Uygulanan ayrım testi tek soru:

> Faz 3'ün yakalama katmanı geldiğinde bu kod yeniden yazılır mı?

Cevap "hayır" olan dört parça yazıldı:

| Parça | Ne yapıyor | Hangi karara dayanıyor |
|---|---|---|
| `onisleme` | BÜYÜK HARF metni cümle düzenine indirir; OCR'ın bozduğundan şüphelenilen yerleri işaretler | #29 zaaf 1 (zorunluluk), #28 hata sınıfları |
| `sozluk` | Terimleri çeviriden önce korur, sonra karşılığını koyar | #22, #29 zaaf 2 |
| `bellek` | Oyun başına JSON: birebir eşleşme önbelleği + terim sözlüğü | #22 |
| `ocr_dil` | Kaynak dilin OCR paketi kurulu mu, değilse nasıl kurulur | #28 ürün gereği |

Cevap "evet" olan hiçbir şey yazılmadı: ekran yakalama, overlay penceresi,
`RegisterHotKey`, model indirme ve çıkarım.

**Motor'a bilerek bağlanmadı.** Karar #22'nin açık sorusu — "Muifly modülü mü,
ayrı bir Mui ürünü mü" — Faz 3'ten sonra bakılacak. Modülün `state::Motor`'la
hiçbir bağı yok, iki cevabın da bedeli düşük kalsın diye. CLAUDE.md'nin
değişmez kuralı burada tetiklenmiyor: modül sistemde hiçbir şey değiştirmiyor,
`monitor` gibi yalnızca okuyor ve kendi dosyasına yazıyor — geri alınacak bir
değişiklik üretmediği için deftere yazacak bir şeyi de yok.

**Arayüz eklenmedi, bilerek.** Çeviri yapamayan bir özelliğin ekranını
koymak, oturum 9'da site'ta düzeltilen hatanın aynısı olurdu: karşılanmamış
bir vaat (ilke 4). Ekran, özellik gerçekten çevirmeye başladığında gelir.

**Ölçülmemiş bir varsayım kodda adlı adınca duruyor.** `sozluk` terimleri
`[[0]]` biçiminde bir işaretle koruyor ve bu işaretin SentencePiece
tokenizer'ından ve greedy çözümlemeden sağ çıkacağı **varsayım**, ölçüm
değil — model bağlandığında ilk sınanacak şey bu. Bu yüzden `geri_koy`
kaybolan işaretleri döndürüyor ve arayüz onları göstermek zorunda: kaybı
sessizce yutan bir tasarım, karar #29'un üçüncü zaafını (sessiz cümle atlama)
bir kez daha üretirdi. Yan faydası, kaybolan işaretin o zaaf için ucuz bir
tespit aracı olması — model bir cümleyi düşürdüyse o cümledeki işaret de
düşer.

**OCR dil sorgusu ikiliye bir bayt eklemedi.** `windows` crate'ine üç WinRT
özelliği (`Foundation`, `Globalization`, `Media_Ocr`) eklendi ve `Cargo.lock`
**değişmedi** — yeni bağımlılık yok, dolayısıyla `ucuncu-taraf-uret.mjs`
çalıştırmak da gerekmedi. Karar #28'in "OCR tarafında model yok" tespitiyle
tutarlı.

---

## #31 — Oturum geçmişi diske yazılıyor, varsayılanı açık; ama iddia taşımıyor

**Karar**: Biten her oyun oturumu `oturum-gecmisi.json`'a yazılıyor
(`monitor::gecmis`), arayüzde kendi sekmesinde gösteriliyor, düz metin rapor
olarak dışa aktarılabiliyor. Ayar **varsayılan açık**; tek düğmeyle
silinebiliyor.

**Neden var.** Şeffaflık günlüğü (`monitor::log`) yalnızca bellekte duruyor ve
programla birlikte ölüyor. Muifly'ın normal kullanımı ise tepside beklemek:
kullanıcı akşam oynuyor, sabah makineyi yeniden başlatıyor ve "dün gece ne
yapıldı" sorusunun cevabı hiçbir yerde kalmıyor. Tasarım ilkesi 2 ("ne
değişti, kullanıcıya gösterilir") program kapandığında kaybolan bir kayıtla
yarım tutuluyordu.

**Varsayılanın açık olması, "en az müdahale" kuralının istisnası değil,
uzantısı.** O kural sistemde bir şey değiştiren ayarlar için: geçmiş
sistemde hiçbir şey değiştirmiyor, kendi veri klasöründeki kendi dosyasına
yazıyor. Kapalı gelseydi kullanıcı dün geceyi ancak **önceden** açmayı akıl
etmişse görebilirdi — yani özelliğin işe yaradığı tek an, hep kaçırılan an
olurdu. `settings::testler::varsayilan_gecmis_acik` bu duruşu tutuyor.

**Telemetri değil, ve bu ayrım koda bakılarak doğrulanabilir.** Dosya
kullanıcının kendi veri klasöründe; hiçbir yere gönderilmiyor, hiçbir yerden
okunmuyor. Dışa aktarma bile bir paylaşım değil bir dosya yazma işlemi: yol
kullanıcının kendi seçtiği kaydetme penceresinden geliyor, program kendi
başına bir yere dosya bırakmıyor.

**Kayıt ölçüm taşır, iddia taşımaz.** Karar #15 canlı karşılaştırmada tek bir
"şu kadar iyileşti" oranı üretmeyi reddediyor; geçmişte bu daha da bağlayıcı,
çünkü oranı okuyacak bağlam (o an oyunda ne olduğu) artık ekranda değil.
Öncesi ve sonrası iki ayrı özet olarak duruyor, aralarında yön işareti bile
yok. Rapor metninde de tek bir yorum cümlesi yok ve
`raporda_iyilesme_iddiasi_yok` testi bunu koruyor — ilke 4'ün
`network_boost::tcp`'deki karşılığının aynısı.

**Oturum = Muifly'ın sistemde bir şey tuttuğu süre, oyunun açık kalma süresi
DEĞİL.** İkisi ayrışabiliyor (kullanıcı oyunun ortasında elle geri alabilir)
ve kayıt hangisini anlattığını bilmek zorunda. Aynı oyun için profil ikinci
kez uygulanırsa oturum **sıfırlanmıyor**; sıfırlansaydı tek bir oyun akşamı,
kullanıcının düğmeye kaç kez bastığı kadar parçaya bölünürdü. Başka bir
sürece geçilirse önceki oturum önce kapanıyor: iki oyunun değişiklikleri tek
kayda karışmamalı.

**Kapasite 200 ve sınır var.** Yıllarca tepside duran bir araçta sınırsız bir
dosya sessizce büyür. 200 oturum, günde bir oyun oynayan biri için yarım
yıldan uzun bir geçmiş ve birkaç yüz kilobayt.

**Bozuk dosyada `Defter::yukle` ile aynı davranış**: hata dönmüyor, dosya
`.bozuk` uzantısıyla kenara alınıyor, program açılıyor. Bir geçmiş dosyası
yüzünden programın hiç açılmaması kaybın kendisinden kötü olurdu. (Çeviri
belleği bilerek bunun tersini yapıyor — gerekçe karar #30'da: orada kaybolan
şey kullanıcının elle yaptığı düzeltmeler.)

**Ayar kapalıyken liste boş DEĞİL, sebebi yazılı.** Boş bir liste "hiç
oynamadın" diye okunurdu; ekran kapalı olduğunu söylüyor ve açacak yeri
gösteriyor.

---

## #32 — Ölçekleme GPU'da yapılıyor; CPU uygulaması yalnızca referans ve test

**Ölçülen kısıt**: 1080p→1440p tek bir kareyi CPU'da ölçeklemek, release
derlemesinde bu makinede:

| Algoritma | Süre (ms/kare) |
|---|---|
| Tam sayı katı | 11.9 |
| Bilinear | 270.2 |
| Lanczos (a=3) | 207.9 |
| xBR | 1293.5 |

60 FPS'in kare bütçesi **16.67 ms**. Aradeğerleme yapan üç yol bütçenin
12-78 katı. Tam sayı katı ucuz görünüyor ama aynı işi yapmıyor: bu ölçek
oranında kat 1'e düşüyor, yani ölçekleme değil kopyalama ölçülmüş oluyor —
ve o bile bütçenin **dörtte üçünü** oyundan çalıyor.

Bu varsayılmadı, ölçüldü ve ölçüm tekrarlanabilir bırakıldı: `cargo test
--release cpu_yolunun_maliyeti -- --ignored --nocapture`. Sayılar makineye
bağlı; büyüklük mertebesi değil.

**Karar**: Gerçek zamanlı yol D3D11 piksel gölgelendiricileri
(`scaling/olcekleme.hlsl`). Yakalanan doku hiç CPU'ya inmiyor: Desktop
Duplication'ın verdiği doku, yakalamayla **aynı cihaz** üzerinde
gölgelendiriciye giriyor ve doğrudan sunum zincirine çiziliyor. İki ayrı
D3D11 cihazı kullanmak her karede paylaşımlı doku aktarımı demekti.

`scaling/algoritma.rs`'teki CPU uygulaması **çalışma zamanında
kullanılmıyor**. İki işi var: gölgelendiricinin ne üretmesi gerektiğinin
tanımı olmak, ve Faz 3'ün birinci kabul kriterini (görüntü kalitesinin
karşılaştırmalı olarak doğrulanması) test edilebilir kılmak. Kalite
testleri sentetik görüntülerle ölçüyor: yumuşak geçişte Lanczos'un
bilinear'dan daha az hata yapması (PSNR), sert köşegen kenarda xBR'nin
bilinear'dan daha az ara ton üretmesi, tam sayı katının hiç yeni renk
üretmemesi.

**Kabul edilen borç**: aynı matematik iki yerde duruyor ve ikisi
birbirinden sessizce ayrılabilir. Tutulan yer sabitler:
`golgelendirici_sabitleri_referansla_ayni` ve
`golgelendiricide_yuv_agirliklari_referansla_ayni` testleri, HLSL metnindeki
sayıların Rust sabitleriyle aynı kaldığını doğruluyor. **Satır satır
eşitliği kanıtlamıyorlar** — kanıtlayacak tek şey gerçek bir ekranda görsel
karşılaştırma ve o iş `tasks.md`'de elle doğrulama olarak duruyor.

**Bilinen ve bilinçli fark**: xBR'de CPU referansı önce sabit iki kat üretip
kalanı Lanczos'a bırakıyor; gölgelendirici kuralı doğrudan hedef
çözünürlükte değerlendiriyor. Tam iki katta ikisi aynı sonucu veriyor.
Ayrıca buradaki xBR, ailenin kenar yönlü karşılaştırma kuralının bir
uygulaması; üst kaynakla piksel-birebir aynılık **iddia edilmiyor**.

**Ölçeklemenin bedeli ölçülüyor ve gösteriliyor.** `scaling/gecikme.rs`
boru hattının kare başına eklediği süreyi tutuyor ve arayüz bunu bir
kazanç gibi değil bir bedel gibi yazıyor. Özet yapısında "kazanç",
"iyileşme" ya da "oran" adında bir alan yok ve `ozette_iyilesme_alani_yok`
testi bunu koruyor — karar #15'in bu modüldeki karşılığı. "Sunum" satırının
dikey eşitleme beklemesini de içerdiği ekranda yazılı; yazılmasaydı o sayı
maliyet sanılırdı.

**Rekabetçi modda ölçekleme kısıtlı değil, kapalı.** Rekabetçi mod
gecikmeyi en aza indirmek için var; ölçekleme her karede ölçülebilir bir
gecikme ekliyor. İkisini aynı anda açık tutmak, kullanıcının seçtiği şeyin
tersini yapmak olurdu. Üç yerde kapalı: profil dosyası doğrulaması
(`schema::dogrula`), profil uygulama yolu (`state`), ve mod geçişi — açıkken
rekabetçi moda geçilirse ölçekleme duruyor. `scaling::testler::
rekabetci_modda_kapali` ve `rekabetci_profilde_olcekleme_zorla_kapaniyor`
bu duruşu tutuyor.

**Deftere yazılmıyor, günlüğe yazılıyor.** `CLAUDE.md`'nin değişmez kuralı
"sistemde bir şey değiştiren her yol deftere + günlüğe yazar" diyor.
Ölçekleme deftere yazmıyor çünkü sistemde geri alınacak bir iz bırakmıyor:
açılan tek şey, sürecin ömrüyle sınırlı bir pencere. Program çökerse pencere
de gider; registry'de kayıt, diskte dosya, bir sonraki açılışta temizlenecek
kalıntı yok. Günlüğe ise başlangıcı, algoritması ve **durma sebebi** yazılıyor.

**İlk çalıştırmada bulunan iki kusur** — ikisi de modülü işlevsiz bırakıyordu
ve gerçek bir ekranda koşturulmadan görülemezdi:

1. **Masaüstünün tamamı ölçekleniyordu.** Desktop Duplication ekranın
   tamamını veriyor; onu kendi boyutunda yeniden çizmek hiçbir şeyi
   büyütmüyor. Ölçeklenmesi gereken şey **oyun penceresinin istemci alanı**.
   Kırpma gölgelendiricide yapılıyor (`kaynak_ofset` / `kaynak_boyut`
   sabitleri), ayrı bir doku kopyasıyla değil: fazladan bir kopya her karede
   bir GPU turu daha demekti. `GetClientRect` kullanılıyor, `GetWindowRect`
   değil — pencereli bir oyunda başlık çubuğunu da büyütmek istemiyoruz.
   Hedef pencere her karede öndeki pencereden okunuyor ve **kendi
   süreçlerimiz eleniyor**: kullanıcı "Başlat"a Muifly penceresinden basıyor,
   elenmese program kendi arayüzünü büyütürdü.
2. **Sunum penceresi kendini yakalıyordu.** Çoğaltma bileşiklenmiş
   masaüstünü veriyor ve üstte duran pencere de onun parçası: kendi
   çıktımızı yakalayıp yeniden çizince ekranda birbirinin içine giren bir
   tünel oluşuyor. `SetWindowDisplayAffinity(WDA_EXCLUDEFROMCAPTURE)` ile
   pencere yakalamanın dışına alınıyor. Bu çağrı Windows 10 sürüm 2004 ile
   geldi; daha eskisinde başarısız oluyor ve **hata yutulmuyor**: durum
   yapısındaki `uyari` alanıyla arayüze çıkıyor ve kullanıcı gördüğü
   bozukluğun sebebini okuyor. Hatadan ayrı bir alan, çünkü ölçekleme
   çalışmaya devam ediyor.

Bu iki kusur, birim testlerinin bu modülde neyi **gösteremediğinin** somut
örneği: algoritmanın matematiği ve sabitlerin tutarlılığı testliydi, ama
"ekranda doğru şey görünüyor mu" sorusunun cevabı ancak `cargo test
gercek_ekranda_bir_tur -- --ignored` ile çıktı.

**Elenen alternatifler**:

- **CPU'da ölçekleyip GDI ile çizmek.** Yukarıdaki tablo. Ayrıca her karede
  bir CPU→GPU→CPU turu, ölçmeye çalıştığımız gecikmenin kendisini büyütürdü.
- **Katmanlı pencere (`WS_EX_LAYERED`).** DXGI'nin çevirme (flip) modeliyle
  çalışmıyor; çevirme modeli olmadan sunumda fazladan bir kopya oluşuyor.
- **Önceden derlenmiş gölgelendirici bayt kodu.** Sürücü ve özellik
  seviyesine bağlı; `D3DCompile` ile çalışma zamanında derlemek, ikiliye
  gömülü tek bir HLSL metniyle her makinede aynı kaynağı kullanmayı
  sağlıyor.
- **`Windows.Graphics.Capture` ile pencere yakalama.** Doğru cevap
  olabilirdi: belirli bir pencerenin içeriğini, üstü örtülü olsa bile
  veriyor. Seçilmedi çünkü WinRT birlikte çalışma katmanı (interop) ayrı bir
  iş ve Desktop Duplication + yakalamadan gizleme aynı sonucu veriyor.
  Kalan fark: başka bir pencere oyunun üstüne gelirse o da yakalanıyor.
  Bilinen sınır; gerçek kullanımda rahatsız ederse `tasks.md`'ye geçmeli.
- **Oranı bozup ekranı doldurmak.** En/boy oranı her zaman korunuyor.
  Görüntüyü yatay esnetmek, kullanıcının istemediği ama fark etmesi de zaman
  alan bir bozulma.

---

## #33 — Faz 3'e, Faz 1 sahada doğrulanmadan başlandı

**Kural neydi**: `CLAUDE.md` ve `ROADMAP.md`, Faz 1'in kabul kriteri olan
5-10 oyunluk manuel test yapılmadan Faz 3'e geçilmemesini söylüyor. Gerekçe
sağlamdı: doğrulanmamış bir temelin üstüne kat çıkmak, bir sorun çıktığında
hangi katta olduğunu bilinemez hale getirir.

**Ne oldu**: Faz 3, proje sahibinin açık isteğiyle, Faz 1 saha testi hâlâ
yapılmamışken yazıldı. Karar sahibinin; burada kayıt altına alınmasının
sebebi, ileride "bu neden atlandı" sorusunun cevapsız kalmaması.

**Taşınan risk**: Faz 1 ve Faz 2'nin bekleyen elle doğrulamaları duruyor
(`tasks.md` → Sıradaki 1-5) ve şimdi Faz 3'ün doğrulaması da onların
üstüne bindi. Gerçek bir oyunda bir sorun görüldüğünde, önceliklendirmeden
mi yakalamadan mı sunumdan mı geldiğini ayırmak, her katman ayrı ayrı
doğrulanmış olsaydı olacağından daha zor.

**Riski azaltan iki şey**: Ölçekleme modülü diğer fazlardan **bağımsız**
çalışıyor — profil uygulanmadan da açılabiliyor, kendi ekranını, kendi
ölçümünü ve kendi durma sebebini gösteriyor. İkincisi, sistemde kalıcı iz
bırakmıyor (karar #32), yani bir hata durumunda temizlenmesi gereken bir
şey de bırakmıyor.

---

## #34 — Ölçekleme penceresinin kaçış yolu ve gizlenme kuralı

**Ne oldu**: Ölçekleme masaüstünde denendi ve makine kullanılamaz hale
geldi. Tıklamalar sonuç vermedi, ekran değişmedi; tek çıkış yolu bilgisayarı
yeniden başlatmak oldu.

**Neden**: Sunum penceresinin beş özelliği tek tek doğruydu ama birlikte
çıkışsız bir kutu oluşturuyordu — tam ekran (`WS_POPUP` + ekran boyu),
üstte (`WS_EX_TOPMOST`), odak almayan (`WS_EX_NOACTIVATE`), tıklanamayan
(`WS_EX_TRANSPARENT`), Alt+Tab'da görünmeyen (`WS_EX_TOOLWINDOW`). Pencereyi
kapatmanın klavyeden ya da fareden **hiçbir yolu yoktu**; tek düğme
Muifly'ın kendi penceresindeydi ve o pencere kaplamanın altında kalıyordu.

Bunu tetikleyen şey ölçeklenecek pencere bulunamadığında devreye giren
yedek davranıştı: masaüstünün tamamını ölçeklemek, yani ekranı ekranın
birebir kopyasıyla kaplamak. Kopya tazelendiği sürece fark edilmiyor; kopya
tazelenmeyi kestiği anda kullanıcının önünde donmuş bir resim kalıyor.
Tıklamalar pencereden geçip altındaki gerçek pencerelere gidiyor ama
sonucu görünmüyor — yani sistem çalışıyor, kullanıcı göremiyor.

**Karar — üç değişiklik**:

1. **Kaçış kısayolu.** Döngü `RegisterHotKey` ile bir kombinasyon
   kaydediyor (`Ctrl+Alt+Shift+S`, meşgulse iki yedek). Kısayol
   `WM_HOTKEY` olarak zaten var olan mesaj kuyruğuna düşüyor.
   **Kaydedilemezse ölçekleme başlamıyor**: kaçış yolu olmayan bir tam
   ekran kaplama, kullanıcıya yeniden başlatmaktan başka çıkış bırakmıyor.
   Kısayolun etiketi arayüzde ölçekleme **açılmadan önce** de yazıyor.

2. **Hedef önde değilken pencere gizleniyor.** "Masaüstünü ölçekle" yedeği
   kaldırıldı. Sunum penceresi yalnızca ölçeklenecek bir pencere öndeyken
   görünüyor; kullanıcı masaüstüne ya da Muifly'a geçtiğinde gizleniyor ve
   ekran kullanıcıya kalıyor. Hedef unutulmuyor — oyuna dönüldüğünde
   ölçekleme kaldığı yerden sürüyor. Pencere ilk **başarılı** çizimden
   sonra gösteriliyor, öncesinde değil.

3. **Kısayolla durdurma günlüğe yazılıyor.** Döngü iş parçacığının Motor'a
   erişimi yok; bayrağı arka plan döngüsü devralıp günlüğe geçiriyor.
   Şeffaflık ilkesi kullanıcının kendi yaptığı durdurma için de geçerli.

**Reddedilenler**:

- **Klavye kancası (`SetWindowsHookEx`).** Tasarım ilkesi 3'ü ihlal ederdi.
  `RegisterHotKey` kanca değil: tuş basışları okunmuyor, sisteme tek bir
  kombinasyon kaydediliyor ve yalnızca o kombinasyon mesaj olarak geliyor.
- **Kaçış yolu olmadan uyarıyla devam etmek.** Kullanıcıya "bu özellik
  makineni kilitleyebilir" deyip yine de açmak, uyarıyı sorumluluk
  aktarımına çevirirdi.
- **"Yeni kare gelmiyorsa kendini durdur" nöbetçisi.** Duran bir oyun
  menüsü de kare üretmiyor; nöbetçi çalışan ölçeklemeyi keserdi. Sorun
  karenin gelmemesi değil, gelmediğinde çıkış olmamasıydı.
- **Pencereyi tıklanabilir yapmak.** Tıklamaların oyuna geçmesi
  ölçeklemenin çalışma şartı; bunu bozmak özelliği bitirirdi.

**Kalan risk**: Ölçeklenen oyunun kendisi çizmeyi keserse pencerede son
kare durmaya devam ediyor. Bu artık kilitlenme değil — kısayol çalışıyor —
ama "donmuş görüntü" hâlâ mümkün ve gerçek oyunla denemede bakılacak
(`tasks.md` → Sıradaki 7).
