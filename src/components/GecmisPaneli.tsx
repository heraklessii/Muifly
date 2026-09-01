/**
 * Oturum geçmişi ekranı.
 *
 * Günlük ekranının kalıcı kardeşi: günlük programla birlikte ölüyor, bu
 * liste diskte duruyor (`src-tauri/src/monitor/gecmis.rs`). Sorusu farklı —
 * günlük "az önce ne oldu", geçmiş "dün gece ne olmuştu" diye soruyor.
 *
 * Üç kural:
 *
 * 1. **Kayıt ölçüm taşır, iddia taşımaz.** Öncesi ve sonrası iki ayrı sütun;
 *    tek bir "şu kadar iyileşti" oranı yok (karar #15). Yön işareti bile
 *    burada yok: geçmiş bir oturumun iki penceresi arasında oyunun kendi
 *    yükü de değişmiş olabilir ve o farkı okuyacak bağlam artık ekranda
 *    değil.
 * 2. **Kapalıysa söyleniyor.** Ayar kapalıyken boş bir liste göstermek,
 *    "hiç oynamadın" gibi okunurdu.
 * 3. **Silmek tek adım değil.** Geri alınamayan bir işlem; onay isteniyor.
 */

import { useMemo, useState } from 'react';

import { milisaniye, sayi, sure, tarihSaat, yuzde } from '../lib/format';
import { oturumAdi, oturumSuresiSn } from '../lib/types';
import type { GecmisOzeti, OturumKaydi, Ozet } from '../lib/types';
import { IconCop, IconDisaAktar, IconGecmis, IconYenile } from './Icons';

interface Props {
  kayitlar: OturumKaydi[];
  ozet: GecmisOzeti | null;
  /** Ayardan kapatıldıysa liste boş kalır; sebebi yazılmalı. */
  gecmisTut: boolean;
  mesgul: boolean;
  onDisaAktar: () => void;
  onTemizle: () => void;
  onYenile: () => void;
  /** Ayarlar sekmesine geçiş — kapalı olduğu söylenirken açacak yer de gösterilmeli. */
  onAyarlara: () => void;
}

/** Bir özetin okunur satırları. Ölçüm yoksa tire çıkıyor (`format.ts`). */
function OzetSatirlari({ ozet }: { ozet: Ozet }) {
  const satirlar: [string, string][] = [
    ['CPU', yuzde(ozet.cpuOrt, 0)],
    ['Bellek', yuzde(ozet.bellekOrt, 0)],
    ['Gecikme', milisaniye(ozet.gecikmeOrtMs, 0)],
    ['Jitter', milisaniye(ozet.jitterMs, 1)],
  ];
  return (
    <>
      {satirlar.map(([ad, deger]) => (
        <div key={ad} className="kv">
          <span className="kv__ad">{ad}</span>
          <span className="kv__deger">{deger}</span>
        </div>
      ))}
    </>
  );
}

function Kart({ kayit }: { kayit: OturumKaydi }) {
  const sn = oturumSuresiSn(kayit);
  return (
    <details className="gecmis-kayit">
      <summary>
        <span className="gecmis-kayit__ad">
          {oturumAdi(kayit)}
          {kayit.profilAdi && <span className="rozet">profil</span>}
        </span>
        <span className="gecmis-kayit__alt">
          {tarihSaat(kayit.baslangic)} · {sure(sn)} · {kayit.modAdi}
        </span>
        <span className="gecmis-kayit__sayi">
          {kayit.uygulanan.length > 0
            ? `${kayit.uygulanan.length} değişiklik`
            : 'değişiklik yok'}
        </span>
      </summary>

      <div className="gecmis-kayit__govde">
        <div>
          <div className="gecmis-kayit__baslik">Uygulananlar</div>
          {kayit.uygulanan.length === 0 ? (
            <p className="field-hint">
              Bu oturumda sistemde hiçbir şey değişmedi.
            </p>
          ) : (
            <ul className="madde-liste">
              {kayit.uygulanan.map((u, i) => (
                <li key={`${u}-${i}`}>{u}</li>
              ))}
            </ul>
          )}
          <div className="kv">
            <span className="kv__ad">Geri alınan</span>
            <span className="kv__deger">{kayit.geriAlinan}</span>
          </div>
          <div className="kv">
            <span className="kv__ad">Süreç</span>
            <span className="kv__deger">{kayit.surec}</span>
          </div>
        </div>

        <div>
          <div className="gecmis-kayit__baslik">Ölçüm</div>
          {kayit.onceki && kayit.sonraki ? (
            <div className="karsilastirma">
              <div className="karsilastirma__sutun">
                <div className="karsilastirma__baslik">Uygulamadan önce</div>
                <OzetSatirlari ozet={kayit.onceki} />
              </div>
              <div className="karsilastirma__sutun sonra">
                <div className="karsilastirma__baslik">Sonra</div>
                <OzetSatirlari ozet={kayit.sonraki} />
              </div>
            </div>
          ) : (
            <p className="field-hint">
              Bu oturumda karşılaştırmaya yetecek ölçüm penceresi oluşmadı.
            </p>
          )}

          {kayit.kare && (
            <>
              <div className="gecmis-kayit__baslik">Kare ölçümü</div>
              <div className="kv">
                <span className="kv__ad">Ortalama</span>
                <span className="kv__deger">{sayi(kayit.kare.ortFps, 1)} kare/sn</span>
              </div>
              <div className="kv">
                <span className="kv__ad">En kötü %1</span>
                <span className="kv__deger">{milisaniye(kayit.kare.p1KotuMs, 1)}</span>
              </div>
              <div className="kv">
                <span className="kv__ad">Oynama</span>
                <span className="kv__deger">{milisaniye(kayit.kare.kareJitterMs, 1)}</span>
              </div>
            </>
          )}
        </div>
      </div>
    </details>
  );
}

