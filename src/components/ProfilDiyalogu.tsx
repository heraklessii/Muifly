/**
 * Profil düzenleme diyaloğu.
 *
 * Arayüz, backend'in doğrulamasını **tekrarlamıyor** — engelliyor. Örnek:
 * `realtime` önceliği seçenek listesinde hiç yok, rekabetçi profilde kare
 * üretimi anahtarı devre dışı. Backend aynı kuralları ayrıca uyguluyor
 * (`profile_engine::schema`), çünkü profil dosyası elle de düzenlenebiliyor.
 * İki kapı da kapalı: `docs/PROFILES.md` bunu açıkça istiyor.
 */

import { useEffect, useMemo, useState } from 'react';

import * as api from '../lib/api';
import {
  GUC_PLANI_SECENEKLERI,
  ONCELIK_SECENEKLERI,
  bosProfil,
  type KatalogGirdisi,
  type Profil,
  type Surec,
} from '../lib/types';
import { IconBilgi, IconKapat } from './Icons';

interface Props {
  profil: Profil | null;
  /** `null` = yeni profil. */
  yeniMi: boolean;
  hibritCpu: boolean;
  dondurmaDestegi: boolean;
  onKapat: () => void;
  onKaydet: (p: Profil) => void;
}

/** Kimlik alanını kullanıcı yazmıyor; addan üretiliyor. */
function kimlikUret(ad: string): string {
  const temiz = ad
    .toLowerCase()
    .replace(/[ğ]/g, 'g')
    .replace(/[ü]/g, 'u')
    .replace(/[ş]/g, 's')
    .replace(/[ı]/g, 'i')
    .replace(/[ö]/g, 'o')
    .replace(/[ç]/g, 'c')
    .replace(/[^a-z0-9]+/g, '_')
    .replace(/^_+|_+$/g, '');
  return temiz || 'profil';
}

