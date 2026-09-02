/**
 * Çevrilecek alanı seçme penceresi.
 *
 * ## Neden donmuş bir görüntü üstünde seçiliyor
 *
 * Alternatif, canlı ekranın üstüne saydam bir pencere açmaktı. Donmuş
 * görüntü iki sebeple daha iyi: kullanıcı **tam olarak neyin okunacağını**
 * görüyor (yakalamanın kendisi ne veriyorsa o), ve seçim yaparken oyunun
 * görüntüsü değişip hedefi kaçırmıyor.
 *
 * Görüntü küçültülmüş geliyor (`ceviri::ekran_goruntusu`); bu doğruluğu
 * etkilemiyor çünkü seçim **oran** olarak kaydediliyor.
 *
 * ## Kaçış yolu
 *
 * Pencere ekranı kaplıyor. Karar #34'ün dersi: kaplayan bir pencerenin
 * kapatma yolu, kaplamadan önce belli olmalı. Burada üç yol var — Esc,
 * "Vazgeç" düğmesi ve pencerenin kendi kapatma tuşu. Esc dinleyicisi
 * yüklenirken bile duruyor: görüntü gelmezse kullanıcı kilitlenmiyor.
 */

import { useCallback, useEffect, useRef, useState } from 'react';

import * as api from '../lib/api';
import type { Alan } from '../lib/types';

interface Nokta {
  x: number;
  y: number;
}

/** Seçimin geçerli sayılması için gereken en küçük kenar (ekran oranı). */
const EN_KUCUK_ORAN = 0.01;

export function AlanSecici({ kimlik }: { kimlik: string }) {
  const [goruntu, setGoruntu] = useState<string | null>(null);
  const [hata, setHata] = useState<string | null>(null);
  const [bas, setBas] = useState<Nokta | null>(null);
  const [son, setSon] = useState<Nokta | null>(null);
  const [kaydediliyor, setKaydediliyor] = useState(false);
  const kutu = useRef<HTMLDivElement>(null);

  const kapat = useCallback(() => {
    api.ceviriAlanSeciciKapat().catch(() => {});
  }, []);

  useEffect(() => {
    api
      .ceviriEkranGoruntusu()
      .then(setGoruntu)
      .catch((e) => setHata(String(e)));
  }, []);

  useEffect(() => {
    const f = (e: KeyboardEvent) => {
      if (e.key === 'Escape') kapat();
    };
    window.addEventListener('keydown', f);
    return () => window.removeEventListener('keydown', f);
  }, [kapat]);

  /** Fare konumunu pencere içindeki orana çevirir. */
  const oran = (e: React.MouseEvent): Nokta => {
    const k = kutu.current?.getBoundingClientRect();
    if (!k || k.width === 0 || k.height === 0) return { x: 0, y: 0 };
    return {
      x: Math.min(1, Math.max(0, (e.clientX - k.left) / k.width)),
      y: Math.min(1, Math.max(0, (e.clientY - k.top) / k.height)),
    };
  };

  const secim: Alan | null =
    bas && son
      ? {
          left: Math.min(bas.x, son.x),
          top: Math.min(bas.y, son.y),
          width: Math.abs(son.x - bas.x),
          height: Math.abs(son.y - bas.y),
        }
      : null;

  const gecerli =
    secim !== null && secim.width >= EN_KUCUK_ORAN && secim.height >= EN_KUCUK_ORAN;

  const kaydet = async (alan: Alan | null) => {
    setKaydediliyor(true);
    try {
      await api.ceviriAlaniKaydet(kimlik, alan);
      kapat();
    } catch (e) {
      setHata(String(e));
      setKaydediliyor(false);
    }
  };

  return (
    <div className="secici">
      <div
        className="secici__tuval"
        ref={kutu}
        onMouseDown={(e) => {
          const n = oran(e);
          setBas(n);
          setSon(n);
        }}
        onMouseMove={(e) => {
          if (bas) setSon(oran(e));
        }}
        onMouseUp={(e) => {
          if (bas) setSon(oran(e));
        }}
      >
        {goruntu ? (
          <img src={goruntu} alt="" draggable={false} />
        ) : (
          <div className="secici__bekle">
            {hata ? hata : 'Ekran görüntüsü alınıyor…'}
          </div>
        )}

        {secim && (
          <div
            className="secici__kutu"
            style={{
              left: `${secim.left * 100}%`,
              top: `${secim.top * 100}%`,
              width: `${secim.width * 100}%`,
              height: `${secim.height * 100}%`,
            }}
          />
        )}
      </div>

      <div className="secici__cubuk">
        <span className="secici__yardim">
          Çevrilecek yazının durduğu yeri fareyle çiz. Seçim{' '}
          <strong>oran olarak</strong> kaydediliyor; başka bir çözünürlükte de
          aynı yere düşer. <kbd className="kbd">Esc</kbd> ile vazgeç.
        </span>
        {secim && (
          <span className="secici__olcu">
            %{Math.round(secim.width * 100)} × %{Math.round(secim.height * 100)}
          </span>
        )}
        <span className="bosluk" />
        <button
          className="button ghost"
          disabled={kaydediliyor}
          onClick={() => kaydet(null)}
        >
          Tüm ekranı kullan
        </button>
        <button className="button ghost" onClick={kapat}>
          Vazgeç
        </button>
        <button
          className="button primary"
          disabled={!gecerli || kaydediliyor}
          onClick={() => secim && kaydet(secim)}
        >
          Alanı kaydet
        </button>
      </div>
    </div>
  );
}