export function GecmisPaneli({
  kayitlar,
  ozet,
  gecmisTut,
  mesgul,
  onDisaAktar,
  onTemizle,
  onYenile,
  onAyarlara,
}: Props) {
  const [arama, setArama] = useState('');
  const [silmeOnayi, setSilmeOnayi] = useState(false);

  const gosterilen = useMemo(() => {
    // Türkçe'ye duyarlı küçültme — `GunlukPaneli` ile aynı gerekçe.
    const q = arama.trim().toLocaleLowerCase('tr');
    if (q === '') return kayitlar;
    return kayitlar.filter((k) =>
      `${oturumAdi(k)} ${k.surec} ${k.profilAdi ?? ''}`
        .toLocaleLowerCase('tr')
        .includes(q),
    );
  }, [kayitlar, arama]);

  return (
    <>
      {!gecmisTut && (
        <div className="serit bilgi">
          <IconGecmis />
          <span>
            Oturum geçmişi <strong>kapalı</strong>. Yeni oturumlar
            kaydedilmiyor; aşağıdaki liste daha önce kaydedilenleri
            gösteriyor.
          </span>
          <button className="button ghost small" onClick={onAyarlara}>
            Ayarlara git
          </button>
        </div>
      )}

      {ozet && ozet.oturumSayisi > 0 && (
        <div className="olcum-izgara">
          <div className="olcum">
            <span className="olcum__etiket">Oturum</span>
            <span className="olcum__deger">{ozet.oturumSayisi}</span>
            <span className="olcum__not">kayıtlı</span>
          </div>
          <div className="olcum">
            <span className="olcum__etiket">Toplam süre</span>
            <span className="olcum__deger">{sure(ozet.toplamSureSn)}</span>
            <span className="olcum__not">Muifly bir şey tuttuğu sürece</span>
          </div>
          <div className="olcum">
            <span className="olcum__etiket">Değişiklik</span>
            <span className="olcum__deger">{ozet.toplamDegisiklik}</span>
            <span className="olcum__not">
              {ozet.olculenOturum > 0
                ? `${ozet.olculenOturum} oturumda kare ölçümü var`
                : 'kare ölçümü yapılmadı'}
            </span>
          </div>
          <div className="olcum">
            <span className="olcum__etiket">En çok</span>
            <span className={`olcum__deger${ozet.enCok ? '' : ' bos'}`}>
              {ozet.enCok ? ozet.enCok.ad : 'ölçüm yok'}
            </span>
            {ozet.enCok && (
              <span className="olcum__not">
                {sure(ozet.enCok.sureSn)} · {ozet.enCok.oturum} oturum
              </span>
            )}
          </div>
        </div>
      )}

      <div className="panel">
        <div className="panel__baslik">
          Oturum geçmişi
          <div className="panel__eylemler">
            <input
              className="text-input"
              type="search"
              value={arama}
              onChange={(e) => setArama(e.target.value)}
              placeholder="Oyun ara"
              aria-label="Geçmişte ara"
            />
            <button className="button ghost" onClick={onYenile} disabled={mesgul}>
              <IconYenile />
              Yenile
            </button>
            <button
              className="button ghost"
              onClick={onDisaAktar}
              disabled={mesgul || kayitlar.length === 0}
              title="Geçmişi düz metin dosyası olarak kaydeder"
            >
              <IconDisaAktar />
              Dışa aktar
            </button>
            <button
              className="button ghost"
              onClick={() => setSilmeOnayi(true)}
              disabled={mesgul || kayitlar.length === 0}
            >
              <IconCop />
              Temizle
            </button>
          </div>
        </div>

        <p className="panel__aciklama">
          Muifly'ın bir oyun için sistemde bir şey tuttuğu her aralık burada.
          Kayıtlar bu bilgisayarda duruyor, hiçbir yere gönderilmiyor.
          Sayılar bu makinede o oturumda ölçülen değerlerdir.
          {arama.trim() !== '' && (
            <>
              {' '}
              <strong>
                {gosterilen.length} / {kayitlar.length} oturum gösteriliyor.
              </strong>
            </>
          )}
        </p>

        {silmeOnayi && (
          <div className="serit uyari">
            <IconCop />
            <span>
              <strong>{kayitlar.length} oturum kaydı silinecek.</strong> Bu
              geri alınamıyor — geçmiş dosyası diskten de kaldırılıyor.
              Sistemde duran değişiklikler etkilenmiyor.
            </span>
            <button
              className="button danger small"
              disabled={mesgul}
              onClick={() => {
                setSilmeOnayi(false);
                onTemizle();
              }}
            >
              Sil
            </button>
            <button className="button ghost small" onClick={() => setSilmeOnayi(false)}>
              Vazgeç
            </button>
          </div>
        )}

        {gosterilen.length === 0 ? (
          <div className="bos-durum">
            <IconGecmis />
            <p>
              {kayitlar.length === 0
                ? 'Henüz kayıtlı oturum yok. Bir oyuna profil uygulandığında ve o oturum kapandığında ilk kayıt burada belirir.'
                : 'Bu aramaya uyan oturum yok.'}
            </p>
          </div>
        ) : (
          <div className="gecmis-liste">
            {gosterilen.map((k) => (
              <Kart key={k.id} kayit={k} />
            ))}
          </div>
        )}
      </div>
    </>
  );
}
