/**
 * Kabuk testleri: backend mock'lanıyor, gerçek `invoke` çağrılmıyor.
 *
 * Korunan iki akış:
 *
 * 1. **Profil düzeltmeleri kullanıcıya gösteriliyor.** Backend bir profili
 *    güvenli hale getirdiğinde (`profile_engine::schema`) düzeltmeleri geri
 *    döndürüyor; bunları sessizce yutmak, kullanıcının kaydettiğini sandığı
 *    profille diskteki profili ayırırdı (şeffaflık ilkesi).
 * 2. **Demo ikilisinde ağ sekmesi hiç görünmüyor.** Kısıtlar `surum.rs`'ten
 *    geliyor; arayüz kapalı özelliği "kapalı" diye göstermek yerine hiç
 *    göstermiyor (karar #20).
 */

import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import App from './App';
import * as api from './lib/api';
import { KISITLAR_DEMO, KISITLAR_TAM, durum, onizleme, ozet, profil } from './test/ornekler';

vi.mock('./lib/api', () => ({
  OLAY_DURUM: 'muifly://durum',
  OLAY_GUNLUK: 'muifly://gunluk',
  OLAY_ORNEK: 'muifly://ornek',
  // Olay aboneliği: testte hiçbir olay yayınlanmıyor, bırakma fonksiyonu boş.
  dinle: vi.fn(async () => () => {}),
  surum: vi.fn(async () => '0.1.0'),
  kisitlar: vi.fn(),
  yapilmayanlar: vi.fn(async () => []),
  durum: vi.fn(),
  gunluk: vi.fn(async () => []),
  ornekler: vi.fn(async () => []),
  ozet: vi.fn(),
  karsilastirma: vi.fn(async () => null),
  // Kare ölçümü paneli açılışta bunu okuyor; yardımcı yokmuş gibi davranıp
  // panel çizilmiyor — App testlerinin konusu değil.
  kareOlcumDurumu: vi.fn(async () => ({
    kullanilabilir: false,
    yetkiGerekiyor: true,
    enKisaSaniye: 5,
    enUzunSaniye: 120,
    aciklama: '',
  })),
  oyunuOlc: vi.fn(),
  profiller: vi.fn(async () => []),
  bekleyenGeriAlmalar: vi.fn(async () => []),
  profilKaydet: vi.fn(async () => []),
  profilSil: vi.fn(async () => undefined),
  profilUygula: vi.fn(),
  ondekineUygula: vi.fn(),
  oturumuKapat: vi.fn(async () => 0),
  hepsiniGeriAl: vi.fn(),
  geriAl: vi.fn(async () => undefined),
  gunlugu_temizle: vi.fn(async () => undefined),
  // Oturum geçmişi: açılışta okunuyor, bu testlerin konusu değil.
  gecmis: vi.fn(async () => []),
  gecmisOzeti: vi.fn(async () => null),
  gecmisiTemizle: vi.fn(async () => undefined),
  gecmisDisaAktar: vi.fn(async () => undefined),
  metinDosyasiHedefi: vi.fn(async () => null),
  // Ölçekleme: sekme açılınca okunuyor, bu testlerin konusu değil.
  olceklemeAlgoritmalari: vi.fn(async () => []),
  olceklemeEkranlari: vi.fn(async () => []),
  olceklemeDurumu: vi.fn(async () => ({
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
  })),
  olceklemeBaslat: vi.fn(async () => undefined),
  olceklemeDurdur: vi.fn(async () => undefined),
  olceklemeAlgoritma: vi.fn(async () => undefined),
  olceklemeDenemesi: vi.fn(),
  ayarlariYaz: vi.fn(async (a) => a),
  otomatikBaslatmaAyarla: vi.fn(async () => false),
  otomatikBaslatmaKomutu: vi.fn(async () => null),
  surecler: vi.fn(async () => []),
  dondurmaAdaylari: vi.fn(async () => []),
  // Kütüphane: bu testlerin konusu değil, komutlar boş dönüyor.
  taninanSurecler: vi.fn(async () => []),
  oyunlariTara: vi.fn(async () => []),
  oyunGorseli: vi.fn(async () => null),
  oyunElleEkle: vi.fn(),
  profilTaslagi: vi.fn(),
  katalogGirdisi: vi.fn(async () => null),
  exeDosyasiSec: vi.fn(async () => null),
  profilDosyasiSec: vi.fn(),
  profilDosyasiHedefi: vi.fn(),
  profilOnizle: vi.fn(),
  profilIceAktar: vi.fn(async () => []),
  profilDisaAktar: vi.fn(async () => undefined),
  profilEtkileri: vi.fn(async () => []),
  // Ağ paneli açıldığında (kısayol testi) çağrılanlar. Hepsi boş dönüyor:
  // bu testlerin konusu ağ ölçümü değil, sekmenin açılması.
  tcpDurumu: vi.fn(async () => null),
  qosIlkeleri: vi.fn(async () => []),
  agAciklamalari: vi.fn(async () => []),
  dnsKarsilastir: vi.fn(async () => []),
  yolTesti: vi.fn(async () => null),
  tcpUygula: vi.fn(),
  qosKaldir: vi.fn(async () => 0),
}));

