/**
 * Testler için örnek veri.
 *
 * Backend'den gelen yapıların birebir aynısı; alan adları `lib/types.ts` ile
 * uyumlu olmak zorunda (o da Rust tarafıyla uyumlu). Fabrika fonksiyonları
 * `kismi` alıyor: her test yalnızca ilgilendiği alanı yazsın.
 */

import { bosProfil, TAM_SURUM } from '../lib/types';
import type { Durum, Kayit, Kisitlar, Onizleme, Ozet, Profil, Satir } from '../lib/types';

export const AYARLAR = {
  otomatikUygula: false,
  oyunCikisGecikmesiSn: 10,
  olcumAraligiSn: 2,
  gecikmeHedefi: '1.1.1.1',
  gecikmeOlcumu: true,
  rekabetciMod: false,
  modBildirimi: false,
  tepsiyeKucult: true,
  tema: 'dark' as const,
};

export function durum(kismi: Partial<Durum> = {}): Durum {
  return {
    mod: { mod: 'bosta' },
    modAdi: 'Boşta',
    ayarlar: AYARLAR,
    bekleyenGeriAlma: 0,
    kaliciDegisiklik: 0,
    dondurmaDestegi: true,
    yonetici: false,
    fpsOlcumu: false,
    cpuHibrit: false,
    mantiksalCekirdek: 8,
    gucPlani: 'Dengeli',
    otomatikBaslatma: false,
    ...kismi,
  };
}

export function ozet(kismi: Partial<Ozet> = {}): Ozet {
  return {
    ornekSayisi: 0,
    cpuOrt: 0,
    bellekOrt: 0,
    gecikmeOrtMs: null,
    jitterMs: null,
    kayipYuzde: 0,
    ...kismi,
  };
}

export function satir(kismi: Partial<Satir> = {}): Satir {
  return {
    id: 1,
    zaman: Date.UTC(2026, 8, 1, 12, 0, 0),
    duzey: 'aksiyon',
    kategori: 'sistem',
    mesaj: 'bir şey yapıldı',
    geriAlmaId: null,
    ...kismi,
  };
}

export function kayit(kismi: Partial<Kayit> = {}): Kayit {
  return {
    id: 1,
    zaman: Date.UTC(2026, 8, 1, 12, 0, 0),
    ozet: 'cs2.exe önceliği yüksek yapıldı',
    kapsam: 'oturum',
    undo: { tur: 'surecOnceligi', pid: 1234, surec: 'cs2.exe', onceki: 32 },
    ...kismi,
  };
}

export function profil(kismi: Partial<Profil> = {}): Profil {
  return {
    ...bosProfil(),
    profile_id: 'cs2',
    display_name: 'Counter-Strike 2',
    executable_names: ['cs2.exe'],
    ...kismi,
  };
}

export function onizleme(kismi: Partial<Onizleme> = {}): Onizleme {
  return {
    profil: profil(),
    dosya: 'cs2.json',
    duzeltmeler: [],
    etkiler: ['Eşleşen uygulama: cs2.exe', 'Güç planına dokunulmaz'],
    uyarilar: [],
    kimlikCakismasi: false,
    cakisanProfiller: [],
    bosKimlik: 'cs2',
    ...kismi,
  };
}

export const KISITLAR_TAM = TAM_SURUM;

export const KISITLAR_DEMO: Kisitlar = {
  demo: true,
  profilSiniri: 1,
  agModulu: false,
  otomatikBaslatma: false,
  profilAktarimi: false,
};
