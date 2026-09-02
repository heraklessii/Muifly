# Modüller — Teknik Detay

> Bu dosya her Rust modülünün sorumluluğunu, kullanılacak Windows API'lerini ve
> dikkat edilmesi gereken noktaları listeler. Claude Code implementasyona başlamadan
> önce ilgili bölümü okumalı.

## system_boost

**Sorumluluk**: Foreground oyun process'ini algılama, önceliklendirme, arka plan
process yönetimi, güç planı, bellek temizliği.

| İşlev | API / Yöntem | Not |
|---|---|---|
| Oyun process algılama | `GetForegroundWindow` + `GetWindowThreadProcessId` | Whitelist/profil ile eşleştir |
| Process önceliklendirme | `SetPriorityClass(HIGH_PRIORITY_CLASS)` | `REALTIME_PRIORITY_CLASS` KULLANMA — sistem çökmesine yol açabilir |
| CPU affinity | `SetProcessAffinityMask` | Hibrit CPU'larda (Intel 12.nesil+) P-core'lara sabitleme mantığını `docs/RISKS.md`'deki notla birlikte uygula |
| Arka plan process durdurma | `NtSuspendProcess` (ntdll, resmi dokümante edilmemiş ama yaygın kullanılan) | Kapatma değil DONDURMA. Oyun kapanınca otomatik `NtResumeProcess` |
| Güç planı | `powercfg /setactive <GUID>` (shell) veya `PowerSetActiveScheme` API | Oyun öncesi kaydet, oyun sonrası eski plana dön |
| Bellek/standby list | Standby list temizleme (community araçlarında `EmptyStandbyList` yaklaşımı) | Riskli/agresif bir işlem — opsiyonel, varsayılan kapalı |
| MMCSS entegrasyonu | `AvSetMmThreadCharacteristics("Games")` | Windows'un kendi oyun thread önceliklendirmesi |

**Kritik**: Her aksiyon `monitor` modülüne bir log event'i göndermeli
(`"13 process duraklatıldı"` gibi) — `DESIGN_PRINCIPLES.md` şeffaflık ilkesi.

## network_boost

**Sorumluluk**: DNS/route optimizasyonu, QoS önceliklendirme, TCP tuning, jitter/
packet loss izleme.

| İşlev | API / Yöntem | Not |
|---|---|---|
| DNS testi | Birden fazla DNS sunucusuna (Cloudflare, Google, ISP) ping/query süresi ölçümü | En hızlıyı otomatik öner, otomatik değiştirme opsiyonel |
| Route testi | Traceroute benzeri path testi, oyun sunucusu IP'sine en düşük gecikmeli yol | Sabit VPN tüneli değil — gerçek zamanlı test |
| QoS | Windows QoS Packet Scheduler API | Oyun process'inin trafiğine öncelik, arka plan indirmelerini throttle et |
| TCP tuning | `TCP_NODELAY` (Nagle kapatma), ACK frequency ayarı | Registry değişiklikleri geri alınabilir olmalı |
| Jitter/packet loss | ICMP ping örnekleme, sürekli arka plan ölçümü | `monitor` modülüne veri besler |

**Kritik**: "Ping'i X ms düşürür" gibi sayısal vaat İÇEREN hiçbir UI metni veya log
mesajı yazılmaz (`DESIGN_PRINCIPLES.md`).

## monitor

**Sorumluluk**: FPS, ping, jitter, CPU/GPU kullanımı canlı ölçüm ve overlay.

| İşlev | API / Yöntem | Not |
|---|---|---|
| FPS ölçümü | DXGI present call sayımı veya PresentMon benzeri yaklaşım | Overlay yoksa da arka planda ölçülebilmeli |
| Overlay | DirectX hook (Faz 2+, dikkatli tasarlanmalı) | Bkz. `RISKS.md` — hook'lar anti-cheat riski taşıyabilir, alternatifi ayrı pencere/widget |
| Öncesi/sonrası snapshot | İki zaman noktasındaki metrik farkını hesaplama | UI'da karşılaştırma raporu için |
| Log akışı | Event tabanlı, `system_boost` ve `network_boost`'tan gelen aksiyon logları | Şeffaflık ilkesi |

