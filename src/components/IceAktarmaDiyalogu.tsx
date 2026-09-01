/**
 * İçe aktarma önizlemesi.
 *
 * `docs/PROFILES.md` güvenlik notu: paylaşılan bir profil
 * `suspend_process_list` gibi alanlar taşıyor — yani başkasının dosyası senin
 * makinende uygulama donduruyor. Bu yüzden akış iki adım: backend'in
 * `profil_onizle` komutu diske hiçbir şey yazmıyor, bu ekran ne geleceğini
 * gösteriyor, kaydetme ayrı bir çağrı.
 *
 * Kimlik çakışmasında varsayılan **üzerine yazmamak**: bir arkadaşından gelen
 * dosyanın senin profilini yok etmesi, geri alınamayan tek işlem olurdu.
 */

import { useEffect, useState } from 'react';

import type { Onizleme } from '../lib/types';
import { IconBilgi, IconKapat, IconUyari } from './Icons';

interface Props {
  onizleme: Onizleme;
  mesgul: boolean;
  onKapat: () => void;
  onAktar: (uzerineYaz: boolean) => void;
}

export function IceAktarmaDiyalogu({ onizleme, mesgul, onKapat, onAktar }: Props) {
  const [uzerineYaz, setUzerineYaz] = useState(false);

  useEffect(() => {
    const f = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onKapat();
    };
    window.addEventListener('keydown', f);
    return () => window.removeEventListener('keydown', f);
  }, [onKapat]);

  const { profil } = onizleme;

  return (
    <div className="perde" onMouseDown={(e) => e.target === e.currentTarget && onKapat()}>
      <div className="diyalog" role="dialog" aria-modal="true" aria-label="Profili içe aktar">
        <div className="diyalog__baslik">
          Profili içe aktar
          <button className="button ghost icon" onClick={onKapat} aria-label="Kapat">
            <IconKapat />
          </button>
        </div>

        <div className="diyalog__govde">
          <div className="kv">
            <span className="kv__ad">Dosya</span>
            <span className="kv__deger selectable">{onizleme.dosya}</span>
          </div>
          <div className="kv">
            <span className="kv__ad">Profil</span>
            <span className="kv__deger">{profil.display_name}</span>
          </div>
          <div className="kv">
            <span className="kv__ad">Kaydedilecek kimlik</span>
            <span className="kv__deger selectable">
              {uzerineYaz ? profil.profile_id : onizleme.bosKimlik}
            </span>
          </div>

          {onizleme.uyarilar.map((u) => (
            <div key={u} className="serit uyari">
              <IconUyari />
              <span>{u}</span>
            </div>
          ))}

          <div>
            <div className="satir__ad">Bu profil uygulanınca ne olacak</div>
            <ul className="madde-liste">
              {onizleme.etkiler.map((e) => (
                <li key={e}>{e}</li>
              ))}
            </ul>
          </div>

          {onizleme.duzeltmeler.length > 0 && (
            <div className="serit bilgi">
              <IconBilgi />
              <span>
                Dosyada düzeltilen noktalar:{' '}
                {onizleme.duzeltmeler.join('; ')}. Kaydedilecek olan, düzeltilmiş
                hali.
              </span>
            </div>
          )}

          {onizleme.kimlikCakismasi && (
            <div className="field row">
              <div>
                <div className="satir__ad">Var olan profilin üzerine yaz</div>
                <div className="field-hint">
                  Kapalıyken senin profilin yerinde kalır, gelen profil{' '}
                  <code>{onizleme.bosKimlik}</code> kimliğiyle eklenir.
                </div>
              </div>
              <button
                className="switch"
                role="switch"
                aria-checked={uzerineYaz}
                aria-label="Var olan profilin üzerine yaz"
                onClick={() => setUzerineYaz(!uzerineYaz)}
              />
            </div>
          )}
        </div>

        <div className="diyalog__alt">
          <button className="button ghost" onClick={onKapat}>
            Vazgeç
          </button>
          <button
            className="button primary"
            disabled={mesgul}
            onClick={() => onAktar(uzerineYaz)}
          >
            {uzerineYaz ? 'Üzerine yaz' : 'İçe aktar'}
          </button>
        </div>
      </div>
    </div>
  );
}
