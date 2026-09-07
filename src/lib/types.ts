/**
 * Backend ile paylaşılan tipler.
 *
 * Rust karşılıkları `serde` ile serileştiriliyor; buradaki alan adları
 * onlarla birebir aynı olmak zorunda. İlgili dosyalar:
 * `src-tauri/src/state.rs`, `ledger.rs`, `monitor/`, `profile_engine/`.
 *
 * Not: profil şeması `#[serde(rename_all = "snake_case")]` kullanıyor çünkü
 * profil dosyaları kullanıcının elle düzenlediği dosyalar ve
 * `docs/PROFILES.md`'deki taslakla aynı görünmeleri gerekiyor. Diğer tipler
 * camelCase.
 */

// ---------------------------------------------------------------------------
// Günlük
// ---------------------------------------------------------------------------

export type Duzey = 'aksiyon' | 'geriAlma' | 'bilgi' | 'uyari' | 'hata';
export type Kategori = 'sistem' | 'ag' | 'profil' | 'olcum' | 'uygulama';

export interface Satir {
  id: number;
  /** Unix milisaniye. */
  zaman: number;
  duzey: Duzey;
  kategori: Kategori;
  mesaj: string;
  /** Doluysa satırın yanında "geri al" düğmesi gösteriliyor. */
  geriAlmaId: number | null;
}

export const DUZEY_ETIKETLERI: Record<Duzey, string> = {
  aksiyon: 'Yapıldı',
  geriAlma: 'Geri alındı',
  bilgi: 'Bilgi',
  uyari: 'Uyarı',
  hata: 'Hata',
};

// ---------------------------------------------------------------------------
// Ölçüm
// ---------------------------------------------------------------------------

export interface Ornek {
  zaman: number;
  cpu: number;
  bellek: number;
  /** Ölçüm yapılamadıysa `null` — sıfır DEĞİL. */
  gecikmeMs: number | null;
}

export interface Ozet {
  ornekSayisi: number;
  cpuOrt: number;
  bellekOrt: number;
  gecikmeOrtMs: number | null;
  jitterMs: number | null;
  kayipYuzde: number;
}

export interface Karsilastirma {
  onceki: Ozet;
  sonraki: Ozet;
  oncekiSaniye: number;
  sonrakiSaniye: number;
}

// ---------------------------------------------------------------------------
// Geri alma defteri
// ---------------------------------------------------------------------------

export type Kapsam = 'oturum' | 'kalici';

export type Undo =
  | { tur: 'surecOnceligi'; pid: number; surec: string; onceki: number }
  | { tur: 'surecAffinite'; pid: number; surec: string; onceki: number }
  | { tur: 'surecDonduruldu'; pid: number; surec: string }
  | { tur: 'gucPlani'; oncekiGuid: string; oncekiAd: string }
  | { tur: 'registryDword'; yol: string; ad: string; onceki: number | null }
  | { tur: 'registryAnahtari'; yol: string };

export interface Kayit {
  id: number;
  zaman: number;
  ozet: string;
  kapsam: Kapsam;
  undo: Undo;
}

// ---------------------------------------------------------------------------
// Modlar
// ---------------------------------------------------------------------------

export type Mod =
  | { mod: 'sistemAcilisi' }
  | { mod: 'bosta' }
  | { mod: 'oyunGenel'; pid: number; surec: string }
  | { mod: 'oyunProfili'; pid: number; surec: string; profilId: string }
  | { mod: 'rekabetci'; pid: number; surec: string; profilId: string | null };

/** Mod rozetinin rengi için: boşta / oyun / rekabetçi. */
export function modRengi(m: Mod): 'bosta' | 'oyun' | 'rekabetci' {
  if (m.mod === 'rekabetci') return 'rekabetci';
  if (m.mod === 'bosta' || m.mod === 'sistemAcilisi') return 'bosta';
  return 'oyun';
}

export function modSureci(m: Mod): string | null {
  return 'surec' in m ? m.surec : null;
}

// ---------------------------------------------------------------------------
// Profiller (snake_case — kullanıcının düzenlediği dosya biçimi)
// ---------------------------------------------------------------------------

export type AffiniteTercihi = 'dokunma' | 'sadece_p_core';

export interface SistemBolumu {
  priority_class: string;
  cpu_affinity: AffiniteTercihi;
  suspend_process_list: string[];
  suspend_whitelist_exempt: string[];
  power_plan: string | null;
}

export interface AgBolumu {
  preferred_dns: string | null;
  qos_priority: boolean;
  tcp_nodelay: boolean;
}

export interface Profil {
  profile_id: string;
  display_name: string;
  executable_names: string[];
  competitive: boolean;
  system: SistemBolumu;
  network: AgBolumu;
  created_by: string;
  shared: boolean;
}

