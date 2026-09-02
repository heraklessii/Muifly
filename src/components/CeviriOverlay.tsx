/**
 * Oyunun üstünde duran çeviri penceresi.
 *
 * Ayrı bir Tauri penceresi; içerik aynı `index.html`den geliyor ve
 * `main.tsx` adres satırındaki `?pencere=ceviri-overlay` ile buraya
 * yönlendiriyor.
 *
 * ## Karar #34'ün dersi burada da geçerli
 *
 * Ölçekleme penceresi bir kez makineyi kullanılamaz hâle getirmişti: tam
 * ekran, tıklanamaz, Alt+Tab'da görünmez ve kapatılacak hiçbir yolu yok.
 * Bu pencere o hataları tek tek yapmıyor — ekranın bir bölümünü kaplıyor,
 * tıklamaları geçirmiyor (yani kapatma düğmesi gerçekten çalışıyor) ve
 * çeviri kapatıldığında kendisi de kapanıyor.
 *
 * ## Kaynak metin gizlenmiyor
 *
 * Karar #29 zaaf 3: model bir cümleyi hata vermeden düşürebiliyor. Bu
 * yüzden overlay yalnızca çeviriyi göstermiyor, kaynağını da gösteriyor —
 * "sadece Türkçesini göster" daha temiz görünürdü ve ölçülmüş bir riski
 * görünmez kılardı.
 */

import { useEffect, useState } from 'react';

import * as api from '../lib/api';
import type { CeviriDurumu, CeviriSonucu } from '../lib/types';
import { ASAMA_ETIKETLERI } from '../lib/types';

export function CeviriOverlay() {
  const [sonuc, setSonuc] = useState<CeviriSonucu | null>(null);
  const [durum, setDurum] = useState<CeviriDurumu | null>(null);

  useEffect(() => {
    // Pencere sonuç geldikten sonra açılıyor; ilk içerik doğrudan okunuyor.
    api.ceviriSonucu().then(setSonuc).catch(() => {});
    api.ceviriDurumu().then(setDurum).catch(() => {});
    const abone = api.dinle<[CeviriDurumu, CeviriSonucu | null]>(api.OLAY_CEVIRI, ([d, s]) => {
      setDurum(d);
      if (s) setSonuc(s);
    });
    return () => {
      abone.then((f) => f()).catch(() => {});
    };
  }, []);

  const kapat = () => {
    api.ceviriOverlayKapat().catch(() => {});
  };

  return (
    <div className="overlay">
      <div className="overlay__ust">
        <span className="overlay__marka">Muifly · çeviri</span>
        {durum && durum.asama !== 'bosta' && (
          <span className="overlay__asama">{ASAMA_ETIKETLERI[durum.asama]}…</span>
        )}
        <span className="bosluk" />
        {durum?.kisayol && <kbd className="kbd">{durum.kisayol}</kbd>}
        <button className="overlay__kapat" onClick={kapat} aria-label="Kapat">
          ✕
        </button>
      </div>

      <div className="overlay__govde">
        {durum?.sonHata && <p className="overlay__hata">{durum.sonHata}</p>}

        {sonuc?.uyarilar.map((u, i) => (
          <p className="overlay__uyari" key={i}>
            {u}
          </p>
        ))}

        {!sonuc || sonuc.birimler.length === 0 ? (
          <p className="overlay__bos">Okunabilir bir yazı çıkmadı.</p>
        ) : (
          sonuc.birimler.map((b, i) => (
            <div className="overlay__birim" key={i}>
              <div className="overlay__kaynak">{b.kaynak}</div>
              <div className="overlay__ceviri">
                {b.bos ? (
                  <em>Bu cümle için bir şey üretilmedi.</em>
                ) : (
                  b.ceviri
                )}
                {b.kayipTerimler.length > 0 && (
                  <span className="overlay__rozet">
                    kayıp terim: {b.kayipTerimler.join(', ')}
                  </span>
                )}
              </div>
            </div>
          ))
        )}
      </div>

      <div className="overlay__dip">
        Makine çevirisi — üstteki satır ekrandan okunan özgün metin.
      </div>
    </div>
  );
}