const sahte = vi.mocked(api);

beforeEach(() => {
  sahte.kisitlar.mockResolvedValue(KISITLAR_TAM);
  sahte.durum.mockResolvedValue(durum());
  sahte.ozet.mockResolvedValue(ozet());
  sahte.profiller.mockResolvedValue([]);
});

/** Açılış yüklemesi bitene kadar bekler. */
async function uygulamayiAc() {
  render(<App />);
  await screen.findByRole('button', { name: /durum/i });
}

describe('App — sürüm kısıtları', () => {
  it('tam sürümde ağ sekmesi görünüyor', async () => {
    await uygulamayiAc();
    expect(screen.getByRole('button', { name: /ağ/i })).toBeTruthy();
  });

  it('demo ikilisinde ağ sekmesi hiç görünmüyor', async () => {
    sahte.kisitlar.mockResolvedValue(KISITLAR_DEMO);
    await uygulamayiAc();

    await waitFor(() => {
      expect(screen.queryByRole('button', { name: /^ağ$/i })).toBeNull();
    });
  });

  it('kısıtlar okunamazsa tam sürüm gibi davranıyor', async () => {
    // Komut hata verse bile arayüz kilitlenmemeli: eksik bilgi yüzünden
    // özellik gizlemek, çalışan bir kurulumu bozuk gösterirdi.
    sahte.kisitlar.mockRejectedValue(new Error('komut yok'));
    await uygulamayiAc();

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /ağ/i })).toBeTruthy();
    });
  });
});

describe('App — klavye kısayolları', () => {
  /**
   * Numaralar GÖRÜNEN sekmelere göre sayılıyor. Demoda ağ sekmesi hiç
   * çizilmediği için Ctrl+3 oradaki üçüncü sekmeyi (Ölçekleme) açmalı; sabit
   * bir eşleme, kullanıcıyı var olmayan bir sekmeye götürürdü.
   */
  it('Ctrl+3 tam sürümde ağ sekmesini açıyor', async () => {
    const kullanici = userEvent.setup();
    await uygulamayiAc();

    await kullanici.keyboard('{Control>}3{/Control}');

    await waitFor(() => {
      const dugme = screen.getByRole('button', { name: /^ağ$/i });
      expect(dugme.getAttribute('aria-current')).toBe('page');
    });
  });

  it('Ctrl+3 demo ikilisinde bir sonraki sekmeyi açıyor', async () => {
    sahte.kisitlar.mockResolvedValue(KISITLAR_DEMO);
    const kullanici = userEvent.setup();
    await uygulamayiAc();
    // Ağ sekmesinin gerçekten çizilmediğinden emin ol, sonra kısayolu dene.
    await waitFor(() => {
      expect(screen.queryByRole('button', { name: /^ağ$/i })).toBeNull();
    });

    await kullanici.keyboard('{Control>}3{/Control}');

    // Ağ sekmesi yokken üçüncü sıra Ölçekleme'ye kayıyor: numaralar
    // ekrandaki sıralamayı takip ediyor, sabit bir sekmeyi değil.
    await waitFor(() => {
      const dugme = screen.getByRole('button', { name: /^ölçekleme$/i });
      expect(dugme.getAttribute('aria-current')).toBe('page');
    });
  });

  /**
   * Geçmiş sekmesi demoda da var: kayıtları göstermek bir ücretli özellik
   * değil, şeffaflık ilkesinin gereği (`monitor::gecmis`).
   */
  it('Geçmiş sekmesi demo ikilisinde de görünüyor', async () => {
    sahte.kisitlar.mockResolvedValue(KISITLAR_DEMO);
    await uygulamayiAc();

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /^geçmiş$/i })).toBeTruthy();
    });
  });
});

