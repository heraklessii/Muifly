/**
 * Ölçekleme ekranının ürün duruşları testle korunuyor:
 *
 * 1. **Rekabetçi modda kapalı ve bunu söylüyor.** Düğmeyi sessizce pasif
 *    bırakmak, kullanıcıya bozuk gibi görünürdü.
 * 2. **Bedel yazılı.** Ekranda "gecikme ekler" cümlesi olmadan bu modül
 *    tasarım ilkesi 2'yi karşılamıyor.
 * 3. **Kalite iddiası yok.** Algoritma listesi bir sıralama değil; ekranda
 *    "en iyi"/"daha iyi" gibi bir yönlendirme bulunmuyor (ilke 4).
 * 4. **Ölçüm yokken sıfır yazılmıyor.** `format.ts`'in kuralı burada da
 *    geçerli: ölçülmemiş bir gecikme "0 ms" diye gösterilemez.
 * 5. **Durma sebebi gösteriliyor.** Yakalama koptuğunda kullanıcı neden
 *    durduğunu ve ne yapacağını okuyabilmeli.
 */

import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { OlceklemePaneli } from './OlceklemePaneli';
import * as api from '../lib/api';
import type { OlceklemeDurumu } from '../lib/types';

vi.mock('../lib/api', () => ({
  olceklemeAlgoritmalari: vi.fn(),
  olceklemeEkranlari: vi.fn(),
  olceklemeDurumu: vi.fn(),
  olceklemeBaslat: vi.fn(async () => undefined),
  olceklemeDurdur: vi.fn(async () => undefined),
  olceklemeAlgoritma: vi.fn(async () => undefined),
  olceklemeDenemesi: vi.fn(),
}));

const sahte = vi.mocked(api);

const BOS_DURUM: OlceklemeDurumu = {
  calisiyor: false,
  algoritma: null,
  ekran: null,
  kaynakGenislik: 0,
  kaynakYukseklik: 0,
  hedefGenislik: 0,
  hedefYukseklik: 0,
  gecikme: null,
  sonEngel: null,
  uyari: null,
  durdurmaKisayoli: null,
  hedefBekleniyor: false,
  kacislaDurduruldu: false,
};

const ALGORITMALAR = [
  {
    anahtar: 'tam_sayi' as const,
    ad: 'Tam sayı katı',
    aciklama: 'Görüntüyü tam sayı katına çıkarır, yeni renk üretmez.',
  },
  {
    anahtar: 'lanczos' as const,
    ad: 'Lanczos',
    aciklama: 'Geniş bir pencereyle aradeğerleme yapar.',
  },
];

function panel(ozellestir: Partial<Parameters<typeof OlceklemePaneli>[0]> = {}) {
  return render(
    <OlceklemePaneli
      rekabetciMod={false}
      olceklemeEkrani={0}
      mesgul={false}
      onIslem={async (calis) => {
        await calis();
      }}
      onBildir={() => {}}
      onEkranDegistir={() => {}}
      onAyarlara={() => {}}
      {...ozellestir}
    />,
  );
}

beforeEach(() => {
  sahte.olceklemeAlgoritmalari.mockResolvedValue(ALGORITMALAR);
  sahte.olceklemeEkranlari.mockResolvedValue([
    { indeks: 0, ad: '\\\\.\\DISPLAY1', genislik: 1920, yukseklik: 1080, birincil: true },
  ]);
  sahte.olceklemeDurumu.mockResolvedValue(BOS_DURUM);
});

