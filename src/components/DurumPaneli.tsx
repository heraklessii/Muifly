/**
 * Durum ekranı: canlı ölçüm, öncesi/sonrası karşılaştırma, hızlı eylemler.
 *
 * Ekranın sözü şu: "şu an ne oluyor ve az önce ne değişti". Sayısal vaat yok;
 * gösterilen her değer bu makinede ölçülmüş (`DESIGN_PRINCIPLES.md` madde 4).
 *
 * Yerleşim yukarıdan aşağı bir öncelik sırası: önce hangi moddayız ve ne
 * yapılabilir (kahraman kartı), sonra ölçümler, sonra karşılaştırma, en sonda
 * sistemde duran değişiklikler. Kullanıcı en sık ilk kutuya bakıyor; oraya
 * ölçüm koyup eylemi aşağı itmek, aracı "izlenen bir gösterge paneli" yapardı
 * — oysa asıl iş uygulamak ve geri almak.
 */

import type { Durum, Karsilastirma, Ornek, Ozet } from '../lib/types';
import { modRengi, modSureci } from '../lib/types';
import { BOS, milisaniye, sayi, sure, yuzde, yon, YON_ETIKETLERI } from '../lib/format';
import {
  IconAsagi,
  IconBilgi,
  IconEsit,
  IconGeriAl,
  IconGunluk,
  IconOkSag,
  IconOynat,
  IconUyari,
  IconYukari,
} from './Icons';
import { KareOlcumu } from './KareOlcumu';
import { MetrikGrafik } from './MetrikGrafik';
import { Sparkline } from './Sparkline';

interface Props {
  durum: Durum;
  ornekler: Ornek[];
  ozet: Ozet;
  karsilastirma: Karsilastirma | null;
  yapilmayanlar: [string, string][];
  onUygula: () => void;
  onOturumuKapat: () => void;
  onHepsiniGeriAl: () => void;
  /** Günlük sekmesine geçiş: "ayrıntılar günlükte" bir yönlendirme olmalı. */
  onGunluge: () => void;
  /** Uzun süren işlemleri sarmalayan yardımcı (meşgul durumu + hata bildirimi). */
  onIslem: (calis: () => Promise<void>) => void;
  mesgul: boolean;
}

/** Mini eğrilerin baktığı son ölçüm penceresi. */
const SPARK_PENCERESI = 40;

function Olcum({
  etiket,
  deger,
  birim,
  not,
  seri,
  tavan,
  renk,
}: {
  etiket: string;
  deger: string;
  birim?: string;
  not?: string;
  /** Eğrisi olan ölçümler için son değerler. Türetilmiş ölçümlerde yok. */
  seri?: (number | null)[];
  tavan?: number;
  renk?: string;
}) {
  const bos = deger === BOS;
  return (
    <div className="olcum">
      <span className="olcum__etiket">{etiket}</span>
      <span className={`olcum__deger${bos ? ' bos' : ''}`}>
        {bos ? 'ölçüm yok' : deger}
        {!bos && birim && <span className="olcum__birim">{birim}</span>}
      </span>
      {not && <span className="olcum__not">{not}</span>}
      {seri && <Sparkline degerler={seri} tavan={tavan} renk={renk} />}
    </div>
  );
}

/**
 * Yön işareti.
 *
 * Bir oran ÜRETİLMİYOR ("%12 iyileşme" gibi): iki pencerenin ölçüm koşulları
 * aynı değil. Yalnızca hangi yöne gittiği yazıyor, büyüklüğü kullanıcı iki
 * değeri yan yana görerek kendisi yorumluyor (`lib/format.ts` → `yon`).
 */
function Yon({ onceki, sonraki }: { onceki: number | null; sonraki: number | null }) {
  const y = yon(onceki, sonraki);
  if (!y) return null;
  const Ikon = y === 'dustu' ? IconAsagi : y === 'artti' ? IconYukari : IconEsit;
  return (
    <span className="yon" data-yon={y}>
      <Ikon />
      {YON_ETIKETLERI[y]}
    </span>
  );
}

/** Karşılaştırmanın tek bir satırı. Sağ sütunda yön işareti de var. */
function Satirlar({
  ozet,
  karsi,
}: {
  ozet: Ozet;
  /** Karşı sütunun özeti; verilirse yön işareti çiziliyor. */
  karsi?: Ozet;
}) {
  const satirlar: { ad: string; deger: string; a: number | null; b: number | null }[] = [
    { ad: 'CPU', deger: yuzde(ozet.cpuOrt, 1), a: karsi?.cpuOrt ?? null, b: ozet.cpuOrt },
    { ad: 'Bellek', deger: yuzde(ozet.bellekOrt, 0), a: karsi?.bellekOrt ?? null, b: ozet.bellekOrt },
    {
      ad: 'Gecikme',
      deger: milisaniye(ozet.gecikmeOrtMs, 1),
      a: karsi?.gecikmeOrtMs ?? null,
      b: ozet.gecikmeOrtMs,
    },
    { ad: 'Jitter', deger: milisaniye(ozet.jitterMs, 1), a: karsi?.jitterMs ?? null, b: ozet.jitterMs },
  ];

  return (
    <>
      {satirlar.map((s) => (
        <div key={s.ad} className="kv">
          <span className="kv__ad">{s.ad}</span>
          <span className="kv__deger">
            {s.deger} {karsi && <Yon onceki={s.a} sonraki={s.b} />}
          </span>
        </div>
      ))}
    </>
  );
}

