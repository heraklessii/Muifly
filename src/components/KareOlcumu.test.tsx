/**
 * Kare ölçümü panelinin dört duruşu testle korunuyor. Hiçbiri kozmetik
 * değil — dördü de bir tasarım ilkesinin arayüzdeki karşılığı:
 *
 * 1. **UAC'ın nedeni istemden ÖNCE yazıyor.** Tasarım ilkesi 5: yetki
 *    "sadece gerektiğinde ve neden istendiği söylenerek" isteniyor. Açıklama
 *    düğmeye basıldıktan sonra gelseydi ilke kâğıt üstünde kalırdı.
 * 2. **Algılanmış oyun yokken ölçüm başlatılamıyor.** Ölçüm hedefini motor
 *    seçiyor; düğme aktif olsaydı kullanıcı sebebini bilmeden hata alırdı.
 * 3. **Yardımcı ikili yoksa panel hiç görünmüyor.** Çalışmayacak bir düğme
 *    göstermek, "neden çalışmıyor" sorusunu kullanıcıya bırakmak olurdu.
 * 4. **Sonuç sayıları vaat gibi sunulmuyor.** Karar #15 ve tasarım ilkesi 4:
 *    gösterilen değerin bu makinede ölçüldüğü açıkça yazılı olmalı.
 */

import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { KareOlcumu } from './KareOlcumu';
import * as api from '../lib/api';
import type { KareOlcumDurumu, OlcumRaporu } from '../lib/types';

vi.mock('../lib/api', () => ({
  kareOlcumDurumu: vi.fn(),
  oyunuOlc: vi.fn(),
}));

const sahte = vi.mocked(api);

const HAZIR: KareOlcumDurumu = {
  kullanilabilir: true,
  yetkiGerekiyor: true,
  enKisaSaniye: 5,
  enUzunSaniye: 120,
  aciklama:
    'Kare süresi ölçümü Windows’un olay izleme altyapısını kullanıyor ve bu ' +
    'altyapı yönetici yetkisi istiyor.',
};

const RAPOR: OlcumRaporu = {
  surec: 'oyun.exe',
  pid: 4242,
  saniye: 20,
  sonuc: {
    kareSayisi: 1200,
    ozet: {
      kareSayisi: 1200,
      sureS: 20,
      ortFps: 60,
      ortMs: 16.6,
      p1KotuMs: 48,
      p1KotuFps: 20.8,
      kareJitterMs: 2.4,
    },
  },
};

/** Testlerde işlem sarmalayıcısı sadece çalıştırıyor. */
const islem = (calis: () => Promise<void>) => {
  void calis();
};

beforeEach(() => {
  vi.clearAllMocks();
  sahte.kareOlcumDurumu.mockResolvedValue(HAZIR);
  sahte.oyunuOlc.mockResolvedValue(RAPOR);
});

test('yetkinin nedeni ölçüm başlatılmadan önce yazıyor', async () => {
  render(<KareOlcumu oyunVar mesgul={false} onIslem={islem} />);

  await waitFor(() => expect(screen.getByRole('button', { name: /Ölç/ })).toBeTruthy());

  // Backend'den gelen açıklama ekranda, henüz hiçbir şeye basılmadan.
  expect(screen.getByText(/yönetici yetkisi istiyor/)).toBeTruthy();
  expect(screen.getByText(/Vazgeçersen hiçbir şey değişmez/)).toBeTruthy();
  expect(sahte.oyunuOlc).not.toHaveBeenCalled();
});

test('algılanmış oyun yokken ölçüm başlatılamıyor ve sebebi yazıyor', async () => {
  render(<KareOlcumu oyunVar={false} mesgul={false} onIslem={islem} />);

  await waitFor(() => expect(screen.getByRole('button', { name: /Ölç/ })).toBeTruthy());

  const dugme = screen.getByRole('button', { name: /Ölç/ });
  expect((dugme as HTMLButtonElement).disabled).toBe(true);
  expect(screen.getByText(/algılanmış bir oyun yok/)).toBeTruthy();
});

test('yardımcı ikili yoksa panel hiç çizilmiyor', async () => {
  sahte.kareOlcumDurumu.mockResolvedValue({ ...HAZIR, kullanilabilir: false });
  const { container } = render(<KareOlcumu oyunVar mesgul={false} onIslem={islem} />);

  await waitFor(() => expect(sahte.kareOlcumDurumu).toHaveBeenCalled());
  expect(container.innerHTML).toBe('');
});

test('ölçüm sonucu ölçüm olarak sunuluyor, vaat olarak değil', async () => {
  render(<KareOlcumu oyunVar mesgul={false} onIslem={islem} />);
  await waitFor(() => expect(screen.getByRole('button', { name: /Ölç/ })).toBeTruthy());

  await userEvent.click(screen.getByRole('button', { name: /Ölç/ }));

  await waitFor(() => expect(screen.getAllByText('En kötü %1').length).toBeGreaterThan(0));

  // Hangi sürecin ölçüldüğü görünüyor: sayının nereden geldiği saklanmıyor.
  expect(screen.getByText('oyun.exe')).toBeTruthy();
  expect(screen.getByText(/bu oturumda\s+ölçüldü; bir vaat değil/)).toBeTruthy();

  // Ortalamanın tek başına yetmediği söyleniyor — ürünün ölçüm duruşu.
  expect(screen.getByText(/Ortalama tek başına yanıltıcı olabilir/)).toBeTruthy();
});

test('özet çıkmayan ölçüm sıfır göstermiyor, durumu anlatıyor', async () => {
  sahte.oyunuOlc.mockResolvedValue({
    ...RAPOR,
    sonuc: { kareSayisi: 4, ozet: null },
  });

  render(<KareOlcumu oyunVar mesgul={false} onIslem={islem} />);
  await waitFor(() => expect(screen.getByRole('button', { name: /Ölç/ })).toBeTruthy());
  await userEvent.click(screen.getByRole('button', { name: /Ölç/ }));

  await waitFor(() =>
    expect(screen.getByText(/özet çıkarmaya yetmedi/)).toBeTruthy()
  );
  // "0 kare/sn" gibi bir sayı YOK: ölçülmeyen şey için sayı uydurulmuyor.
  expect(screen.queryAllByText('En kötü %1')).toHaveLength(0);
});
