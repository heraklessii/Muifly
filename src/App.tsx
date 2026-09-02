/**
 * Uygulama kabuğu: kenar çubuğu, başlık çubuğu, içerik, durum çubuğu.
 *
 * Durum tek yerde tutuluyor ve panellere prop olarak iniyor. Bir durum
 * kütüphanesi eklenmedi: altı ekran ve tek bir veri kaynağı için ekstra bir
 * soyutlama katmanı, okumayı kolaylaştırmıyor.
 *
 * Backend'den gelen olaylar (`muifly://durum`, `muifly://gunluk`,
 * `muifly://ornek`) tek bir `useEffect` içinde dinleniyor; arayüz kendiliğinden
 * yoklama (polling) yapmıyor — arka plan döngüsü zaten haber veriyor.
 *
 * Yerleşimin iki kararı:
 *
 * 1. **Mod her ekranda görünür.** Kenar çubuğunun dibindeki canlı kart, hangi
 *    sekmede olursan ol modu, öndeki süreci ve son ölçümleri gösteriyor.
 *    Kullanıcı "şu an ne oluyor" sorusu için Durum sekmesine dönmek zorunda
 *    kalmamalı.
 * 2. **Mod rengi kabuktan yayılıyor.** `data-mod` kabukta duruyor, `--mod`
 *    jetonunu oradan alan her parça (nabız, kahraman kartı, durum çubuğu)
 *    aynı anda değişiyor.
 */

import { useCallback, useEffect, useMemo, useState } from 'react';

import { AgPaneli } from './components/AgPaneli';
import { AyarlarPaneli } from './components/AyarlarPaneli';
import { CeviriPaneli } from './components/CeviriPaneli';
import { DurumPaneli } from './components/DurumPaneli';
import { GecmisPaneli } from './components/GecmisPaneli';
import { GunlukPaneli } from './components/GunlukPaneli';
import {
  IconAg,
  IconAy,
  IconAyarlar,
  IconCeviri,
  IconDurum,
  IconGecmis,
  IconGunes,
  IconGunluk,
  IconMuifly,
  IconOlcekleme,
  IconProfil,
  IconUyari,
} from './components/Icons';
import { IceAktarmaDiyalogu } from './components/IceAktarmaDiyalogu';
import { KutuphaneDiyalogu } from './components/KutuphaneDiyalogu';
import { OlceklemePaneli } from './components/OlceklemePaneli';
import { ProfilDiyalogu } from './components/ProfilDiyalogu';
import { ProfilPaneli } from './components/ProfilPaneli';
import { Sparkline } from './components/Sparkline';
import { Toasts, useToasts } from './components/Toasts';
import * as api from './lib/api';
import { sayi } from './lib/format';
import { modRengi, modSureci, TAM_SURUM } from './lib/types';
import type {
  Ayarlar,
  Durum,
  GecmisOzeti,
  Karsilastirma,
  Kayit,
  Kisitlar,
  Onizleme,
  Ornek,
  OturumKaydi,
  Ozet,
  Profil,
  Satir,
  UygulamaSonucu,
} from './lib/types';

type Sekme =
  | 'durum'
  | 'profiller'
  | 'ag'
  | 'olcekleme'
  | 'ceviri'
  | 'gunluk'
  | 'gecmis'
  | 'ayarlar';

/**
 * Sekme tanımı. `alt` başlık çubuğunda okunuyor: her ekran ne olduğunu bir
 * cümleyle söylüyor, kullanıcı doğru yerde olup olmadığını denemeden bilsin.
 */
