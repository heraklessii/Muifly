# etw-sonda — atılacak fizibilite denemesi

Bu **ürün kodu değil**. Karar #14/#27'nin dayandığı ölçümü tekrar
üretilebilir kılmak için duruyor. Sorusu cevaplandıktan sonra silinebilir.

Ürünün kendi ETW kodu `src-tauri/src/monitor/etw.rs`'de; buradaki kopya
kasten bağımsız, çünkü sondanın ana ikiliyi derlemeden ve Tauri'yi
başlatmadan koşabilmesi gerekiyor.

## Ne soruyor

1. Yükseltilmemiş bir süreç gerçek zamanlı ETW oturumu açabiliyor mu?
   **Cevaplandı: hayır** — `StartTraceW` → `ERROR_ACCESS_DENIED (5)`.
   Karar #27 bu ölçüme dayanıyor.
2. Yükseltilmiş bir süreç, **başka** bir sürecin DXGI/D3D9 Present
   olaylarını görüyor mu? — açık.
3. Present aralıklarından anlamlı bir kare süresi çıkıyor mu? — açık.

## Nasıl çalıştırılır

```
cargo build --release
```

Sonra **yönetici olarak açılmış** bir terminalde, ekranda 3B sunum yapan bir
uygulama açıkken:

```
target\release\etw-sonda.exe 15
```

Argüman kaç saniye dinleneceği (varsayılan 10). Çıktı, süreç başına sunum
sayısı, ortalama FPS, ortalama kare süresi ve en kötü %1.

## Neyi kanıtlamaz

Sonda PID süzgeci **kullanmıyor**, bütün süreçleri dinliyor — kapsamı görmek
için bilerek böyle. Ürün kodu tam tersini yapıyor (`saglayicilari_ac`,
`EVENT_FILTER_TYPE_PID`): ölçümün maliyetini ölçülen oyunun üstünden
kaldırmak gerekiyor. Yani buradaki olay hacmi ürünün hacmi değil.
