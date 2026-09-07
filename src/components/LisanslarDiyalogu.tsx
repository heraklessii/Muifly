/**
 * Üçüncü taraf lisanslar.
 *
 * İki tasarım kararı:
 *
 * 1. **Metinler talep üzerine geliyor.** Liste 258 bileşen; lisans
 *    metinlerinin tamamı bir megabaytın üstünde. Açılışta hepsini çekmek,
 *    kullanıcının okuyacağı tek metin için gereksiz. Bir bileşen açıldığında
 *    `ucuncuTarafMetni` çağrılıyor ve sonuç bellekte tutuluyor.
 *
 * 2. **Metni olmayan bileşen gizlenmiyor.** Bazı paketler yayımlanan
 *    sürümlerinde LICENSE dosyası taşımıyor. Onları listeden çıkarmak listeyi
 *    temiz gösterirdi ama eksik yapardı; SPDX kimliği ve kaynak adresiyle
 *    duruyorlar, eksik olan da açıkça yazıyor.
 */

import { useEffect, useMemo, useState } from 'react';

import * as api from '../lib/api';
import type { UcuncuTarafBileseni, UcuncuTarafListesi, UcuncuTarafTuru } from '../lib/types';
import { IconKapat, IconUyari } from './Icons';

interface Props {
  onKapat: () => void;
}

const TUR_ADI: Record<UcuncuTarafTuru, string> = {
  rust: 'Rust',
  npm: 'npm',
  varlik: 'varlık',
};

function anahtar(b: UcuncuTarafBileseni): string {
  return `${b.tur}:${b.ad}@${b.surum}`;
}

/**
 * Kaynak adresi paket üstverisinden geliyor; ne olduğu garanti değil.
 * Yalnızca `https://` ile başlayanlar bağlantı olarak açılıyor, gerisi düz
 * metin kalıyor.
 */
function acilabilirMi(kaynak: string | null): kaynak is string {
  return typeof kaynak === 'string' && kaynak.startsWith('https://');
}

export function LisanslarDiyalogu({ onKapat }: Props) {
  const [liste, setListe] = useState<UcuncuTarafListesi | null>(null);
  const [hata, setHata] = useState<string | null>(null);
  const [arama, setArama] = useState('');
  const [acik, setAcik] = useState<string | null>(null);
  const [metinler, setMetinler] = useState<Record<number, string>>({});
  const [metinHatasi, setMetinHatasi] = useState<string | null>(null);

  useEffect(() => {
    api
      .ucuncuTarafListesi()
      .then(setListe)
      .catch((e) => setHata(String(e)));
  }, []);

  // Escape ile kapanma: diyaloglarda beklenen davranış.
  useEffect(() => {
    const f = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onKapat();
    };
    window.addEventListener('keydown', f);
    return () => window.removeEventListener('keydown', f);
  }, [onKapat]);

  const suzulmus = useMemo(() => {
    const bilesenler = liste?.bilesenler ?? [];
    const q = arama.trim().toLocaleLowerCase('tr');
    if (!q) return bilesenler;
    return bilesenler.filter(
      (b) =>
        b.ad.toLocaleLowerCase('tr').includes(q) ||
        (b.lisans ?? '').toLocaleLowerCase('tr').includes(q),
    );
  }, [liste, arama]);

  async function ac(b: UcuncuTarafBileseni) {
    const k = anahtar(b);
    if (acik === k) {
      setAcik(null);
      return;
    }
    setAcik(k);
    setMetinHatasi(null);

    if (b.metinNo === null || metinler[b.metinNo] !== undefined) return;
    try {
      const metin = await api.ucuncuTarafMetni(b.metinNo);
      setMetinler((m) => ({ ...m, [b.metinNo as number]: metin }));
    } catch (e) {
      setMetinHatasi(String(e));
    }
  }

  return (
    <div className="perde" onMouseDown={(e) => e.target === e.currentTarget && onKapat()}>
      <div className="diyalog genis" role="dialog" aria-modal="true" aria-label="Üçüncü taraf lisanslar">
        <div className="diyalog__baslik">
          Üçüncü taraf lisanslar
          <button className="button ghost icon" onClick={onKapat} aria-label="Kapat">
            <IconKapat />
          </button>
        </div>

        <div className="diyalog__govde">
          <p className="field-hint">
            Muifly aşağıdaki bileşenleri kendisiyle birlikte dağıtıyor. Her birinin lisansı
            kendi sahibine aittir ve Muifly lisans sözleşmesinden bağımsızdır.
          </p>

          {hata && (
            <div className="serit uyari">
              <IconUyari />
              <span>Liste okunamadı: {hata}</span>
            </div>
          )}

          {!liste && !hata && <p className="field-hint">Yükleniyor…</p>}

          {liste && (
            <>
              <label className="field">
                <span>Ara</span>
                <input
                  className="text-input"
                  value={arama}
                  placeholder="bileşen adı ya da lisans (örn. tauri, MIT)"
                  onChange={(e) => setArama(e.target.value)}
                />
              </label>

              <div className="field-hint">
                {suzulmus.length} / {liste.bilesenler.length} bileşen
              </div>

              <div className="lisans-liste">
                {suzulmus.map((b) => {
                  const k = anahtar(b);
                  const acildi = acik === k;
                  return (
                    <div className="lisans" key={k}>
                      <button
                        className="lisans__baslik"
                        aria-expanded={acildi}
                        onClick={() => void ac(b)}
                      >
                        <span className="satir__ad">
                          {b.ad} <span className="lisans__surum">{b.surum}</span>
                        </span>
                        <span className="rozet">{TUR_ADI[b.tur]}</span>
                        <span className="rozet vurgu">{b.lisans ?? 'lisans bildirilmemiş'}</span>
                      </button>

                      {acildi && (
                        <div className="lisans__govde">
                          {b.kaynak && (
                            <div className="kv">
                              <span className="kv__ad">Kaynak</span>
                              {acilabilirMi(b.kaynak) ? (
                                <button
                                  className="kv__deger baglanti"
                                  onClick={() => void api.adresiAc(b.kaynak as string)}
                                >
                                  {b.kaynak}
                                </button>
                              ) : (
                                <span className="kv__deger">{b.kaynak}</span>
                              )}
                            </div>
                          )}

                          {b.metinNo === null ? (
                            <div className="serit bilgi">
                              <IconUyari />
                              <span>
                                Bu paket yayımlanan sürümünde lisans metni taşımıyor. Yukarıdaki
                                SPDX kimliği ve kaynak adresi paketin kendi bildirimidir.
                              </span>
                            </div>
                          ) : metinler[b.metinNo] !== undefined ? (
                            <pre className="lisans__metin selectable">{metinler[b.metinNo]}</pre>
                          ) : metinHatasi ? (
                            <div className="serit uyari">
                              <IconUyari />
                              <span>Metin okunamadı: {metinHatasi}</span>
                            </div>
                          ) : (
                            <p className="field-hint">Yükleniyor…</p>
                          )}
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>

              <p className="field-hint">
                Liste {liste.uretildi} tarihinde {liste.hedef} derlemesi için üretildi ve
                uygulamanın içine gömüldü.
              </p>
            </>
          )}
        </div>

        <div className="diyalog__alt">
          <button className="button" onClick={onKapat}>
            Kapat
          </button>
        </div>
      </div>
    </div>
  );
}