const SEKMELER: {
  id: Sekme;
  ad: string;
  alt: string;
  Ikon: typeof IconDurum;
}[] = [
  {
    id: 'durum',
    ad: 'Durum',
    alt: 'Canlı ölçüm, sistemde duran değişiklikler ve tek tıkla geri alma.',
    Ikon: IconDurum,
  },
  {
    id: 'profiller',
    ad: 'Profiller',
    alt: 'Oyun başına ayar takımları. Her biri okunabilir bir JSON dosyası.',
    Ikon: IconProfil,
  },
  {
    id: 'ag',
    ad: 'Ağ',
    alt: 'Ölçüm ve gözlem. DNS ve yönlendirme Muifly tarafından değiştirilmez.',
    Ikon: IconAg,
  },
  {
    id: 'olcekleme',
    ad: 'Ölçekleme',
    alt: 'Ekranı okuyup büyütür. Oyuna dokunmaz; eklediği gecikme ölçülüp gösterilir.',
    Ikon: IconOlcekleme,
  },
  {
    id: 'ceviri',
    ad: 'Çeviri',
    alt: 'Tuşa basınca ekrandaki yazıyı okur ve bu bilgisayarda çevirir. Hiçbir metin dışarı gitmez.',
    Ikon: IconCeviri,
  },
  {
    id: 'gunluk',
    ad: 'Günlük',
    alt: 'Muifly’ın sistemde yaptığı her şey, zaman damgası ve geri alma ile.',
    Ikon: IconGunluk,
  },
  {
    id: 'gecmis',
    ad: 'Geçmiş',
    alt: 'Biten oyun oturumları. Bu bilgisayarda duruyor, hiçbir yere gönderilmiyor.',
    Ikon: IconGecmis,
  },
  {
    id: 'ayarlar',
    ad: 'Ayarlar',
    alt: 'Davranış, ölçüm ve uygulama tercihleri — her birinin gerekçesiyle.',
    Ikon: IconAyarlar,
  },
];

/** Mod kartındaki ve ölçüm kutularındaki mini eğrilerin baktığı pencere. */
const SPARK_PENCERESI = 40;

