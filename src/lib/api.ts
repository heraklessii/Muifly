/**
 * Backend köprüsü — `invoke` çağrılarının tek toplandığı yer.
 *
 * Bileşenler `invoke('profil_uygula', ...)` gibi dizeler taşımıyor; komut
 * adları burada bir kez yazılıyor. Komut adı değişirse tek dosya güncelleniyor
 * ve TypeScript geri kalanı yakalıyor.
 */

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { open as openDialog, save as saveDialog } from '@tauri-apps/plugin-dialog';
import { openUrl } from '@tauri-apps/plugin-opener';

import type {
  Ayarlar,
  DnsSonucu,
  Durum,
  Karsilastirma,
  Kayit,
  KatalogGirdisi,
  Kisitlar,
  Onizleme,
  Ornek,
  Oyun,
  Ozet,
  Profil,
  QosIlkesi,
  Satir,
  Surec,
  Taslak,
  TcpDurumu,
  UcuncuTarafListesi,
  UygulamaSonucu,
  YolSonucu,
} from './types';

/** `src-tauri/src/commands.rs` içindeki sabitlerle birebir aynı olmak zorunda. */
export const OLAY_DURUM = 'muifly://durum';
export const OLAY_GUNLUK = 'muifly://gunluk';
export const OLAY_ORNEK = 'muifly://ornek';

export const surum = () => invoke<string>('surum');
/** Demo/tam sürüm ayrımı. Açılışta bir kez okunuyor; çalışırken değişmez. */
export const kisitlar = () => invoke<Kisitlar>('kisitlar');
export const durum = () => invoke<Durum>('durum');

export const gunluk = (adet?: number) => invoke<Satir[]>('gunluk', { adet: adet ?? null });
export const gunlugu_temizle = () => invoke<void>('gunlugu_temizle');

export const ornekler = () => invoke<Ornek[]>('ornekler');
export const ozet = () => invoke<Ozet>('ozet');
export const karsilastirma = () => invoke<Karsilastirma | null>('karsilastirma');

export const bekleyenGeriAlmalar = () => invoke<Kayit[]>('bekleyen_geri_almalar');
export const geriAl = (id: number) => invoke<void>('geri_al', { id });
export const hepsiniGeriAl = () => invoke<UygulamaSonucu>('hepsini_geri_al');

export const profiller = () => invoke<Profil[]>('profiller');
/** Dönen liste: backend'in profile uyguladığı düzeltmeler. Boşsa her şey yolunda. */
export const profilKaydet = (profil: Profil) => invoke<string[]>('profil_kaydet', { profil });
export const profilSil = (kimlik: string) => invoke<void>('profil_sil', { kimlik });
/** Kayıtlı bir profilin ne yapacağı — önizlemedeki metinlerin aynısı. */
export const profilEtkileri = (kimlik: string) =>
  invoke<string[]>('profil_etkileri', { kimlik });
export const profilDisaAktar = (kimlik: string, yol: string) =>
  invoke<void>('profil_disa_aktar', { kimlik, yol });
/** Dosyayı okur ve ne yapacağını anlatır — diske hiçbir şey yazmaz. */
export const profilOnizle = (yol: string) => invoke<Onizleme>('profil_onizle', { yol });
/** Önizlenen profili kaydeder. Dönen liste: doğrulamanın yaptığı düzeltmeler. */
export const profilIceAktar = (profil: Profil, uzerineYaz: boolean) =>
  invoke<string[]>('profil_ice_aktar', { profil, uzerineYaz });
export const profilUygula = (kimlik: string) =>
  invoke<UygulamaSonucu>('profil_uygula', { kimlik });
export const ondekineUygula = () => invoke<UygulamaSonucu>('ondekine_uygula');
export const oturumuKapat = () => invoke<number>('oturumu_kapat');

export const surecler = () => invoke<Surec[]>('surecler');
export const dondurmaAdaylari = () => invoke<string[]>('dondurma_adaylari');

/**
 * Oyun kütüphanesi. Hepsi yerel okuma — ağ isteği yok (karar #25).
 *
 * Tarama saniyeler sürebiliyor (kurulum klasörleri geziliyor), bu yüzden
 * backend tarafı ayrı iş parçacığında koşuyor.
 */
export const oyunlariTara = () => invoke<Oyun[]>('oyunlari_tara');
/** Kapak görseli ya da exe ikonu, `data:` adresi olarak. Yoksa `null`. */
export const oyunGorseli = (kimlik: string) =>
  invoke<string | null>('oyun_gorseli', { kimlik });
