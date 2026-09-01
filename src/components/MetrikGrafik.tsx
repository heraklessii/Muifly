/**
 * Ölçüm grafiği — bağımlılıksız SVG.
 *
 * Bir grafik kütüphanesi eklenmedi: iki çizgi çizmek için 50-100 KB'lik bir
 * paket, "hafif araç" vaadiyle çelişirdi ve tema jetonlarını kendi renk
 * sistemine çevirmek gerekirdi.
 *
 * Üç kural:
 *
 * 1. **İki ölçek var.** CPU yüzde (0-100, sabit) ve gecikme (veriye göre
 *    otomatik). Gecikmenin sabit bir tavanı yok çünkü 15 ms ile 300 ms aynı
 *    eksende çizilirse ikisinden biri okunmaz oluyor. Tavan efsanede yazıyor,
 *    yoksa iki çizginin aynı ölçekte olduğu sanılır.
 * 2. **Ölçüm yapılamayan noktalar boşluk bırakıyor**, sıfıra düşmüyor: sıfıra
 *    düşen bir çizgi "gecikme yok" gibi okunurdu (`DESIGN_PRINCIPLES.md` md. 4).
 * 3. **Metin SVG'nin dışında.** `preserveAspectRatio="none"` çizgiyi pencere
 *    genişliğine yayıyor; içerideki bir `<text>` de yatay olarak gerilirdi.
 *    Eksen etiketleri ve imleç okuması bu yüzden HTML katmanında.
 */

import { useMemo, useState, type PointerEvent } from 'react';

import type { Ornek } from '../lib/types';
import { milisaniye, saat, yuzde } from '../lib/format';

const GENISLIK = 600;
const YUKSEKLIK = 180;
const PAY = 8;

interface Props {
  ornekler: Ornek[];
  /** Ölçüm aralığı — grafiğin kaç saniyeyi gösterdiğini yazmak için. */
  aralikSn?: number;
}

/** Bir değerin dikey konumu. */
function yKoordinat(deger: number, enBuyuk: number): number {
  const oran = Math.min(Math.max(deger / enBuyuk, 0), 1);
  return YUKSEKLIK - PAY - oran * (YUKSEKLIK - PAY * 2);
}

/** Değerleri SVG yol dizesine çevirir; `null` değerlerde yolu kesiyor. */
function yol(degerler: (number | null)[], enBuyuk: number): { cizgi: string; alan: string } {
  if (degerler.length < 2 || enBuyuk <= 0) return { cizgi: '', alan: '' };
  const adim = (GENISLIK - PAY * 2) / (degerler.length - 1);
  let cizgi = '';
  let alan = '';
  let kalemDe = false;

  degerler.forEach((deger, i) => {
    const x = PAY + i * adim;
    if (deger == null) {
      // Kayıp ölçüm: çizgiyi kes, sıfıra indirme.
      if (kalemDe) alan += `L${(x - adim).toFixed(1)} ${YUKSEKLIK - PAY}Z `;
      kalemDe = false;
      return;
    }
    const y = yKoordinat(deger, enBuyuk);
    cizgi += `${kalemDe ? 'L' : 'M'}${x.toFixed(1)} ${y.toFixed(1)} `;
    alan += kalemDe
      ? `L${x.toFixed(1)} ${y.toFixed(1)} `
      : `M${x.toFixed(1)} ${YUKSEKLIK - PAY}L${x.toFixed(1)} ${y.toFixed(1)} `;
    kalemDe = true;
  });

  if (kalemDe) alan += `L${(GENISLIK - PAY).toFixed(1)} ${YUKSEKLIK - PAY}Z`;
  return { cizgi: cizgi.trim(), alan: alan.trim() };
}

