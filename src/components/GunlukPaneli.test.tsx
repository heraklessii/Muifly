/**
 * Şeffaflık günlüğünün iki kuralı testle korunuyor:
 *
 * 1. "Geri al" düğmesi yalnızca defterde HÂLÂ duran kayıtlar için görünür.
 * 2. Geri alınmış satır listeden silinmez, yalnızca düğmesi düşer.
 *
 * İkisi de ürün duruşu (`docs/DESIGN_PRINCIPLES.md` ilke 1 ve 2), kozmetik
 * değil: geri alınmış bir şey için düğme göstermek kullanıcıyı yanıltır,
 * satırı silmek ise geçmişi saklamak olur.
 */

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { GunlukPaneli } from './GunlukPaneli';
import { kayit, satir } from '../test/ornekler';

function panel(ozellestir: Partial<Parameters<typeof GunlukPaneli>[0]> = {}) {
  return render(
    <GunlukPaneli
      satirlar={[]}
      bekleyenler={[]}
      mesgul={false}
      onGeriAl={() => {}}
      onTemizle={() => {}}
      {...ozellestir}
    />,
  );
}

describe('GunlukPaneli', () => {
  it('defterde duran kayıt için geri al düğmesi gösteriyor', () => {
    panel({
      satirlar: [satir({ id: 10, geriAlmaId: 7, mesaj: 'cs2.exe önceliği yüksek yapıldı' })],
      bekleyenler: [kayit({ id: 7 })],
    });

    // Biri bekleyenler listesinde, biri günlük satırında.
    expect(screen.getAllByRole('button', { name: /geri al/i }).length).toBe(2);
  });

  it('defterde olmayan kayıt için geri al düğmesi göstermiyor', () => {
    panel({
      // Satır geri alma kimliği taşıyor ama kayıt artık defterde yok:
      // değişiklik zaten geri alınmış.
      satirlar: [satir({ id: 10, geriAlmaId: 7, duzey: 'geriAlma' })],
      bekleyenler: [],
    });

    expect(screen.queryByRole('button', { name: /geri al/i })).toBeNull();
  });

  it('geri alınmış satırı listeden silmiyor', () => {
    panel({
      satirlar: [satir({ id: 10, geriAlmaId: 7, duzey: 'geriAlma', mesaj: 'güç planı geri yüklendi' })],
      bekleyenler: [],
    });

    expect(screen.getByText('güç planı geri yüklendi')).toBeTruthy();
  });

  it('meşgulken geri alma düğmesi tıklanamıyor', () => {
    panel({
      satirlar: [satir({ geriAlmaId: 7 })],
      bekleyenler: [kayit({ id: 7 })],
      mesgul: true,
    });

    for (const dugme of screen.getAllByRole('button', { name: /geri al/i })) {
      expect((dugme as HTMLButtonElement).disabled).toBe(true);
    }
  });
});

describe('GunlukPaneli — arama', () => {
  const satirlar = [
    satir({ id: 1, mesaj: 'discord.exe donduruldu' }),
    satir({ id: 2, mesaj: 'güç planı yüksek performans yapıldı' }),
    satir({ id: 3, mesaj: 'İndirme sırasında ölçüm atlandı', duzey: 'uyari' }),
  ];

  it('mesaja göre süzüyor', async () => {
    const kullanici = userEvent.setup();
    panel({ satirlar });

    await kullanici.type(screen.getByLabelText('Günlükte ara'), 'discord');

    expect(screen.getByText('discord.exe donduruldu')).toBeTruthy();
    expect(screen.queryByText(/güç planı/)).toBeNull();
  });

  it('Türkçe büyük İ ile yazılmış satırı küçük i ile buluyor', async () => {
    // `toLowerCase()` "İ"yi noktalı bir "i"ye çeviriyor; yerelli küçültme
    // olmadan bu arama boş dönerdi.
    const kullanici = userEvent.setup();
    panel({ satirlar });

    await kullanici.type(screen.getByLabelText('Günlükte ara'), 'indirme');

    expect(screen.getByText(/İndirme sırasında/)).toBeTruthy();
  });

  it('süzgeç açıkken kaç satırın gösterildiğini yazıyor', async () => {
    // Kullanıcı eksik bir listeye baktığını bilmeli (şeffaflık ilkesi).
    const kullanici = userEvent.setup();
    panel({ satirlar });

    await kullanici.type(screen.getByLabelText('Günlükte ara'), 'discord');

    expect(screen.getByText('1 / 3 satır gösteriliyor.')).toBeTruthy();
  });

  it('süzgeç yokken sayaç görünmüyor', () => {
    panel({ satirlar });
    expect(screen.queryByText(/satır gösteriliyor/)).toBeNull();
  });
});