/** Kullanıcının seçtiği bir .exe'yi kütüphaneye ekler. */
export const oyunElleEkle = (yol: string) => invoke<Oyun>('oyun_elle_ekle', { yol });
/** Profil taslağı üretir. Hiçbir şey kaydetmez, hiçbir şey uygulamaz. */
export const profilTaslagi = (ad: string, exeler: string[]) =>
  invoke<Taslak>('profil_taslagi', { ad, exeler });
/** Bir exe gömülü katalogda tanınıyor mu? */
export const katalogGirdisi = (exe: string) =>
  invoke<KatalogGirdisi | null>('katalog_girdisi', { exe });
/** Şu an çalışan süreçlerden katalogda tanınan oyunlar. */
export const taninanSurecler = () => invoke<KatalogGirdisi[]>('taninan_surecler');

export const dnsKarsilastir = (deneme?: number) =>
  invoke<DnsSonucu[]>('dns_karsilastir', { deneme: deneme ?? null });
export const yolTesti = (hedef: string) => invoke<YolSonucu>('yol_testi', { hedef });
export const tcpDurumu = () => invoke<TcpDurumu>('tcp_durumu');
export const tcpUygula = () => invoke<UygulamaSonucu>('tcp_uygula');
export const qosIlkeleri = () => invoke<QosIlkesi[]>('qos_ilkeleri');
export const qosKaldir = () => invoke<number>('qos_kaldir');
export const agAciklamalari = () => invoke<[string, string][]>('ag_aciklamalari');

export const ayarlar = () => invoke<Ayarlar>('ayarlar');
export const ayarlariYaz = (a: Ayarlar) => invoke<Ayarlar>('ayarlari_yaz', { ayarlar: a });
export const otomatikBaslatmaAyarla = (acik: boolean) =>
  invoke<boolean>('otomatik_baslatma_ayarla', { acik });
export const otomatikBaslatmaKomutu = () => invoke<string | null>('otomatik_baslatma_komutu');

export const yapilmayanlar = () => invoke<[string, string][]>('yapilmayanlar');

/** Üçüncü taraf bileşenler — metinler hariç (EULA madde 8). */
export const ucuncuTarafListesi = () => invoke<UcuncuTarafListesi>('ucuncu_taraf_listesi');
/** Tek bir lisans metni. `no`, listedeki `metinNo` alanı. */
export const ucuncuTarafMetni = (no: number) => invoke<string>('ucuncu_taraf_metni', { no });

/**
 * Bir adresi sistemin varsayılan tarayıcısında açar.
 *
 * Webview içinde gezinmiyoruz: uygulama penceresi bir tarayıcı değil ve
 * dışarıdaki bir sayfayı burada açmak, kullanıcının nerede olduğunu
 * belirsizleştirir. İzin `capabilities/default.json` → `opener:allow-open-url`.
 */
export const adresiAc = (url: string) => openUrl(url);

/**
 * Dosya seçme/kaydetme pencereleri.
 *
 * Eklenti çağrıları da bu dosyadan geçiyor: bileşenler tek bir köprü tanısın
 * ve testlerde tek yer taklit edilsin. İzinler `capabilities/default.json` →
 * `dialog:allow-open`, `dialog:allow-save`.
 */
const PROFIL_SUZGECI = [{ name: 'Muifly profili', extensions: ['json'] }];
const EXE_SUZGECI = [{ name: 'Program', extensions: ['exe'] }];

export async function profilDosyasiSec(): Promise<string | null> {
  const secilen = await openDialog({ multiple: false, filters: PROFIL_SUZGECI });
  return typeof secilen === 'string' ? secilen : null;
}

/**
 * Oyunun exe'sini kullanıcıya seçtirir.
 *
 * Kütüphane taramasının kaçırdığı her şeyin çıkış yolu: Microsoft Store
 * oyunları, taşınabilir kurulumlar, emülatörler. Yol **kullanıcının kendi
 * seçtiği** dosya penceresinden geliyor.
 */
export async function exeDosyasiSec(): Promise<string | null> {
  const secilen = await openDialog({ multiple: false, filters: EXE_SUZGECI });
  return typeof secilen === 'string' ? secilen : null;
}

export function profilDosyasiHedefi(onerilenAd: string): Promise<string | null> {
  return saveDialog({ defaultPath: onerilenAd, filters: PROFIL_SUZGECI });
}

/** Olay aboneliği. Dönen fonksiyon aboneliği bırakıyor. */
export function dinle<T>(olay: string, geriCagri: (yuk: T) => void): Promise<UnlistenFn> {
  return listen<T>(olay, (e) => geriCagri(e.payload));
}
