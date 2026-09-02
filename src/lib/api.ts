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
  Alan,
  AlgoritmaAnahtari,
  AlgoritmaBilgisi,
  Ayarlar,
  CeviriBellegi,
  CeviriDurumu,
  CeviriSonucu,
  DnsSonucu,
  Durum,
  Ekran,
  GecmisOzeti,
  Karsilastirma,
  KareOlcumDurumu,
  Kayit,
  KatalogGirdisi,
  Kisitlar,
  ModelDurumu,
  OcrDilDurumu,
  Onizleme,
  OlceklemeDurumu,
  OlcumRaporu,
  Ornek,
  OturumKaydi,
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
  YakalamaDenemesi,
  YolSonucu,
} from './types';

/** `src-tauri/src/commands.rs` içindeki sabitlerle birebir aynı olmak zorunda. */
export const OLAY_DURUM = 'muifly://durum';
export const OLAY_GUNLUK = 'muifly://gunluk';
export const OLAY_ORNEK = 'muifly://ornek';
export const OLAY_CEVIRI = 'muifly://ceviri';
export const OLAY_CEVIRI_INDIRME = 'muifly://ceviri-indirme';

export const surum = () => invoke<string>('surum');
/** Demo/tam sürüm ayrımı. Açılışta bir kez okunuyor; çalışırken değişmez. */
export const kisitlar = () => invoke<Kisitlar>('kisitlar');
export const durum = () => invoke<Durum>('durum');

export const gunluk = (adet?: number) => invoke<Satir[]>('gunluk', { adet: adet ?? null });
export const gunlugu_temizle = () => invoke<void>('gunlugu_temizle');

/**
 * Oturum geçmişi. Diskteki kayıtlar, yeniden eskiye.
 *
 * Ölçüm penceresi ve kare özeti kaydın içinde geliyor: arayüz geçmiş bir
 * oturum için ikinci bir sorgu yapmıyor, çünkü o veri artık canlı değil.
 */
export const gecmis = () => invoke<OturumKaydi[]>('gecmis');
export const gecmisOzeti = () => invoke<GecmisOzeti>('gecmis_ozeti');
/** Geçmişi siler — dosya dahil. Geri alınamaz. */
export const gecmisiTemizle = () => invoke<void>('gecmisi_temizle');
/** Geçmişi düz metin rapor olarak verilen dosyaya yazar. */
export const gecmisDisaAktar = (yol: string) => invoke<void>('gecmis_disa_aktar', { yol });

export const ornekler = () => invoke<Ornek[]>('ornekler');
export const ozet = () => invoke<Ozet>('ozet');
export const karsilastirma = () => invoke<Karsilastirma | null>('karsilastirma');

/**
 * Kare ölçümü. Hedefi backend seçiyor: arayüz PID taşımıyor.
 *
 * Sebep, karar #25'in gerekçesinden başka somut bir şey de içeriyor —
 * kullanıcı düğmeye bastığı anda öndeki pencere Muifly'ın kendisi olur.
 * Motorun mod durumu oyunu alt-tab tamponuyla hatırlıyor.
 */
export const kareOlcumDurumu = () => invoke<KareOlcumDurumu>('kare_olcum_durumu');
export const oyunuOlc = (saniye: number) => invoke<OlcumRaporu>('oyunu_olc', { saniye });

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

// --- Ölçekleme (Faz 3) -----------------------------------------------------

export const olceklemeEkranlari = () => invoke<Ekran[]>('olcekleme_ekranlari');
export const olceklemeAlgoritmalari = () =>
  invoke<AlgoritmaBilgisi[]>('olcekleme_algoritmalari');
export const olceklemeDurumu = () => invoke<OlceklemeDurumu>('olcekleme_durumu');
export const olceklemeBaslat = (algoritma: AlgoritmaAnahtari) =>
  invoke<void>('olcekleme_baslat', { algoritma });
export const olceklemeDurdur = () => invoke<void>('olcekleme_durdur');
/** Çalışırken algoritma değiştirir; ekran kararmıyor. */
export const olceklemeAlgoritma = (algoritma: AlgoritmaAnahtari) =>
  invoke<void>('olcekleme_algoritma', { algoritma });
/** Kare üretimini (Faz 4) çalışırken açar/kapatır. */
export const olceklemeUretimi = (acik: boolean) =>
  invoke<void>('olcekleme_uretimi', { acik });
