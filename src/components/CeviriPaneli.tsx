/**
 * Ekran çevirisi ekranı (Faz 5).
 *
 * Dört kural, dördü de ölçülmüş bir şeyden çıkıyor:
 *
 * 1. **Kaynak metin her zaman çevirinin yanında.** Karar #29 zaaf 3, modelin
 *    bir cümleyi hata vermeden düşürebildiğini ve çıktının akıcı göründüğü
 *    için fark edilmediğini ölçtü. Kaynağı gizleyen bir tasarım o ölçümden
 *    sonra tercih edilemez.
 * 2. **Kusurlar rozetle görünüyor.** Boş çeviri, kaybolan terim ve OCR
 *    şüphesi ayrı ayrı işaretleniyor. "Muhtemelen doğrudur" demek, kara
 *    kutunun ta kendisi olurdu (tasarım ilkesi 2).
 * 3. **Bedel önce yazılıyor.** Model yarım gigabayt iniyor ve çalışırken
 *    CPU kullanıyor; ikisi de indirme düğmesinden önce söyleniyor.
 * 4. **Sayısal vaat yok.** Ölçülen süreler gösteriliyor, "şu kadar hızlı"
 *    denmiyor (tasarım ilkesi 4).
 */

import { useCallback, useEffect, useState } from 'react';

import * as api from '../lib/api';
import type {
  CeviriBellegi,
  CeviriDurumu,
  CeviriSonucu,
  Ekran,
  IndirmeIlerlemesi,
  ModelDurumu,
  OcrDilDurumu,
  Profil,
} from '../lib/types';
import { ASAMA_ETIKETLERI } from '../lib/types';
import { IconArti, IconCeviri, IconCop, IconOynat, IconTik, IconUyari, IconYenile } from './Icons';

interface Props {
  ceviriEkrani: number;
  overlayAcik: boolean;
  profiller: Profil[];
  mesgul: boolean;
  onIslem: (calis: () => Promise<void>) => Promise<void>;
  onBildir: (tur: 'basari' | 'bilgi' | 'hata', mesaj: string) => void;
  onEkranDegistir: (indeks: number) => void;
  onOverlayDegistir: (acik: boolean) => void;
  onAyarlara: () => void;
}

/** Baytı okunur hale getirir. */
function bayt(deger: number): string {
  const mb = deger / (1024 * 1024);
  if (mb >= 1024) return `${(mb / 1024).toFixed(2)} GB`;
  return `${Math.round(mb)} MB`;
}

/** Bir alanın kullanıcıya gösterilen özeti. */
function alanOzeti(p: Profil): string {
  const b = p.ceviri?.region;
  if (!b) return 'Alan seçilmemiş — ekranın tamamı okunur';
  const y = (x: number) => Math.round(x * 100);
  return `Seçili bölge: sol %${y(b.left)}, üst %${y(b.top)} — %${y(b.width)} × %${y(b.height)}`;
}