/** Yeni profil iskeleti — backend'deki `Profil::yeni` ile aynı varsayılanlar. */
export function bosProfil(): Profil {
  return {
    profile_id: '',
    display_name: '',
    executable_names: [],
    competitive: false,
    system: {
      priority_class: 'high',
      cpu_affinity: 'dokunma',
      suspend_process_list: [],
      suspend_whitelist_exempt: [],
      power_plan: null,
    },
    network: { preferred_dns: null, qos_priority: false, tcp_nodelay: false },
    created_by: 'user',
    shared: false,
  };
}

/**
 * İçe aktarma önizlemesi — `src-tauri/src/profile_engine/aktarim.rs`.
 *
 * Önizleme diske hiçbir şey yazmıyor: kullanıcı ne geldiğini görmeden
 * kaydetme adımı çalışmıyor (`docs/PROFILES.md` güvenlik notu).
 */
export interface Onizleme {
  profil: Profil;
  dosya: string;
  /** Doğrulamanın dosyada değiştirdikleri. */
  duzeltmeler: string[];
  /** Bu profil uygulanınca ne olacağı. */
  etkiler: string[];
  /** Okunmadan geçilmemesi gereken maddeler. */
  uyarilar: string[];
  kimlikCakismasi: boolean;
  /** Aynı oyunu hedefleyen mevcut profillerin görünen adları. */
  cakisanProfiller: string[];
  /** Üzerine yazılmazsa kullanılacak kimlik. */
  bosKimlik: string;
}

export const ONCELIK_SECENEKLERI: { deger: string; etiket: string }[] = [
  { deger: 'high', etiket: 'Yüksek' },
  { deger: 'above_normal', etiket: 'Normalin üstü' },
  { deger: 'normal', etiket: 'Normal' },
];

export const GUC_PLANI_SECENEKLERI: { deger: string | null; etiket: string }[] = [
  { deger: null, etiket: 'Dokunma' },
  { deger: 'high_performance', etiket: 'Yüksek performans' },
  { deger: 'ultimate_performance', etiket: 'Üstün performans (varsa)' },
  { deger: 'balanced', etiket: 'Dengeli' },
];

// ---------------------------------------------------------------------------
// Durum ve ayarlar
// ---------------------------------------------------------------------------

export interface Ayarlar {
  otomatikUygula: boolean;
  oyunCikisGecikmesiSn: number;
  olcumAraligiSn: number;
  gecikmeHedefi: string;
  gecikmeOlcumu: boolean;
  rekabetciMod: boolean;
  /** Mod değiştiğinde masaüstü bildirimi. Varsayılan kapalı. */
  modBildirimi: boolean;
  tepsiyeKucult: boolean;
  tema: 'dark' | 'light';
  /** Biten oyun oturumlarını diske kaydet. Varsayılan açık. */
  gecmisTut: boolean;
}

export interface Durum {
  mod: Mod;
  modAdi: string;
  ayarlar: Ayarlar;
  bekleyenGeriAlma: number;
  kaliciDegisiklik: number;
  dondurmaDestegi: boolean;
  yonetici: boolean;
  cpuHibrit: boolean;
  mantiksalCekirdek: number;
  gucPlani: string | null;
  otomatikBaslatma: boolean;
}

export interface UygulamaSonucu {
  uygulanan: string[];
  atlanan: string[];
  hatalar: string[];
}

/**
 * Kare ölçümünün bu makinede yapılabilirliği — `monitor::olcum`.
 *
 * `aciklama` backend'den geliyor (karar #17) ve UAC istemi ÇIKMADAN ÖNCE
 * gösteriliyor: kullanıcı neden yetki istendiğini istemden önce okumalı.
 */
export interface KareOlcumDurumu {
  kullanilabilir: boolean;
  yetkiGerekiyor: boolean;
  enKisaSaniye: number;
  enUzunSaniye: number;
  aciklama: string;
}

/** Bir ölçüm penceresinin kare özeti — `monitor::frames::KareOzeti`. */
export interface KareOzeti {
  kareSayisi: number;
  sureS: number;
  ortFps: number;
  ortMs: number;
  /** En kötü %1 karenin ortalama süresi. */
  p1KotuMs: number;
  p1KotuFps: number;
  /** Ardışık kare süreleri arasındaki ortalama mutlak fark. */
  kareJitterMs: number;
}

/** Özet `null` olabilir: oturum açıldı ama özet çıkaracak kadar kare gelmedi. */
export interface KareSonucu {
  ozet: KareOzeti | null;
  kareSayisi: number;
}

export interface OlcumRaporu {
  surec: string;
  pid: number;
  saniye: number;
  sonuc: KareSonucu;
}

// ---------------------------------------------------------------------------
// Oturum geçmişi — `src-tauri/src/monitor/gecmis.rs`
// ---------------------------------------------------------------------------

/**
 * Biten bir oturumun kaydı.
 *
 * `onceki`/`sonraki` iki AYRI özet olarak geliyor ve öyle de gösteriliyor:
 * tek bir "iyileşme oranı" backend'de de üretilmiyor (karar #15).
 */