describe('App — profil kaydetme', () => {
  it('backend düzeltme döndürdüğünde kullanıcıya gösteriyor', async () => {
    sahte.profilKaydet.mockResolvedValue([
      "'realtime' önceliği desteklenmiyor, 'high' yapıldı",
    ]);
    const kullanici = userEvent.setup();
    await uygulamayiAc();

    await kullanici.click(screen.getByRole('button', { name: /profiller/i }));
    await kullanici.click(screen.getByRole('button', { name: /yeni profil/i }));

    await kullanici.type(screen.getByPlaceholderText(/Counter-Strike 2/i), 'CS2');
    await kullanici.type(screen.getByPlaceholderText('cs2.exe'), 'cs2.exe');
    await kullanici.click(screen.getByRole('button', { name: /^kaydet$/i }));

    expect(await screen.findByText(/'realtime' önceliği desteklenmiyor/)).toBeTruthy();
  });

  it('düzeltme yoksa yalnızca kaydedildi diyor', async () => {
    sahte.profilKaydet.mockResolvedValue([]);
    const kullanici = userEvent.setup();
    await uygulamayiAc();

    await kullanici.click(screen.getByRole('button', { name: /profiller/i }));
    await kullanici.click(screen.getByRole('button', { name: /yeni profil/i }));
    await kullanici.type(screen.getByPlaceholderText(/Counter-Strike 2/i), 'CS2');
    await kullanici.type(screen.getByPlaceholderText('cs2.exe'), 'cs2.exe');
    await kullanici.click(screen.getByRole('button', { name: /^kaydet$/i }));

    expect(await screen.findByText(/profil kaydedildi/i)).toBeTruthy();
  });

  it('demo sınırı dolduğunda yeni profil düğmesi kapalı', async () => {
    sahte.kisitlar.mockResolvedValue(KISITLAR_DEMO);
    sahte.profiller.mockResolvedValue([profil()]);
    const kullanici = userEvent.setup();
    await uygulamayiAc();

    await kullanici.click(screen.getByRole('button', { name: /profiller/i }));

    await waitFor(() => {
      const dugme = screen.getByRole('button', { name: /yeni profil/i }) as HTMLButtonElement;
      expect(dugme.disabled).toBe(true);
    });
    // Neden kapalı olduğu yazıyor: kapalı bir düğme gerekçesiz bırakılmıyor.
    expect(screen.getByText(/tek profil oluşturulabiliyor/i)).toBeTruthy();
  });
});

