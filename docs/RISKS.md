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
  - Overlay'i oyun içine değil, ayrı bir her-zaman-üstte pencere/widget olarak sun.
  - Bu karar netleşmeden overlay implementasyonuna başlanmamalı.
- **Kabul edilen sonuç** (karar #22): Ayrı üst pencere seçildi. Bunun bedeli,
  overlay'in **exclusive fullscreen'de çalışmamasıdır**; kenarlıksız modda
  çalışır. Bu bir hata değil, seçilen mimarinin doğal sınırı — mağaza sayfasında
  ve özelliğin kendi ekranında baştan yazılmalı, kullanıcı satın aldıktan sonra
  keşfetmemeli.

## Anti-Cheat ile Etkileşim (Scaling Modülü)

- Desktop Duplication API ile ekran yakalama, oyun process'ine dokunmadığı için
  düşük risklidir ama sıfır risk garantisi verilemez — bazı agresif anti-cheat'ler
  ekran yakalama API çağrılarını genel olarak izleyebilir.
- **Azaltma**: Kullanıcıya bu belirsizlik netçe iletilmeli (Lossless Scaling'in de
  yaptığı gibi: "topluluk güvenli buluyor ama garanti edilemez").

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

## Yerel Model Boyutu ve Kalitesi (Ekran Çevirisi, Faz 5)

- Yerel bir çeviri modeli kabaca 100–300 MB disk ve yüklüyken yüzlerce MB RAM
  demek. Karar #1 Electron'u tam da bu gerekçeyle elemişti; aynı argüman bu kez
  ürünün kendisine karşı çalışır.
- **Azaltma**: Model ikiliye gömülmez, isteğe bağlı indirilir ve boştayken
  bellekten düşürülür (karar #22). OCR tarafında bedel yok —
  `Windows.Media.Ocr` işletim sisteminde hazır.
- İkinci ve daha sinsi risk **kalite**: yerel küçük modeller oyun diyaloğunda
  (deyim, fantezi terminolojisi, kısa UI parçaları) vasat kalır ve kullanıcının
  kıyas noktası DeepL'dir. Bu, özelliği en çok tehdit eden şey ve en az kontrol
  edilebilen şey.
- **Azaltma**: Faz 5 açılmadan önce gerçek oyun diyaloğuyla ölçülmeli
  (`ROADMAP.md` → Faz 5, fizibilite soruları). Ayrıca çeviri belleği, doğrusu
  bir kez girildiğinde modelin o metindeki zayıflığını kalıcı olarak devre dışı
  bırakır — kalite sorununun tek yapısal panzehiri bu.
- Sayısal vaat yasağı (ilke 4) burada da geçerli: "%X doğruluk" gibi bir iddia
  kullanılmaz.

## Sayısal Vaat Riski (Pazarlama/UI Metni)

- Geliştirme sırasında "kolay satış" cazibesiyle sayısal iddialar UI'a sızabilir
  (örn. bir geliştirici yorumu "bu %20 hızlandırır" gibi bir string yazabilir).
- **Azaltma**: UI metin review'unda `DESIGN_PRINCIPLES.md` madde 4'e karşı kontrol
  edilmeli, bu bir stil tercihi değil kural.