function OzetSutunu({
  baslik,
  ozet,
  sn,
  sinif,
  karsi,
}: {
  baslik: string;
  ozet: Ozet;
  sn: number;
  sinif?: string;
  karsi?: Ozet;
}) {
  return (
    <div className={`karsilastirma__sutun${sinif ? ` ${sinif}` : ''}`}>
      <div className="karsilastirma__baslik">
        {baslik} · {sure(sn)}
      </div>
      <Satirlar ozet={ozet} karsi={karsi} />
    </div>
  );
}

export function DurumPaneli({
  durum,
  ornekler,
  ozet,
  karsilastirma,
  yapilmayanlar,
  onUygula,
  onOturumuKapat,
  onHepsiniGeriAl,
  onGunluge,
  onIslem,
  mesgul,
}: Props) {
  const oyunda = durum.mod.mod !== 'bosta' && durum.mod.mod !== 'sistemAcilisi';
  const surec = modSureci(durum.mod);
  const son = ornekler.slice(-SPARK_PENCERESI);

  return (
    <>
      {/* --- Kahraman kartı: hangi moddayız, ne yapılabilir --------------- */}
      <section className="hero" data-mod={modRengi(durum.mod)}>
        <div className="hero__govde">
          <span className="hero__etiket">
            <span className="nabiz" />
            Şu anki mod
          </span>
          <span className="hero__ad">{durum.modAdi}</span>
          {surec && <span className="hero__surec">{surec}</span>}
          <p className="hero__aciklama">
            {oyunda ? (
              <>
                Bir oyun önde. Profili uygulamak sistemde değişiklik yapar; yapılan
                her şey günlüğe ve geri alma defterine yazılır, tek tıkla geri
                alınabilir.
              </>
            ) : (
              <>
                Şu an bir oyun algılanmadı. Tam ekran bir oyun öne geldiğinde mod
                kendiliğinden değişir; optimizasyonun uygulanması için Ayarlar'daki
                otomatik uygulama açık olmalı — varsayılan olarak kapalıdır, çünkü
                program sen bakmadan sistemini değiştirmemeli.
              </>
            )}
          </p>
        </div>

        <div className="hero__eylemler">
          <button
            className={`button buyuk${oyunda ? ' primary' : ''}`}
            onClick={onUygula}
            disabled={mesgul}
          >
            <IconOynat />
            {oyunda ? 'Optimizasyonu uygula' : 'Öndeki uygulamaya uygula'}
          </button>
          {durum.bekleyenGeriAlma > 0 && (
            <button className="button" onClick={onHepsiniGeriAl} disabled={mesgul}>
              <IconGeriAl />
              Varsayılana dön
            </button>
          )}
        </div>
      </section>

      {/* --- Canlı ölçümler ---------------------------------------------- */}
      <div className="panel">
        <div className="panel__baslik">
          Canlı ölçüm
          <span className="panel__eylemler">
            <span className="olcum__not">
              {ozet.ornekSayisi > 0
                ? `${ozet.ornekSayisi} örnek`
                : 'ölçüm birikiyor'}
            </span>
          </span>
        </div>

        <div className="olcum-izgara">
          <Olcum
            etiket="CPU"
            deger={sayi(ozet.cpuOrt, 0)}
            birim="%"
            not="son ölçümlerin ortalaması"
            seri={son.map((o) => o.cpu)}
            tavan={100}
          />
          <Olcum
            etiket="Bellek"
            deger={sayi(ozet.bellekOrt, 0)}
            birim="%"
            seri={son.map((o) => o.bellek)}
            tavan={100}
            renk="var(--info)"
          />
          <Olcum
            etiket="Gecikme"
            deger={sayi(ozet.gecikmeOrtMs, 1)}
            birim="ms"
            not={durum.ayarlar.gecikmeHedefi}
            seri={son.map((o) => o.gecikmeMs)}
            renk="var(--warning)"
          />
          {/* Jitter ve paket kaybı türetilmiş değerler: zaman serileri yok,
              o yüzden eğrileri de yok. Olmayan bir seriyi uydurmak,
              gösterilen her şeyin ölçülmüş olduğu kuralını bozardı. */}
          <Olcum
            etiket="Jitter"
            deger={sayi(ozet.jitterMs, 1)}
            birim="ms"
            not="ardışık ölçümlerin farkı"
          />
          <Olcum
            etiket="Paket kaybı"
            deger={ozet.ornekSayisi > 0 ? sayi(ozet.kayipYuzde, 0) : BOS}
            birim="%"
            not="cevapsız ölçüm oranı"
          />
          {/* Kare ölçümü buradaki sürekli akışın parçası DEĞİL: yükseltilmiş
              yetki istiyor ve arka planda çalışamaz (karar #27). Kendi
              paneli, kendi düğmesi var. */}
        </div>

        <MetrikGrafik ornekler={ornekler} aralikSn={durum.ayarlar.olcumAraligiSn} />
      </div>

      {/* --- Kare ölçümü -------------------------------------------------- */}
      <KareOlcumu oyunVar={oyunda} mesgul={mesgul} onIslem={onIslem} />

      {/* --- Öncesi / sonrası -------------------------------------------- */}
      {karsilastirma && (
        <div className="panel">
          <div className="panel__baslik">Öncesi / sonrası</div>
          <p className="panel__aciklama">
            İki pencerenin ölçümü yan yana. Tek bir <strong>iyileşme oranı</strong>{' '}
            gösterilmiyor: oyun içi yük iki pencerede aynı olmadığı için böyle bir
            oran yanıltıcı olurdu. Yön belirtiliyor, yorum sende.
          </p>
          <div className="karsilastirma">
            <OzetSutunu
              baslik="Öncesi"
              ozet={karsilastirma.onceki}
              sn={karsilastirma.oncekiSaniye}
            />
            <span className="karsilastirma__ok" aria-hidden="true">
              <IconOkSag />
            </span>
            <OzetSutunu
              baslik="Sonrası"
              ozet={karsilastirma.sonraki}
              sn={karsilastirma.sonrakiSaniye}
              sinif="sonra"
              karsi={karsilastirma.onceki}
            />
          </div>
          <p className="field-hint">
            Bunlar bu makinede, bu oturumda ölçülen değerlerdir; başka bir oturumda
            farklı çıkabilir.
          </p>
        </div>
      )}

      {/* --- Sistemde duran değişiklikler --------------------------------- */}
      {durum.bekleyenGeriAlma > 0 && (
        <div className="panel">
          <div className="panel__baslik">
            Bekleyen değişiklikler
            <span className="rozet vurgu">{durum.bekleyenGeriAlma}</span>
          </div>
          <p className="panel__aciklama">
            Sistemde şu an duran {durum.bekleyenGeriAlma} değişiklik var
            {durum.kaliciDegisiklik > 0 &&
              ` (${durum.kaliciDegisiklik} tanesi kalıcı: oyun kapansa da kalkmaz)`}
            . Hepsi tek tıkla geri alınabilir.
          </p>
          <div className="satir__eylemler">
            {oyunda && (
              <button className="button" onClick={onOturumuKapat} disabled={mesgul}>
                <IconGeriAl />
                Oturumluk olanları geri al
              </button>
            )}
            <button className="button danger" onClick={onHepsiniGeriAl} disabled={mesgul}>
              <IconGeriAl />
              Varsayılana dön
            </button>
            {/* Tek tek geri alma günlükte: buradan oraya bir yol olmalı,
                yoksa "ayrıntılar günlükte" cümlesi kullanıcıyı arattırıyor. */}
            <button className="button ghost" onClick={onGunluge}>
              <IconGunluk />
              Günlükte gör
            </button>
          </div>
        </div>
      )}

      {!durum.yonetici && (
        <div className="serit uyari">
          <IconUyari />
          <span>
            Yönetici yetkisi olmadan çalışıyor. Süreç önceliği ve dondurma çoğu
            uygulamada çalışır; sistem geneli ağ ayarları (Nagle, QoS) için
            Muifly'ı yönetici olarak başlatman gerekir. Sürekli yükseltilmiş
            çalışmıyor olması bilinçli bir tercih.
          </span>
        </div>
      )}

      {/* --- Ne yapmaz ---------------------------------------------------- */}
      {yapilmayanlar.length > 0 && (
        <div className="panel">
          <div className="panel__baslik">Muifly ne yapmaz</div>
          <p className="panel__aciklama">
            Bir performans aracının yapmadıklarını söylemesi, yaptıklarını
            saymasından daha çok şey anlatır.
          </p>
          <div className="yapilmayan-izgara">
            {yapilmayanlar.map(([ad, aciklama]) => (
              <div key={ad} className="yapilmayan">
                <IconBilgi />
                <div>
                  <div className="yapilmayan__ad">{ad}</div>
                  <div className="yapilmayan__aciklama">{aciklama}</div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </>
  );
}