describe('App — profil içe/dışa aktarma', () => {
  /** Profiller sekmesini açar. */
  async function profillerSekmesi(kullanici: ReturnType<typeof userEvent.setup>) {
    await uygulamayiAc();
    await kullanici.click(screen.getByRole('button', { name: /profiller/i }));
  }

  it('dosya seçilince önce önizleme gösteriliyor, diske yazılmıyor', async () => {
    // `docs/PROFILES.md` güvenlik notu: başkasının profili, içeriği
    // gösterilmeden uygulanmıyor.
    sahte.profilDosyasiSec.mockResolvedValue('C:\\gelen\\cs2.json');
    sahte.profilOnizle.mockResolvedValue(
      onizleme({
        uyarilar: ['Bu profil senin makinende şu uygulamaları donduracak: discord.exe'],
      }),
    );
    const kullanici = userEvent.setup();
    await profillerSekmesi(kullanici);

    await kullanici.click(screen.getByRole('button', { name: 'İçe aktar' }));

    expect(await screen.findByText(/discord\.exe/)).toBeTruthy();
    expect(sahte.profilIceAktar).not.toHaveBeenCalled();
  });

  it('onaylanınca üzerine yazmadan aktarıyor', async () => {
    sahte.profilDosyasiSec.mockResolvedValue('C:\\gelen\\cs2.json');
    sahte.profilOnizle.mockResolvedValue(onizleme());
    const kullanici = userEvent.setup();
    await profillerSekmesi(kullanici);

    await kullanici.click(screen.getByRole('button', { name: 'İçe aktar' }));
    const diyalog = await screen.findByRole('dialog');
    await kullanici.click(within(diyalog).getByRole('button', { name: 'İçe aktar' }));

    await waitFor(() => {
      expect(sahte.profilIceAktar).toHaveBeenCalledWith(expect.anything(), false);
    });
  });

  it('kimlik çakışmasında varsayılan üzerine yazmamak', async () => {
    sahte.profilDosyasiSec.mockResolvedValue('C:\\gelen\\cs2.json');
    sahte.profilOnizle.mockResolvedValue(
      onizleme({ kimlikCakismasi: true, bosKimlik: 'cs2-2' }),
    );
    const kullanici = userEvent.setup();
    await profillerSekmesi(kullanici);

    await kullanici.click(screen.getByRole('button', { name: 'İçe aktar' }));

    const anahtar = await screen.findByRole('switch', { name: /üzerine yaz/i });
    expect(anahtar.getAttribute('aria-checked')).toBe('false');
    // Hangi kimlikle kaydedileceği görünüyor: sürpriz olmamalı.
    expect(screen.getAllByText('cs2-2').length).toBeGreaterThan(0);
  });

  it('dosya seçilmezse hiçbir şey olmuyor', async () => {
    sahte.profilDosyasiSec.mockResolvedValue(null);
    const kullanici = userEvent.setup();
    await profillerSekmesi(kullanici);

    await kullanici.click(screen.getByRole('button', { name: 'İçe aktar' }));

    await waitFor(() => {
      expect(sahte.profilOnizle).not.toHaveBeenCalled();
    });
  });

  it('demo ikilisinde aktarım düğmeleri kapalı ve gerekçesi yazıyor', async () => {
    sahte.kisitlar.mockResolvedValue(KISITLAR_DEMO);
    sahte.profiller.mockResolvedValue([profil()]);
    const kullanici = userEvent.setup();
    await profillerSekmesi(kullanici);

    await waitFor(() => {
      const dugme = screen.getByRole('button', { name: 'İçe aktar' }) as HTMLButtonElement;
      expect(dugme.disabled).toBe(true);
      expect(dugme.title).toMatch(/demo/i);
    });
  });

  it('dışa aktarma seçilen dosyaya yazıyor', async () => {
    sahte.profiller.mockResolvedValue([profil()]);
    sahte.profilDosyasiHedefi.mockResolvedValue('C:\\paylas\\cs2.json');
    const kullanici = userEvent.setup();
    await profillerSekmesi(kullanici);

    await kullanici.click(
      await screen.findByRole('button', { name: /profilini dışa aktar/i }),
    );

    await waitFor(() => {
      expect(sahte.profilDisaAktar).toHaveBeenCalledWith('cs2', 'C:\\paylas\\cs2.json');
    });
  });
});

describe('App — ayarlar', () => {
  it('bildirim anahtarı varsayılan kapalı ve açılınca kaydediliyor', async () => {
    // Varsayılanın kapalı olması bir ürün duruşu (`settings.rs`): kurulduğu
    // anda bildirim atan bir araç, ilk izlenimini gürültüyle veriyor.
    const kullanici = userEvent.setup();
    await uygulamayiAc();
    await kullanici.click(screen.getByRole('button', { name: /ayarlar/i }));

    const anahtar = await screen.findByRole('switch', { name: /bildirim göster/i });
    expect(anahtar.getAttribute('aria-checked')).toBe('false');

    await kullanici.click(anahtar);

    await waitFor(() => {
      expect(sahte.ayarlariYaz).toHaveBeenCalledWith(
        expect.objectContaining({ modBildirimi: true }),
      );
    });
  });
});

describe('App — profilin ne yapacağı', () => {
  it('satırdaki özet açılınca backend metinlerini gösteriyor', async () => {
    // Metinler backend'de (karar #17): arayüz ikinci bir cümle kurmuyor,
    // içe aktarma önizlemesiyle aynı listeyi gösteriyor.
    sahte.profiller.mockResolvedValue([profil()]);
    sahte.profilEtkileri.mockResolvedValue([
      'Eşleşen uygulama: cs2.exe',
      'Oyun boyunca dondurulacak uygulamalar: discord.exe',
    ]);
    const kullanici = userEvent.setup();
    await uygulamayiAc();
    await kullanici.click(screen.getByRole('button', { name: /profiller/i }));

    await kullanici.click(await screen.findByText('Ne yapacak?'));

    expect(await screen.findByText(/dondurulacak uygulamalar: discord\.exe/)).toBeTruthy();
    expect(sahte.profilEtkileri).toHaveBeenCalledWith('cs2');
  });

  it('özet açılmadan komut çağrılmıyor', async () => {
    sahte.profiller.mockResolvedValue([profil()]);
    const kullanici = userEvent.setup();
    await uygulamayiAc();
    await kullanici.click(screen.getByRole('button', { name: /profiller/i }));

    await screen.findByText('Ne yapacak?');
    expect(sahte.profilEtkileri).not.toHaveBeenCalled();
  });
});
