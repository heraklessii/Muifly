/**
 * Profil listesi.
 *
 * Profil dosyalarının nerede durduğu kullanıcıya söyleniyor: dosyalar
 * okunabilir JSON ve paylaşılabilir (`docs/DISTRIBUTION.md` — kaynak kapalı,
 * profil formatı açık). İçe/dışa aktarma da bu yüzden var: paylaşım dosya
 * alışverişi olarak kalıyor, merkezi bir sunucu ya da hesap sistemi yok
 * (`docs/PROFILES.md`).
 *
 * Yerleşim satır değil kart: bir profilde okunacak dört ayrı şey var (ad,
 * eşleşen dosyalar, ne yapacağı, eylemler). Tek satıra sıkıştırıldığında
 * bunlardan üçü kısaltılıyor ve "ne yapacak" hep kapalı kalıyordu.
 */

import { useMemo, useState } from 'react';

import * as api from '../lib/api';
import type { Durum, Kisitlar, Profil } from '../lib/types';
import { modSureci } from '../lib/types';
import {
  IconArti,
  IconBilgi,
  IconCop,
  IconDisaAktar,
  IconDuzenle,
  IconIceAktar,
  IconKutuphane,
  IconOynat,
  IconProfil,
} from './Icons';

interface Props {
  profiller: Profil[];
  durum: Durum;
  kisitlar: Kisitlar;
  mesgul: boolean;
  onYeni: () => void;
  onKutuphane: () => void;
  onDuzenle: (p: Profil) => void;
  onSil: (p: Profil) => void;
  onUygula: (p: Profil) => void;
  onIceAktar: () => void;
  onDisaAktar: (p: Profil) => void;
}

/** Arama kutusunun görünmeye başladığı profil sayısı. */
const ARAMA_ESIGI = 5;

/**
 * "Bu profil ne yapacak" listesi.
 *
 * Metinler backend'den geliyor (`profile_engine::aktarim::etkiler`) — içe
 * aktarma önizlemesiyle aynı cümleler. Kullanıcının kendi profili için de
 * aynı soruya aynı cevabın verilmesi gerekiyor; ayrıca sayısal vaat yasağı
 * tek yerde test ediliyor (karar #17).
 *
 * Açılınca yükleniyor: liste uzunsa her satır için komut çağırmanın anlamı
 * yok.
 */
function EtkiOzeti({ kimlik }: { kimlik: string }) {
  const [etkiler, setEtkiler] = useState<string[] | null>(null);
  const [hata, setHata] = useState(false);

  return (
    <details
      className="etki"
      onToggle={(e) => {
        if (!(e.currentTarget as HTMLDetailsElement).open || etkiler || hata) return;
        api
          .profilEtkileri(kimlik)
          .then(setEtkiler)
          .catch(() => setHata(true));
      }}
    >
      <summary>Ne yapacak?</summary>
      {hata ? (
        <p className="field-hint">Profil okunamadı.</p>
      ) : etkiler == null ? (
        <p className="field-hint">Okunuyor…</p>
      ) : (
        <ul className="madde-liste">
          {etkiler.map((e) => (
            <li key={e}>{e}</li>
          ))}
        </ul>
      )}
    </details>
  );
}

/** Profilin ne içerdiğini tek bakışta anlatan rozetler. */
function Rozetler({ p }: { p: Profil }) {
  const rozetler: string[] = [];
  if (p.system.suspend_process_list.length > 0) {
    rozetler.push(`${p.system.suspend_process_list.length} uygulama dondurulacak`);
  }
  if (p.system.power_plan) rozetler.push('güç planı');
  if (p.system.cpu_affinity === 'sadece_p_core') rozetler.push('P-core');
  if (p.network.qos_priority) rozetler.push('QoS');
  if (p.network.tcp_nodelay) rozetler.push('Nagle kapalı');

  if (rozetler.length === 0) {
    return <span className="field-hint">Yalnızca süreç önceliği değişecek.</span>;
  }

  return (
    <div className="profil-kart__rozetler">
      {rozetler.map((r) => (
        <span key={r} className="rozet">
          {r}
        </span>
      ))}
    </div>
  );
}

