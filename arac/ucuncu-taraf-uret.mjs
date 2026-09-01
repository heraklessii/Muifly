/**
 * Üçüncü taraf bildirimlerini üretir → `src-tauri/ucuncu-taraf.json`
 *
 *     node arac/ucuncu-taraf-uret.mjs
 *
 * Neden bir betik, `cargo-about` değil: cargo-about kurulu değil ve kurmak
 * ağ + uzun bir derleme istiyor. Buradaki iş zaten çevrimdışı yapılabiliyor —
 * bütün veri `cargo tree`, `cargo metadata`, `npm ls` ve kayıt önbelleğindeki
 * LICENSE dosyalarında duruyor. Ek bir araç bağımlılığı taşımadan aynı sonucu
 * veriyorsa, taşımıyoruz.
 *
 * EULA madde 8 uygulama içinde bir "Üçüncü taraf lisanslar" bölümü vaat
 * ediyor. Bu dosya o bölümün verisi; `src-tauri/src/ucuncu_taraf.rs` onu
 * ikiliye gömüyor. Bağımlılık sürümleri değiştiğinde betik yeniden
 * çalıştırılmalı — `ucuncu_taraf::testler` bunu unutulursa yakalıyor.
 *
 * Kapsam: ikiliye GERÇEKTEN giren şeyler. Rust tarafında `-e normal`
 * (geliştirme ve derleme bağımlılıkları hariç), npm tarafında `--omit=dev`.
 * Test ya da derleme sırasında kullanılıp dağıtılmayan bir kütüphanenin
 * lisansını kullanıcıya göstermek, listeyi okunmaz yapmaktan başka bir işe
 * yaramaz.
 */

