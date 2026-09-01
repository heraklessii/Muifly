/**
 * Kare ölçümü paneli.
 *
 * Faz 2'nin son parçası. Ölçüm yükseltilmiş yetki istiyor (karar #27) ve bu
 * panelin en önemli işi o yetkiyi **istemeden önce nedenini söylemek** —
 * tasarım ilkesi 5: "UAC sadece gerektiğinde ve neden istendiği söylenerek".
 *
 * Bu yüzden akış iki adımlı: kullanıcı önce açıklamayı ve süreyi görüyor,
 * "Ölç" dediğinde UAC çıkıyor. Panelde hiçbir yerde sayısal vaat yok; ölçüm
 * bittiğinde gösterilen her sayının kullanıcının kendi makinesinde ölçüldüğü
 * açıkça yazıyor (tasarım ilkesi 4, karar #15).
 */

import { useEffect, useState } from 'react';

import * as api from '../lib/api';
import { milisaniye, sayi } from '../lib/format';
import type { KareOlcumDurumu, OlcumRaporu } from '../lib/types';
import { IconBilgi, IconKalkan, IconOynat } from './Icons';

interface Props {
  /** Motorun şu an bir oyun algılayıp algılamadığı. */
  oyunVar: boolean;
  mesgul: boolean;
  onIslem: (calis: () => Promise<void>) => void;
}

/** Varsayılan ölçüm süresi. Kısa tutuluyor: yükseltilmiş süreç uzun yaşamasın. */
const VARSAYILAN_SANIYE = 20;

export function KareOlcumu({ oyunVar, mesgul, onIslem }: Props) {
  const [durum, setDurum] = useState<KareOlcumDurumu | null>(null);
  const [saniye, setSaniye] = useState(VARSAYILAN_SANIYE);
  const [rapor, setRapor] = useState<OlcumRaporu | null>(null);

  useEffect(() => {
    api
      .kareOlcumDurumu()
      .then(setDurum)
      .catch(() => setDurum(null));
  }, []);

  // Yardımcı ikili yoksa paneli hiç göstermiyoruz: çalışmayacak bir düğme
  // göstermek, "neden çalışmıyor" sorusunu kullanıcıya bırakmak olurdu.
  if (!durum?.kullanilabilir) return null;

  const ozet = rapor?.sonuc.ozet ?? null;

  return (
    <div className="panel">
      <div className="panel__baslik">
        Kare ölçümü
        <div className="panel__eylemler">
          <label className="olcum-sure">
            <span>Süre</span>
            <input
              type="number"
              className="text-input"
              min={durum.enKisaSaniye}
              max={durum.enUzunSaniye}
              value={saniye}
              disabled={mesgul}
              onChange={(e) => setSaniye(Number(e.target.value))}
              aria-label="Ölçüm süresi (saniye)"
            />
            <span>sn</span>
          </label>
          <button
            className="button primary"
            disabled={mesgul || !oyunVar}
            onClick={() =>
              onIslem(async () => {
                const r = await api.oyunuOlc(saniye);
                setRapor(r);
              })
            }
          >
            <IconOynat />
            Ölç
          </button>
        </div>
      </div>

      <p className="panel__aciklama">{durum.aciklama}</p>

      {durum.yetkiGerekiyor && (
        <div className="serit bilgi">
          <IconKalkan />
          <span>
            <strong>Ölç</strong> dediğinde Windows bir yetki penceresi açacak. Yetkiyi
            isteyen Muifly değil, yalnızca ölçümü yapan yardımcı; işi bitince kapanıyor.
            Vazgeçersen hiçbir şey değişmez.
          </span>
        </div>
      )}

      {!oyunVar && (
        <div className="serit bilgi">
          <IconBilgi />
          <span>
            Ölçüm için algılanmış bir oyun yok. Oyunu açıp bir kez öne getir, sonra
            buraya dön — hangi oyunun ölçüleceğine Muifly'ın algıladığı mod karar
            veriyor, bu pencereye bakılmıyor.
          </span>
        </div>
      )}

      {rapor && !ozet && (
        <div className="serit bilgi">
          <IconBilgi />
          <span>
            <strong>{rapor.surec}</strong> için {rapor.sonuc.kareSayisi} sunum toplandı;
            özet çıkarmaya yetmedi. Oyun küçültülmüşse ya da ölçüm çok kısaysa bu olur.
          </span>
        </div>
      )}

      {rapor && ozet && (
        <>
          <p className="panel__aciklama">
            <strong>{rapor.surec}</strong> · {rapor.saniye} saniye ·{' '}
            {ozet.kareSayisi} sunum. Buradaki sayılar senin makinende, bu oturumda
            ölçüldü; bir vaat değil.
          </p>
          <div className="olcum-izgara">
            <Kutu
              etiket="Ortalama"
              deger={sayi(ozet.ortFps, 1)}
              birim="kare/sn"
              not={`${milisaniye(ozet.ortMs, 1)} ms kare süresi`}
            />
            <Kutu
              etiket="En kötü %1"
              deger={sayi(ozet.p1KotuFps, 1)}
              birim="kare/sn"
              not={`${milisaniye(ozet.p1KotuMs, 1)} ms — takılmaların göründüğü yer`}
            />
            <Kutu
              etiket="Kare oynaması"
              deger={milisaniye(ozet.kareJitterMs, 1)}
              birim="ms"
              not="ardışık kareler arasındaki fark"
            />
          </div>
          <div className="serit bilgi">
            <IconBilgi />
            <span>
              Ortalama tek başına yanıltıcı olabilir: arada tek bir uzun kare
              ortalamayı zor değiştirir ama oynarken hissedilir. <strong>En kötü %1</strong>{' '}
              ve <strong>kare oynaması</strong> tam da onu gösteriyor.
            </span>
          </div>
        </>
      )}
    </div>
  );
}

function Kutu({
  etiket,
  deger,
  birim,
  not,
}: {
  etiket: string;
  deger: string;
  birim: string;
  not: string;
}) {
  return (
    <div className="olcum">
      <span className="olcum__etiket">{etiket}</span>
      <span className="olcum__deger">
        {deger}
        <span className="olcum__birim">{birim}</span>
      </span>
      <span className="olcum__not">{not}</span>
    </div>
  );
}
