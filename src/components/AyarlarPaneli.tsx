/**
 * Ayarlar.
 *
 * Her ayarın yanında **ne yaptığı ve neden varsayılanının o olduğu** yazıyor.
 * Bir performans aracında "gelişmiş ayar" diye açıklamasız anahtarlar
 * bırakmak, kullanıcının anlamadan bir şeyi açmasına yol açıyor — sonra da
 * bir sorun çıkınca sebebi bulunamıyor.
 *
 * Ayarlar dört panele bölündü (Davranış / Ölçüm / Uygulama / Sistem) ve her
 * panelin başında ne işe yaradığını söyleyen bir cümle var. Tek uzun listede
 * "gecikme hedefi" ile "Windows ile başlat" yan yana duruyordu; ikisi aynı
 * soruya cevap vermiyor.
 */

import { useEffect, useState } from 'react';

import * as api from '../lib/api';
import type { Ayarlar, Durum, Kisitlar } from '../lib/types';
import { IconBilgi, IconKalkan } from './Icons';
import { LisanslarDiyalogu } from './LisanslarDiyalogu';

interface Props {
  ayarlar: Ayarlar;
  durum: Durum;
  kisitlar: Kisitlar;
  surum: string;
  onDegistir: (a: Ayarlar) => void;
  onOtomatikBaslatma: (acik: boolean) => void;
}

function Anahtar({
  ad,
  aciklama,
  acik,
  devreDisi,
  onDegistir,
}: {
  ad: string;
  aciklama: string;
  acik: boolean;
  devreDisi?: boolean;
  onDegistir: () => void;
}) {
  return (
    <div className="field row">
      <div>
        <div className="satir__ad">{ad}</div>
        <div className="field-hint">{aciklama}</div>
      </div>
      <button
        className="switch"
        role="switch"
        aria-checked={acik}
        aria-label={ad}
        disabled={devreDisi}
        onClick={onDegistir}
      />
    </div>
  );
}

/** Sayı alanı — birimi etikette değil alanın yanında. */
function Sayi({
  ad,
  aciklama,
  birim,
  deger,
  enAz,
  enCok,
  devreDisi,
  onDegistir,
}: {
  ad: string;
  aciklama: string;
  birim: string;
  deger: number;
  enAz: number;
  enCok: number;
  devreDisi?: boolean;
  onDegistir: (d: number) => void;
}) {
  return (
    <div className="field row">
      <div>
        <div className="satir__ad">{ad}</div>
        <div className="field-hint">{aciklama}</div>
      </div>
      <span className="sayi-alan">
        <input
          className="text-input"
          type="number"
          min={enAz}
          max={enCok}
          value={deger}
          disabled={devreDisi}
          aria-label={ad}
          onChange={(e) => onDegistir(Number(e.target.value))}
        />
        <span>{birim}</span>
      </span>
    </div>
  );
}

