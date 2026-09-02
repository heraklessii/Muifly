/**
 * Ölçekleme ekranı (Faz 3).
 *
 * Üç kural, üçü de tasarım ilkelerinden çıkıyor:
 *
 * 1. **Bedel önce yazılıyor.** Ölçekleme her kareye gecikme ekler. Bu cümle
 *    açıklamanın dibinde değil başında; ölçülen süre de çalışırken ekranda
 *    duruyor. Rakip araçların çoğu bu sayıyı hiç göstermiyor.
 * 2. **Kalite iddiası yok.** Algoritmalar "daha iyi/daha kötü" diye
 *    sıralanmıyor; her biri ne yaptığını söylüyor, karar kullanıcının.
 *    Açıklamalar backend'den geliyor (tasarım ilkesi 4'ün testi orada).
 * 3. **Rekabetçi modda kapalı olduğu söyleniyor.** Düğmeyi sessizce pasif
 *    bırakmak, kullanıcıya bozuk gibi görünürdü.
 */

import { useCallback, useEffect, useState } from 'react';

import * as api from '../lib/api';
import { milisaniye, sayi } from '../lib/format';
import type {
  AlgoritmaAnahtari,
  AlgoritmaBilgisi,
  Ekran,
  OlceklemeDurumu,
} from '../lib/types';
import { IconOlcekleme, IconUyari, IconYenile } from './Icons';

interface Props {
  /** Rekabetçi mod açıkken ölçekleme hiç açılmıyor. */
  rekabetciMod: boolean;
  /** Ayarlardaki ekran seçimi. */
  olceklemeEkrani: number;
  mesgul: boolean;
  onIslem: (calis: () => Promise<void>) => Promise<void>;
  onBildir: (tur: 'basari' | 'bilgi' | 'hata', mesaj: string) => void;
  onEkranDegistir: (indeks: number) => void;
  /** "Rekabetçi mod açık" derken kapatacak yer de gösterilmeli. */
  onAyarlara: () => void;
}

/** Durum yoklama aralığı — yalnızca çalışırken. */
const YOKLAMA_MS = 1000;

