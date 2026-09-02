/**
 * Çeviri panelinin beş duruşu testle korunuyor. Hiçbiri kozmetik değil —
 * beşi de ölçülmüş bir bulgunun ya da bir tasarım ilkesinin arayüzdeki
 * karşılığı:
 *
 * 1. **Kaynak metin çevirinin yanında.** Karar #29 zaaf 3: model bir cümleyi
 *    hata vermeden düşürebiliyor ve çıktı akıcı göründüğü için fark
 *    edilmiyor. Yalnızca Türkçeyi gösteren bir tasarım daha temiz görünürdü
 *    ve ölçülmüş bir riski görünmez kılardı.
 * 2. **Boş çeviri ve kaybolan terim işaretleniyor.** Aynı zaafın iki görünür
 *    belirtisi; sessizce geçmeleri "muhtemelen doğrudur" demek olurdu.
 * 3. **Modelin bedeli indirme düğmesinden önce yazıyor.** Yarım gigabayt ve
 *    CPU kullanımı, kullanıcı düğmeye basmadan önce okunabilmeli.
 * 4. **Kısayolun kanca olmadığı söyleniyor.** Tasarım ilkesi 3'ün arayüzdeki
 *    karşılığı: anti-cheat'in baktığı ayrım kullanıcıya da anlatılıyor.
 * 5. **Kapalıyken "Şimdi çevir" pasif.** Çalışmayacak bir düğme göstermek,
 *    "neden olmadı" sorusunu kullanıcıya bırakmak olurdu.
 */

import { render, screen, waitFor } from '@testing-library/react';

import { CeviriPaneli } from './CeviriPaneli';
import * as api from '../lib/api';
import type {
  CeviriBellegi,
  CeviriDurumu,
  CeviriSonucu,
  ModelDurumu,
  Profil,
} from '../lib/types';
import { bosProfil } from '../lib/types';

vi.mock('../lib/api', () => ({
  OLAY_CEVIRI: 'muifly://ceviri',
  OLAY_CEVIRI_INDIRME: 'muifly://ceviri-indirme',
  dinle: vi.fn(),
  ceviriDurumu: vi.fn(),
  ceviriSonucu: vi.fn(),
  ceviriModelDurumu: vi.fn(),
  ceviriBellegi: vi.fn(),
  ceviriDilDurumu: vi.fn(),
  olceklemeEkranlari: vi.fn(),
  ceviriSimdi: vi.fn(),
  ceviriAc: vi.fn(),
  ceviriKapat: vi.fn(),
  ceviriModelIndir: vi.fn(),
  ceviriModelIndirmeyiDurdur: vi.fn(),
  ceviriModelSil: vi.fn(),
  ceviriModelDogrula: vi.fn(),
  ceviriAlanSeciciAc: vi.fn(),
  ceviriAlaniKaydet: vi.fn(),
  ceviriDuzelt: vi.fn(),
  ceviriKaydiSil: vi.fn(),
  ceviriTerimEkle: vi.fn(),
  ceviriTerimSil: vi.fn(),
  ceviriBelleginiTemizle: vi.fn(),
}));

const sahte = vi.mocked(api);

const DURUM: CeviriDurumu = {
  acik: true,
  kisayol: 'Ctrl+Alt+T',
  asama: 'bosta',
  modelBellekte: true,
  yakalamaMs: 120,
  ocrMs: 18,
  ceviriMs: 240,
  sonHata: null,
  oyun: 'skyrim',
  kaynakDil: 'en',
  hedefDil: 'tr',
  tumEkran: false,
};

const MODEL_KURULU: ModelDurumu = {
  kurulu: true,
  toplamBayt: 538_149_242,
  dizin: 'C:\\Users\\x\\AppData\\Roaming\\Muifly\\ceviri\\model',
  eksikler: [],
  kaynak: 'onnx-community/opus-mt-tc-big-en-tr (int8)',
};

const BELLEK: CeviriBellegi = {
  oyun: 'skyrim',
  kaynakDil: 'en',
  hedefDil: 'tr',
  terimler: { Longsword: 'Uzun Kılıç' },
  kayitlar: [],
};