export default function App() {
  const { toastlar, goster, dusur } = useToasts();

  const [sekme, setSekme] = useState<Sekme>('durum');
  const [durum, setDurum] = useState<Durum | null>(null);
  const [satirlar, setSatirlar] = useState<Satir[]>([]);
  const [ornekler, setOrnekler] = useState<Ornek[]>([]);
  const [ozet, setOzet] = useState<Ozet | null>(null);
  const [karsilastirma, setKarsilastirma] = useState<Karsilastirma | null>(null);
  const [profiller, setProfiller] = useState<Profil[]>([]);
  const [bekleyenler, setBekleyenler] = useState<Kayit[]>([]);
  const [gecmis, setGecmis] = useState<OturumKaydi[]>([]);
  const [gecmisOzeti, setGecmisOzeti] = useState<GecmisOzeti | null>(null);
  const [yapilmayanlar, setYapilmayanlar] = useState<[string, string][]>([]);
  const [surum, setSurum] = useState('');
  const [kisitlar, setKisitlar] = useState<Kisitlar>(TAM_SURUM);
  const [mesgul, setMesgul] = useState(false);
  const [diyalog, setDiyalog] = useState<{ profil: Profil | null; yeni: boolean } | null>(null);
  const [onizleme, setOnizleme] = useState<Onizleme | null>(null);
  /** Kütüphane penceresi açık mı? Profil penceresinden bağımsız: kullanıcı
   *  oyunu seçtikten sonra kütüphane kapanıp profil penceresi açılıyor. */
  const [kutuphane, setKutuphane] = useState(false);

  /** Backend'den türetilen her şeyi tazeler. */
  const tazele = useCallback(async () => {
    const [d, g, o, oz, k, p, b, gc, gcOz] = await Promise.all([
      api.durum(),
      api.gunluk(200),
      api.ornekler(),
      api.ozet(),
      api.karsilastirma(),
      api.profiller(),
      api.bekleyenGeriAlmalar(),
      api.gecmis(),
      api.gecmisOzeti(),
    ]);
    setDurum(d);
    setSatirlar(g);
    setOrnekler(o);
    setOzet(oz);
    setKarsilastirma(k);
    setProfiller(p);
    setBekleyenler(b);
    setGecmis(gc);
    setGecmisOzeti(gcOz);
  }, []);

  useEffect(() => {
    api.surum().then(setSurum).catch(() => setSurum(''));
    api.kisitlar().then(setKisitlar).catch(() => setKisitlar(TAM_SURUM));
    api.yapilmayanlar().then(setYapilmayanlar).catch(() => setYapilmayanlar([]));
    tazele().catch((e) => goster('hata', String(e)));
  }, [tazele, goster]);

  // Backend olayları. Abonelikler bileşen kaldırılınca bırakılıyor.
  useEffect(() => {
    const birakilacaklar: Promise<() => void>[] = [
      api.dinle<Durum>(api.OLAY_DURUM, (d) => {
        setDurum(d);
        // Mod değiştiğinde defter ve günlük de değişmiş olabilir.
        api.bekleyenGeriAlmalar().then(setBekleyenler).catch(() => {});
        api.karsilastirma().then(setKarsilastirma).catch(() => {});
      }),
      api.dinle<Satir[]>(api.OLAY_GUNLUK, () => {
        api.gunluk(200).then(setSatirlar).catch(() => {});
      }),
      api.dinle<Ornek>(api.OLAY_ORNEK, (o) => {
        // Örnekler akış halinde geliyor; tam listeyi her seferinde çekmek
        // yerine yereldeki listeye ekleniyor. Tavan backend'deki tampon
        // kapasitesiyle aynı olmak zorunda değil: grafik son pencereye bakıyor.
        setOrnekler((mevcut) => [...mevcut, o].slice(-600));
        api.ozet().then(setOzet).catch(() => {});
      }),
    ];
    return () => {
      birakilacaklar.forEach((p) => p.then((f) => f()).catch(() => {}));
    };
  }, []);

  // Tema `<html data-theme>` üzerinden; CSS'in tamamı bu jetona bağlı.
  useEffect(() => {
    if (durum) document.documentElement.dataset.theme = durum.ayarlar.tema;
  }, [durum?.ayarlar.tema]);

  // Görünen sekmeler: demoda ağ modülü yok, sekme "kapalı" diye gösterilmek
  // yerine hiç gösterilmiyor (karar #20).
  const gorunen = useMemo(
    () => SEKMELER.filter(({ id }) => id !== 'ag' || kisitlar.agModulu),
    [kisitlar.agModulu],
  );

  /**
   * Ctrl+1..n ile sekme değiştirme.
   *
   * Numaralar GÖRÜNEN sekmelere göre: demoda ağ sekmesi yokken Ctrl+3 Günlük'ü
   * açıyor. Kısayolun ekrandaki sıralamayı takip etmesi, gizli bir sekmeye
   * götürmesinden iyi.
   */
  useEffect(() => {
    const f = (e: KeyboardEvent) => {
      if (!e.ctrlKey || e.altKey || e.shiftKey) return;
      // Diyalog açıkken arkadaki sekmeyi değiştirmek, kaydedilmemiş bir formu
      // görünmez hale getirirdi.
      if (diyalog || onizleme || kutuphane) return;
      const no = Number(e.key);
      if (!Number.isInteger(no) || no < 1 || no > gorunen.length) return;
      e.preventDefault();
      setSekme(gorunen[no - 1].id);
    };
    window.addEventListener('keydown', f);
    return () => window.removeEventListener('keydown', f);
  }, [gorunen, diyalog, onizleme, kutuphane]);

  /** Uzun süren bir işlemi çalıştırır, hatayı bildirir, sonunda tazeler. */
  const islem = useCallback(
    async (calis: () => Promise<void>) => {
      setMesgul(true);
      try {
        await calis();
      } catch (e) {
        goster('hata', String(e));
      } finally {
        setMesgul(false);
        tazele().catch(() => {});
      }
    },
    [goster, tazele],
  );

  /** Uygulama sonucunu kullanıcıya özetler. */
  const sonucuBildir = useCallback(
    (s: UygulamaSonucu) => {
      if (s.hatalar.length > 0) {
        goster('hata', s.hatalar[0]);
      }
      if (s.uygulanan.length > 0) {
        goster(
          'basari',
          `${s.uygulanan.length} değişiklik uygulandı. Ayrıntılar günlükte.`,
        );
      } else if (s.hatalar.length === 0) {
        goster('bilgi', 'Uygulanacak bir değişiklik yoktu.');
      }
    },
    [goster],
  );

  /**
   * İçe aktarmanın ilk adımı: dosyayı seç, ne yapacağını göster.
   *
   * Kaydetme burada değil `IceAktarmaDiyalogu`nun onayında: kullanıcı içeriği
   * görmeden diske bir şey yazılmıyor (`docs/PROFILES.md` güvenlik notu).
   */
  const iceAktarmayiBaslat = useCallback(async () => {
    try {
      const yol = await api.profilDosyasiSec();
      if (!yol) return;
      setOnizleme(await api.profilOnizle(yol));
    } catch (e) {
      goster('hata', String(e));
    }
  }, [goster]);

  const disaAktar = useCallback(
    async (p: Profil) => {
      try {
        const yol = await api.profilDosyasiHedefi(`${p.profile_id}.json`);
        if (!yol) return;
        await api.profilDisaAktar(p.profile_id, yol);
        goster('basari', `'${p.display_name}' dosyaya kaydedildi.`);
      } catch (e) {
        goster('hata', String(e));
      }
    },
    [goster],
  );

  /**
   * Geçmiş raporunu dosyaya yazar.
   *
   * Yol kullanıcının kaydetme penceresinden geliyor; iptal ederse hiçbir şey
   * yazılmıyor. Dosya yalnızca diske yazılıyor, hiçbir yere gönderilmiyor.
   */
  const gecmisiDisaAktar = useCallback(async () => {
    try {
      const yol = await api.metinDosyasiHedefi('muifly-oturum-gecmisi.txt');
      if (!yol) return;
      await api.gecmisDisaAktar(yol);
      goster('basari', 'Oturum geçmişi dosyaya kaydedildi.');
    } catch (e) {
      goster('hata', String(e));
    }
  }, [goster]);

  const ayarDegistir = useCallback(
    (a: Ayarlar) => {
      // İyimser güncelleme: anahtarın hemen tepki vermesi gerekiyor, aksi
      // halde tıklama kaybolmuş gibi hissettiriyor.
      setDurum((d) => (d ? { ...d, ayarlar: a } : d));
      api.ayarlariYaz(a).catch((e) => {
        goster('hata', String(e));
        tazele().catch(() => {});
      });
    },
    [goster, tazele],
  );

  if (!durum || !ozet) {
    return (
      <div className="acilis">
        <IconMuifly />
        <span>Muifly başlatılıyor…</span>
      </div>
    );
  }

  const ondeki = modSureci(durum.mod);
  const renk = modRengi(durum.mod);
  const sayfa = SEKMELER.find((s) => s.id === sekme)!;
  const sonOrnekler = ornekler.slice(-SPARK_PENCERESI);

  return (
    <div className="shell" data-mod={renk}>
      {/* Uzun süren işlem göstergesi: tek yerde, her düğmede ayrı ayrı değil. */}
      {mesgul && <div className="ilerleme" role="progressbar" aria-label="İşlem sürüyor" />}

      <aside className="rail">
        <div className="rail__marka">
          <IconMuifly />
          <span className="brand">
            <strong>Muifly</strong>
            <span>{surum ? `v${surum}${kisitlar.demo ? ' · demo' : ''}` : 'yükleniyor'}</span>
          </span>
        </div>

        <nav className="rail__nav" aria-label="Bölümler">
          {gorunen.map(({ id, ad, Ikon }, i) => {
            const rozet = id === 'gunluk' && durum.bekleyenGeriAlma > 0;
            return (
              <button
                key={id}
                className={sekme === id ? 'is-active' : ''}
                aria-current={sekme === id ? 'page' : undefined}
                // Rozet sayısı erişilebilir ada karışmasın diye ayrıca
                // yazılıyor; ekran okuyucu "Günlük 3" değil ne olduğunu duysun.
                aria-label={
                  rozet
                    ? `${ad}, ${durum.bekleyenGeriAlma} geri alınabilir değişiklik`
                    : undefined
                }
                onClick={() => setSekme(id)}
              >
                <Ikon />
                <span>{ad}</span>
                {rozet ? (
                  <span className="rail__rozet" aria-hidden="true">
                    {durum.bekleyenGeriAlma}
                  </span>
                ) : (
                  <span className="kbd rail__kbd" aria-hidden="true">
                    Ctrl {i + 1}
                  </span>
                )}
              </button>
            );
          })}
        </nav>

        <span className="rail__bosluk" />

        {/* Her ekranda görünen canlı mod kartı. */}
        <div className="mod-karti">
          <div className="mod-karti__ust">
            <span className="nabiz" />
            <span className="mod-karti__ad">{durum.modAdi}</span>
          </div>
          {ondeki && <div className="mod-karti__surec">{ondeki}</div>}
          <Sparkline
            degerler={sonOrnekler.map((o) => o.cpu)}
            tavan={100}
            renk="var(--mod)"
            className="olcum__spark"
          />
          <div className="mod-karti__olcum">
            <span>
              CPU <b>{sayi(ozet.cpuOrt, 0)}%</b>
            </span>
            <span>
              RAM <b>{sayi(ozet.bellekOrt, 0)}%</b>
            </span>
          </div>
        </div>
      </aside>

      <div className="ana">
        <header className="baslik-cubugu">
          <div>
            <h1>{sayfa.ad}</h1>
            <p>{sayfa.alt}</p>
          </div>
          <div className="baslik-cubugu__eylemler">
            <button
              className="button ghost icon"
              aria-label="Temayı değiştir"
              title={durum.ayarlar.tema === 'dark' ? 'Açık temaya geç' : 'Koyu temaya geç'}
              onClick={() =>
                ayarDegistir({
                  ...durum.ayarlar,
                  tema: durum.ayarlar.tema === 'dark' ? 'light' : 'dark',
                })
              }
            >
              {durum.ayarlar.tema === 'dark' ? <IconGunes /> : <IconAy />}
            </button>
          </div>
        </header>

        <main className="icerik">
          {sekme === 'durum' && (
            <DurumPaneli
              durum={durum}
              ornekler={ornekler}
              ozet={ozet}
              karsilastirma={karsilastirma}
              yapilmayanlar={yapilmayanlar}
              mesgul={mesgul}
              onIslem={islem}
              onUygula={() =>
                islem(async () => {
                  const s = await api.ondekineUygula();
                  sonucuBildir(s);
                })
              }
              onGunluge={() => setSekme('gunluk')}
              onOturumuKapat={() =>
                islem(async () => {
                  const adet = await api.oturumuKapat();
                  goster('basari', `${adet} değişiklik geri alındı.`);
                })
              }
              onHepsiniGeriAl={() =>
                islem(async () => {
                  const s = await api.hepsiniGeriAl();
                  sonucuBildir(s);
                })
              }
            />
          )}

          {sekme === 'profiller' && (
            <ProfilPaneli
              profiller={profiller}
              durum={durum}
              kisitlar={kisitlar}
              mesgul={mesgul}
              onYeni={() => setDiyalog({ profil: null, yeni: true })}
              onKutuphane={() => setKutuphane(true)}
              onDuzenle={(p) => setDiyalog({ profil: p, yeni: false })}
              onSil={(p) =>
                islem(async () => {
                  await api.profilSil(p.profile_id);
                  goster('basari', `'${p.display_name}' silindi.`);
                })
              }
              onUygula={(p) =>
                islem(async () => {
                  const s = await api.profilUygula(p.profile_id);
                  sonucuBildir(s);
                })
              }
              onIceAktar={iceAktarmayiBaslat}
              onDisaAktar={disaAktar}
            />
          )}

          {sekme === 'ag' && kisitlar.agModulu && (
            <AgPaneli
              yonetici={durum.yonetici}
              gecikmeHedefi={durum.ayarlar.gecikmeHedefi}
              mesgul={mesgul}
              onIslem={islem}
              onBildir={goster}
            />
          )}

          {sekme === 'olcekleme' && (
            <OlceklemePaneli
              rekabetciMod={durum.ayarlar.rekabetciMod}
              olceklemeEkrani={durum.ayarlar.olceklemeEkrani}
              mesgul={mesgul}
              onIslem={islem}
              onBildir={goster}
              onEkranDegistir={(indeks) =>
                ayarDegistir({ ...durum.ayarlar, olceklemeEkrani: indeks })
              }
              onAyarlara={() => setSekme('ayarlar')}
            />
          )}

          {sekme === 'ceviri' && (
            <CeviriPaneli
              ceviriEkrani={durum.ayarlar.ceviriEkrani}
              overlayAcik={durum.ayarlar.ceviriOverlay}
              profiller={profiller}
              mesgul={mesgul}
              onIslem={islem}
              onBildir={goster}
              onEkranDegistir={(indeks) =>
                ayarDegistir({ ...durum.ayarlar, ceviriEkrani: indeks })
              }
              onOverlayDegistir={(acik) =>
                ayarDegistir({ ...durum.ayarlar, ceviriOverlay: acik })
              }
              onAyarlara={() => setSekme('ayarlar')}
            />
          )}

          {sekme === 'gunluk' && (
            <GunlukPaneli
              satirlar={satirlar}
              bekleyenler={bekleyenler}
              mesgul={mesgul}
              onGeriAl={(id) =>
                islem(async () => {
                  await api.geriAl(id);
                })
              }
              onTemizle={() =>
                islem(async () => {
                  await api.gunlugu_temizle();
                })
              }
            />
          )}

          {sekme === 'gecmis' && (
            <GecmisPaneli
              kayitlar={gecmis}
              ozet={gecmisOzeti}
              gecmisTut={durum.ayarlar.gecmisTut}
              mesgul={mesgul}
              onDisaAktar={gecmisiDisaAktar}
              onYenile={() => tazele().catch((e) => goster('hata', String(e)))}
              onAyarlara={() => setSekme('ayarlar')}
              onTemizle={() =>
                islem(async () => {
                  await api.gecmisiTemizle();
                  goster('basari', 'Oturum geçmişi silindi.');
                })
              }
            />
          )}

          {sekme === 'ayarlar' && (
            <AyarlarPaneli
              ayarlar={durum.ayarlar}
              durum={durum}
              kisitlar={kisitlar}
              surum={surum}
              onDegistir={ayarDegistir}
              onOtomatikBaslatma={(acik) =>
                islem(async () => {
                  await api.otomatikBaslatmaAyarla(acik);
                })
              }
            />
          )}
        </main>

        <footer className="statusbar">
          <span className="statusbar__mod">
            <span className="nabiz" />
            {durum.modAdi}
          </span>
          {ondeki && <span>Önde: {ondeki}</span>}
          <span className="bosluk" />
          {!durum.yonetici && (
            <span
              className="statusbar__uyari"
              title="Sistem geneli ağ ayarları için yönetici yetkisi gerekiyor"
            >
              <IconUyari />
              yönetici yetkisi yok
            </span>
          )}
          {durum.bekleyenGeriAlma > 0 && (
            <span>{durum.bekleyenGeriAlma} geri alınabilir değişiklik</span>
          )}
          <span>
            {durum.mantiksalCekirdek > 0 && `${durum.mantiksalCekirdek} çekirdek`}
          </span>
        </footer>
      </div>

      {diyalog && (
        <ProfilDiyalogu
          profil={diyalog.profil}
          yeniMi={diyalog.yeni}
          hibritCpu={durum.cpuHibrit}
          dondurmaDestegi={durum.dondurmaDestegi}
          onKapat={() => setDiyalog(null)}
          onKaydet={(p) => {
            setDiyalog(null);
            islem(async () => {
              const duzeltmeler = await api.profilKaydet(p);
              // Backend profili güvenli hale getirdiyse kullanıcı bunu
              // görmeli — sessiz düzeltme, şeffaflık ilkesine aykırı.
              if (duzeltmeler.length > 0) {
                goster('bilgi', duzeltmeler[0]);
              } else {
                goster('basari', 'Profil kaydedildi.');
              }
            });
          }}
        />
      )}

      {kutuphane && (
        <KutuphaneDiyalogu
          onKapat={() => setKutuphane(false)}
          onTaslak={(t) => {
            // Taslak kaydedilmiyor: profil penceresi onunla açılıyor ve
            // kaydetme kararı her zamanki yerde kalıyor.
            setKutuphane(false);
            setDiyalog({ profil: t.profil, yeni: true });
          }}
        />
      )}

      {onizleme && (
        <IceAktarmaDiyalogu
          onizleme={onizleme}
          mesgul={mesgul}
          onKapat={() => setOnizleme(null)}
          onAktar={(uzerineYaz) => {
            const gelen = onizleme.profil;
            setOnizleme(null);
            islem(async () => {
              const duzeltmeler = await api.profilIceAktar(gelen, uzerineYaz);
              if (duzeltmeler.length > 0) {
                goster('bilgi', duzeltmeler[0]);
              } else {
                goster('basari', `'${gelen.display_name}' içe aktarıldı.`);
              }
            });
          }}
        />
      )}

      <Toasts toastlar={toastlar} dusur={dusur} />
    </div>
  );
}
