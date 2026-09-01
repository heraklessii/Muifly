/**
 * Oyun kütüphanesi: profil açarken exe adını elle yazma zorunluluğunu
 * kaldıran ekran.
 *
 * İki adım, bilinçli olarak:
 *
 * 1. **Izgara** — kurulu oyunlar, kapak görselleriyle. Kullanıcı oyunu
 *    tanıdığı görselden seçiyor, `r5apex.exe` diye bir şey bilmesi
 *    gerekmiyor.
 * 2. **Onay** — taslağın ne içerdiği ve NEDEN öyle olduğu yazıyor; hangi
 *    exe'lerin profile gireceği kullanıcının seçimi.
 *
 * İkinci adım profil içe aktarmadaki iki adımlı önizlemenin (karar #23)
 * aynı gerekçesiyle var: hazır gelen bir profil, kullanıcı ne olduğunu
 * görmeden kaydedilmemeli.
 *
 * Görseller ve adlar **yerelden** geliyor — Steam'in disk önbelleği, Epic'in
 * manifestleri, exe ikonları. Bu ekran hiçbir zaman ağa çıkmıyor (karar #25).
 */

import { useEffect, useMemo, useRef, useState } from 'react';

import * as api from '../lib/api';
import { KAYNAK_ETIKETLERI, type Oyun, type Taslak } from '../lib/types';
import { IconArti, IconBilgi, IconKapat, IconKutuphane, IconYenile } from './Icons';

interface Props {
  onKapat: () => void;
  /** Kullanıcı taslağı onayladı: profil penceresi bununla açılacak. */
  onTaslak: (t: Taslak) => void;
}

/** Arama kutusunun görünmeye başladığı oyun sayısı. */
const ARAMA_ESIGI = 6;

/**
 * Kapak görseli.
 *
 * Görseller taramada değil, kart ekranda görününce isteniyor: yüz oyunluk
 * bir kütüphanede hepsini birden almak megabaytlarca veriyi boşuna taşırdı.
 * `IntersectionObserver` yoksa (test ortamı) doğrudan yükleniyor — kaybolan
 * tek şey tembellik oluyor, görsel yine geliyor.
 */
function Kapak({ oyun }: { oyun: Oyun }) {
  const [adres, setAdres] = useState<string | null>(null);
  const [istendi, setIstendi] = useState(false);
  const kutu = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!oyun.gorselVar || istendi) return;
    const el = kutu.current;
    if (!el || typeof IntersectionObserver === 'undefined') {
      setIstendi(true);
      return;
    }
    const gozlemci = new IntersectionObserver((girdiler) => {
      if (girdiler.some((g) => g.isIntersecting)) {
        setIstendi(true);
        gozlemci.disconnect();
      }
    });
    gozlemci.observe(el);
    return () => gozlemci.disconnect();
  }, [oyun.gorselVar, istendi]);

  useEffect(() => {
    if (!istendi) return;
    let gecerli = true;
    api
      .oyunGorseli(oyun.kimlik)
      .then((d) => gecerli && setAdres(d))
      .catch(() => gecerli && setAdres(null));
    return () => {
      gecerli = false;
    };
  }, [istendi, oyun.kimlik]);

  return (
    <div className="oyun-kart__kapak" ref={kutu}>
      {adres ? (
        <img src={adres} alt="" loading="lazy" />
      ) : (
        // Yer tutucu: oyunun baş harfleri. Boş bir dikdörtgen, kartın
        // yüklenmediği izlenimi verirdi.
        <span className="oyun-kart__harf">{basHarfler(oyun.ad)}</span>
      )}
    </div>
  );
}

/** Görselsiz kartlar için en fazla iki harf. */
export function basHarfler(ad: string): string {
  const parcalar = ad
    .split(/[\s:_-]+/)
    .map((p) => p.trim())
    .filter(Boolean);
  if (parcalar.length === 0) return '?';
  if (parcalar.length === 1) return parcalar[0].slice(0, 2).toLocaleUpperCase('tr');
  return (parcalar[0][0] + parcalar[1][0]).toLocaleUpperCase('tr');
}