export function ProfilPaneli({
  profiller,
  durum,
  kisitlar,
  mesgul,
  onYeni,
  onKutuphane,
  onDuzenle,
  onSil,
  onUygula,
  onIceAktar,
  onDisaAktar,
}: Props) {
  const [arama, setArama] = useState('');

  const ondeki = modSureci(durum.mod);
  // Sınır profil sayısında: var olan profil düzenlenebilir, yenisi eklenemez.
  const sinirDoldu =
    kisitlar.profilSiniri !== null && profiller.length >= kisitlar.profilSiniri;

  const gosterilen = useMemo(() => {
    const q = arama.trim().toLocaleLowerCase('tr');
    if (!q) return profiller;
    return profiller.filter(
      (p) =>
        p.display_name.toLocaleLowerCase('tr').includes(q) ||
        p.executable_names.some((e) => e.includes(q)),
    );
  }, [profiller, arama]);

  return (
    <div className="panel">
      <div className="panel__baslik">
        Oyun profilleri
        {profiller.length > 0 && <span className="rozet">{profiller.length}</span>}
        <div className="panel__eylemler">
          {profiller.length >= ARAMA_ESIGI && (
            <input
              className="text-input"
              type="search"
              value={arama}
              onChange={(e) => setArama(e.target.value)}
              placeholder="Profil ara"
              aria-label="Profil ara"
            />
          )}
          {/* Demoda aktarım kapalı: düğme gizlenmiyor, gerekçesi yazılıyor —
              kullanıcı özelliğin var olduğunu bilmeli. */}
          <button
            className="button"
            disabled={mesgul || !kisitlar.profilAktarimi || sinirDoldu}
            onClick={onIceAktar}
            title={
              kisitlar.profilAktarimi
                ? 'Bir profil dosyası seç; ne yapacağı gösterilir'
                : 'Profil aktarımı demo sürümde kapalı'
            }
          >
            <IconIceAktar />
            İçe aktar
          </button>
          {/* Kütüphane, "yeni profil"in kolay yolu: exe adını elle yazmak
              yerine kurulu oyunlardan seçtiriyor. Elle yazma yolu duruyor —
              kütüphanenin bulamadığı oyunlar için tek çıkış o. */}
          <button
            className="button"
            disabled={sinirDoldu}
            onClick={onKutuphane}
            title="Steam ve Epic kütüphaneni diskten okur; hiçbir sunucuya sorulmaz"
          >
            <IconKutuphane />
            Kütüphaneden ekle
          </button>
          <button className="button primary" disabled={sinirDoldu} onClick={onYeni}>
            <IconArti />
            Yeni profil
          </button>
        </div>
      </div>

      {sinirDoldu && (
        <div className="serit bilgi">
          <IconBilgi />
          <span>
            Demo sürümde tek profil oluşturulabiliyor. Var olan profili
            düzenlemek, uygulamak ve geri almak sınırsız.
          </span>
        </div>
      )}

      <p className="panel__aciklama">
        Her profil bir JSON dosyası olarak <code>%APPDATA%\Muifly\profiller</code>{' '}
        altında duruyor. Metin editöründe açıp okuyabilir, bir arkadaşına
        gönderebilirsin.
      </p>

      {profiller.length === 0 ? (
        <div className="bos-durum">
          <IconProfil />
          <strong>Henüz profil yok</strong>
          <p>
            Bir oyun için profil oluştur: hangi uygulamaların dondurulacağını,
            hangi güç planının kullanılacağını sen belirle.
          </p>
          <button className="button primary" onClick={onKutuphane} disabled={sinirDoldu}>
            <IconKutuphane />
            Kütüphaneden seç
          </button>
          <button className="button ghost" onClick={onYeni} disabled={sinirDoldu}>
            <IconArti />
            Elle oluştur
          </button>
        </div>
      ) : gosterilen.length === 0 ? (
        <div className="bos-durum">
          <IconProfil />
          <p>Bu aramaya uyan profil yok.</p>
        </div>
      ) : (
        <div className="profil-izgara">
          {gosterilen.map((p) => {
            // Öndeki uygulama bu profile aitse kart vurgulanıyor: kullanıcı
            // hangi profilin şu an geçerli olduğunu aramamalı.
            const aktif =
              ondeki != null && p.executable_names.includes(ondeki.toLowerCase());
            return (
              <article key={p.profile_id} className={`profil-kart${aktif ? ' vurgulu' : ''}`}>
                <div className="profil-kart__ust">
                  <h3 className="profil-kart__ad">{p.display_name}</h3>
                  {p.competitive && <span className="rozet uyari">rekabetçi</span>}
                  {aktif && <span className="rozet vurgu">önde</span>}
                </div>

                <div className="profil-kart__exe">{p.executable_names.join(' · ')}</div>

                <Rozetler p={p} />

                <EtkiOzeti kimlik={p.profile_id} />

                <div className="profil-kart__eylemler">
                  <button
                    className="button small"
                    onClick={() => onUygula(p)}
                    disabled={mesgul || !aktif}
                    title={
                      aktif
                        ? 'Bu profili şimdi uygula'
                        : 'Uygulamak için oyunun önde olması gerekiyor'
                    }
                  >
                    <IconOynat />
                    Uygula
                  </button>
                  <button className="button small" onClick={() => onDuzenle(p)}>
                    <IconDuzenle />
                    Düzenle
                  </button>
                  <span className="bosluk" />
                  <button
                    className="button small ghost icon"
                    onClick={() => onDisaAktar(p)}
                    disabled={mesgul || !kisitlar.profilAktarimi}
                    aria-label={`${p.display_name} profilini dışa aktar`}
                    title={
                      kisitlar.profilAktarimi
                        ? 'Profili bir dosyaya kaydet'
                        : 'Profil aktarımı demo sürümde kapalı'
                    }
                  >
                    <IconDisaAktar />
                  </button>
                  <button
                    className="button small ghost danger icon"
                    onClick={() => onSil(p)}
                    aria-label={`${p.display_name} profilini sil`}
                  >
                    <IconCop />
                  </button>
                </div>
              </article>
            );
          })}
        </div>
      )}
    </div>
  );
}
