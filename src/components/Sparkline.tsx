/**
 * Mini eğri — ölçüm kutularının ve mod kartının altındaki küçük çizgi.
 *
 * Eksen, ızgara, etiket yok: bu çizginin işi bir sayı okutmak değil, o sayının
 * son birkaç ölçümde ne yaptığını (düz mü, tırmanıyor mu, dalgalı mı) tek
 * bakışta göstermek. Sayının kendisi zaten kutunun içinde yazıyor.
 *
 * Ölçülemeyen noktalar (`null`) çizgide **boşluk** bırakıyor, sıfıra düşmüyor
 * — `MetrikGrafik` ile aynı kural, aynı gerekçe (`docs/DESIGN_PRINCIPLES.md`
 * madde 4: ölçüm yokluğu sıfır gibi gösterilmez).
 */

import { useMemo } from 'react';

const GENISLIK = 100;
const YUKSEKLIK = 26;

interface Props {
  degerler: (number | null)[];
  /** Sabit tavan (ör. yüzdeler için 100). Verilmezse veriye göre ölçekleniyor. */
  tavan?: number;
  renk?: string;
  /** Çizginin altını dolduran degrade. Kalabalık ızgaralarda kapatılabiliyor. */
  dolgu?: boolean;
  className?: string;
}

export function Sparkline({
  degerler,
  tavan,
  renk = 'var(--accent)',
  dolgu = true,
  className,
}: Props) {
  const { cizgi, alan } = useMemo(() => {
    const olculen = degerler.filter((d): d is number => d != null);
    if (degerler.length < 2 || olculen.length < 2) return { cizgi: '', alan: '' };

    const enBuyuk = tavan ?? Math.max(...olculen) * 1.15;
    if (!(enBuyuk > 0)) return { cizgi: '', alan: '' };

    const adim = GENISLIK / (degerler.length - 1);
    const y = (deger: number) => {
      const oran = Math.min(Math.max(deger / enBuyuk, 0), 1);
      // 1px pay: en tepedeki nokta kenarlıkta kesilmesin.
      return 1 + (1 - oran) * (YUKSEKLIK - 2);
    };

    let cizgi = '';
    let alan = '';
    let kalemDe = false;

    degerler.forEach((deger, i) => {
      if (deger == null) {
        // Kayıp ölçüm: çizgiyi kes. Alan dolgusu da burada kapanıyor.
        if (kalemDe) alan += `L${(i - 1) * adim} ${YUKSEKLIK}Z `;
        kalemDe = false;
        return;
      }
      const x = i * adim;
      const yy = y(deger);
      cizgi += `${kalemDe ? 'L' : 'M'}${x.toFixed(1)} ${yy.toFixed(1)} `;
      alan += kalemDe
        ? `L${x.toFixed(1)} ${yy.toFixed(1)} `
        : `M${x.toFixed(1)} ${YUKSEKLIK}L${x.toFixed(1)} ${yy.toFixed(1)} `;
      kalemDe = true;
    });

    if (kalemDe) alan += `L${GENISLIK} ${YUKSEKLIK}Z`;

    return { cizgi: cizgi.trim(), alan: alan.trim() };
  }, [degerler, tavan]);

  if (!cizgi) return null;

  // Degradenin kimliği renge bağlı: aynı ekranda iki farklı renkte eğri varsa
  // ikisi de kendi degradesini kullanmalı.
  const kimlik = `spark-${renk.replace(/[^a-z0-9]/gi, '')}`;

  return (
    <svg
      className={className ?? 'olcum__spark'}
      viewBox={`0 0 ${GENISLIK} ${YUKSEKLIK}`}
      preserveAspectRatio="none"
      aria-hidden="true"
      focusable="false"
    >
      {dolgu && (
        <>
          <defs>
            <linearGradient id={kimlik} x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor={renk} stopOpacity="0.28" />
              <stop offset="100%" stopColor={renk} stopOpacity="0" />
            </linearGradient>
          </defs>
          <path d={alan} fill={`url(#${kimlik})`} stroke="none" />
        </>
      )}
      <path
        d={cizgi}
        fill="none"
        stroke={renk}
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
        vectorEffect="non-scaling-stroke"
      />
    </svg>
  );
}