export interface OturumKaydi {
  id: number;
  baslangic: number;
  bitis: number;
  surec: string;
  /** Katalogdan ya da profilden gelen okunur ad. Bilinmiyorsa `null`. */
  oyunAdi: string | null;
  profilAdi: string | null;
  modAdi: string;
  uygulanan: string[];
  geriAlinan: number;
  onceki: Ozet | null;
  sonraki: Ozet | null;
  kare: KareOzeti | null;
}

export interface EnCok {
  ad: string;
  sureSn: number;
  oturum: number;
}

export interface GecmisOzeti {
  oturumSayisi: number;
  toplamSureSn: number;
  toplamDegisiklik: number;
  olculenOturum: number;
  enCok: EnCok | null;
}

/** Kayıtta gösterilecek ad: oyun adı bilinmiyorsa exe. */
export function oturumAdi(k: OturumKaydi): string {
  return k.oyunAdi ?? k.surec;
}

/** Oturumun süresi, saniye. Negatif olamaz (saat geri alınmış makine). */
export function oturumSuresiSn(k: OturumKaydi): number {
  return Math.max(0, Math.round((k.bitis - k.baslangic) / 1000));
}

// ---------------------------------------------------------------------------
// Ağ
// ---------------------------------------------------------------------------

export interface DnsSonucu {
  ad: string;
  adres: string;
  ortalamaMs: number | null;
  cevap: number;
  deneme: number;
  /** Sistemin şu an kullandığı çözümleyici mi? */
  sisteminKullandigi: boolean;
}

export interface YolDugumu {
  atlama: number;
  adres: string | null;
  gecikmeMs: number | null;
}

export interface YolSonucu {
  hedef: string;
  dugumler: YolDugumu[];
  ulasildi: boolean;
}

export interface TcpDurumu {
  nagleKapaliArayuz: number;
  toplamArayuz: number;
  throttlingIndex: number | null;
  sistemYanit: number | null;
  yoneticiGerekiyor: boolean;
}

export interface QosIlkesi {
  ad: string;
  uygulama: string | null;
  dscp: number | null;
  bizim: boolean;
}

export interface Surec {
  pid: number;
  ad: string;
}

/* ---------------------------------------------------------------------------
 * Oyun kütüphanesi
 *
 * Veri `src-tauri/src/library/` içinden geliyor ve tamamı YEREL: Steam'in
 * disk üzerindeki manifest ve görsel önbelleği, Epic'in manifest dosyaları,
 * exe'lerin kendi ikonları. Hiçbir ağ isteği yok (karar #25).
 * ------------------------------------------------------------------------- */

export type OyunKaynagi = 'steam' | 'epic' | 'calisan' | 'elle';

export const KAYNAK_ETIKETLERI: Record<OyunKaynagi, string> = {
  steam: 'Steam',
  epic: 'Epic Games',
  calisan: 'Çalışıyor',
  elle: 'Elle eklendi',
};

export interface Oyun {
  /** `"steam:431960"`, `"epic:…"`, `"elle:oyun.exe"`. */
  kimlik: string;
  ad: string;
  kaynak: OyunKaynagi;
  kurulum: string;
  /** Aday exe adları, en olası önde. */
  exeler: string[];
  /** Görsel var mı? Görselin kendisi kart görününce ayrıca isteniyor. */
  gorselVar: boolean;
}

/** Gömülü katalogda tanınan bir oyun (`src-tauri/katalog.json`). */
export interface KatalogGirdisi {
  exe: string;
  ad: string;
  rekabetci: boolean;
  not: string | null;
}

/**
 * Kütüphaneden üretilen profil taslağı.
 *
 * `aciklamalar` taslağın NEDEN böyle olduğunu anlatıyor ve gösterilmesi
 * şart: hazır gelen bir profilin gerekçesi görünmezse kara kutu olur
 * (şeffaflık ilkesi).
 */
export interface Taslak {
  profil: Profil;
  aciklamalar: string[];
}

/* ---------------------------------------------------------------------------
 * Üçüncü taraf bildirimleri
 *
 * Veri `src-tauri/ucuncu-taraf.json`'dan geliyor; alan adları
 * `ucuncu_taraf.rs` ile birebir aynı.
 * ------------------------------------------------------------------------- */

/** `rust` ikiliye linklenen crate, `npm` dist'e giren paket, `varlik` gömülü dosya. */
export type UcuncuTarafTuru = 'rust' | 'npm' | 'varlik';

export interface UcuncuTarafBileseni {
  ad: string;
  surum: string;
  tur: UcuncuTarafTuru;
  /** SPDX ifadesi. Paket bildirmediyse `null`. */
  lisans: string | null;
  kaynak: string | null;
  /** `api.ucuncuTarafMetni` için indeks. `null` = paket metin yayımlamamış. */
  metinNo: number | null;
}

export interface UcuncuTarafListesi {
  uretildi: string;
  hedef: string;
  bilesenler: UcuncuTarafBileseni[];
}