describe('OlceklemePaneli', () => {
  it('eklenen gecikmeyi bir bedel olarak yazıyor', async () => {
    panel();
    // "Bedeli var" cümlesi ekranda olmak zorunda: bu modülün şeffaflık
    // ilkesini karşıladığı yer.
    expect(screen.getByText(/bedeli var/i)).toBeTruthy();
    expect(screen.getByText(/oyunun kendisine dokunulmaz/i)).toBeTruthy();
  });

  it('rekabetçi modda başlatmıyor ve sebebini söylüyor', async () => {
    panel({ rekabetciMod: true });
    expect(screen.getByText(/rekabetçi mod açık, ölçekleme kapalı/i)).toBeTruthy();
    const baslat = screen.getByRole('button', { name: /başlat/i }) as HTMLButtonElement;
    expect(baslat.disabled).toBe(true);
  });

  it('algoritma listesinde kalite sıralaması yok', async () => {
    panel();
    await waitFor(() => expect(screen.getByText('Lanczos')).toBeTruthy());
    // Tasarım ilkesi 4: seçeneklerin hiçbiri "önerilen" diye işaretlenmiyor
    // ve aralarında bir sıralama kurulmuyor. Kontrol seçenek satırlarında:
    // paneldeki "hiçbiri daha iyi değil" cümlesi zaten bu duruşu anlatıyor.
    for (const secenek of screen.getAllByRole('radio')) {
      const satir = (secenek.closest('label')?.textContent ?? '').toLowerCase();
      for (const yasak of ['daha iyi', 'en iyi', 'önerilen', 'tavsiye', 'varsayılan']) {
        expect(satir).not.toContain(yasak);
      }
    }
  });

  it('ölçüm yokken sıfır göstermiyor', async () => {
    panel();
    expect(screen.getByText(/henüz ölçüm yok/i)).toBeTruthy();
    expect(screen.getByText(/ödenen bedeldir/i)).toBeTruthy();
  });

  it('ölçüm varken kare başına süreyi ve en kötü kareyi gösteriyor', async () => {
    sahte.olceklemeDurumu.mockResolvedValue({
      ...BOS_DURUM,
      calisiyor: true,
      algoritma: 'lanczos',
      ekran: 0,
      kaynakGenislik: 1920,
      kaynakYukseklik: 1080,
      hedefGenislik: 2560,
      hedefYukseklik: 1440,
      gecikme: {
        kareSayisi: 300,
        ortMs: 2.5,
        p1KotuMs: 9.25,
        enKotuMs: 14,
        yakalamaOrtMs: 0.8,
        olceklemeOrtMs: 0,
        sunumOrtMs: 1.7,
        bosTur: 12,
      },
    });
    panel();
    await waitFor(() => expect(screen.getByText(/2,50 ms|2\.50 ms/)).toBeTruthy());
    expect(screen.getByText(/en kötü %1/i)).toBeTruthy();
    // Dikey eşitleme beklemesinin sunum süresine dahil olduğu yazılı:
    // yazılmasaydı "1,70 ms sunum" satırı yanlış okunurdu.
    expect(screen.getByText(/dikey eşitleme beklemesi dahil/i)).toBeTruthy();
  });

  it('çalışırken bir kısıt varsa hata gibi değil uyarı gibi gösteriyor', async () => {
    // Ölçekleme sürüyor; "durdu" demek yanlış olurdu, hiç dememek ise
    // kullanıcının gördüğü bozukluğu açıklamasız bırakırdı.
    sahte.olceklemeDurumu.mockResolvedValue({
      ...BOS_DURUM,
      calisiyor: true,
      uyari: 'Bu Windows sürümü pencereyi yakalamanın dışına çıkaramıyor',
    });
    panel();
    await waitFor(() =>
      expect(screen.getByText(/ölçekleme çalışıyor, ama bir kısıt var/i)).toBeTruthy(),
    );
    expect(screen.queryByText(/ölçekleme durdu/i)).toBeNull();
  });

  it('durma sebebini gösteriyor', async () => {
    sahte.olceklemeDurumu.mockResolvedValue({
      ...BOS_DURUM,
      sonEngel: 'ekran okunamadı — oyun münhasır tam ekranda olabilir',
    });
    panel();
    await waitFor(() => expect(screen.getByText(/ölçekleme durdu/i)).toBeTruthy());
    expect(screen.getByText(/münhasır tam ekranda olabilir/i)).toBeTruthy();
  });

  it('çalışırken algoritma değiştirmek yeniden başlatmıyor', async () => {
    sahte.olceklemeDurumu.mockResolvedValue({
      ...BOS_DURUM,
      calisiyor: true,
      algoritma: 'tam_sayi',
    });
    const kullanici = userEvent.setup();
    panel();
    await waitFor(() => expect(screen.getByText('Lanczos')).toBeTruthy());

    await kullanici.click(screen.getByRole('radio', { name: /lanczos/i }));

    await waitFor(() => expect(sahte.olceklemeAlgoritma).toHaveBeenCalledWith('lanczos'));
    // Yeniden başlatma yok: ekranın bir anlığına kararması gerekmiyor.
    expect(sahte.olceklemeDurdur).not.toHaveBeenCalled();
    expect(sahte.olceklemeBaslat).not.toHaveBeenCalled();
  });

  it('ekran seçimi ölçekleme çalışırken kilitli', async () => {
    sahte.olceklemeDurumu.mockResolvedValue({ ...BOS_DURUM, calisiyor: true });
    panel();
    await waitFor(() => {
      const secim = screen.getByRole('radio', { name: /DISPLAY1/i }) as HTMLInputElement;
      expect(secim.disabled).toBe(true);
    });
  });

  it('ekran listesi okunamazsa sebebi yazılıyor', async () => {
    sahte.olceklemeEkranlari.mockRejectedValue(new Error('ekran bulunamadı'));
    panel();
    await waitFor(() => expect(screen.getByText(/ekran listesi okunamadı/i)).toBeTruthy());
  });
});
