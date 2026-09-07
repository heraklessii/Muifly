# Tasarım İlkeleri

> Bu ilkeler pazarlama tercihi değil, mimari zorunluluktur. Claude Code bu
> ilkelerden sapan bir implementasyon önerisiyle karşılaşırsa (kendi önerisi de
> dahil) önce kullanıcıya sormalı, sessizce uygulamamalı.

## 1. Tersine Çevrilebilirlik

Her optimizasyon işlemi geri alınabilir olmalı, istisnasız.

- Process suspend → kapatma değil **dondurma** (`NtSuspendProcess`), oyun kapanınca
  otomatik `NtResumeProcess`.
- Güç planı değişikliği → önceki plan kaydedilir, oyun sonrası geri yüklenir.
- DNS/network ayarları → orijinal değerler kaydedilir, "varsayılana dön" butonu her
  zaman erişilebilir olmalı.
- Registry değişiklikleri (TCP tuning gibi) → değişiklik öncesi değer loglanır ve
  geri yükleme fonksiyonu birlikte yazılır, asla tek yönlü değil.

**Gerekçe**: Kullanıcı güveni bu araç için hayati. Rakiplerin çoğu (özellikle
"PC temizleyici" kategorisindekiler) kalıcı ve açıklanmamış değişiklikler yaparak
güven kaybediyor.

## 2. Şeffaflık

Program ne yaptığını her zaman göstermeli.

- Her aksiyon (process durduruldu, DNS değişti, güç planı değişti) `monitor`
  modülüne log event'i olarak gönderilir ve UI'da görünür olur.
- "Arka planda sessizce çalışan kara kutu" olmamalı — kullanıcı istediğinde tam
  aktivite geçmişini görebilmeli.
- Öncesi/sonrası ölçülebilir kanıt (FPS, ping, jitter grafiği) sunulmalı; rakiplerin
  çoğu bunu sunmuyor ve kullanıcı "gerçekten iyileşti mi" tahmin etmek zorunda kalıyor.

**Kaynak açık olması bunun yerine geçmez.** Muifly açık kaynaktır
(`DISTRIBUTION.md`, Apache 2.0) ama buradaki şeffaflık ayrı bir iş:
programın çalışırken ne yaptığını göstermesi. Kimse bir aracı her
çalıştırmadan önce kaynağını okumuyor; günlük olmadan kaynak da yetmez.

## 3. Anti-Cheat Güvenliği

**Kesin kural**: Process injection, DLL injection, memory hooking, oyun sürecine
herhangi bir şey enjekte etme YASAK.

- Sadece resmi/dokümante Windows API'leri kullanılır: `SetPriorityClass`,
  `SetProcessAffinityMask`, `NtSuspendProcess`/`NtResumeProcess` (yaygın kullanılan
  ama gayrı resmi — bilinen risk, `RISKS.md`'de not edilmiştir), `powercfg`,
  QoS Packet Scheduler, ETW (kare ölçümü).
- Oyunun kare süresi ETW ile **dışarıdan** okunur — oyun sürecine dokunulmaz,
  sistemin kendi olay akışı dinlenir. Bu, DLL injection'dan mimari olarak
  tamamen farklıdır ve anti-cheat sistemleri tarafından genellikle sorun
  olarak görülmez (kesin garanti verilemez, kullanıcıya bu netlikle
  iletilmeli).
- Overlay ihtiyacı DirectX hook gerektiriyorsa yapılmaz; alternatif olarak
  ayrı bir pencere/widget tercih edilir.

**Gerekçe**: Kullanıcının hesabının banlanması, bir "performans aracı" için kabul
edilemez bir risktir. Bu ilke pazarlık konusu değildir.

## 4. Sayısal Vaat Yok

- "Ping'i 20ms düşürür", "FPS'i %50 artırır" gibi garanti edilemeyen sabit sayısal
  iddialar UI metninde, pazarlama metninde veya log mesajlarında KULLANILMAZ.
- Bunun yerine: "jitter'ı azaltır", "gecikmeyi optimize eder", "tespit edilen en
  hızlı DNS'e yönlendirir" gibi doğrulanabilir, süreç odaklı dil kullanılır.
- Kullanıcıya gösterilen öncesi/sonrası ölçüm kendi sonucudur — bu, iddia değil,
  o oturuma özel gerçek veridir ve böyle sunulur.

**Gerekçe**: Ağ gecikmesi büyük ölçüde ISP/mesafe/routing altyapısına bağlıdır ve
program bunu garanti edemez. Yanlış vaat hem teknik olarak yanlış hem de güven kırıcı.

## 5. Minimum ve Açık Admin Yetkisi

- Arka plan izleme servisi sürekli admin/UAC yükseltilmiş olarak ÇALIŞMAZ.
- Tam yetki gerektiren işlemler (process suspend, priority, power plan) sadece o an,
  UAC promptu ile ve kullanıcıya "bu neden isteniyor" açıklamasıyla istenir.
- "Bu program her zaman tam sistem yetkisiyle arka planda çalışıyor" hissi
  yaratılmamalı.

**Gerekçe**: Güvenlik yüzeyini küçültmek ve kullanıcı güvenini korumak.

## Faz Disiplini

Bir faz stabil ve kullanıcı tarafından **sahada** doğrulanmadan bir sonrakine
geçilmez. Sıra atlanmaz — bkz. `ROADMAP.md`.

Bu kuralın bir kez çiğnenmesinin bedeli ödendi: Faz 1 saha testi yapılmadan
Faz 3 ve 4 yazıldı (karar #33), ikisi de gözle denendiğinde yeterince iyi
çıkmadı ve karar #39'la tamamen silindi. Yazılan kod kadar, o kod için
verilmeyen saha testi de kayıptı.
