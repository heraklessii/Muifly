/**
 * Üçüncü taraf lisans ekranının iki duruşu testle korunuyor:
 *
 * 1. **Lisans metni olmayan bileşen gizlenmiyor.** Listeyi temiz göstermek
 *    için onları atlamak, EULA madde 8'in vaat ettiği listeyi eksik yapardı.
 *    Eksik olan neyse o yazılıyor.
 * 2. **Metinler talep üzerine geliyor.** Açılışta hiçbir lisans metni
 *    çekilmiyor; bir megabaytlık veriyi kimsenin okumadığı bir ekran için
 *    taşımanın anlamı yok.
 *
 * İkisi de kozmetik değil: birincisi bir yasal vaat, ikincisi ekranın
 * açılabilir kalmasının sebebi.
 */

import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { LisanslarDiyalogu } from './LisanslarDiyalogu';
import * as api from '../lib/api';
import type { UcuncuTarafBileseni } from '../lib/types';

vi.mock('../lib/api', () => ({
  ucuncuTarafListesi: vi.fn(),
  ucuncuTarafMetni: vi.fn(),
  adresiAc: vi.fn(async () => undefined),
}));

const sahte = vi.mocked(api);

const TAURI: UcuncuTarafBileseni = {
  ad: 'tauri',
  surum: '2.11.5',
  tur: 'rust',
  lisans: 'Apache-2.0 OR MIT',
  kaynak: 'https://github.com/tauri-apps/tauri',
  metinNo: 0,
};

/** Yayımlanan sürümünde LICENSE dosyası taşımayan gerçek bir örnek. */
const METINSIZ: UcuncuTarafBileseni = {
  ad: 'webview2-com',
  surum: '0.38.2',
  tur: 'rust',
  lisans: 'MIT',
  kaynak: 'https://github.com/wravery/webview2-rs',
  metinNo: null,
};

const FONT: UcuncuTarafBileseni = {
  ad: 'Outfit',
  surum: 'değişken (variable)',
  tur: 'varlik',
  lisans: 'OFL-1.1',
  kaynak: 'https://github.com/Outfitio/Outfit-Fonts',
  metinNo: 1,
};

beforeEach(() => {
  sahte.ucuncuTarafListesi.mockResolvedValue({
    uretildi: '2026-09-01',
    hedef: 'x86_64-pc-windows-msvc',
    bilesenler: [TAURI, METINSIZ, FONT],
  });
  sahte.ucuncuTarafMetni.mockResolvedValue('MIT License\n\nCopyright (c) ornek');
});

function diyalog() {
  return render(<LisanslarDiyalogu onKapat={() => {}} />);
}

describe('LisanslarDiyalogu', () => {
  it('bileşenleri sürüm ve lisansıyla listeliyor', async () => {
    diyalog();
    expect(await screen.findByText('tauri')).toBeTruthy();
    expect(screen.getByText('2.11.5')).toBeTruthy();
    expect(screen.getByText('Apache-2.0 OR MIT')).toBeTruthy();
  });

  it('açılışta hiçbir lisans metni çekmiyor', async () => {
    diyalog();
    await screen.findByText('tauri');
    expect(sahte.ucuncuTarafMetni).not.toHaveBeenCalled();
  });

  it('bileşen açılınca metni o zaman çekip gösteriyor', async () => {
    const kullanici = userEvent.setup();
    diyalog();

    await kullanici.click(await screen.findByRole('button', { name: /tauri/ }));

    expect(sahte.ucuncuTarafMetni).toHaveBeenCalledWith(0);
    expect(await screen.findByText(/Copyright \(c\) ornek/)).toBeTruthy();
  });

  it('aynı bileşen ikinci kez açılınca metni yeniden çekmiyor', async () => {
    const kullanici = userEvent.setup();
    diyalog();

    const dugme = await screen.findByRole('button', { name: /tauri/ });
    await kullanici.click(dugme); // aç
    await screen.findByText(/Copyright \(c\) ornek/);
    await kullanici.click(dugme); // kapat
    await kullanici.click(dugme); // yeniden aç

    expect(sahte.ucuncuTarafMetni).toHaveBeenCalledTimes(1);
  });

  it('lisans metni olmayan bileşeni listeden gizlemiyor', async () => {
    diyalog();
    expect(await screen.findByText('webview2-com')).toBeTruthy();
  });

  it('lisans metni olmayan bileşende eksiği açıkça yazıyor', async () => {
    const kullanici = userEvent.setup();
    diyalog();

    await kullanici.click(await screen.findByRole('button', { name: /webview2-com/ }));

    expect(screen.getByText(/lisans metni taşımıyor/)).toBeTruthy();
    // Metin yok; boşuna backend'e sorulmuyor.
    expect(sahte.ucuncuTarafMetni).not.toHaveBeenCalled();
  });

  it('dağıtılan yazı tipini de listeliyor', async () => {
    diyalog();
    expect(await screen.findByText('Outfit')).toBeTruthy();
    expect(screen.getByText('OFL-1.1')).toBeTruthy();
  });

  it('arama listeyi süzüyor', async () => {
    const kullanici = userEvent.setup();
    diyalog();
    await screen.findByText('tauri');

    await kullanici.type(screen.getByRole('textbox', { name: /ara/i }), 'outfit');

    await waitFor(() => expect(screen.queryByText('tauri')).toBeNull());
    expect(screen.getByText('Outfit')).toBeTruthy();
  });

  it('liste okunamazsa sessiz kalmıyor', async () => {
    sahte.ucuncuTarafListesi.mockRejectedValue(new Error('gömülü dosya bozuk'));
    diyalog();
    expect(await screen.findByText(/Liste okunamadı/)).toBeTruthy();
  });
});