/** Bir zaafın üç belirtisini birden taşıyan sonuç. */
const SONUC: CeviriSonucu = {
  ham: 'Keep your guard up. This one bites back.',
  uyarilar: [],
  bellekten: 0,
  ceviriMs: 240,
  birimler: [
    {
      kaynak: 'Keep your guard up.',
      ceviri: 'Tetikte ol.',
      koken: 'makine',
      korunanTerimler: [],
      kayipTerimler: [],
      bos: false,
    },
    {
      kaynak: 'This one bites back.',
      ceviri: '',
      koken: 'makine',
      korunanTerimler: [],
      kayipTerimler: ['Longsword'],
      bos: true,
    },
  ],
};

const props = {
  ceviriEkrani: 0,
  overlayAcik: true,
  profiller: [] as Profil[],
  mesgul: false,
  onIslem: (calis: () => Promise<void>) => {
    void calis();
    return Promise.resolve();
  },
  onBildir: () => {},
  onEkranDegistir: () => {},
  onOverlayDegistir: () => {},
  onAyarlara: () => {},
};

beforeEach(() => {
  vi.clearAllMocks();
  sahte.dinle.mockResolvedValue(() => {});
  sahte.ceviriDurumu.mockResolvedValue(DURUM);
  sahte.ceviriSonucu.mockResolvedValue(SONUC);
  sahte.ceviriModelDurumu.mockResolvedValue(MODEL_KURULU);
  sahte.ceviriBellegi.mockResolvedValue(BELLEK);
  sahte.ceviriDilDurumu.mockResolvedValue({
    durum: 'var',
    dil: { etiket: 'en-US', ad: 'English (United States)' },
  });
  sahte.olceklemeEkranlari.mockResolvedValue([]);
});

test('kaynak metin çevirinin yanında duruyor', async () => {
  render(<CeviriPaneli {...props} />);
  // Karar #29 zaaf 3: çeviri kaynaksız gösterilmiyor.
  expect(await screen.findByText('Keep your guard up.')).toBeTruthy();
  expect(screen.getByText('Tetikte ol.')).toBeTruthy();
});

test('boş çeviri ve kaybolan terim işaretleniyor', async () => {
  render(<CeviriPaneli {...props} />);
  expect(await screen.findByText('boş çeviri')).toBeTruthy();
  expect(screen.getByText(/kayıp terim: Longsword/)).toBeTruthy();
});

test('modelin bedeli indirme düğmesinden önce yazıyor', async () => {
  sahte.ceviriModelDurumu.mockResolvedValue({ ...MODEL_KURULU, kurulu: false });
  render(<CeviriPaneli {...props} />);

  const dugme = await screen.findByRole('button', { name: /Modeli indir/ });
  const panel = dugme.closest('.panel');
  expect(panel).not.toBeNull();
  // Boyut ve CPU bedeli aynı panelde, düğmeye basmadan okunabiliyor.
  expect(panel!.textContent).toMatch(/kuruluma dahil değil/);
  expect(panel!.textContent).toMatch(/CPU kullanıyor/);
  expect(dugme.textContent).toMatch(/MB|GB/);
});

test('kısayolun kanca olmadığı yazıyor', async () => {
  render(<CeviriPaneli {...props} />);
  await waitFor(() =>
    expect(screen.getByText(/klavye kancası değil/i)).toBeTruthy(),
  );
});

test('çeviri kapalıyken şimdi çevir pasif', async () => {
  sahte.ceviriDurumu.mockResolvedValue({ ...DURUM, acik: false, kisayol: null });
  render(<CeviriPaneli {...props} />);
  await waitFor(() =>
    expect((screen.getByRole('button', { name: /Şimdi çevir/ }) as HTMLButtonElement).disabled).toBe(true),
  );
});

test('profil yokken alan seçimi yerine yol gösteriliyor', async () => {
  render(<CeviriPaneli {...props} />);
  // Alan profile yazılıyor (karar #22); profil yoksa düğme değil açıklama.
  expect(await screen.findByText(/Henüz profil yok/)).toBeTruthy();
  expect(screen.queryByRole('button', { name: 'Alanı seç' })).toBeNull();
});

test('profil varken alanın seçili olup olmadığı okunabiliyor', async () => {
  const p = bosProfil();
  p.profile_id = 'skyrim';
  p.display_name = 'Skyrim';
  p.executable_names = ['skyrimse.exe'];
  render(<CeviriPaneli {...props} profiller={[p]} />);
  expect(await screen.findByText(/Alan seçilmemiş/)).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Alanı seç' })).toBeTruthy();
});
