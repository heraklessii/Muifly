/**
 * Ağ ekranı: DNS karşılaştırması, yol testi, TCP/QoS ayarları.
 *
 * Bu ekranın en önemli özelliği, **yapmadıklarını söylemesi**. DNS ölçülüyor
 * ama değiştirilmiyor; yol gösteriliyor ama yönlendirilmiyor. Rakiplerin
 * abarttığı alanda abartmamak, ürünün konumlandırmasının merkezi
 * (`docs/PRODUCT_VISION.md`).
 *
 * Ölçüm sonuçları sayının yanında bir de çubukla gösteriliyor. Çubuk
 * **göreli**: en yavaş sonuç tam uzunluk. Mutlak bir ölçek kullanılmadı çünkü
 * "iyi ping" diye evrensel bir eşik yok ve öyle bir eşik çizmek, sayısal vaat
 * yasağının (`DESIGN_PRINCIPLES.md` madde 4) arka kapıdan ihlali olurdu.
 */

import { useEffect, useState } from 'react';

import * as api from '../lib/api';
import type { DnsSonucu, QosIlkesi, TcpDurumu, YolSonucu } from '../lib/types';
import { BOS, milisaniye } from '../lib/format';
import { IconAg, IconBilgi, IconGeriAl, IconUyari, IconYenile } from './Icons';

interface Props {
  yonetici: boolean;
  gecikmeHedefi: string;
  mesgul: boolean;
  onIslem: (calis: () => Promise<void>) => void;
  onBildir: (tur: 'bilgi' | 'basari' | 'hata', mesaj: string) => void;
}

/** Yol testinde gecikmenin en çok arttığı atlama — backend'dekiyle aynı mantık. */
function enBuyukSicrama(yol: YolSonucu | null): number | null {
  if (!yol) return null;
  let onceki: number | null = null;
  let enIyi: { atlama: number; artis: number } | null = null;
  for (const d of yol.dugumler) {
    if (d.gecikmeMs == null) continue;
    if (onceki != null) {
      const artis = d.gecikmeMs - onceki;
      if (artis > 0 && (!enIyi || artis > enIyi.artis)) {
        enIyi = { atlama: d.atlama, artis };
      }
    }
    onceki = d.gecikmeMs;
  }
  return enIyi?.atlama ?? null;
}

/** Bir ölçüm kümesindeki en büyük değer — çubukların göreli tabanı. */
function enBuyuk(degerler: (number | null)[]): number {
  const olculen = degerler.filter((d): d is number => d != null);
  return olculen.length > 0 ? Math.max(...olculen) : 0;
}

