/**
 * Kütüphane ekranının üç duruşu testle korunuyor:
 *
 * 1. **Taslak kaydedilmiyor, gösteriliyor.** Onay adımı olmadan profil
 *    üretmek, karar #23'ün (içe aktarmada iki adımlı önizleme) aynı
 *    sorununu bu kapıdan geri getirirdi.
 * 2. **Taslağın gerekçesi ekranda.** `aciklamalar` gösterilmezse hazır gelen
 *    profil kara kutu olur — şeffaflık ilkesi.
 * 3. **Görseller ızgarayla birlikte değil, kart başına isteniyor.** Yüz
 *    oyunluk bir kütüphanede hepsini birden almak megabaytlarca veri demek.
 *
 * Görsellerin yerelden geldiği (karar #25) burada test edilemiyor; onu
 * `library` modülünün Rust tarafı koruyor.
 */

import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { KutuphaneDiyalogu, basHarfler } from './KutuphaneDiyalogu';
import * as api from '../lib/api';
import type { Oyun, Taslak } from '../lib/types';

vi.mock('../lib/api', () => ({
  oyunlariTara: vi.fn(),
  oyunGorseli: vi.fn(),
  oyunElleEkle: vi.fn(),
  profilTaslagi: vi.fn(),
  exeDosyasiSec: vi.fn(),
}));

const sahte = vi.mocked(api);

const CS2: Oyun = {
  kimlik: 'steam:730',
  ad: 'Counter-Strike 2',
  kaynak: 'steam',
  kurulum: 'C:\\Steam\\steamapps\\common\\Counter-Strike Global Offensive',
  exeler: ['cs2.exe', 'bugsplat.exe'],
  gorselVar: true,
};

const NTE: Oyun = {
  kimlik: 'epic:b675fd',
  ad: 'NTE: Neverness to Everness',
  kaynak: 'epic',
  kurulum: 'C:\\Games\\NTE',
  exeler: ['nteglobalgame.exe'],
  gorselVar: false,
};

const TASLAK: Taslak = {
  profil: {
    profile_id: 'counter_strike_2',
    display_name: 'Counter-Strike 2',
    executable_names: ['cs2.exe'],
    competitive: true,
    system: {
      priority_class: 'high',
      cpu_affinity: 'dokunma',
      suspend_process_list: [],
      suspend_whitelist_exempt: [],
      power_plan: null,
    },
    network: { preferred_dns: null, qos_priority: false, tcp_nodelay: false },
    scaling: { enabled: false, algorithm: null, frame_generation: false },
    created_by: 'user',
    shared: false,
  },
  aciklamalar: [
    "'cs2.exe' katalogda tanındı: Counter-Strike 2",
    'Rekabetçi oyun olarak işaretlendi.',
  ],
};

beforeEach(() => {
  sahte.oyunlariTara.mockResolvedValue([CS2, NTE]);
  sahte.oyunGorseli.mockResolvedValue('data:image/png;base64,AAAA');
  sahte.profilTaslagi.mockResolvedValue(TASLAK);
});

test('oyunlar kaynak etiketleriyle listeleniyor', async () => {
  render(<KutuphaneDiyalogu onKapat={vi.fn()} onTaslak={vi.fn()} />);

  expect(await screen.findByText('Counter-Strike 2')).toBeTruthy();
  expect(screen.getByText('NTE: Neverness to Everness')).toBeTruthy();
  expect(screen.getByText('Steam')).toBeTruthy();
  expect(screen.getByText('Epic Games')).toBeTruthy();
});

test('görsel yalnızca görseli olan oyun için isteniyor', async () => {
  render(<KutuphaneDiyalogu onKapat={vi.fn()} onTaslak={vi.fn()} />);
  await screen.findByText('Counter-Strike 2');

  // NTE'nin `gorselVar` alanı false: onun için istek çıkmamalı.
  await waitFor(() => expect(sahte.oyunGorseli).toHaveBeenCalledWith('steam:730'));
  expect(sahte.oyunGorseli).not.toHaveBeenCalledWith('epic:b675fd');
});

test('oyun seçmek profil kaydetmiyor, gerekçeleriyle onay soruyor', async () => {
  const onTaslak = vi.fn();
  render(<KutuphaneDiyalogu onKapat={vi.fn()} onTaslak={onTaslak} />);

  await userEvent.click(await screen.findByText('Counter-Strike 2'));

  // Taslağın NEDEN böyle olduğu ekranda.
  expect(
    await screen.findByText("'cs2.exe' katalogda tanındı: Counter-Strike 2"),
  ).toBeTruthy();
  expect(screen.getByText(/henüz kaydedilmedi/)).toBeTruthy();

  // Onaya kadar dışarı hiçbir şey verilmiyor.
  expect(onTaslak).not.toHaveBeenCalled();

  await userEvent.click(screen.getByRole('button', { name: /Profil penceresini aç/ }));
  expect(onTaslak).toHaveBeenCalledTimes(1);
  expect(onTaslak.mock.calls[0][0].profil.executable_names).toEqual(['cs2.exe']);
});

test('aday exe işaretlenmezse onay düğmesi kapalı', async () => {
  render(<KutuphaneDiyalogu onKapat={vi.fn()} onTaslak={vi.fn()} />);
  await userEvent.click(await screen.findByText('Counter-Strike 2'));

  const onay = await screen.findByRole('button', { name: /Profil penceresini aç/ });
  expect((onay as HTMLButtonElement).disabled).toBe(false);

  // Taslağın seçtiği tek exe kaldırılınca profil eşleşemez olur.
  await userEvent.click(screen.getByRole('checkbox', { name: 'cs2.exe' }));
  expect((onay as HTMLButtonElement).disabled).toBe(true);
});

test('ek aday exe işaretlenebiliyor', async () => {
  const onTaslak = vi.fn();
  render(<KutuphaneDiyalogu onKapat={vi.fn()} onTaslak={onTaslak} />);
  await userEvent.click(await screen.findByText('Counter-Strike 2'));

  await userEvent.click(await screen.findByRole('checkbox', { name: 'bugsplat.exe' }));
  await userEvent.click(screen.getByRole('button', { name: /Profil penceresini aç/ }));

  expect(onTaslak.mock.calls[0][0].profil.executable_names).toEqual([
    'cs2.exe',
    'bugsplat.exe',
  ]);
});

test('kurulu oyun yoksa elle seçme yolu anlatılıyor', async () => {
  sahte.oyunlariTara.mockResolvedValue([]);
  render(<KutuphaneDiyalogu onKapat={vi.fn()} onTaslak={vi.fn()} />);

  expect(await screen.findByText('Kurulu oyun bulunamadı')).toBeTruthy();
  expect(screen.getByRole('button', { name: /\.exe dosyası seç/ })).toBeTruthy();
});

test('baş harfler görselsiz kartlar için üretiliyor', () => {
  expect(basHarfler('Counter-Strike 2')).toBe('CS');
  expect(basHarfler('NTE: Neverness to Everness')).toBe('NN');
  expect(basHarfler('Terraria')).toBe('TE');
  expect(basHarfler('   ')).toBe('?');
});