## scaling (Faz 3 — yazıldı)

**Sorumluluk**: Post-process spatial upscaling (Faz 3), ileride ML tabanlı frame
generation (Faz 4).

| İşlev | API / Yöntem | Not |
|---|---|---|
| Ekran yakalama | Desktop Duplication (`IDXGIOutputDuplication`) | ✅ Kare, yakalamayla aynı D3D11 cihazında kalıyor; CPU'ya inmiyor |
| Spatial upscaling | D3D11 piksel gölgelendiricileri | ✅ Tam sayı katı, bilinear, Lanczos (a=3), xBR |
| Sunum | `WS_EX_TOPMOST\|NOACTIVATE\|TRANSPARENT\|TOOLWINDOW` + DXGI çevirme zinciri | ✅ Odak almıyor, tıklama geçiriyor, Alt+Tab'da görünmüyor |
| Kaçış kısayolu | `RegisterHotKey` (kanca değil) | ✅ Yukarıdaki beş bayrak birlikte kapatılamayan bir kutu yapıyor; kısayol kaydedilemezse ölçekleme başlamıyor (karar #34) |
| Hedef önde değilken | Sunum penceresi gizleniyor | ✅ Masaüstünde ve Muifly'a bakarken ekran kullanıcıya kalıyor; hedef unutulmuyor (karar #34) |
| Gecikme ölçümü | Kare başına yakalama/sunum süreleri | ✅ Eklenen bedel; kazanç iddiası taşımıyor |
| Kare üretimi (Faz 4a) | Piramitli blok eşleme + çift yönlü warp, HLSL | ✅ ML yok. Varsayılan kapalı, rekabetçi modda kapalı; bir kareyi elde tutuyor ve bu bedel algoritma hızıyla azalmıyor (karar #35) |
| Kare üretimi doğruluğu | `hareket.rs` CPU referansı + sentetik gerçek-referans | ✅ Bilinen kaydırma → beklenen vektör; gözle değil testle |
| Frame generation ML (Faz 4b) | Eğitilmiş model | ⬜ Fizibilite yapıldı, **açılmadı** — `docs/FRAME_GENERATION.md` |
| İkinci GPU offload | Çoklu adaptör (iGPU+dGPU) tespiti ve hesaplama dağıtımı | ⬜ Faz 3/4 sonrası "nice to have" |

**Kritik**: Bu modül oyun process'ine HİÇBİR ŞEY enjekte etmez, sadece ekranı okur.
Mimari gerekçe: `ARCHITECTURE.md` → "Ekran Yakalama / Scaling Mimarisi",
uygulama kararları `decisions.md` #32.

**Neden CPU'da bir ikinci uygulama var**: `algoritma.rs` çalışma zamanında
kullanılmıyor. Gölgelendiricinin ne üretmesi gerektiğinin tanımı ve Faz 3'ün
"görüntü kalitesi karşılaştırmalı olarak doğrulanmış olsun" kriterinin test
edilebilir hali. Ölçüm, CPU yolunun gerçek zamanlı olmadığını gösteriyor
(karar #32).

**Rekabetçi modda kapalı** — kısıtlı değil, kapalı. Ölçekleme gecikme
ekliyor; rekabetçi mod o gecikmeyi en aza indirmek için var. Üç kapı:
profil doğrulaması, profil uygulama yolu, mod geçişi.

**Deftere yazmıyor, günlüğe yazıyor.** Sistemde geri alınacak bir iz
bırakmıyor: açılan tek şey sürecin ömrüyle sınırlı bir pencere.

## profile_engine

**Sorumluluk**: JSON profil yükleme/kaydetme, oyun algılama, mod state yönetimi.

Şema ve mod tanımları için: `PROFILES.md`

## library

**Sorumluluk**: Kurulu oyunları bulmak — ad, kapak görseli, aday exe listesi.
Profil açarken exe adının elle yazılmasını gereksiz kılıyor.

Tamamı **yerel okuma**: Steam manifestleri ve görsel önbelleği, Epic
manifestleri, exe ikonları. Oyun kütüphanesi için tek bir ağ isteği yok
(karar #25). Sistemde hiçbir şey değiştirmiyor; ürettiği tek şey kullanıcının
onaylayacağı bir profil taslağı.

---

# Uygulama Durumu (1 Eylül 2026)

Yukarıdaki tablolar **tasarımı** anlatıyor; aşağısı kodda ne olduğunu.
Bir modülü değiştirmeden önce buraya bak.

## Dosya haritası

| Modül | Dosya | Durum |
|---|---|---|
| Süreç algılama | `system_boost/detect.rs` | ✅ Öndeki pencere, süreç listesi, tam ekran sezgisi, `DOKUNULMAZ` listesi |
| Öncelik / affinite | `system_boost/priority.rs` | ✅ `REALTIME` tip seviyesinde imkânsız (karar #4). Hibrit CPU topolojisi + P-core maskesi |
| Dondurma | `system_boost/suspend.rs` | ✅ `NtSuspendProcess` çalışma anında aranıyor; yoksa özellik kapanıyor, program açılıyor |
| Güç planı | `system_boost/power.rs` | ✅ Oku/değiştir/geri yükle. Plan **oluşturulmuyor** (karar #11) |
| Açılış modu | `system_boost/startup.rs` | ◐ Windows ile başlatma var; servis geciktirme ertelendi (karar #12) |
| DNS | `network_boost/dns.rs` | ◐ Ölçüm ve öneri var; otomatik uygulama bilinçli olarak yok (karar #6) |
| Gecikme / jitter / yol | `network_boost/latency.rs` | ✅ `IcmpSendEcho` (karar #8), TTL tabanlı yol testi, en büyük sıçrama |
| TCP ayarları | `network_boost/tcp.rs` | ✅ Nagle (arayüz başına), ağ kısıtlaması, sistem yanıt payı — hepsi geri alınabilir |
| QoS | `network_boost/qos.rs` | ✅ DSCP 46 ilkesi. Yalnızca `Muifly-` ön ekli ilkelere dokunuyor (karar #13) |
| Şeffaflık günlüğü | `monitor/log.rs` | ✅ 500 satırlık halka tampon. Geri alınan satır silinmiyor, düğmesi düşüyor |
| Ölçüm | `monitor/metrics.rs` | ✅ CPU/bellek/gecikme örnekleme, jitter (ardışık fark), kayıp oranı |
| Kare süresi istatistiği | `monitor/frames.rs` | ✅ Saf: ortalama, en kötü %1, kare jitter. Windows API yok, doğrudan test ediliyor |
| Kare ölçümü (ETW) | `monitor/etw.rs` | ✅ Oturum + tüketici; sağlayıcılar çekirdek tarafında PID süzgeciyle açılıyor. Gerçek makinede doğrulandı (karar #27) |
| Ölçümü başlatma | `monitor/olcum.rs` + `bin/muifly-olcum.rs` | ✅ Ayrı ve kısa ömürlü yükseltilmiş yardımcı. Ana uygulama yükselmiyor; UAC yalnızca kullanıcı ölçümü başlatınca |
| Kare ölçümü arayüzü | `components/KareOlcumu.tsx` | ✅ İki adımlı: önce yetkinin nedeni, sonra UAC. Hedefi motor seçiyor, arayüz PID taşımıyor. Beş testle korunuyor |
| Oturum geçmişi | `monitor/gecmis.rs` | ✅ Biten oturumlar diske; kapasite 200, bozuk dosya programı kilitlemiyor. Öncesi/sonrası iki ayrı özet, oran yok (karar #31) |
| Geçmiş arayüzü | `components/GecmisPaneli.tsx` | ✅ Liste + özet şeridi + arama + onaylı silme + düz metin rapor. Ayar kapalıysa sebebi yazıyor. On testle korunuyor |
| Profil şeması | `profile_engine/schema.rs` | ✅ Doğrulama + güvenli hale getirme; düzeltmeler kullanıcıya gösteriliyor |
| Profil deposu | `profile_engine/store.rs` | ✅ Ayrı JSON dosyaları (karar #9), yol kaçışı engelli |
| Profil aktarımı | `profile_engine/aktarim.rs` | ✅ İki adımlı içe aktarma: önizleme diske yazmıyor, çakışmada ezmiyor (karar #23). Demoda kapalı |
| Mod seçimi | `profile_engine/mod.rs` | ✅ Beş mod, saf `mod_sec` fonksiyonu |
| Geri alma defteri | `ledger.rs` | ✅ Diske yazılıyor (karar #3), bozuk dosya programı kilitlemiyor |
| Geri alma uygulayıcı | `revert.rs` | ✅ Kısmi başarısızlık: başarısız kayıt deftere iade ediliyor |
| Registry | `registry.rs` | ✅ Her yazma bir `Undo` döndürüyor. "Yaz ve unut" fonksiyonu yok |
| Sistem tepsisi | `tray.rs` | ✅ Simge + menü (Göster / Öndekine uygula / Varsayılana dön / Çıkış). Menü işlemleri motordan geçiyor, bildirimle sonuçlanıyor. Mod değişimi bildirimi ayara bağlı, varsayılanı kapalı (karar #24) |
| Sürüm kısıtları | `surum.rs` | ✅ Demo/tam ayrımı derleme bayrağıyla (karar #20). Zaman sınırı yok, testle korunuyor |
| Üçüncü taraf bildirimleri | `ucuncu_taraf.rs` | ✅ `ucuncu-taraf.json` ikiliye gömülü, tembel ayrıştırılıyor. Veriyi `arac/ucuncu-taraf-uret.mjs` üretiyor (karar #21) |
| Steam kütüphanesi | `library/steam.rs` | ✅ `libraryfolders.vdf` + `appmanifest_*.acf` + `librarycache` kapakları — diskten, ağsız (karar #25) |
| Epic kütüphanesi | `library/epic.rs` | ✅ `Manifests/*.item`; `LaunchExecutable` alanı exe'yi doğrudan veriyor |
| Exe adayları | `library/exe.rs` | ✅ Gürültü elemesi + puanlama. Sıra öneri, karar değil |
| VDF/ACF çözümleyici | `library/vdf.rs` | ✅ Hoşgörülü; bozuk dosya kütüphaneyi kaybettirmiyor |
| Exe ikonu | `library/ikon.rs` | ✅ `PrivateExtractIconsW` → GDI → PNG. Kapağı olmayan oyunların görsel kaynağı |
| PNG yazıcı | `library/png.rs` | ✅ Sıkıştırmasız; yeni bağımlılık eklememek için (karar #21) |
| Oyun katalogu | `profile_engine/katalog.rs` | ✅ Gömülü `katalog.json`: exe → oyun adı + rekabetçi bayrağı. Hazır ayar taşımıyor (karar #26) |
| Çeviri önişleme | `ceviri/onisleme.rs` | ✅ BÜYÜK HARF metni cümle düzenine indirir; OCR'ın bozduğundan şüphelenilen yeri **işaretler, düzeltmez** (karar #29 zaaf 1, #28) |
| Terim sözlüğü | `ceviri/sozluk.rs` | ✅ Terimler çeviriden önce çıkarılıp yerlerine işaret konuyor; model terimi hiç görmüyor. İşaretin modelden sağ çıktığı **ölçülmedi** (karar #30) |
| Çeviri belleği | `ceviri/bellek.rs` | ✅ Oyun başına JSON. Makine kaydı kullanıcı kaydını ezemiyor, budama kullanıcı kayıtlarına dokunmuyor (karar #22) |
| OCR dil kontrolü | `ceviri/ocr_dil.rs` | ✅ Kaynak dilin OCR paketi kurulu mu; değilse nasıl kurulacağı söyleniyor. Karar mantığı WinRT'den ayrı, her platformda test ediliyor (karar #28) |
| Ekran yakalama | `scaling/yakalama.rs` | ✅ Desktop Duplication; çok kartlı makinelerde ekranı süren adaptörle açılıyor. Münhasır tam ekranda çalışmaz ve bunu **ne yapılacağını söyleyerek** bildirir |
| Ölçekleme algoritmaları | `scaling/algoritma.rs` | ✅ CPU **referansı** — çalışma zamanında kullanılmıyor. Kalite karşılaştırma testleri burada (karar #32) |
| Ölçekleme gölgelendiricileri | `scaling/olcekleme.hlsl` | ✅ Gerçek zamanlı yol. Tam sayı / bilinear / Lanczos / xBR, `D3DCompile` ile çalışma zamanında derleniyor |
| Sunum penceresi | `scaling/sunum.rs` | ✅ Odak almayan, tıklama geçiren, Alt+Tab'da görünmeyen üstteki pencere + DXGI çevirme zinciri |
| Ölçekleme gecikmesi | `scaling/gecikme.rs` | ✅ Boru hattının **eklediği** süre. İyileşme alanı taşımıyor, testle korunuyor (karar #15, #32) |
| Ölçekleme akışı | `scaling/mod.rs` | ✅ İş parçacığı denetimi, rekabetçi mod kapısı, yakalama denemesi |
| Kare üretimi | `scaling/` | ⬜ Faz 4 — profil dosyasında açılsa bile `dogrula` kapatıyor |

## Neden bazı satırlar ◐

`◐` işaretli modüller "yarım kalmış" değil, **bilinçli olarak dar**. Gerekçeleri
`docs/decisions.md`'de numaralı duruyor. Bir sonraki oturum bunları "eksik iş"
sanıp tamamlamaya çalışmasın; kararı değiştirmek istiyorsa önce gerekçeyi
okusun.

## Test durumu

331 birim testi geçiyor (`cargo test` ve `cargo test --features demo`), arayüz
tarafında 57 test (`npm test` — vitest + jsdom, backend mock’lu).
Testlerin bir kısmı **ürün duruşlarını koruyor**,
yalnızca kodu değil:

| Test | Neyi koruyor |
|---|---|
| `gercek_zamanli_oncelik_ayristirilamiyor` | Profil dosyası sistemi kilitleyecek bir öncelik isteyemez |
| `hicbir_sinif_realtime_degerini_uretmiyor` | Enum'a yanlışlıkla realtime eklenmesi |
| `oyunun_kendisi_dondurulamiyor` | Aracın yapabileceği en kötü hata |
| `varsayilan_liste_bos` | Dondurma listesi varsayılan boş kalır (karar #10) |
| `tunel_destegi_yok` | VPN tüneli eklenmez |
| `placebo_ozellikler_kapali` | Bellek "temizleme" ve MMCSS eklenmez (karar #16) |
| `aciklamalarda_sayisal_vaat_yok` | Arayüz metinlerine sayısal iddia sızmaz (ilke 4) |
| `aciklama_kosullu_dil_kullaniyor` | QoS açıklaması ne zaman işe yaradığını söyler |
| `dosya_adi_yol_kacisini_engelliyor` | Profil kimliği ile klasör dışına yazılamaz |
| `bozuk_defter_programi_kilitlemiyor` | Bozuk defter geri alma arayüzünü kapatamaz |
| `basarisiz_islem_deftere_yazilmiyor` | Yapılmamış değişikliğin geri alma kaydı oluşmaz |
| `isaret_yoksa_karsilastirma_yok` | Uydurulmuş "öncesi" gösterilmez (karar #15) |
| `kisitlarda_zaman_alani_yok` | Demoya zaman sınırı / deneme sayacı girmez (karar #20) |
| `menude_sayisal_vaat_yok` | Tepsi menüsü metinlerine sayısal iddia sızmaz (ilke 4) |
| `cok_gruplu_makinede_p_core_maskesi_verilmiyor` | Yanlış affinite maskesi uygulanmaz |
| `defterde olmayan kayıt için geri al düğmesi göstermiyor` (arayüz) | Geri alınmış değişiklik için düğme gösterilmez |
| `backend düzeltme döndürdüğünde kullanıcıya gösteriyor` (arayüz) | Profil düzeltmeleri sessizce yutulmaz |
| `demo ikilisinde ağ sekmesi hiç görünmüyor` (arayüz) | Demo kapsamı arayüzde de geçerli (karar #20) |
| `yazi_tipi_metniyle_birlikte_listede` | Dağıtılan yazı tipinin OFL metni ikiliden düşmez |
| `listedeki_surumler_cargo_lock_ile_ayni` | Bağımlılık yükseltilince bildirim listesi bayat kalmaz (karar #21) |
| `lisans metni olmayan bileşeni listeden gizlemiyor` (arayüz) | Liste temiz görünsün diye eksiltilmez (karar #21) |
| `taslak_hicbir_sureci_dondurmuyor` | Katalog dondurma listesi ya da güç planı önermez (karar #26) |
| `katalog_notlarinda_sayisal_vaat_yok` | Katalog notlarına sayısal iddia sızmaz (ilke 4) |
| `cok_genel_exe_adi_yok` | Katalog `game.exe` gibi genel bir adla alakasız süreci oyun sanmaz |
| `oyun seçmek profil kaydetmiyor, gerekçeleriyle onay soruyor` (arayüz) | Kütüphaneden gelen profil onaysız kaydedilmez (karar #23, #26) |
| `görsel yalnızca görseli olan oyun için isteniyor` (arayüz) | Izgara açılışta tüm görselleri birden çekmez |
| `raporda_iyilesme_iddiasi_yok` | Geçmiş raporu ölçüm taşır, "iyileşti" demez (ilke 4, karar #31) |
| `varsayilan_gecmis_acik` | Geçmiş kapalı gelirse ancak önceden açmayı akıl eden görür (karar #31) |
| `ölçüm penceresini iki ayrı sütun olarak gösteriyor, oran üretmiyor` (arayüz) | Geçmişte tek bir "şu kadar iyileşti" oranı üretilmez (karar #15, #31) |
| `makine_kullanici_kaydinin_ustune_yazamiyor` | Makine çevirisi kullanıcının onayladığı kaydı ezemez (karar #30) |
| `onbellek_temizligi_kullanici_kayitlarina_dokunmuyor` | Yer açmak için kullanıcının elle düzeltmesi atılmaz (karar #30) |
| `bitisik_kelime_suphesi_duzeltilmeye_calisilmiyor` | OCR şüphesi işaretlenir, tahminle düzeltilmez (karar #28, #29) |
| `dusen_isaret_kayip_olarak_bildiriliyor` | Terim koruma işareti düşerse sessizce yutulmaz (karar #30) |
| `kurulum_metninde_sayisal_vaat_yok` | OCR kurulum yönlendirmesine sayısal iddia sızmaz (ilke 4) |

Bu testlerden biri düşerse, düşüren kişi bir ürün kararını değiştiriyor
demektir. Testi "düzeltmeden" önce kararı tartış.