export function AgPaneli({ yonetici, gecikmeHedefi, mesgul, onIslem, onBildir }: Props) {
  const [dns, setDns] = useState<DnsSonucu[] | null>(null);
  const [yol, setYol] = useState<YolSonucu | null>(null);
  const [hedef, setHedef] = useState(gecikmeHedefi);
  const [tcp, setTcp] = useState<TcpDurumu | null>(null);
  const [qos, setQos] = useState<QosIlkesi[]>([]);
  const [aciklamalar, setAciklamalar] = useState<[string, string][]>([]);

  const yenile = () => {
    api.tcpDurumu().then(setTcp).catch(() => setTcp(null));
    api.qosIlkeleri().then(setQos).catch(() => setQos([]));
  };

  useEffect(() => {
    yenile();
    api.agAciklamalari().then(setAciklamalar).catch(() => setAciklamalar([]));
  }, []);

  const sicrama = enBuyukSicrama(yol);
  const bizimQos = qos.filter((q) => q.bizim);
  const dnsTavani = enBuyuk((dns ?? []).map((d) => d.ortalamaMs));
  const yolTavani = enBuyuk((yol?.dugumler ?? []).map((d) => d.gecikmeMs));

  return (
    <>
      {/* --- DNS ---------------------------------------------------------- */}
      <div className="panel">
        <div className="panel__baslik">
          DNS karşılaştırması
          <div className="panel__eylemler">
            <button
              className="button primary"
              disabled={mesgul}
              onClick={() =>
                onIslem(async () => {
                  const s = await api.dnsKarsilastir(3);
                  setDns(s);
                })
              }
            >
              <IconYenile />
              Ölç
            </button>
          </div>
        </div>

        <p className="panel__aciklama">
          Her çözümleyiciye gerçek bir DNS sorgusu gönderilip cevap süresi
          ölçülür. <strong>Muifly DNS ayarını değiştirmez</strong> — sonucu
          gösterir, değiştirme kararı ve işlemi sende. Bunun sebebi, adaptör
          DNS'ini programın değiştirmesinin yanlış gittiğinde seni internetsiz
          bırakabilmesi.
        </p>

        {dns == null ? (
          <div className="bos-durum">
            <IconAg />
            <strong>Henüz ölçüm yapılmadı</strong>
            <p>
              "Ölç" bir dizi DNS sunucusuna sorgu gönderir. Ağa yalnızca senin
              başlattığın ölçümler için çıkılır ve her çıkış günlüğe yazılır.
            </p>
          </div>
        ) : (
          <>
            {/* Uzun çubuk = yavaş. Bu cümle olmadan çubuk "daha çok = daha
                iyi" diye okunabiliyor. */}
            <p className="field-hint">
              Çubuk uzunluğu ölçülen süreyle orantılı: kısa olan daha hızlı
              cevap verdi. Ölçek bu turdaki en yavaş sonuca göre, mutlak bir
              eşiğe göre değil.
            </p>
            <div className="satir-liste">
            {dns.map((d) => (
              <div
                key={d.adres}
                className={`dns-satir${d.sisteminKullandigi ? ' vurgulu' : ''}`}
              >
                <div className="dns-satir__ad">
                  {d.ad}
                  {d.sisteminKullandigi && (
                    <span className="rozet vurgu">şu an kullanılıyor</span>
                  )}
                </div>
                <div className="dns-satir__deger">
                  {d.ortalamaMs == null ? (
                    <span className="olcum__deger bos">cevap yok</span>
                  ) : (
                    milisaniye(d.ortalamaMs, 1)
                  )}
                </div>
                <div className="dns-satir__alt">
                  {d.adres} · {d.cevap}/{d.deneme} cevap
                </div>
                <div className="dns-satir__cubuk">
                  <i
                    style={{
                      width:
                        d.ortalamaMs != null && dnsTavani > 0
                          ? `${(d.ortalamaMs / dnsTavani) * 100}%`
                          : '0%',
                    }}
                  />
                </div>
              </div>
            ))}
            </div>
          </>
        )}
      </div>

      {/* --- Yol testi ---------------------------------------------------- */}
      <div className="panel">
        <div className="panel__baslik">Yol testi</div>
        <p className="panel__aciklama">
          Paketlerin hedefe giderken hangi düğümlerden geçtiğini ve gecikmenin
          nerede biriktiğini gösterir. <strong>Yol değiştirilmez</strong>;
          yönlendirme tablosuna dokunmak yanlış gittiğinde bağlantını
          kesebilirdi. Buradaki bilgi, servis sağlayıcınla konuşurken işine
          yarar.
        </p>

        <div className="satir__eylemler">
          <input
            className="text-input"
            value={hedef}
            onChange={(e) => setHedef(e.target.value)}
            placeholder="1.1.1.1 veya sunucu adresi"
            aria-label="Yol testi hedefi"
            style={{ flex: '1 1 auto' }}
          />
          <button
            className="button"
            disabled={mesgul || hedef.trim().length === 0}
            onClick={() =>
              onIslem(async () => {
                const s = await api.yolTesti(hedef.trim());
                setYol(s);
              })
            }
          >
            Test et
          </button>
        </div>

        {yol && (
          <>
            <div className="hop-liste">
              {yol.dugumler.map((d) => (
                <div
                  key={d.atlama}
                  className={`hop${sicrama === d.atlama ? ' sicrama' : ''}`}
                >
                  <span className="hop__no">{d.atlama}</span>
                  <span className="hop__adres">{d.adres ?? '* (cevap yok)'}</span>
                  <span className="hop__cubuk">
                    <i
                      style={{
                        width:
                          d.gecikmeMs != null && yolTavani > 0
                            ? `${(d.gecikmeMs / yolTavani) * 100}%`
                            : '0%',
                      }}
                    />
                  </span>
                  <span className="hop__ms">
                    {d.gecikmeMs == null ? BOS : milisaniye(d.gecikmeMs, 0)}
                  </span>
                </div>
              ))}
            </div>
            {!yol.ulasildi && (
              <div className="serit">
                <IconBilgi />
                <span>
                  Hedefe ulaşılamadı. Bu her zaman bir sorun değil: birçok
                  sunucu ICMP'ye cevap vermiyor.
                </span>
              </div>
            )}
            {sicrama != null && (
              <div className="serit bilgi">
                <IconBilgi />
                <span>
                  Gecikme en çok <strong>{sicrama}. atlamada</strong> arttı. Bu
                  bir gözlem; o düğüm senin ağının dışında olabilir.
                </span>
              </div>
            )}
          </>
        )}
      </div>

      {/* --- Sistem ağ ayarları ------------------------------------------- */}
      <div className="panel">
        <div className="panel__baslik">
          Sistem ağ ayarları
          <div className="panel__eylemler">
            <button className="button ghost icon" onClick={yenile} aria-label="Yenile" title="Yenile">
              <IconYenile />
            </button>
          </div>
        </div>

        {!yonetici && (
          <div className="serit uyari">
            <IconUyari />
            <span>
              Bu ayarlar sistem geneli ve yönetici yetkisi ister. Muifly'ı
              yönetici olarak başlatmadan uygulanamazlar.
            </span>
          </div>
        )}

        {tcp && (
          <div className="olcum-izgara">
            <div className="olcum">
              <span className="olcum__etiket">Nagle kapalı arayüz</span>
              <span className="olcum__deger">
                {tcp.nagleKapaliArayuz}
                <span className="olcum__birim">/ {tcp.toplamArayuz}</span>
              </span>
              <span className="olcum__not">küçük paketler beklemeden gider</span>
            </div>
            <div className="olcum">
              <span className="olcum__etiket">Ağ kısıtlaması</span>
              <span className="olcum__deger bos">
                {tcp.throttlingIndex === 0xffffffff
                  ? 'kaldırıldı'
                  : tcp.throttlingIndex == null
                    ? 'Windows varsayılanı'
                    : String(tcp.throttlingIndex)}
              </span>
              <span className="olcum__not">NetworkThrottlingIndex</span>
            </div>
            <div className="olcum">
              <span className="olcum__etiket">Muifly QoS ilkesi</span>
              <span className="olcum__deger">{bizimQos.length}</span>
              <span className="olcum__not">yalnızca Muifly- ön ekli olanlar</span>
            </div>
          </div>
        )}

        <div className="yapilmayan-izgara">
          {aciklamalar.map(([ad, metin]) => (
            <div key={ad} className="yapilmayan">
              <IconBilgi />
              <div>
                <div className="yapilmayan__ad">{ad}</div>
                <div className="yapilmayan__aciklama">{metin}</div>
              </div>
            </div>
          ))}
        </div>

        <div className="satir__eylemler">
          <button
            className="button"
            disabled={mesgul || !yonetici}
            onClick={() =>
              onIslem(async () => {
                const s = await api.tcpUygula();
                yenile();
                if (s.hatalar.length > 0) {
                  onBildir('hata', s.hatalar[0]);
                } else {
                  onBildir('basari', 'Ağ ayarları uygulandı ve deftere yazıldı.');
                }
              })
            }
          >
            Ağ ayarlarını uygula
          </button>
          {bizimQos.length > 0 && (
            <button
              className="button danger"
              disabled={mesgul || !yonetici}
              onClick={() =>
                onIslem(async () => {
                  const adet = await api.qosKaldir();
                  yenile();
                  onBildir('basari', `${adet} QoS ilkesi kaldırıldı.`);
                })
              }
            >
              <IconGeriAl />
              QoS ilkelerini kaldır
            </button>
          )}
        </div>

        <p className="field-hint">
          Uygulanan her ayarın önceki değeri geri alma defterine yazılır. Durum
          ekranındaki "Varsayılana dön" hepsini geri alır.
        </p>
      </div>
    </>
  );
}