export function AyarlarPaneli({
  ayarlar,
  durum,
  kisitlar,
  surum,
  onDegistir,
  onOtomatikBaslatma,
}: Props) {
  const [baslatmaKomutu, setBaslatmaKomutu] = useState<string | null>(null);
  const [lisanslarAcik, setLisanslarAcik] = useState(false);

  useEffect(() => {
    api.otomatikBaslatmaKomutu().then(setBaslatmaKomutu).catch(() => setBaslatmaKomutu(null));
  }, [durum.otomatikBaslatma]);

  return (
    <>
      <div className="panel">
        <div className="panel__baslik">Davranış</div>
        <p className="panel__aciklama">
          Muifly'ın kendiliğinden ne yapacağı. Varsayılanların hepsi
          "kendiliğinden bir şey yapma" tarafında.
        </p>

        <div className="ayar-grup">
          <Anahtar
            ad="Oyun algılanınca profili kendiliğinden uygula"
            aciklama="Kapalıyken optimizasyonu sen başlatırsın. Varsayılan kapalı: program sen bakmadan sistemini değiştirmemeli."
            acik={ayarlar.otomatikUygula}
            onDegistir={() => onDegistir({ ...ayarlar, otomatikUygula: !ayarlar.otomatikUygula })}
          />

          <Anahtar
            ad="Mod değişince bildirim göster"
            aciklama="Oyun algılandığında ve oyundan çıkışta kısa bir masaüstü bildirimi. Varsayılan kapalı: pencere kapalıyken ne olduğunu görmek işe yarıyor ama her Alt+Tab'da bildirim gürültü oluyor."
            acik={ayarlar.modBildirimi}
            onDegistir={() => onDegistir({ ...ayarlar, modBildirimi: !ayarlar.modBildirimi })}
          />

          <Anahtar
            ad="Rekabetçi mod"
            aciklama="Açıkken tüm oyunlar rekabetçi sayılır: kare üretimi ve agresif ölçekleme hiçbir profilde açılmaz."
            acik={ayarlar.rekabetciMod}
            onDegistir={() => onDegistir({ ...ayarlar, rekabetciMod: !ayarlar.rekabetciMod })}
          />

          <Sayi
            ad="Oyundan çıkınca geri alma gecikmesi"
            aciklama="Alt+Tab yaptığında optimizasyonun hemen kalkmaması için tampon; her sekmede kalkıp geri gelmesi hem gereksiz hem gürültülü."
            birim="sn"
            deger={ayarlar.oyunCikisGecikmesiSn}
            enAz={0}
            enCok={300}
            onDegistir={(d) =>
              onDegistir({ ...ayarlar, oyunCikisGecikmesiSn: Number.isFinite(d) ? d : 0 })
            }
          />
        </div>
      </div>

      <div className="panel">
        <div className="panel__baslik">Ölçüm</div>
        <p className="panel__aciklama">
          Muifly ağa yalnızca buradaki ölçümler için çıkar. Gecikme ölçümü
          kapalıyken tek bir ICMP paketi bile gönderilmez.
        </p>

        <div className="ayar-grup">
          <Anahtar
            ad="Gecikme ölçümü"
            aciklama="Kapalıyken hiçbir ICMP paketi gönderilmez; Durum ekranındaki gecikme ve jitter kutuları boş kalır."
            acik={ayarlar.gecikmeOlcumu}
            onDegistir={() => onDegistir({ ...ayarlar, gecikmeOlcumu: !ayarlar.gecikmeOlcumu })}
          />

          <div className="field row">
            <div>
              <div className="satir__ad">Gecikme ölçüm hedefi</div>
              <div className="field-hint">
                Varsayılan 1.1.1.1 (anycast): ölçülen şey bağlantının
                kararlılığı, belirli bir sunucuya olan mesafe değil. Oyun
                sunucunun adresini biliyorsan onu yazabilirsin.
              </div>
            </div>
            <input
              className="text-input adres-alan"
              value={ayarlar.gecikmeHedefi}
              aria-label="Gecikme ölçüm hedefi"
              onChange={(e) => onDegistir({ ...ayarlar, gecikmeHedefi: e.target.value })}
              disabled={!ayarlar.gecikmeOlcumu}
            />
          </div>

          <Sayi
            ad="Ölçüm aralığı"
            aciklama="Her ölçüm bir ICMP paketi demek; sıklaştırmanın bir maliyeti var."
            birim="sn"
            deger={ayarlar.olcumAraligiSn}
            enAz={1}
            enCok={60}
            devreDisi={!ayarlar.gecikmeOlcumu}
            onDegistir={(d) =>
              onDegistir({ ...ayarlar, olcumAraligiSn: Number.isFinite(d) && d > 0 ? d : 1 })
            }
          />
        </div>
      </div>

      <div className="panel">
        <div className="panel__baslik">Uygulama</div>

        <div className="ayar-grup">
          <Anahtar
            ad="Windows ile başlat"
            aciklama={
              kisitlar.otomatikBaslatma
                ? 'Kullanıcı kaydına yazılır, sistem geneline değil — yönetici yetkisi gerekmiyor. Tepside açılır, pencere gelmez.'
                : 'Demo sürümde kapalı. Kayda hiçbir şey yazılmıyor.'
            }
            acik={durum.otomatikBaslatma}
            // Açık kalmışsa (tam sürümden gelen kayıt) kapatmak serbest:
            // geri alma hiçbir sürümde kilitlenmiyor.
            devreDisi={!kisitlar.otomatikBaslatma && !durum.otomatikBaslatma}
            onDegistir={() => onOtomatikBaslatma(!durum.otomatikBaslatma)}
          />

          <Anahtar
            ad="Kapatınca tepsiye in"
            aciklama="Pencere kapatıldığında program arka planda kalır ve mod izlemeye devam eder."
            acik={ayarlar.tepsiyeKucult}
            onDegistir={() => onDegistir({ ...ayarlar, tepsiyeKucult: !ayarlar.tepsiyeKucult })}
          />

          <div className="field row">
            <div>
              <div className="satir__ad">Tema</div>
              <div className="field-hint">
                Üst çubuktaki düğmeyle de değiştirilebilir. Seçim ayarlara
                yazılıyor, bir sonraki açılışta korunuyor.
              </div>
            </div>
            <div className="segment" role="group" aria-label="Tema">
              <button
                type="button"
                aria-pressed={ayarlar.tema === 'dark'}
                onClick={() => onDegistir({ ...ayarlar, tema: 'dark' })}
              >
                Koyu
              </button>
              <button
                type="button"
                aria-pressed={ayarlar.tema === 'light'}
                onClick={() => onDegistir({ ...ayarlar, tema: 'light' })}
              >
                Açık
              </button>
            </div>
          </div>
        </div>

        {baslatmaKomutu && (
          <div className="serit bilgi">
            <IconBilgi />
            <span>
              Kayda yazılan komut: <code className="selectable">{baslatmaKomutu}</code>
            </span>
          </div>
        )}
      </div>

      <div className="panel">
        <div className="panel__baslik">Sistem</div>
        <p className="panel__aciklama">
          Muifly'ın bu makinede ne gördüğü. Bir şey "kullanılamıyor" yazıyorsa
          o özellik sessizce atlanır, program yine de çalışır.
        </p>

        <div>
          <div className="kv">
            <span className="kv__ad">Sürüm</span>
            <span className="kv__deger">
              {surum}
              {kisitlar.demo && ' · demo'}
            </span>
          </div>
          <div className="kv">
            <span className="kv__ad">Mantıksal çekirdek</span>
            <span className="kv__deger">{durum.mantiksalCekirdek || '—'}</span>
          </div>
          <div className="kv">
            <span className="kv__ad">Hibrit CPU</span>
            <span className="kv__deger">{durum.cpuHibrit ? 'evet' : 'hayır'}</span>
          </div>
          <div className="kv">
            <span className="kv__ad">Aktif güç planı</span>
            <span className="kv__deger">{durum.gucPlani ?? '—'}</span>
          </div>
          <div className="kv">
            <span className="kv__ad">Süreç dondurma</span>
            <span className="kv__deger">
              {durum.dondurmaDestegi ? 'kullanılabilir' : 'kullanılamıyor'}
            </span>
          </div>
          <div className="kv">
            <span className="kv__ad">Yönetici yetkisi</span>
            <span className="kv__deger">{durum.yonetici ? 'var' : 'yok'}</span>
          </div>
        </div>

        <div className="serit bilgi">
          <IconKalkan />
          <span>
            Muifly telemetri toplamaz: kullanım istatistiği, çökme raporu ya da
            analytics gönderilmez. Ağa yalnızca senin başlattığın ölçümler için
            çıkılır ve her çıkış günlüğe yazılır.
          </span>
        </div>
      </div>

      {/*
        Üçüncü taraf bildirimleri lisans sözleşmesinin madde 8'inde vaat
        ediliyor; demo ya da tam sürüm ayrımı yapmadan her ikilide açık.
        Dağıtılan bileşenin lisansı, hangi sürümü kullandığına bağlı değil.
      */}
      <div className="panel">
        <div className="panel__baslik">Yasal</div>
        <div className="field row">
          <div>
            <div className="satir__ad">Üçüncü taraf lisanslar</div>
            <div className="field-hint">
              Muifly'ın birlikte dağıttığı kütüphaneler, paketler ve yazı tipi —
              sürümleri, lisansları ve lisans metinleri.
            </div>
          </div>
          <button className="button" onClick={() => setLisanslarAcik(true)}>
            Göster
          </button>
        </div>
      </div>

      {lisanslarAcik && <LisanslarDiyalogu onKapat={() => setLisanslarAcik(false)} />}
    </>
  );
}
