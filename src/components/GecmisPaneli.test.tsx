/**
 * Oturum geçmişi ekranının dört ürün duruşu testle korunuyor:
 *
 * 1. **Silme tek tıkla olmuyor.** Geri alınamayan bir işlem; onay isteniyor.
 * 2. **Ayar kapalıyken sebebi yazılıyor.** Boş bir liste "hiç oynamadın"
 *    gibi okunurdu.
 * 3. **Kayıtta iyileşme iddiası yok** (karar #15): öncesi ve sonrası iki
 *    ayrı sütun, tek bir oran değil.
 * 4. **Ölçüm yoksa sıfır yazılmıyor** — `format.ts`'in kuralı burada da
 *    geçerli.
 */

import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { GecmisPaneli } from './GecmisPaneli';
import { gecmisOzeti, oturumKaydi, ozet } from '../test/ornekler';

function panel(ozellestir: Partial<Parameters<typeof GecmisPaneli>[0]> = {}) {
  return render(
    <GecmisPaneli
      kayitlar={[]}
      ozet={null}
      gecmisTut
      mesgul={false}
      onDisaAktar={() => {}}
      onTemizle={() => {}}
      onYenile={() => {}}
      onAyarlara={() => {}}
      {...ozellestir}
    />,
  );
}

describe('GecmisPaneli', () => {
  it('kayıt yokken ne zaman kayıt oluşacağını anlatıyor', () => {
    panel();
    expect(screen.getByText(/henüz kayıtlı oturum yok/i)).toBeTruthy();
  });

  it('oyun adını, süreyi ve değişiklik sayısını listeliyor', () => {
    panel({ kayitlar: [oturumKaydi()], ozet: gecmisOzeti() });

    // Ad hem özet şeridinde hem kayıtta, süre hem toplamda hem kayıtta
    // geçiyor; test varlığına bakıyor, kaç kez göründüğüne değil.
    expect(screen.getAllByText(/counter-strike 2/i).length).toBeGreaterThan(0);
    expect(screen.getAllByText(/45 dk/).length).toBeGreaterThan(0);
    expect(screen.getByText(/1 değişiklik/)).toBeTruthy();
  });

  it('oyun adı bilinmiyorsa exe adını gösteriyor', () => {
    // Uydurma bir ad üretmiyoruz: katalogda yoksa dosya adı yazıyor.
    panel({ kayitlar: [oturumKaydi({ oyunAdi: null, surec: 'bilinmeyen.exe' })] });
    expect(screen.getAllByText(/bilinmeyen\.exe/).length).toBeGreaterThan(0);
  });

  /** Karar #15: iki pencere yan yana, tek bir "iyileşme" sayısı değil. */
  it('ölçüm penceresini iki ayrı sütun olarak gösteriyor, oran üretmiyor', () => {
    panel({
      kayitlar: [
        oturumKaydi({
          onceki: ozet({ ornekSayisi: 10, cpuOrt: 80 }),
          sonraki: ozet({ ornekSayisi: 10, cpuOrt: 40 }),
        }),
      ],
    });

    expect(screen.getByText(/uygulamadan önce/i)).toBeTruthy();
    expect(screen.getByText('80%')).toBeTruthy();
    expect(screen.getByText('40%')).toBeTruthy();
    // "%50 iyileşme" gibi bir cümle hiçbir yerde yok.
    expect(screen.queryByText(/iyileş/i)).toBeNull();
  });

  it('ölçüm penceresi oluşmadıysa sıfır yazmıyor', () => {
    panel({ kayitlar: [oturumKaydi({ onceki: null, sonraki: null })] });
    expect(screen.getByText(/ölçüm penceresi oluşmadı/i)).toBeTruthy();
    expect(screen.queryByText('0%')).toBeNull();
  });

  it('temizleme onay istiyor ve vazgeçilebiliyor', async () => {
    const kullanici = userEvent.setup();
    const temizle = vi.fn();
    panel({ kayitlar: [oturumKaydi()], onTemizle: temizle });

    await kullanici.click(screen.getByRole('button', { name: /temizle/i }));
    expect(temizle).not.toHaveBeenCalled();

    const uyari = screen.getByText(/geri alınamıyor/i);
    expect(uyari).toBeTruthy();

    await kullanici.click(screen.getByRole('button', { name: /vazgeç/i }));
    expect(temizle).not.toHaveBeenCalled();

    await kullanici.click(screen.getByRole('button', { name: /temizle/i }));
    await kullanici.click(screen.getByRole('button', { name: /^sil$/i }));
    expect(temizle).toHaveBeenCalledTimes(1);
  });

  it('geçmiş kapalıyken sebebini yazıyor ve ayarlara yönlendiriyor', async () => {
    const kullanici = userEvent.setup();
    const ayarlara = vi.fn();
    panel({ gecmisTut: false, onAyarlara: ayarlara });

    expect(screen.getByText(/kaydedilmiyor/i)).toBeTruthy();
    await kullanici.click(screen.getByRole('button', { name: /ayarlara git/i }));
    expect(ayarlara).toHaveBeenCalled();
  });

  it('arama süzerken kaç kaydın gösterildiğini yazıyor', async () => {
    const kullanici = userEvent.setup();
    panel({
      kayitlar: [
        oturumKaydi({ id: 1, oyunAdi: 'Counter-Strike 2', surec: 'cs2.exe' }),
        oturumKaydi({ id: 2, oyunAdi: 'Elden Ring', surec: 'eldenring.exe' }),
      ],
    });

    await kullanici.type(screen.getByLabelText(/geçmişte ara/i), 'elden');

    expect(screen.getByText(/1 \/ 2 oturum gösteriliyor/)).toBeTruthy();
    expect(screen.queryByText(/counter-strike 2/i)).toBeNull();
  });

  it('kayıt yokken dışa aktarma düğmesi kapalı', () => {
    panel();
    const dugme = screen.getByRole('button', { name: /dışa aktar/i }) as HTMLButtonElement;
    expect(dugme.disabled).toBe(true);
  });

  it('kare ölçümü olan oturumda ölçülen değerleri gösteriyor', () => {
    panel({
      kayitlar: [
        oturumKaydi({
          kare: {
            kareSayisi: 7200,
            sureS: 60,
            ortFps: 120,
            ortMs: 8.3,
            p1KotuMs: 21.4,
            p1KotuFps: 46.7,
            kareJitterMs: 1.9,
          },
        }),
      ],
    });

    const kayit = screen.getByText(/kare ölçümü/i).closest('div')!;
    expect(within(kayit.parentElement!).getByText(/120\.0 kare\/sn/)).toBeTruthy();
    expect(screen.getByText('21.4 ms')).toBeTruthy();
  });
});