/** Pencereyi açmadan yakalamanın çalışıp çalışmadığını dener. */
export const olceklemeDenemesi = () => invoke<YakalamaDenemesi>('olcekleme_denemesi');


// --- Ekran çevirisi (Faz 5) ------------------------------------------------

export const ceviriDurumu = () => invoke<CeviriDurumu>('ceviri_durumu');
export const ceviriSonucu = () => invoke<CeviriSonucu | null>('ceviri_sonucu');
/** Kısayolu sisteme kaydeder. Kaydedilemezse hata döner ve ayar yazılmaz. */
export const ceviriAc = () => invoke<void>('ceviri_ac');
export const ceviriKapat = () => invoke<void>('ceviri_kapat');
/** Kısayola basmakla aynı şey — özelliği pencereden denemek için. */
export const ceviriSimdi = () => invoke<void>('ceviri_simdi');
/** Kaynak dilin OCR paketi kurulu mu (karar #28). */
export const ceviriDilDurumu = () => invoke<OcrDilDurumu>('ceviri_dil_durumu');

export const ceviriModelDurumu = () => invoke<ModelDurumu>('ceviri_model_durumu');
/** İndirme uzun sürüyor; ilerleme `OLAY_CEVIRI_INDIRME` ile akıyor. */
export const ceviriModelIndir = () => invoke<void>('ceviri_model_indir');
export const ceviriModelIndirmeyiDurdur = () =>
  invoke<void>('ceviri_model_indirmeyi_durdur');
export const ceviriModelSil = () => invoke<void>('ceviri_model_sil');
/** Diskteki dosyaların SHA-256'sını beklenenle karşılaştırır. Yavaş. */
export const ceviriModelDogrula = () => invoke<void>('ceviri_model_dogrula');

/** Alan seçici penceresini açar. */
export const ceviriAlanSeciciAc = (kimlik: string) =>
  invoke<void>('ceviri_alan_secici_ac', { kimlik });
export const ceviriAlanSeciciKapat = () => invoke<void>('ceviri_alan_secici_kapat');
export const ceviriOverlayKapat = () => invoke<void>('ceviri_overlay_kapat');
/** Alan seçicinin gösterdiği donmuş ekran görüntüsü (`data:` adresi). */
export const ceviriEkranGoruntusu = () => invoke<string>('ceviri_ekran_goruntusu');
/** Seçilen alanı profile yazar. `null` = ekranın tamamı. */
export const ceviriAlaniKaydet = (kimlik: string, alan: Alan | null) =>
  invoke<string[]>('ceviri_alani_kaydet', { kimlik, alan });

export const ceviriBellegi = () => invoke<CeviriBellegi>('ceviri_bellegi');
/** Kullanıcının düzelttiği çeviri. Makine çevirisiyle ezilmiyor (karar #22). */
export const ceviriDuzelt = (metin: string, ceviri: string) =>
  invoke<void>('ceviri_duzelt', { metin, ceviri });
export const ceviriKaydiSil = (metin: string) => invoke<void>('ceviri_kaydi_sil', { metin });
export const ceviriTerimEkle = (terim: string, karsilik: string) =>
  invoke<void>('ceviri_terim_ekle', { terim, karsilik });
export const ceviriTerimSil = (terim: string) => invoke<void>('ceviri_terim_sil', { terim });
/** Makine kayıtlarını siler; kullanıcı düzeltmelerine dokunmaz. */
export const ceviriBelleginiTemizle = () => invoke<void>('ceviri_bellegini_temizle');

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
const METIN_SUZGECI = [{ name: 'Metin dosyası', extensions: ['txt'] }];

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

/**
 * Rapor dosyasının hedefi.
 *
 * Program kendi başına bir yere dosya bırakmıyor: yolu her zaman kullanıcı
 * seçiyor. Kullanıcı iptal ederse `null` dönüyor ve hiçbir şey yazılmıyor.
 */
export function metinDosyasiHedefi(onerilenAd: string): Promise<string | null> {
  return saveDialog({ defaultPath: onerilenAd, filters: METIN_SUZGECI });
}

/** Olay aboneliği. Dönen fonksiyon aboneliği bırakıyor. */
export function dinle<T>(olay: string, geriCagri: (yuk: T) => void): Promise<UnlistenFn> {
  return listen<T>(olay, (e) => geriCagri(e.payload));
}
