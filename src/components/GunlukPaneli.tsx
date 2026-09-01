/**
 * Şeffaflık günlüğü.
 *
 * Tasarım ilkesi 2'nin ekranı. İki kural:
 *
 * 1. Geri alınan satır **silinmiyor**, yalnızca "geri al" düğmesi düşüyor.
 *    Geçmiş, geri alındıktan sonra da geçmiş.
 * 2. Süzme var ama varsayılan "hepsi": kullanıcı bir şeyin gizlendiğini
 *    hissetmemeli. Süzgeç açıkken kaç satırın gizlendiği yazıyor.
 *
 * Süzgeç açılır liste yerine bölümlenmiş seçici: üç seçenek var ve hangisinin
 * açık olduğu, listeye bakarken de görünmeli.
 */

import { useMemo, useState } from 'react';

import { DUZEY_ETIKETLERI, type Duzey, type Kayit, type Satir } from '../lib/types';
import { saat } from '../lib/format';
import { IconCop, IconGeriAl, IconGunluk } from './Icons';

interface Props {
  satirlar: Satir[];
  bekleyenler: Kayit[];
  mesgul: boolean;
  onGeriAl: (id: number) => void;
  onTemizle: () => void;
}

type Suzgec = 'hepsi' | 'degisiklik' | 'sorun';

const SUZGEC_ETIKETLERI: Record<Suzgec, string> = {
  hepsi: 'Hepsi',
  degisiklik: 'Değişiklikler',
  sorun: 'Uyarı ve hatalar',
};

function suzgeceUyuyor(duzey: Duzey, s: Suzgec): boolean {
  if (s === 'hepsi') return true;
  if (s === 'degisiklik') return duzey === 'aksiyon' || duzey === 'geriAlma';
  return duzey === 'uyari' || duzey === 'hata';
}

export function GunlukPaneli({ satirlar, bekleyenler, mesgul, onGeriAl, onTemizle }: Props) {
  const [suzgec, setSuzgec] = useState<Suzgec>('hepsi');
  const [arama, setArama] = useState('');

  const gosterilen = useMemo(() => {
    // Türkçe'ye duyarlı küçültme: `toLowerCase()` "İ"yi noktalı bir "i"ye
    // çeviriyor ve "İndirme" araması "indirme" ile eşleşmiyor.
    const q = arama.trim().toLocaleLowerCase('tr');
    return satirlar.filter(
      (s) =>
        suzgeceUyuyor(s.duzey, suzgec) &&
        (q === '' || s.mesaj.toLocaleLowerCase('tr').includes(q)),
    );
  }, [satirlar, suzgec, arama]);

  const suzuluyor = suzgec !== 'hepsi' || arama.trim() !== '';

  // Bir satırın "geri al" düğmesi, ilgili kayıt hâlâ defterde duruyorsa
  // gösteriliyor: geri alınmış bir şey için düğme göstermek yanıltıcı olurdu.
  const bekleyenIdler = useMemo(
    () => new Set(bekleyenler.map((k) => k.id)),
    [bekleyenler],
  );

  return (
    <>
      {bekleyenler.length > 0 && (
        <div className="panel">
          <div className="panel__baslik">
            Geri alınabilir değişiklikler
            <span className="rozet vurgu">{bekleyenler.length}</span>
          </div>
          <p className="panel__aciklama">
            Sistemde şu an duran değişiklikler. Defter diske yazılıyor: program
            zorla kapansa bile bunlar bir sonraki açılışta geri alınabilir kalır.
          </p>
          <div className="satir-liste">
            {bekleyenler.map((k) => (
              <div key={k.id} className="satir">
                <div className="satir__govde">
                  <div className="satir__ad">
                    {k.ozet}
                    <span className={`rozet${k.kapsam === 'kalici' ? ' uyari' : ''}`}>
                      {k.kapsam === 'kalici' ? 'kalıcı' : 'oyun oturumu'}
                    </span>
                  </div>
                  <div className="satir__alt">{saat(k.zaman)}</div>
                </div>
                <button
                  className="button small"
                  disabled={mesgul}
                  onClick={() => onGeriAl(k.id)}
                >
                  <IconGeriAl />
                  Geri al
                </button>
              </div>
            ))}
          </div>
        </div>
      )}

      <div className="panel">
        <div className="panel__baslik">
          Aktivite günlüğü
          <div className="panel__eylemler">
            <div className="segment" role="group" aria-label="Günlük süzgeci">
              {(Object.keys(SUZGEC_ETIKETLERI) as Suzgec[]).map((s) => (
                <button
                  key={s}
                  type="button"
                  aria-pressed={suzgec === s}
                  onClick={() => setSuzgec(s)}
                >
                  {SUZGEC_ETIKETLERI[s]}
                </button>
              ))}
            </div>
            <input
              className="text-input"
              type="search"
              value={arama}
              onChange={(e) => setArama(e.target.value)}
              placeholder="Günlükte ara"
              aria-label="Günlükte ara"
            />
            <button
              className="button ghost"
              onClick={onTemizle}
              title="Yalnızca listeyi temizler; sistemde duran değişiklikler kalır"
            >
              <IconCop />
              Temizle
            </button>
          </div>
        </div>

        <p className="panel__aciklama">
          Muifly'ın sistemde yaptığı her şey burada. Geri alınan satırlar
          listeden silinmiyor — ne olduğunu sonradan da görebilmelisin.
          {/* Süzgeç açıkken kaç satırın gizlendiği yazıyor: kullanıcı eksik
              bir listeye baktığını bilmeli (şeffaflık ilkesi). */}
          {suzuluyor && (
            <>
              {' '}
              <strong>
                {gosterilen.length} / {satirlar.length} satır gösteriliyor.
              </strong>
            </>
          )}
        </p>

        {gosterilen.length === 0 ? (
          <div className="bos-durum">
            <IconGunluk />
            <p>
              {satirlar.length === 0
                ? 'Henüz kayıt yok. Muifly sistemde bir şey değiştirdiğinde ilk satır burada belirir.'
                : 'Bu süzgeçte gösterilecek satır yok.'}
            </p>
          </div>
        ) : (
          <div className="log-liste">
            {gosterilen.map((s) => (
              <div key={s.id} className="log" data-duzey={s.duzey}>
                <span className="log__saat">{saat(s.zaman)}</span>
                <span className="log__etiket">{DUZEY_ETIKETLERI[s.duzey]}</span>
                <span className="log__mesaj">{s.mesaj}</span>
                {s.geriAlmaId != null && bekleyenIdler.has(s.geriAlmaId) ? (
                  <button
                    className="button ghost small"
                    disabled={mesgul}
                    onClick={() => onGeriAl(s.geriAlmaId!)}
                  >
                    Geri al
                  </button>
                ) : (
                  <span />
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </>
  );
}
