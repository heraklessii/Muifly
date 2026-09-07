# Bilinen Riskler

> Bu liste "yapılmasın" demek değil, "bilerek ve dikkatli yapılsın" demek.
> Yeni bir risk fark edildiğinde buraya eklenmeli.

## Gayrı Resmi API Kullanımı — `NtSuspendProcess` / `NtResumeProcess`

- Bu fonksiyonlar `ntdll.dll` içinde ama resmi olarak dokümante edilmemiş.
- Yaygın kullanılıyor (birçok process manager aracı kullanıyor) ama Microsoft
  gelecekte davranışını değiştirebilir.
- **Azaltma**: Suspend/resume mantığını test edilebilir, izole bir modülde tut;
  Windows güncellemesi sonrası davranış değişirse tek noktadan güncellenebilsin.

## `REALTIME_PRIORITY_CLASS` Kullanma

- Process önceliklendirmede `HIGH_PRIORITY_CLASS` kullanılmalı, `REALTIME_PRIORITY_CLASS`
  DEĞİL. Realtime öncelik, sistem servislerini (fare/klavye sürücüleri dahil) aç
  bırakarak sistem donmasına/çökmesine yol açabilir.

## CPU Affinity ve Hibrit CPU'lar

- Intel 12. nesil ve sonrası (P-core/E-core) sistemlerde yanlış affinity ataması
  performansı artıracağına düşürebilir.
- **Azaltma**: Affinity ayarını varsayılan kapalı yap, sadece kullanıcı açıkça
  etkinleştirirse uygula; CPU topolojisini doğru tespit et (yanlış tespit riski var).

## Overlay için DirectX Hook İhtiyacı

- FPS overlay'i doğru yapmak genelde DirectX/OpenGL/Vulkan çağrılarını hook'lamayı
  gerektirir (PresentMon'un yaptığı gibi).
- Hook'lama, `DESIGN_PRINCIPLES.md`'deki "injection yok" ilkesiyle gerilim yaratır.
- **Azaltma seçenekleri**:
  - PresentMon'un kullandığı ETW (Event Tracing for Windows) tabanlı, hook'suz FPS
    ölçüm yöntemini araştır — bu, process'e dokunmadan sistem event'lerinden veri
    okur, daha güvenli bir alternatif.
    **Araştırıldı (karar #27)**: yol açık ama bedeli var — gerçek zamanlı ETW
    oturumu yükseltilmiş yetki istiyor (`ERROR_ACCESS_DENIED`, ölçüldü). Bu
    yüzden ölçüm sürekli değil, kullanıcının başlattığı süreli bir pencere.
  - Oyun içi overlay hiç yapılmaz; ölçüm Muifly penceresinde gösterilir.
- **Kabul edilen sonuç**: oyun içi overlay yok. Kare ölçümü ETW ile
  dışarıdan okunuyor ve sonucu uygulamanın kendi penceresinde duruyor.
  Karar #39 ile üstte duran pencerelerin hepsi (ölçekleme sunumu, çeviri
  overlay'i) kaldırıldı; geriye hook ihtiyacı olan bir yol kalmadı.

## Registry / TCP Tuning Geri Alma

- TCP/IP stack ayarları (Nagle, ACK frequency) registry üzerinden yapılıyorsa,
  yanlış geri alma sistemin network ayarlarını bozabilir.
- **Azaltma**: Her değişiklik öncesi orijinal registry değerini oku ve sakla,
  değişiklik ve geri alma fonksiyonlarını birlikte yaz ve test et, asla sadece
  "yeni değeri yaz" — eski değeri de mutlaka kaydet.

## Standby List / Bellek Temizleme

- Agresif bellek/standby list temizleme, temizleme anında kısa süreli performans
  düşüşüne (yeniden yükleme maliyeti) yol açabilir.
- **Azaltma**: Varsayılan kapalı, opsiyonel özellik olarak sun.

## Sayısal Vaat Riski (Pazarlama/UI Metni)

- Geliştirme sırasında "kolay satış" cazibesiyle sayısal iddialar UI'a sızabilir
  (örn. bir geliştirici yorumu "bu %20 hızlandırır" gibi bir string yazabilir).
- **Azaltma**: UI metin review'unda `DESIGN_PRINCIPLES.md` madde 4'e karşı kontrol
  edilmeli, bu bir stil tercihi değil kural.

## Kapsam Genişlemesi

- Bu dosyanın en pahalı riski kodda değil planda çıktı: proje sistem ve ağ
  tarafı sahada doğrulanmadan görüntü ölçekleme (Faz 3), kare üretimi
  (Faz 4a) ve ekran çevirisi (Faz 5) yazdı. Üçü de kod olarak bitti,
  üçü de yeterince iyi çalışmadı ve üçü de karar #39'la silindi.
- **Azaltma**: `DESIGN_PRINCIPLES.md` → Faz Disiplini. Bir faz sahada
  doğrulanmadan bir sonrakine geçilmez. Bu kural zaten yazılıydı; eksik
  olan uygulanmasıydı.