import { execFileSync, execSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const KOK = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const SRC_TAURI = path.join(KOK, 'src-tauri');
const CIKTI = path.join(SRC_TAURI, 'ucuncu-taraf.json');

/** Windows hedefi: Muifly yalnızca burayı paketliyor (tauri.conf → nsis, msi). */
const HEDEF = 'x86_64-pc-windows-msvc';

/**
 * Depoya elle konmuş, paket yöneticisinden gelmeyen varlıklar.
 * Yazı tipi uygulamaya gömülü (`styles.css`), yani dağıtılıyor; lisansı da
 * dağıtılmak zorunda.
 */
const VARLIKLAR = [
  {
    ad: 'Outfit',
    surum: 'değişken (variable)',
    lisans: 'OFL-1.1',
    kaynak: 'https://github.com/Outfitio/Outfit-Fonts',
    lisansDosyasi: 'src/assets/fonts/LICENSE-OFL.txt',
  },
];

const LISANS_DESENI = /^(licen[sc]e|copying|notice|unlicense|ofl)([-._].*)?(\.(md|txt))?$/i;

const CALISTIRMA_SECENEKLERI = { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 };

/** cargo gerçek bir çalıştırılabilir; kabuksuz, argümanlar dizi olarak. */
function cargo(argumanlar) {
  return execFileSync('cargo', argumanlar, { ...CALISTIRMA_SECENEKLERI, cwd: SRC_TAURI });
}

/**
 * npm Windows'ta bir `.cmd`; Node onu kabuksuz çalıştırmayı reddediyor.
 * Komut sabit bir dize olarak veriliyor — kabuğa dışarıdan hiçbir değer
 * geçmediği için birleştirme burada zararsız.
 */
function npm(komut) {
  return execSync(`npm ${komut}`, { ...CALISTIRMA_SECENEKLERI, cwd: KOK });
}

/** Bir paket dizinindeki lisans dosyalarını okuyup tek metinde birleştirir. */
function lisansMetniniOku(dizin) {
  let girisler;
  try {
    girisler = fs.readdirSync(dizin, { withFileTypes: true });
  } catch {
    return null;
  }

  const dosyalar = girisler
    .filter((g) => g.isFile() && LISANS_DESENI.test(g.name))
    .map((g) => g.name)
    .sort();

  if (dosyalar.length === 0) return null;

  const parcalar = [];
  for (const ad of dosyalar) {
    const metin = fs.readFileSync(path.join(dizin, ad), 'utf8').replace(/\r\n/g, '\n').trim();
    if (metin) parcalar.push(dosyalar.length > 1 ? `--- ${ad} ---\n\n${metin}` : metin);
  }
  return parcalar.length ? parcalar.join('\n\n') : null;
}

/** Rust: ikiliye giren crate'ler. */
function rustBilesenleri() {
  // `{p}` → "ad vSÜRÜM [(*)]", `{l}` → SPDX ifadesi, `{r}` → depo adresi
  const cikti = cargo([
    'tree',
    '-e',
    'normal',
    '--target',
    HEDEF,
    '--prefix',
    'none',
    '--format',
    '{p}|{l}|{r}',
  ]);

  const gorulen = new Map();
  for (const satir of cikti.split('\n')) {
    const temiz = satir.trim().replace(/\s*\(\*\)$/, '');
    if (!temiz) continue;

    const [paket, lisans = '', kaynak = ''] = temiz.split('|');
    const eslesme = /^(\S+)\s+v(\S+)/.exec(paket.trim());
    if (!eslesme) continue;

    const [, ad, surum] = eslesme;
    if (ad === 'muifly') continue; // kendimiz üçüncü taraf değiliz

    gorulen.set(`${ad}@${surum}`, {
      ad,
      surum,
      tur: 'rust',
      lisans: lisans.trim() || null,
      kaynak: kaynak.trim() || null,
    });
  }

  // Lisans METİNLERİ crate'in kayıt önbelleğindeki dizininde; yolu metadata veriyor.
  const metadata = JSON.parse(cargo(['metadata', '--format-version', '1']));
  const yollar = new Map(
    metadata.packages.map((p) => [`${p.name}@${p.version}`, path.dirname(p.manifest_path)]),
  );

  for (const [anahtar, bilesen] of gorulen) {
    const dizin = yollar.get(anahtar);
    bilesen.metin = dizin ? lisansMetniniOku(dizin) : null;
  }

  return [...gorulen.values()];
}

/** npm: `dist/` içine giren paketler. */
function npmBilesenleri() {
  const agac = JSON.parse(npm('ls --omit=dev --all --json'));

  const gorulen = new Map();
  (function yuru(bagimliliklar) {
    for (const [ad, dugum] of Object.entries(bagimliliklar ?? {})) {
      if (!gorulen.has(ad)) gorulen.set(ad, dugum.version ?? null);
      yuru(dugum.dependencies);
    }
  })(agac.dependencies);

  const bilesenler = [];
  for (const [ad, surum] of gorulen) {
    const dizin = path.join(KOK, 'node_modules', ...ad.split('/'));
    let paket = {};
    try {
      paket = JSON.parse(fs.readFileSync(path.join(dizin, 'package.json'), 'utf8'));
    } catch {
      /* kurulu değilse aşağıdaki null'larla geçer */
    }

    const depo = paket.repository;
    bilesenler.push({
      ad,
      surum: paket.version ?? surum,
      tur: 'npm',
      lisans: typeof paket.license === 'string' ? paket.license : null,
      kaynak: (typeof depo === 'string' ? depo : depo?.url)?.replace(/^git\+|\.git$/g, '') ?? null,
      metin: lisansMetniniOku(dizin),
    });
  }
  return bilesenler;
}

/** Depoya elle konmuş varlıklar. */
function varlikBilesenleri() {
  return VARLIKLAR.map((v) => {
    const yol = path.join(KOK, v.lisansDosyasi);
    let metin = null;
    try {
      metin = fs.readFileSync(yol, 'utf8').replace(/\r\n/g, '\n').trim() || null;
    } catch {
      /* aşağıda uyarı veriliyor */
    }
    if (!metin) console.warn(`  uyarı: ${v.ad} için lisans metni yok (${v.lisansDosyasi})`);
    return { ad: v.ad, surum: v.surum, tur: 'varlik', lisans: v.lisans, kaynak: v.kaynak, metin };
  });
}

function uret() {
  console.log('Rust bağımlılıkları okunuyor…');
  const rust = rustBilesenleri();
  console.log(`  ${rust.length} crate`);

  console.log('npm bağımlılıkları okunuyor…');
  const npmPaketleri = npmBilesenleri();
  console.log(`  ${npmPaketleri.length} paket`);

  console.log('Depodaki varlıklar okunuyor…');
  const varliklar = varlikBilesenleri();
  console.log(`  ${varliklar.length} varlık`);

  const hepsi = [...rust, ...npmPaketleri, ...varliklar].sort(
    (a, b) => a.ad.localeCompare(b.ad, 'en') || a.surum.localeCompare(b.surum, 'en'),
  );

  // Aynı lisans metni yüzlerce crate'te tekrarlıyor; bir kez saklayıp indeksle
  // atıfta bulunuyoruz. Yoksa gömülecek dosya megabaytlara çıkardı.
  const metinIndeksi = new Map();
  const metinler = [];
  const bilesenler = hepsi.map(({ metin, ...geri }) => {
    if (metin == null) return { ...geri, metinNo: null };
    if (!metinIndeksi.has(metin)) {
      metinIndeksi.set(metin, metinler.length);
      metinler.push(metin);
    }
    return { ...geri, metinNo: metinIndeksi.get(metin) };
  });

  const metinsiz = bilesenler.filter((b) => b.metinNo === null);
  if (metinsiz.length) {
    console.warn(`\nLisans metni bulunamayan ${metinsiz.length} bileşen:`);
    for (const b of metinsiz) console.warn(`  ${b.ad} ${b.surum} (${b.lisans ?? 'lisans yok'})`);
  }

  // Yerel tarih, UTC değil: geceyarısından sonra üreteci çalıştıran biri
  // dosyada dünün tarihini görüp veriyi bayat sanmasın.
  const bugun = new Date();
  const iki = (n) => String(n).padStart(2, '0');

  const veri = {
    uretildi: `${bugun.getFullYear()}-${iki(bugun.getMonth() + 1)}-${iki(bugun.getDate())}`,
    hedef: HEDEF,
    bilesenler,
    metinler,
  };

  fs.writeFileSync(CIKTI, JSON.stringify(veri, null, 2) + '\n', 'utf8');

  const kb = (fs.statSync(CIKTI).size / 1024).toFixed(0);
  console.log(
    `\n${path.relative(KOK, CIKTI)} yazıldı — ` +
      `${bilesenler.length} bileşen, ${metinler.length} benzersiz lisans metni, ${kb} KB`,
  );
}

uret();