export function OlceklemePaneli({
  rekabetciMod,
  olceklemeEkrani,
  mesgul,
  onIslem,
  onBildir,
  onEkranDegistir,
  onAyarlara,
}: Props) {
  const [algoritmalar, setAlgoritmalar] = useState<AlgoritmaBilgisi[]>([]);
  const [ekranlar, setEkranlar] = useState<Ekran[]>([]);
  const [ekranHatasi, setEkranHatasi] = useState<string | null>(null);
  const [durum, setDurum] = useState<OlceklemeDurumu | null>(null);
  const [secili, setSecili] = useState<AlgoritmaAnahtari>('tam_sayi');

  const durumuOku = useCallback(() => {
    api.olceklemeDurumu().then(setDurum).catch(() => {});
  }, []);

  useEffect(() => {
    api.olceklemeAlgoritmalari().then(setAlgoritmalar).catch(() => setAlgoritmalar([]));
    api
      .olceklemeEkranlari()
      .then((e) => {
        setEkranlar(e);
        setEkranHatasi(null);
      })
      .catch((e) => {
        setEkranlar([]);
        setEkranHatasi(String(e));
      });
    durumuOku();
  }, [durumuOku]);

  // Yoklama yalnızca ölçekleme çalışırken: boştayken saniyede bir komut
  // göndermek, aracın kendisinin yük olması demek olurdu.
  useEffect(() => {
    if (!durum?.calisiyor) return;
    const t = window.setInterval(durumuOku, YOKLAMA_MS);
    return () => window.clearInterval(t);
  }, [durum?.calisiyor, durumuOku]);

  // Çalışan ölçeklemenin algoritması ekrandaki seçimle aynı görünsün.
  useEffect(() => {
    if (durum?.calisiyor && durum.algoritma) setSecili(durum.algoritma);
  }, [durum?.calisiyor, durum?.algoritma]);

  const calisiyor = durum?.calisiyor ?? false;
  const gecikme = durum?.gecikme ?? null;

  const algoritmaSec = (a: AlgoritmaAnahtari) => {
    setSecili(a);
    // Çalışırken değiştirmek yeniden başlatma gerektirmiyor; ekran
    // kararmıyor ve ölçüm tamponu backend'de sıfırlanıyor.
    if (calisiyor) {
      api
        .olceklemeAlgoritma(a)
        .then(durumuOku)
        .catch((e) => onBildir('hata', String(e)));
    }
  };

  return (
    <>
      {/* --- Ne yaptığı ve bedeli ---------------------------------------- */}
      <div className="panel">
        <div className="panel__baslik">
          Ölçekleme
          <div className="panel__eylemler">
            <button
              className="button ghost"
              disabled={mesgul || calisiyor}
              onClick={() =>
                onIslem(async () => {
                  const d = await api.olceklemeDenemesi();
                  onBildir(
                    'basari',
                    d.yeniKare
                      ? `Yakalama çalışıyor: ${d.genislik}×${d.yukseklik}.`
                      : `Yakalama açıldı (${d.genislik}×${d.yukseklik}) ama bu sürede ekran değişmedi — hareketsiz bir masaüstünde beklenen bir sonuç.`,
                  );
                })
              }
            >
              <IconYenile />
              Yakalamayı dene
            </button>
            {calisiyor ? (
              <button
                className="button"
                disabled={mesgul}
                onClick={() =>
                  onIslem(async () => {
                    await api.olceklemeDurdur();
                    durumuOku();
                    onBildir('basari', 'Ölçekleme durduruldu.');
                  })
                }
              >
                Durdur
              </button>
            ) : (
              <button
                className="button primary"
                disabled={mesgul || rekabetciMod}
                onClick={() =>
                  onIslem(async () => {
                    await api.olceklemeBaslat(secili);
                    durumuOku();
                    onBildir('basari', 'Ölçekleme başladı.');
                  })
                }
              >
                <IconOlcekleme />
                Başlat
              </button>
            )}
          </div>
        </div>

        <p className="panel__aciklama">
          Ekran görüntüsü Windows'un masaüstü çoğaltma arayüzünden okunur,
          seçilen algoritmayla büyütülür ve oyunun üstünde duran bir pencerede
          gösterilir. <strong>Oyunun kendisine dokunulmaz</strong>: belleğine
          yazılmaz, çağrıları yönlendirilmez, süreci açılmaz.
        </p>
        <p className="panel__aciklama">
          <strong>Bedeli var:</strong> araya giren her adım kareyi geciktirir.
          Eklenen süre çalışırken aşağıda ölçülüp gösteriliyor. Oyun münhasır
          tam ekrandaysa yakalama yapılamaz — kenarlıksız pencere modu gerekir.
        </p>

        {/*
          Kaçış kısayolu ölçekleme AÇILMADAN önce de yazıyor. Ekranı
          kaplayan pencerenin nasıl kapatılacağını ancak kaplandıktan sonra
          öğrenen kullanıcı için o bilgi yok demektir (karar #34).
        */}
        <p className="panel__aciklama">
          <strong>Durdurma kısayolu:</strong>{' '}
          <kbd className="kbd">
            {durum?.durdurmaKisayoli ?? 'Ctrl+Alt+Shift+S'}
          </kbd>
          . Ölçekleme
          penceresi ekranı kaplar, tıklanamaz ve Alt+Tab'da görünmez; her
          durumda klavyeden kapatılabilmesi için bu kısayol kaydedilir.
          Kaydedilemezse ölçekleme başlatılmaz.
        </p>

        {durum?.hedefBekleniyor && calisiyor && (
          <div className="serit uyari">
            <IconUyari />
            <div>
              <strong>Ölçekleme açık, ekranda henüz bir şey yok.</strong>
              <p>
                Ölçeklenecek pencere önde değil. Ölçekleme yalnızca bir oyun
                ya da uygulama penceresi öndeyken çizer — masaüstünde ve
                Muifly'a bakarken ekran sana kalır.
              </p>
            </div>
          </div>
        )}

        {rekabetciMod && (
          <div className="serit uyari">
            <IconUyari />
            <div>
              <strong>Rekabetçi mod açık, ölçekleme kapalı.</strong>
              <p>
                Rekabetçi mod gecikmeyi en aza indirmek için var; ölçekleme
                gecikme ekliyor. İkisi aynı anda açık olmuyor.
              </p>
              <button className="button ghost" onClick={onAyarlara}>
                Ayarlara git
              </button>
            </div>
          </div>
        )}

        {durum?.uyari && calisiyor && (
          <div className="serit uyari">
            <IconUyari />
            <div>
              <strong>Ölçekleme çalışıyor, ama bir kısıt var.</strong>
              <p>{durum.uyari}</p>
            </div>
          </div>
        )}

        {durum?.sonEngel && !calisiyor && (
          <div className="serit uyari">
            <IconUyari />
            <div>
              <strong>Ölçekleme durdu.</strong>
              <p>{durum.sonEngel}</p>
            </div>
          </div>
        )}
      </div>

      {/* --- Algoritma ---------------------------------------------------- */}
      <div className="panel">
        <div className="panel__baslik">Algoritma</div>
        <p className="panel__aciklama">
          Hiçbiri "daha iyi" değil; farklı şeyler yapıyorlar. Çalışırken
          değiştirebilirsin, ekran kararmaz.
        </p>

        <div className="satir-liste">
          {algoritmalar.map((a) => (
            <label
              key={a.anahtar}
              className={`secim-satir${secili === a.anahtar ? ' vurgulu' : ''}`}
            >
              <input
                type="radio"
                name="olcekleme-algoritma"
                value={a.anahtar}
                checked={secili === a.anahtar}
                onChange={() => algoritmaSec(a.anahtar)}
              />
              <span className="secim-satir__govde">
                <span className="secim-satir__ad">{a.ad}</span>
                <span className="secim-satir__alt">{a.aciklama}</span>
              </span>
            </label>
          ))}
        </div>
      </div>

      {/*
        --- Kare üretimi (Faz 4) --------------------------------------
        Bedeli anahtardan ÖNCE yazıyor. Bu panelin 1 numaralı kuralının
        (dosya başı) kare üretimindeki karşılığı: burada eklenen gecikme
        ölçekleminkinden farklı, çünkü algoritma hızlandıkça azalmıyor.
      */}
      <div className="panel">
        <div className="panel__baslik">Kare üretimi</div>
        <p className="panel__aciklama">
          İki gerçek kare arasına, ikisinden hesaplanmış bir kare koyar.
          Hareket tahmini ekran görüntüsünden yapılır;{' '}
          <strong>oyunun kendisine yine dokunulmaz</strong>.
        </p>
        <p className="panel__aciklama">
          <strong>Bedeli ölçeklemeninkinden farklı:</strong> ara kare, iki
          gerçek karenin <em>ikisi de</em> elde olmadan hesaplanamaz. Yani
          ikinci gerçek kare bir sunum turu bekletilir. Bu bekleme daha hızlı
          bir ekran kartıyla <em>azalmaz</em> — kare üretiminin tanımında
          vardır. Aşağıdaki "Kare üretimi" satırı yalnızca hesabın CPU
          süresini gösterir, bu beklemeyi değil.
        </p>

        {durum?.uretimUyarisi && (
          <div className="serit uyari">
            <IconUyari />
            <div>
              <strong>Bu makinede beklendiği gibi çalışmayabilir.</strong>
              <p>{durum.uretimUyarisi}</p>
            </div>
          </div>
        )}

        <label className="secim-satir">
          <input
            type="checkbox"
            checked={durum?.uretimAcik ?? false}
            disabled={!calisiyor || durum?.uretimKullanilabilir === false}
            onChange={(e) =>
              onIslem(async () => {
                await api.olceklemeUretimi(e.target.checked);
                durumuOku();
              })
            }
          />
          <span className="secim-satir__govde">
            <span className="secim-satir__ad">Kare üretimini aç</span>
            <span className="secim-satir__alt">
              {calisiyor
                ? 'Açıp kapatmak ekranı karartmaz — farkı aynı sahnede görebilirsin.'
                : 'Ölçekleme çalışırken açılabilir.'}
            </span>
          </span>
        </label>
      </div>

      {/* --- Ekran -------------------------------------------------------- */}
      <div className="panel">
        <div className="panel__baslik">Ekran</div>
        {ekranHatasi ? (
          <div className="bos-durum">
            <IconUyari />
            <strong>Ekran listesi okunamadı</strong>
            <p>{ekranHatasi}</p>
          </div>
        ) : (
          <div className="satir-liste">
            {ekranlar.map((e) => (
              <label
                key={e.indeks}
                className={`secim-satir${olceklemeEkrani === e.indeks ? ' vurgulu' : ''}`}
              >
                <input
                  type="radio"
                  name="olcekleme-ekran"
                  value={e.indeks}
                  checked={olceklemeEkrani === e.indeks}
                  disabled={calisiyor}
                  onChange={() => onEkranDegistir(e.indeks)}
                />
                <span className="secim-satir__govde">
                  <span className="secim-satir__ad">
                    {e.ad}
                    {e.birincil && <span className="rozet">birincil</span>}
                  </span>
                  <span className="secim-satir__alt">
                    {e.genislik}×{e.yukseklik}
                  </span>
                </span>
              </label>
            ))}
          </div>
        )}
        {calisiyor && (
          <p className="field-hint">
            Ekran seçimi ölçekleme dururken değiştirilebilir.
          </p>
        )}
      </div>

      {/* --- Ölçülen gecikme ---------------------------------------------- */}
      <div className="panel">
        <div className="panel__baslik">Eklenen gecikme</div>
        {gecikme == null ? (
          <div className="bos-durum">
            <IconOlcekleme />
            <strong>Henüz ölçüm yok</strong>
            <p>
              Ölçekleme çalıştığında boru hattının kare başına ne kadar
              sürdüğü burada görünür. Bu bir kazanç değil, ödenen bedeldir.
            </p>
          </div>
        ) : (
          <>
            <div className="olcum-izgara">
              <div className="olcum">
                <span className="olcum__etiket">Kare başına</span>
                <span className="olcum__deger">{milisaniye(gecikme.ortMs, 2)}</span>
              </div>
              <div className="olcum">
                <span className="olcum__etiket">En kötü %1</span>
                <span className="olcum__deger">{milisaniye(gecikme.p1KotuMs, 2)}</span>
              </div>
              <div className="olcum">
                <span className="olcum__etiket">En kötü kare</span>
                <span className="olcum__deger">{milisaniye(gecikme.enKotuMs, 2)}</span>
              </div>
              <div className="olcum">
                <span className="olcum__etiket">Ölçülen kare</span>
                <span className="olcum__deger">{sayi(gecikme.kareSayisi, 0)}</span>
              </div>
            </div>

            <div className="satir-liste">
              <div className="kv">
                <span className="kv__ad">Yakalama</span>
                <span className="kv__deger">{milisaniye(gecikme.yakalamaOrtMs, 2)}</span>
              </div>
              {gecikme.uretilenKare > 0 && (
                <div className="kv">
                  <span className="kv__ad">Kare üretimi (hesap süresi)</span>
                  <span className="kv__deger">{milisaniye(gecikme.uretimOrtMs, 2)}</span>
                </div>
              )}
              <div className="kv">
                <span className="kv__ad">Sunum (dikey eşitleme beklemesi dahil)</span>
                <span className="kv__deger">{milisaniye(gecikme.sunumOrtMs, 2)}</span>
              </div>
              {gecikme.uretilenKare > 0 && (
                <div className="kv">
                  <span className="kv__ad">Üretilen kare</span>
                  <span className="kv__deger">{sayi(gecikme.uretilenKare, 0)}</span>
                </div>
              )}
              {gecikme.yenilemeHz != null && (
                <div className="kv">
                  <span className="kv__ad">Ekran yenileme</span>
                  <span className="kv__deger">{gecikme.yenilemeHz} Hz</span>
                </div>
              )}
              <div className="kv">
                <span className="kv__ad">Yeni kare gelmeyen tur</span>
                <span className="kv__deger">{sayi(gecikme.bosTur, 0)}</span>
              </div>
              {durum && durum.kaynakGenislik > 0 && (
                <div className="kv">
                  <span className="kv__ad">Çözünürlük</span>
                  <span className="kv__deger">
                    {durum.kaynakGenislik}×{durum.kaynakYukseklik} →{' '}
                    {durum.hedefGenislik}×{durum.hedefYukseklik}
                  </span>
                </div>
              )}
            </div>

            <p className="field-hint">
              Süreler CPU tarafında ölçülüyor. "Sunum" satırı ekranın kendi
              yenileme hızını beklemeyi de içeriyor; o bekleme bir maliyet
              değil, kareyi yırtılmadan göstermenin bedeli. Bu sayılar senin
              makinende ölçüldü — başka bir makinede farklı çıkar.
            </p>
          </>
        )}
      </div>
    </>
  );
}
