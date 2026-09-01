/**
 * Kısa bildirimler.
 *
 * Kalıcı bilgi buraya değil günlüğe gidiyor: bir toast kaybolur, günlük
 * kalır. Toast yalnızca "az önce şu oldu" demek için (şeffaflık ilkesinin
 * kaydı günlüktedir, toast onun anlık yansıması).
 */

import { useCallback, useEffect, useRef, useState } from 'react';

import { IconBilgi, IconHata, IconKapat, IconTik } from './Icons';

export type ToastTuru = 'bilgi' | 'basari' | 'hata';

export interface Toast {
  id: number;
  tur: ToastTuru;
  mesaj: string;
}

/** Hata toast'ları kendiliğinden kapanmıyor: kullanıcı okumadan kaybolmamalı. */
const SURELER: Record<ToastTuru, number> = {
  bilgi: 4000,
  basari: 4000,
  hata: 0,
};

export function useToasts() {
  const [toastlar, setToastlar] = useState<Toast[]>([]);
  const sonrakiId = useRef(1);
  const zamanlayicilar = useRef<Map<number, number>>(new Map());

  const dusur = useCallback((id: number) => {
    setToastlar((t) => t.filter((x) => x.id !== id));
    const z = zamanlayicilar.current.get(id);
    if (z !== undefined) {
      window.clearTimeout(z);
      zamanlayicilar.current.delete(id);
    }
  }, []);

  const goster = useCallback(
    (tur: ToastTuru, mesaj: string) => {
      const id = sonrakiId.current++;
      setToastlar((t) => [...t, { id, tur, mesaj }]);
      const sure = SURELER[tur];
      if (sure > 0) {
        zamanlayicilar.current.set(
          id,
          window.setTimeout(() => dusur(id), sure),
        );
      }
      return id;
    },
    [dusur],
  );

  // Bileşen kaldırılırsa bekleyen zamanlayıcılar da kalkmalı.
  useEffect(() => {
    const harita = zamanlayicilar.current;
    return () => harita.forEach((z) => window.clearTimeout(z));
  }, []);

  return { toastlar, goster, dusur };
}

/** Türün ikonu. Renk `styles.css`ten, şekil buradan. */
const IKONLAR: Record<ToastTuru, typeof IconBilgi> = {
  bilgi: IconBilgi,
  basari: IconTik,
  hata: IconHata,
};

export function Toasts({ toastlar, dusur }: { toastlar: Toast[]; dusur: (id: number) => void }) {
  if (toastlar.length === 0) return null;
  return (
    <div className="toastlar">
      {toastlar.map((t) => {
        const Ikon = IKONLAR[t.tur];
        return (
        <div key={t.id} className="toast" data-tur={t.tur} role="status">
          <Ikon />
          <span className="selectable">{t.mesaj}</span>
          <button
            className="button ghost small icon"
            onClick={() => dusur(t.id)}
            aria-label="Bildirimi kapat"
          >
            <IconKapat />
          </button>
        </div>
        );
      })}
    </div>
  );
}
