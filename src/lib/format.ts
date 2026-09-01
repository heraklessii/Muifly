/**
 * Sayı ve zaman biçimlendirme.
 *
 * Tek kural: **ölçüm yoksa sıfır yazılmıyor.** `null` gelen bir gecikme
 * değeri "0 ms" olarak gösterilirse kullanıcı mükemmel bir bağlantı gördüğünü
 * sanır; oysa ölçüm hiç yapılamamıştır. Bu dosyadaki fonksiyonlar bu ayrımı
 * koruyor ve boş değer için tire döndürüyor (`DESIGN_PRINCIPLES.md` madde 4).
 */

/** Ölçüm yokluğunu gösteren işaret. */
export const BOS = '—';

export function yuzde(deger: number | null | undefined, basamak = 0): string {
  if (deger == null || !Number.isFinite(deger)) return BOS;
  return `${deger.toFixed(basamak)}%`;
}

export function milisaniye(deger: number | null | undefined, basamak = 0): string {
  if (deger == null || !Number.isFinite(deger)) return BOS;
  return `${deger.toFixed(basamak)} ms`;
}

/** Birimsiz sayı — birim ayrı bir `<span>`da gösterilecekse. */
export function sayi(deger: number | null | undefined, basamak = 0): string {
  if (deger == null || !Number.isFinite(deger)) return BOS;
  return deger.toFixed(basamak);
}

export function saat(zamanMs: number): string {
  const d = new Date(zamanMs);
  return d.toLocaleTimeString('tr-TR', {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  });
}

/**
 * Saniyeyi okunur süreye çevirir.
 *
 * Saat eşiğinde saniye düşüyor: "2 sa 14 dk 37 sn" okunmuyor ve bir oyun
 * oturumunun uzunluğunda saniyenin bilgi değeri yok. Rust tarafındaki
 * `monitor::gecmis::sure_metni` aynı eşikleri kullanıyor — rapor dosyası ile
 * ekrandaki değer ayrışmasın.
 */
export function sure(saniye: number): string {
  if (!Number.isFinite(saniye) || saniye <= 0) return BOS;
  if (saniye < 60) return `${Math.round(saniye)} sn`;
  const dakika = Math.floor(saniye / 60);
  if (dakika >= 60) return `${Math.floor(dakika / 60)} sa ${dakika % 60} dk`;
  const kalan = Math.round(saniye % 60);
  return kalan === 0 ? `${dakika} dk` : `${dakika} dk ${kalan} sn`;
}

/** Tarih + saat, yerel biçimde. Geçmiş listesinde kullanılıyor. */
export function tarihSaat(zamanMs: number): string {
  return new Date(zamanMs).toLocaleString('tr-TR', {
    day: '2-digit',
    month: '2-digit',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

/**
 * İki ölçümü karşılaştıran nötr cümle.
 *
 * "%12 iyileşme" gibi bir oran ÜRETİLMİYOR: ölçüm koşulları iki pencerede
 * aynı değil (oyun içi yük değişiyor) ve tek bir orana indirgemek yanıltıcı
 * olurdu. Yön belirtiliyor, büyüklük iki değer yan yana gösterilerek
 * kullanıcıya bırakılıyor.
 */
export function yon(onceki: number | null, sonraki: number | null): 'dustu' | 'artti' | 'ayni' | null {
  if (onceki == null || sonraki == null) return null;
  const fark = sonraki - onceki;
  // Ölçüm gürültüsü eşiği: bu bandın altındaki fark "aynı" sayılıyor.
  if (Math.abs(fark) < 0.5) return 'ayni';
  return fark < 0 ? 'dustu' : 'artti';
}

export const YON_ETIKETLERI: Record<'dustu' | 'artti' | 'ayni', string> = {
  dustu: 'düştü',
  artti: 'arttı',
  ayni: 'değişmedi',
};