export function KutuphaneDiyalogu({ onKapat, onTaslak }: Props) {
  const [oyunlar, setOyunlar] = useState<Oyun[] | null>(null);
  const [arama, setArama] = useState('');
  const [secilen, setSecilen] = useState<Oyun | null>(null);
  const [taslak, setTaslak] = useState<Taslak | null>(null);
  const [secilenExeler, setSecilenExeler] = useState<string[]>([]);
  const [hata, setHata] = useState<string | null>(null);

  function tara() {
    setOyunlar(null);
    setHata(null);
    api
      .oyunlariTara()
      .then(setOyunlar)
      .catch(() => {
        setOyunlar([]);
        setHata('Kütüphane taranamadı.');
      });
  }

  useEffect(tara, []);

  useEffect(() => {
    const f = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return;
      // Escape önce onay adımından ızgaraya dönüyor: kullanıcı yanlış oyunu
      // seçtiyse en yakın geri adım bu.
      if (secilen) setSecilen(null);
      else onKapat();
    };
    window.addEventListener('keydown', f);
    return () => window.removeEventListener('keydown', f);
  }, [onKapat, secilen]);

  const gosterilen = useMemo(() => {
    if (!oyunlar) return [];
    const q = arama.trim().toLocaleLowerCase('tr');
    if (!q) return oyunlar;
    return oyunlar.filter(
      (o) =>
        o.ad.toLocaleLowerCase('tr').includes(q) ||
        o.exeler.some((e) => e.includes(q)),
    );
  }, [oyunlar, arama]);

  function sec(oyun: Oyun) {
    setSecilen(oyun);
    setTaslak(null);
    api
      .profilTaslagi(oyun.ad, oyun.exeler)
      .then((t) => {
        setTaslak(t);
        setSecilenExeler(t.profil.executable_names);
      })
      .catch(() => setHata('Taslak üretilemedi.'));
  }

  async function elleSec() {
    const yol = await api.exeDosyasiSec();
    if (!yol) return;
    try {
      const oyun = await api.oyunElleEkle(yol);
      setOyunlar((o) => [...(o ?? []).filter((x) => x.kimlik !== oyun.kimlik), oyun]);
      sec(oyun);
    } catch {
      setHata('Seçilen dosya okunamadı.');
    }
  }

  function exeyiDegistir(ad: string, acik: boolean) {
    setSecilenExeler((s) => (acik ? [...s, ad] : s.filter((x) => x !== ad)));
  }

  function onayla() {
    if (!taslak) return;
    onTaslak({
      ...taslak,
      profil: { ...taslak.profil, executable_names: secilenExeler },
    });
  }

  return (
    <div className="perde" onMouseDown={(e) => e.target === e.currentTarget && onKapat()}>
      <div className="diyalog genis" role="dialog" aria-modal="true">
        <div className="diyalog__baslik">
          {secilen ? secilen.ad : 'Kütüphaneden oyun seç'}
          <button className="button ghost icon" onClick={onKapat} aria-label="Kapat">
            <IconKapat />
          </button>
        </div>

        <div className="diyalog__govde">
          {hata && (
            <div className="serit uyari">
              <IconBilgi />
              <span>{hata}</span>
            </div>
          )}

          {secilen ? (
            <>
              <p className="panel__aciklama">
                Aşağıdaki profil <strong>henüz kaydedilmedi</strong>. Onaylayınca
                profil penceresi açılır; oradan da değiştirebilirsin.
              </p>

              {taslak == null ? (
                <p className="field-hint">Taslak hazırlanıyor…</p>
              ) : (
                <>
                  <ul className="madde-liste">
                    {taslak.aciklamalar.map((a) => (
                      <li key={a}>{a}</li>
                    ))}
                  </ul>

                  <div className="field">
                    <span>Profile girecek dosyalar</span>
                    <span className="field-hint">
                      Oyunun penceresini açan dosyayı seç. Başlatıcılar ve
                      yardımcı süreçler genelde işaretlenmemeli.
                    </span>
                    <div className="secim-liste">
                      {secilen.exeler.map((ad) => (
                        <label key={ad} className="secim">
                          <input
                            type="checkbox"
                            checked={secilenExeler.includes(ad)}
                            onChange={(e) => exeyiDegistir(ad, e.target.checked)}
                          />
                          {ad}
                        </label>
                      ))}
                    </div>
                  </div>

                  {secilen.kurulum && (
                    <p className="field-hint">
                      Kurulum klasörü: <code>{secilen.kurulum}</code>
                    </p>
                  )}
                </>
              )}
            </>
          ) : oyunlar == null ? (
            <div className="bos-durum">
              <IconKutuphane />
              <p>Kütüphane taranıyor…</p>
            </div>
          ) : (
            <>
              <p className="panel__aciklama">
                Steam ve Epic kütüphanen <strong>diskten</strong> okundu. Oyun
                adları, kapak görselleri ve dosya adları senin makinendeki
                kurulumdan geliyor — hiçbir sunucuya sorulmadı.
              </p>

              {oyunlar.length >= ARAMA_ESIGI && (
                <input
                  className="text-input"
                  type="search"
                  value={arama}
                  onChange={(e) => setArama(e.target.value)}
                  placeholder="Oyun ara"
                  aria-label="Oyun ara"
                />
              )}

              {oyunlar.length === 0 ? (
                <div className="bos-durum">
                  <IconKutuphane />
                  <strong>Kurulu oyun bulunamadı</strong>
                  <p>
                    Steam ve Epic kurulu değilse ya da oyunların başka bir
                    başlatıcıdaysa, oyunun <code>.exe</code> dosyasını doğrudan
                    seçebilirsin.
                  </p>
                </div>
              ) : gosterilen.length === 0 ? (
                <div className="bos-durum">
                  <IconKutuphane />
                  <p>Bu aramaya uyan oyun yok.</p>
                </div>
              ) : (
                <div className="oyun-izgara">
                  {gosterilen.map((o) => (
                    <button key={o.kimlik} className="oyun-kart" onClick={() => sec(o)}>
                      <Kapak oyun={o} />
                      <div className="oyun-kart__alt">
                        <span className="oyun-kart__ad" title={o.ad}>
                          {o.ad}
                        </span>
                        <span className="rozet">{KAYNAK_ETIKETLERI[o.kaynak]}</span>
                      </div>
                    </button>
                  ))}
                </div>
              )}
            </>
          )}
        </div>

        <div className="diyalog__alt">
          {secilen ? (
            <>
              <button className="button ghost" onClick={() => setSecilen(null)}>
                Geri
              </button>
              <span className="bosluk" />
              <button
                className="button primary"
                disabled={taslak == null || secilenExeler.length === 0}
                onClick={onayla}
              >
                <IconArti />
                Profil penceresini aç
              </button>
            </>
          ) : (
            <>
              <button className="button ghost" onClick={tara} disabled={oyunlar == null}>
                <IconYenile />
                Yeniden tara
              </button>
              <button className="button" onClick={elleSec}>
                .exe dosyası seç
              </button>
              <span className="bosluk" />
              <button className="button ghost" onClick={onKapat}>
                Vazgeç
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