export function MetrikGrafik({ ornekler, aralikSn }: Props) {
  // İmlecin durduğu örneğin sırası. `null` = imleç grafiğin dışında.
  const [imlec, setImlec] = useState<number | null>(null);

  const { cpu, gecikme, gecikmeTavani, gecikmeVar } = useMemo(() => {
    const cpuDegerleri = ornekler.map((o) => o.cpu);
    const gecikmeDegerleri = ornekler.map((o) => o.gecikmeMs);
    const olculen = gecikmeDegerleri.filter((g): g is number => g != null);

    // Tavan, en büyük ölçümün biraz üstünde: çizgi tam tepeye yapışmasın.
    const tavan = olculen.length > 0 ? Math.max(...olculen) * 1.25 : 0;

    return {
      cpu: yol(cpuDegerleri, 100),
      gecikme: yol(gecikmeDegerleri, tavan),
      gecikmeTavani: tavan,
      gecikmeVar: olculen.length > 0,
    };
  }, [ornekler]);

  if (ornekler.length < 2) {
    return (
      <div className="bos-durum">
        <strong>Ölçüm birikiyor</strong>
        <p>
          Grafik ikinci ölçümden sonra çizilmeye başlıyor. Aralık Ayarlar'dan
          değiştirilebilir.
        </p>
      </div>
    );
  }

  const adim = (GENISLIK - PAY * 2) / (ornekler.length - 1);
  const secili = imlec != null ? ornekler[imlec] : null;
  // Okuma kutusunun yatay konumu — yüzde olarak, çünkü SVG esniyor.
  const imlecYuzde = imlec != null ? ((PAY + imlec * adim) / GENISLIK) * 100 : 0;

  function imleciTasi(e: PointerEvent<SVGSVGElement>) {
    const kutu = e.currentTarget.getBoundingClientRect();
    if (kutu.width === 0) return;
    const oran = (e.clientX - kutu.left) / kutu.width;
    const x = oran * GENISLIK;
    const sira = Math.round((x - PAY) / adim);
    setImlec(Math.min(Math.max(sira, 0), ornekler.length - 1));
  }

  const kapsananSn = aralikSn ? ornekler.length * aralikSn : null;

  return (
    <>
      {/* Sarmal yalnızca çizimi kapsıyor: imleç noktaları yüzde konumlarını
          bu kutuya göre alıyor, efsane dışarıda kalmalı. */}
      <div className="grafik-sarmal">
        <svg
          className="grafik"
          viewBox={`0 0 ${GENISLIK} ${YUKSEKLIK}`}
          preserveAspectRatio="none"
          role="img"
          aria-label="CPU kullanımı ve gecikme grafiği"
          onPointerMove={imleciTasi}
          onPointerLeave={() => setImlec(null)}
        >
          {/* Yatay kılavuzlar: %25 / %50 / %75. Izgara sönük, veri değil
              referans — çizgilerden daha belirgin olmamalı. */}
          {[0.25, 0.5, 0.75].map((o) => (
            <line
              key={o}
              x1={PAY}
              x2={GENISLIK - PAY}
              y1={yKoordinat(o * 100, 100)}
              y2={yKoordinat(o * 100, 100)}
              stroke="var(--border)"
              strokeDasharray={o === 0.5 ? undefined : '3 5'}
              vectorEffect="non-scaling-stroke"
            />
          ))}

          <defs>
            <linearGradient id="cpuDolgu" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="var(--accent)" stopOpacity="0.26" />
              <stop offset="100%" stopColor="var(--accent)" stopOpacity="0" />
            </linearGradient>
          </defs>

          <path d={cpu.alan} fill="url(#cpuDolgu)" stroke="none" />

          {gecikmeVar && (
            <path
              d={gecikme.cizgi}
              fill="none"
              stroke="var(--warning)"
              strokeWidth="1.6"
              strokeLinejoin="round"
              vectorEffect="non-scaling-stroke"
            />
          )}

          <path
            d={cpu.cizgi}
            fill="none"
            stroke="var(--accent)"
            strokeWidth="1.8"
            strokeLinejoin="round"
            vectorEffect="non-scaling-stroke"
          />

          {/* İmleç çizgisi. Noktalar SVG'de değil HTML katmanında: yatay
              gerdirme bir daireyi elipse çevirirdi. */}
          {secili && imlec != null && (
            <line
              x1={PAY + imlec * adim}
              x2={PAY + imlec * adim}
              y1={PAY}
              y2={YUKSEKLIK - PAY}
              stroke="var(--border-strong)"
              vectorEffect="non-scaling-stroke"
            />
          )}
        </svg>

        {secili && (
          <>
            <span
              className="grafik-nokta"
              style={{
                left: `${imlecYuzde}%`,
                top: `${(yKoordinat(secili.cpu, 100) / YUKSEKLIK) * 100}%`,
                background: 'var(--accent)',
              }}
            />
            {secili.gecikmeMs != null && gecikmeTavani > 0 && (
              <span
                className="grafik-nokta"
                style={{
                  left: `${imlecYuzde}%`,
                  top: `${(yKoordinat(secili.gecikmeMs, gecikmeTavani) / YUKSEKLIK) * 100}%`,
                  background: 'var(--warning)',
                }}
              />
            )}
          </>
        )}

        {/* Eksen etiketleri: solda CPU yüzdesi, sağda gecikme tavanı. */}
        <div className="grafik-eksen">
          <span style={{ top: 0 }}>100%</span>
          <span style={{ top: '50%' }}>50%</span>
          {gecikmeVar && (
            <span className="sag" style={{ top: 0 }}>
              {Math.round(gecikmeTavani)} ms
            </span>
          )}
        </div>

        {secili && (
          <div
            className="grafik-okuma"
            style={{
              // Kenarlarda kutu dışarı taşmasın diye konum sıkıştırılıyor.
              left: `clamp(72px, ${imlecYuzde}%, calc(100% - 72px))`,
            }}
          >
            <div className="grafik-okuma__saat">{saat(secili.zaman)}</div>
            <div>
              CPU <b>{yuzde(secili.cpu, 0)}</b> · Bellek{' '}
              <b>{yuzde(secili.bellek, 0)}</b>
            </div>
            <div>
              Gecikme{' '}
              <b>
                {secili.gecikmeMs == null
                  ? 'ölçülemedi'
                  : milisaniye(secili.gecikmeMs, 1)}
              </b>
            </div>
          </div>
        )}
      </div>

      <div className="grafik-efsane">
        <span>
          <i style={{ background: 'var(--accent)' }} /> CPU · %0-100
        </span>
        {gecikmeVar ? (
          <span>
            <i style={{ background: 'var(--warning)' }} /> Gecikme · 0-
            {Math.round(gecikmeTavani)} ms
          </span>
        ) : (
          <span>Gecikme ölçümü yok</span>
        )}
        <span style={{ marginLeft: 'auto' }}>
          {ornekler.length} örnek
          {kapsananSn ? ` · yaklaşık son ${Math.round(kapsananSn)} sn` : ''}
        </span>
      </div>
    </>
  );
}