export function ProfilDiyalogu({
  profil,
  yeniMi,
  hibritCpu,
  dondurmaDestegi,
  onKapat,
  onKaydet,
}: Props) {
  const [taslak, setTaslak] = useState<Profil>(() => profil ?? bosProfil());
  const [surecler, setSurecler] = useState<Surec[]>([]);
  const [adaylar, setAdaylar] = useState<string[]>([]);
  const [taninanlar, setTaninanlar] = useState<KatalogGirdisi[]>([]);
  const [tumSurecler, setTumSurecler] = useState(false);
  const [exeMetni, setExeMetni] = useState(() => (profil?.executable_names ?? []).join(', '));

  useEffect(() => {
    api.surecler().then(setSurecler).catch(() => setSurecler([]));
    api.dondurmaAdaylari().then(setAdaylar).catch(() => setAdaylar([]));
    // Çalışan süreçlerden katalogda tanınanlar: kullanıcı oyunu açıksa exe
    // adını yazmak zorunda kalmasın.
    api.taninanSurecler().then(setTaninanlar).catch(() => setTaninanlar([]));
  }, []);

  /** Alanda zaten yazılı olan exe adları. */
  const secilenExeler = useMemo(
    () =>
      exeMetni
        .split(',')
        .map((s) => s.trim().toLowerCase())
        .filter(Boolean),
    [exeMetni],
  );

  /** Bir exe adını alana ekler ya da çıkarır. */
  function exeyiDegistir(ad: string) {
    const normal = ad.trim().toLowerCase();
    if (!normal) return;
    const yeni = secilenExeler.includes(normal)
      ? secilenExeler.filter((x) => x !== normal)
      : [...secilenExeler, normal];
    setExeMetni(yeni.join(', '));
  }

  /** Dosya penceresinden bir .exe seçtirir — kütüphanenin bulamadığı oyunlar için. */
  async function dosyadanSec() {
    const yol = await api.exeDosyasiSec();
    if (!yol) return;
    const ad = yol.split(/[\\/]/).pop() ?? '';
    exeyiDegistir(ad);
  }

  // Escape ile kapanma: diyaloglarda beklenen davranış.
  useEffect(() => {
    const f = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onKapat();
    };
    window.addEventListener('keydown', f);
    return () => window.removeEventListener('keydown', f);
  }, [onKapat]);

  /**
   * Dondurma listesinde gösterilecek adaylar: önerilenler + o an çalışan
   * süreçler + profilde zaten seçili olanlar.
   *
   * Seçili olanların listede kalması şart: kullanıcı Discord'u kapattıysa
   * profildeki seçim ekrandan kaybolmamalı, yoksa kaydetmek onu sessizce
   * siler.
   */
  const dondurmaListesi = useMemo(() => {
    const kume = new Set<string>([
      ...adaylar,
      ...taslak.system.suspend_process_list,
      ...surecler.map((s) => s.ad),
    ]);
    return [...kume].sort();
  }, [adaylar, surecler, taslak.system.suspend_process_list]);

  const gecerli =
    taslak.display_name.trim().length > 0 && exeMetni.trim().length > 0;

  function kaydet() {
    const exeler = exeMetni
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean);
    onKaydet({
      ...taslak,
      profile_id: taslak.profile_id || kimlikUret(taslak.display_name),
      executable_names: exeler,
    });
  }

  function dondurmayiDegistir(ad: string, secili: boolean) {
    setTaslak((t) => ({
      ...t,
      system: {
        ...t.system,
        suspend_process_list: secili
          ? [...t.system.suspend_process_list, ad]
          : t.system.suspend_process_list.filter((x) => x !== ad),
      },
    }));
  }

  return (
    <div className="perde" onMouseDown={(e) => e.target === e.currentTarget && onKapat()}>
      <div className="diyalog" role="dialog" aria-modal="true">
        <div className="diyalog__baslik">
          {yeniMi ? 'Yeni profil' : taslak.display_name || 'Profil'}
          <button className="button ghost icon" onClick={onKapat} aria-label="Kapat">
            <IconKapat />
          </button>
        </div>

        <div className="diyalog__govde">
          <label className="field">
            <span>Oyun adı</span>
            <input
              className="text-input"
              value={taslak.display_name}
              onChange={(e) => setTaslak({ ...taslak, display_name: e.target.value })}
              placeholder="Örnek: Counter-Strike 2"
              autoFocus
            />
          </label>

          <div className="field">
            <label className="field">
              <span>Çalıştırılabilir dosya adları</span>
              <input
                className="text-input"
                value={exeMetni}
                onChange={(e) => setExeMetni(e.target.value)}
                placeholder="cs2.exe"
              />
            </label>
            <span className="field-hint">
              Virgülle ayır. Büyük/küçük harf önemsiz, tam yol da yazabilirsin —
              yalnızca dosya adı kullanılır. Aşağıdakilere tıklayarak da
              ekleyebilirsin.
            </span>

            {/* Tanınan oyunlar önce: kullanıcı oyunu açıksa aradığı satır
                genelde bu ilk grupta. */}
            {taninanlar.length > 0 && (
              <div className="cip-liste">
                {taninanlar.map((g) => (
                  <button
                    key={g.exe}
                    type="button"
                    className={`cip${secilenExeler.includes(g.exe) ? ' secili' : ''}`}
                    onClick={() => exeyiDegistir(g.exe)}
                    title={`${g.exe} — şu an çalışıyor`}
                  >
                    {g.ad}
                  </button>
                ))}
              </div>
            )}

            <div className="cip-liste">
              <button type="button" className="cip" onClick={dosyadanSec}>
                .exe dosyası seç…
              </button>
              <button
                type="button"
                className="cip"
                onClick={() => setTumSurecler((a) => !a)}
                aria-expanded={tumSurecler}
              >
                {tumSurecler ? 'Çalışan uygulamaları gizle' : 'Çalışan uygulamalardan seç…'}
              </button>
            </div>

            {tumSurecler && (
              <div className="secim-liste">
                {surecler.map((s) => (
                  <label key={s.ad} className="secim">
                    <input
                      type="checkbox"
                      checked={secilenExeler.includes(s.ad)}
                      onChange={() => exeyiDegistir(s.ad)}
                    />
                    {s.ad}
                  </label>
                ))}
              </div>
            )}
          </div>

          <div className="field row">
            <div>
              <div className="satir__ad">Rekabetçi mod</div>
              <div className="field-hint">
                Gecikmeye duyarlı oyunlar için. Kare üretimi ve agresif ölçekleme
                bu profilde hiçbir zaman açılmaz.
              </div>
            </div>
            <button
              className="switch"
              role="switch"
              aria-checked={taslak.competitive}
              aria-label="Rekabetçi mod"
              onClick={() => setTaslak({ ...taslak, competitive: !taslak.competitive })}
            />
          </div>

          <label className="field">
            <span>Oyun süreç önceliği</span>
            <select
              className="select"
              value={taslak.system.priority_class}
              onChange={(e) =>
                setTaslak({
                  ...taslak,
                  system: { ...taslak.system, priority_class: e.target.value },
                })
              }
            >
              {ONCELIK_SECENEKLERI.map((o) => (
                <option key={o.deger} value={o.deger}>
                  {o.etiket}
                </option>
              ))}
            </select>
            <span className="field-hint">
              Gerçek zamanlı öncelik listede yok: sistem servislerini aç bırakıp
              makineyi kilitleyebiliyor.
            </span>
          </label>

          <label className="field">
            <span>Güç planı</span>
            <select
              className="select"
              value={taslak.system.power_plan ?? ''}
              onChange={(e) =>
                setTaslak({
                  ...taslak,
                  system: {
                    ...taslak.system,
                    power_plan: e.target.value === '' ? null : e.target.value,
                  },
                })
              }
            >
              {GUC_PLANI_SECENEKLERI.map((o) => (
                <option key={o.etiket} value={o.deger ?? ''}>
                  {o.etiket}
                </option>
              ))}
            </select>
            <span className="field-hint">
              Oyun kapanınca önceki planına dönülür.
            </span>
          </label>

          {hibritCpu && (
            <div className="field row">
              <div>
                <div className="satir__ad">Performans çekirdeklerine sabitle</div>
                <div className="field-hint">
                  Hibrit CPU'larda oyunu P-core'lara bağlar. Her oyunda iyi
                  sonuç vermiyor; kapalı bırakmak güvenli.
                </div>
              </div>
              <button
                className="switch"
                role="switch"
                aria-checked={taslak.system.cpu_affinity === 'sadece_p_core'}
                aria-label="Performans çekirdeklerine sabitle"
                onClick={() =>
                  setTaslak({
                    ...taslak,
                    system: {
                      ...taslak.system,
                      cpu_affinity:
                        taslak.system.cpu_affinity === 'sadece_p_core'
                          ? 'dokunma'
                          : 'sadece_p_core',
                    },
                  })
                }
              />
            </div>
          )}

          <div className="field">
            <span>Oyun sırasında dondurulacak uygulamalar</span>
            <span className="field-hint">
              Dondurulan uygulama kapanmaz, yalnızca durur — oyundan çıkınca
              kaldığı yerden devam eder. Windows sistem süreçleri listede yok.
            </span>
            {!dondurmaDestegi && (
              <div className="serit uyari">
                <IconBilgi />
                <span>
                  Bu Windows sürümünde dondurma kullanılamıyor; seçimler
                  kaydedilir ama uygulanmaz.
                </span>
              </div>
            )}
            <div className="secim-liste">
              {dondurmaListesi.map((ad) => {
                const kendisi = exeMetni.toLowerCase().includes(ad);
                return (
                  <label
                    key={ad}
                    className={`secim${kendisi ? ' engelli' : ''}`}
                    title={kendisi ? 'Oyunun kendisi dondurulamaz' : undefined}
                  >
                    <input
                      type="checkbox"
                      disabled={kendisi}
                      checked={taslak.system.suspend_process_list.includes(ad)}
                      onChange={(e) => dondurmayiDegistir(ad, e.target.checked)}
                    />
                    {ad}
                  </label>
                );
              })}
            </div>
          </div>

          <div className="field row">
            <div>
              <div className="satir__ad">QoS önceliklendirme</div>
              <div className="field-hint">
                Oyunun paketlerine öncelik işareti koyar. Yalnızca yerel ağdaki
                cihazlar bu işareti dikkate alırsa etkili olur.
              </div>
            </div>
            <button
              className="switch"
              role="switch"
              aria-checked={taslak.network.qos_priority}
              aria-label="QoS önceliklendirme"
              onClick={() =>
                setTaslak({
                  ...taslak,
                  network: { ...taslak.network, qos_priority: !taslak.network.qos_priority },
                })
              }
            />
          </div>

          <div className="field row">
            <div>
              <div className="satir__ad">Nagle birleştirmesini kapat</div>
              <div className="field-hint">
                Küçük paketler beklemeden gönderilir. Sistem geneli bir ayar ve
                yönetici yetkisi ister; geri alınabilir.
              </div>
            </div>
            <button
              className="switch"
              role="switch"
              aria-checked={taslak.network.tcp_nodelay}
              aria-label="Nagle birleştirmesini kapat"
              onClick={() =>
                setTaslak({
                  ...taslak,
                  network: { ...taslak.network, tcp_nodelay: !taslak.network.tcp_nodelay },
                })
              }
            />
          </div>
        </div>

        <div className="diyalog__alt">
          <button className="button ghost" onClick={onKapat}>
            Vazgeç
          </button>
          <button className="button primary" onClick={kaydet} disabled={!gecerli}>
            Kaydet
          </button>
        </div>
      </div>
    </div>
  );
}