export function CeviriPaneli({
  ceviriEkrani,
  overlayAcik,
  profiller,
  mesgul,
  onIslem,
  onBildir,
  onEkranDegistir,
  onOverlayDegistir,
  onAyarlara,
}: Props) {
  const [durum, setDurum] = useState<CeviriDurumu | null>(null);
  const [sonuc, setSonuc] = useState<CeviriSonucu | null>(null);
  const [model, setModel] = useState<ModelDurumu | null>(null);
  const [dil, setDil] = useState<OcrDilDurumu | null>(null);
  const [bellek, setBellek] = useState<CeviriBellegi | null>(null);
  const [ekranlar, setEkranlar] = useState<Ekran[]>([]);
  const [indirme, setIndirme] = useState<IndirmeIlerlemesi | null>(null);
  const [duzeltilen, setDuzeltilen] = useState<{ metin: string; taslak: string } | null>(null);
  const [terim, setTerim] = useState('');
  const [karsilik, setKarsilik] = useState('');

  const tazele = useCallback(() => {
    api.ceviriDurumu().then(setDurum).catch(() => {});
    api.ceviriSonucu().then(setSonuc).catch(() => {});
    api.ceviriModelDurumu().then(setModel).catch(() => {});
    api.ceviriBellegi().then(setBellek).catch(() => setBellek(null));
  }, []);

  useEffect(() => {
    tazele();
    api.olceklemeEkranlari().then(setEkranlar).catch(() => setEkranlar([]));
    // Kurulu dil paketleri oturum sırasında değişmiyor: bir kez okunuyor.
    api.ceviriDilDurumu().then(setDil).catch(() => setDil(null));
  }, [tazele]);

  // Sonuç kısayoldan geliyor; kullanıcı bu ekranda değilken de üretiliyor.
  useEffect(() => {
    const abone = api.dinle<[CeviriDurumu, CeviriSonucu | null]>(api.OLAY_CEVIRI, ([d, s]) => {
      setDurum(d);
      if (s) setSonuc(s);
      api.ceviriBellegi().then(setBellek).catch(() => {});
    });
    return () => {
      abone.then((f) => f()).catch(() => {});
    };
  }, []);

  useEffect(() => {
    const abone = api.dinle<IndirmeIlerlemesi>(api.OLAY_CEVIRI_INDIRME, (i) => {
      setIndirme(i.bitti ? null : i);
      if (i.bitti) {
        api.ceviriModelDurumu().then(setModel).catch(() => {});
        if (i.hata) onBildir('hata', i.hata);
        else onBildir('basari', 'Çeviri modeli indirildi ve doğrulandı.');
      }
    });
    return () => {
      abone.then((f) => f()).catch(() => {});
    };
  }, [onBildir]);

  // Bir istek sürerken aşama değişiyor; yoklama yalnızca o sırada.
  useEffect(() => {
    if (!durum?.acik || durum.asama === 'bosta') return;
    const t = window.setInterval(() => {
      api.ceviriDurumu().then(setDurum).catch(() => {});
    }, 400);
    return () => window.clearInterval(t);
  }, [durum?.acik, durum?.asama]);

  const acik = durum?.acik ?? false;
  const kurulu = model?.kurulu ?? false;

  return (
    <>
      {/* --- Ne yaptığı, kısayolu, durumu -------------------------------- */}
      <div className="panel">
        <div className="panel__baslik">
          Ekran çevirisi
          <div className="panel__eylemler">
            <button
              className="button"
              disabled={mesgul || !acik}
              onClick={() =>
                onIslem(async () => {
                  await api.ceviriSimdi();
                  onBildir('bilgi', 'Çeviri isteği gönderildi.');
                })
              }
            >
              <IconOynat />
              Şimdi çevir
            </button>
            {acik ? (
              <button
                className="button"
                disabled={mesgul}
                onClick={() =>
                  onIslem(async () => {
                    await api.ceviriKapat();
                    tazele();
                    onBildir('basari', 'Ekran çevirisi kapatıldı, kısayol sisteme bırakıldı.');
                  })
                }
              >
                Kapat
              </button>
            ) : (
              <button
                className="button primary"
                disabled={mesgul}
                onClick={() =>
                  onIslem(async () => {
                    await api.ceviriAc();
                    tazele();
                    onBildir('basari', 'Ekran çevirisi açıldı.');
                  })
                }
              >
                <IconCeviri />
                Aç
              </button>
            )}
          </div>
        </div>

        <p className="panel__aciklama">
          Kısayola bastığında seçili ekran alanı okunur, yazı{' '}
          <strong>Windows'un kendi metin tanımasıyla</strong> çıkarılır ve{' '}
          <strong>bu bilgisayarda çalışan</strong> bir modelle İngilizceden
          Türkçeye çevrilir. Hiçbir metin dışarı gönderilmez.
        </p>
        <p className="panel__aciklama">
          <strong>Sürekli çeviri yok, bilerek.</strong> Hareket eden ekranda
          yazı yarı çizilmiş ya da bulanık yakalanır; okumanın güvenilir olduğu
          tek an duran bir diyalog kutusudur.
        </p>
        <p className="panel__aciklama">
          <strong>Kısayol bir klavye kancası değil.</strong> Tuş basışların
          okunmuyor; sisteme tek bir kombinasyon kaydediliyor ve yalnızca o
          kombinasyon Muifly'a bir mesaj olarak geliyor. Kaydedilemezse özellik
          açılmıyor — çalışmayan bir kısayolu açık göstermek yanlış olurdu.
        </p>

        {acik && (
          <p className="panel__aciklama">
            Kısayol: <kbd className="kbd">{durum?.kisayol ?? '—'}</kbd> · Çeviri
            belleği: <code>{durum?.oyun || '—'}</code> · Alan:{' '}
            <strong>{durum?.tumEkran ? 'tüm ekran' : 'seçili bölge'}</strong> ·
            Model: <strong>{durum?.modelBellekte ? 'bellekte' : 'bellekte değil'}</strong>
          </p>
        )}

        {durum && durum.asama !== 'bosta' && (
          <div className="serit bilgi">
            <IconYenile />
            <div>
              <strong>{ASAMA_ETIKETLERI[durum.asama]}…</strong>
            </div>
          </div>
        )}

        {durum?.sonHata && (
          <div className="serit uyari">
            <IconUyari />
            <div>
              <strong>Son istek tamamlanamadı.</strong>
              <p>{durum.sonHata}</p>
            </div>
          </div>
        )}

        {durum && (durum.yakalamaMs !== null || durum.ocrMs !== null) && (
          <div className="olcum-izgara">
            <div className="olcum">
              <span className="olcum__etiket">Ekranı okuma</span>
              <span className="olcum__deger">
                {durum.yakalamaMs ?? '—'}
                <span className="olcum__birim">ms</span>
              </span>
            </div>
            <div className="olcum">
              <span className="olcum__etiket">Yazıyı çıkarma</span>
              <span className="olcum__deger">
                {durum.ocrMs ?? '—'}
                <span className="olcum__birim">ms</span>
              </span>
            </div>
            <div className="olcum">
              <span className="olcum__etiket">Çeviri</span>
              <span className="olcum__deger">
                {durum.ceviriMs ?? '—'}
                <span className="olcum__birim">ms</span>
              </span>
            </div>
          </div>
        )}

        <div className="field">
          <span>Okunacak ekran</span>
          <select
            className="select"
            value={ceviriEkrani}
            disabled={ekranlar.length === 0}
            onChange={(e) => onEkranDegistir(Number(e.target.value))}
          >
            {ekranlar.length === 0 && <option value={0}>Ekran listesi okunamadı</option>}
            {ekranlar.map((e) => (
              <option key={e.indeks} value={e.indeks}>
                {e.indeks + 1}. {e.ad} ({e.genislik}×{e.yukseklik})
                {e.birincil ? ' — birincil' : ''}
              </option>
            ))}
          </select>
        </div>

        <div className="field row">
          <div>
            <div className="satir__ad">Sonucu oyunun üstünde göster</div>
            <div className="field-hint">
              Kapatırsan sonuç yalnızca bu ekranda durur. Üstte duran pencere
              münhasır tam ekranda görünmez — kenarlıksız pencere modu gerekir.
            </div>
          </div>
          <button
            className="switch"
            role="switch"
            aria-checked={overlayAcik}
            aria-label="Sonucu oyunun üstünde göster"
            onClick={() => onOverlayDegistir(!overlayAcik)}
          />
        </div>
      </div>

      {/* --- Model -------------------------------------------------------- */}
      <div className="panel">
        <div className="panel__baslik">
          Çeviri modeli
          <div className="panel__eylemler">
            {indirme ? (
              <button
                className="button"
                onClick={() =>
                  onIslem(async () => {
                    await api.ceviriModelIndirmeyiDurdur();
                    onBildir('bilgi', 'İndirme durduruluyor.');
                  })
                }
              >
                İndirmeyi durdur
              </button>
            ) : kurulu ? (
              <>
                <button
                  className="button"
                  disabled={mesgul}
                  onClick={() =>
                    onIslem(async () => {
                      await api.ceviriModelDogrula();
                      onBildir('basari', 'Dosyaların tamamı beklenen dosyalar.');
                    })
                  }
                >
                  <IconTik />
                  Doğrula
                </button>
                <button
                  className="button danger"
                  disabled={mesgul}
                  onClick={() =>
                    onIslem(async () => {
                      await api.ceviriModelSil();
                      api.ceviriModelDurumu().then(setModel).catch(() => {});
                      onBildir(
                        'basari',
                        'Model dosyaları silindi. Çeviri belleğine dokunulmadı.',
                      );
                    })
                  }
                >
                  <IconCop />
                  Sil
                </button>
              </>
            ) : (
              <button
                className="button primary"
                disabled={mesgul}
                onClick={() =>
                  onIslem(async () => {
                    await api.ceviriModelIndir();
                  })
                }
              >
                Modeli indir ({bayt(model?.toplamBayt ?? 0)})
              </button>
            )}
          </div>
        </div>

        <p className="panel__aciklama">
          Model <strong>kuruluma dahil değil</strong>, isteğe bağlı iniyor:
          ekran çevirisini hiç kullanmayacak birine yarım gigabayt ödetmek
          doğru olmazdı. İnen her dosya SHA-256 ile doğrulanıyor; tutmayan bir
          dosya diskte bırakılmıyor.
        </p>
        <p className="panel__aciklama">
          <strong>Çalışırken CPU kullanıyor.</strong> Çeviri tuşa basınca ve bir
          kez çalışıyor, sürekli değil; yine de iki çekirdekle sınırlı — oyunun
          çekirdeklerini almaması için. Bir süre kullanılmazsa model bellekten
          düşüyor.
        </p>
        {model && (
          <p className="panel__aciklama">
            Durum: <strong>{kurulu ? 'kurulu' : 'kurulu değil'}</strong> · Kaynak:{' '}
            <code>{model.kaynak}</code>
            <br />
            Klasör: <code>{model.dizin}</code>
          </p>
        )}

        {indirme && (
          <div className="ilerleme-kutu">
            <div className="ilerleme-kutu__ust">
              <span>
                {indirme.ad} ({indirme.sira}/{indirme.adet})
              </span>
              <span>
                {bayt(indirme.inen)} / {bayt(indirme.toplam)}
              </span>
            </div>
            <progress value={indirme.inen} max={indirme.toplam || 1} />
          </div>
        )}

        {dil?.durum === 'yok' && (
          <div className="serit uyari">
            <IconUyari />
            <div>
              <strong>'{dil.etiket}' için metin tanıma paketi kurulu değil.</strong>
              <p>{dil.nasilKurulur}</p>
              <p>
                Şu an tanınan diller:{' '}
                {dil.mevcut.length > 0
                  ? dil.mevcut.map((d) => d.etiket).join(', ')
                  : 'hiçbiri'}
                .
              </p>
            </div>
          </div>
        )}
      </div>

      {/* --- Alan --------------------------------------------------------- */}
      <div className="panel">
        <div className="panel__baslik">Çevrilecek alan</div>
        <p className="panel__aciklama">
          Alan <strong>oyunun profiline</strong> yazılıyor, çünkü diyalog
          kutusunun yeri oyuna göre değişiyor. Oran olarak saklandığı için aynı
          profil başka bir çözünürlükte de doğru yere düşüyor. Seçmezsen ekranın
          tamamı okunur; bu da çalışır, sadece etraftaki yazılar da girer.
        </p>

        {profiller.length === 0 ? (
          <div className="serit bilgi">
            <IconUyari />
            <div>
              <strong>Henüz profil yok.</strong>
              <p>
                Alan bir profile yazılıyor. Profiller ekranından oyun için bir
                profil oluşturduktan sonra buraya dön.
              </p>
            </div>
          </div>
        ) : (
          <div className="satir-liste">
            {profiller.map((p) => (
              <div key={p.profile_id} className="secim-satir">
                <span className="secim-satir__govde">
                  <span className="secim-satir__ad">{p.display_name}</span>
                  <span className="secim-satir__alt">{alanOzeti(p)}</span>
                </span>
                <span className="panel__eylemler">
                  <button
                    className="button ghost"
                    disabled={mesgul}
                    onClick={() =>
                      onIslem(async () => {
                        await api.ceviriAlanSeciciAc(p.profile_id);
                      })
                    }
                  >
                    Alanı seç
                  </button>
                  {p.ceviri?.region && (
                    <button
                      className="button ghost"
                      disabled={mesgul}
                      onClick={() =>
                        onIslem(async () => {
                          await api.ceviriAlaniKaydet(p.profile_id, null);
                          onBildir('basari', 'Alan temizlendi, ekranın tamamı okunacak.');
                        })
                      }
                    >
                      Temizle
                    </button>
                  )}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* --- Son sonuç ---------------------------------------------------- */}
      <div className="panel">
        <div className="panel__baslik">
          Son çeviri
          <div className="panel__eylemler">
            <span className="rozet">
              {sonuc
                ? `${sonuc.birimler.length} birim · ${sonuc.bellekten} bellekten · ${sonuc.ceviriMs} ms`
                : 'henüz istek yok'}
            </span>
          </div>
        </div>

        <p className="panel__aciklama">
          Kaynak metin her zaman çevirinin üstünde duruyor. Sebebi ölçülmüş: bu
          tür bir model bir cümleyi <strong>hata vermeden</strong> düşürebiliyor
          ve çıktı akıcı göründüğü için fark edilmiyor. Kaynağı gizleyen bir
          tasarım o riski görünmez kılardı.
        </p>

        {sonuc?.uyarilar.map((u, i) => (
          <div className="serit uyari" key={i}>
            <IconUyari />
            <div>
              <p>{u}</p>
            </div>
          </div>
        ))}

        {sonuc && sonuc.birimler.length === 0 && (
          <p className="bos-durum">Bu istekte okunabilir bir yazı çıkmadı.</p>
        )}

        {sonuc?.birimler.map((b, i) => (
          <div className="ceviri-birim" key={`${i}-${b.kaynak}`}>
            <div className="ceviri-birim__kaynak">{b.kaynak}</div>
            {duzeltilen?.metin === b.kaynak ? (
              <>
                <textarea
                  className="text-input ceviri-birim__alan"
                  value={duzeltilen.taslak}
                  rows={2}
                  onChange={(e) => setDuzeltilen({ metin: b.kaynak, taslak: e.target.value })}
                />
                <div className="ceviri-birim__alt">
                  <button
                    className="button primary"
                    disabled={mesgul}
                    onClick={() =>
                      onIslem(async () => {
                        await api.ceviriDuzelt(b.kaynak, duzeltilen.taslak);
                        setDuzeltilen(null);
                        tazele();
                        onBildir(
                          'basari',
                          'Düzeltmen kaydedildi; bu metin bir daha modele gitmeyecek.',
                        );
                      })
                    }
                  >
                    Kaydet
                  </button>
                  <button className="button ghost" onClick={() => setDuzeltilen(null)}>
                    Vazgeç
                  </button>
                </div>
              </>
            ) : (
              <div className="ceviri-birim__ceviri">
                {b.bos ? <em>Model bu cümle için bir şey üretmedi.</em> : b.ceviri}
              </div>
            )}
            <div className="ceviri-birim__alt">
              <span className={`rozet${b.koken === 'kullanici' ? ' vurgu' : ''}`}>
                {b.koken === 'kullanici' ? 'senin düzeltmen' : 'makine çevirisi'}
              </span>
              {b.korunanTerimler.length > 0 && (
                <span className="rozet">korunan: {b.korunanTerimler.join(', ')}</span>
              )}
              {b.kayipTerimler.length > 0 && (
                <span className="rozet uyari">kayıp terim: {b.kayipTerimler.join(', ')}</span>
              )}
              {b.bos && <span className="rozet uyari">boş çeviri</span>}
              <span className="bosluk" />
              <button
                className="button ghost small"
                onClick={() => setDuzeltilen({ metin: b.kaynak, taslak: b.ceviri })}
              >
                Düzelt
              </button>
              <button
                className="button ghost small"
                disabled={mesgul}
                onClick={() =>
                  onIslem(async () => {
                    await api.ceviriKaydiSil(b.kaynak);
                    tazele();
                    onBildir('bilgi', 'Kayıt silindi; bu metin bir daha çevrilecek.');
                  })
                }
              >
                Sil
              </button>
            </div>
          </div>
        ))}
      </div>

      {/* --- Bellek ve terim sözlüğü -------------------------------------- */}
      <div className="panel">
        <div className="panel__baslik">
          Çeviri belleği ve terim sözlüğü
          <div className="panel__eylemler">
            <button className="button ghost" disabled={mesgul} onClick={() => tazele()}>
              <IconYenile />
              Yenile
            </button>
            <button
              className="button"
              disabled={mesgul}
              onClick={() =>
                onIslem(async () => {
                  await api.ceviriBelleginiTemizle();
                  tazele();
                  onBildir('basari', 'Makine kayıtları silindi, düzeltmelerin duruyor.');
                })
              }
            >
              Önbelleği temizle
            </button>
          </div>
        </div>

        <p className="panel__aciklama">
          Öğrenme burada: aynı metin bir daha geldiğinde model hiç çalışmıyor,
          kayıt aynen kullanılıyor. Model <strong>ince ayar görmüyor</strong> —
          zamanla kayan bir model, neyin neden değiştiğini göremediğin bir kara
          kutu olurdu. Dosya oyun başına bir JSON; bir metin editöründe açıp
          okuyabilir, düzeltebilir, silebilirsin.
        </p>
        <p className="panel__aciklama">
          Terim sözlüğü ölçülmüş bir boşluğu kapatıyor: genel amaçlı bir çeviri
          modeli oyun terimlerini bilmiyor. Sözlükteki terimler çeviriden önce
          metinden çıkarılıp yerlerine bir işaret konuyor — model terimi hiç
          görmüyor, dolayısıyla bozamıyor. Karşılığı kendisiyle aynı olan bir
          terim ise hiç çevrilmiyor (özel adlar için).
        </p>

        {bellek && (
          <p className="panel__aciklama">
            <code>{bellek.oyun}</code> · {bellek.kayitlar.length} kayıt ·{' '}
            {Object.keys(bellek.terimler).length} terim
          </p>
        )}

        <div className="terim-ekle">
          <label className="field">
            <span>Terim (İngilizce)</span>
            <input
              className="text-input"
              value={terim}
              placeholder="Longsword"
              onChange={(e) => setTerim(e.target.value)}
            />
          </label>
          <label className="field">
            <span>Karşılığı</span>
            <input
              className="text-input"
              value={karsilik}
              placeholder="Uzun Kılıç"
              onChange={(e) => setKarsilik(e.target.value)}
            />
          </label>
          <button
            className="button"
            disabled={mesgul || !terim.trim()}
            onClick={() =>
              onIslem(async () => {
                await api.ceviriTerimEkle(terim, karsilik.trim() || terim);
                setTerim('');
                setKarsilik('');
                tazele();
              })
            }
          >
            <IconArti />
            Ekle
          </button>
        </div>

        {bellek && Object.keys(bellek.terimler).length > 0 && (
          <div className="satir-liste">
            {Object.entries(bellek.terimler).map(([t, k]) => (
              <div key={t} className="secim-satir">
                <span className="secim-satir__govde">
                  <span className="secim-satir__ad">
                    {t} → {k}
                  </span>
                  <span className="secim-satir__alt">
                    {t === k
                      ? 'Aynı bırakılıyor — çevrilmeyecek'
                      : 'Modelden gizlenip sonra yerine konuyor'}
                  </span>
                </span>
                <button
                  className="button ghost icon"
                  aria-label={`${t} terimini sil`}
                  disabled={mesgul}
                  onClick={() =>
                    onIslem(async () => {
                      await api.ceviriTerimSil(t);
                      tazele();
                    })
                  }
                >
                  <IconCop />
                </button>
              </div>
            ))}
          </div>
        )}

        <p className="panel__aciklama">
          Bir çeviriyi beğenmediysen <strong>"Düzelt"</strong> kullan: reddetmek
          yalnızca yanlış olduğunu söyler, doğrusunu söylemez. Özellik hiç geri
          bildirim vermesen de tam çalışmak zorunda — düzeltme bir zorunluluk
          değil, bir kısayol.{' '}
          <button className="button ghost small" onClick={onAyarlara}>
            Ayarlar
          </button>
        </p>
      </div>
    </>
  );
}
