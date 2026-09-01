/**
 * Public vitrin deposunun içeriğini üretir.
 *
 *     node arac/vitrin-hazirla.mjs <hedef-klasor>
 *
 * `docs/DISTRIBUTION.md` iki depo tanımlıyor:
 *
 * - **Muifly-dev** (private) — bu depo. Kaynak, belgeler, araçlar.
 * - **Muifly** (public) — yalnızca vitrin: tanıtım sayfası, README, EULA,
 *   Releases'te demo ikilisi, Issues'ta hata bildirimi. **Kaynak kod yok.**
 *
 * Bu betik ikinci deponun içeriğini üretiyor. Var olma sebebi tek bir cümle:
 * *elle kopyalama er ya da geç `src/` klasörünü de götürür.* Kopyalama bir
 * **izin listesine** dayanıyor — yeni bir klasör eklendiğinde varsayılan
 * davranış onu dışarıda bırakmak.
 *
 * Betik hedefe yalnızca dosya yazıyor: git komutu çalıştırmıyor, uzak depoya
 * bir şey göndermiyor. Yayınlama kararı ve `git push` insana ait.
 */

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const KOK = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');

/**
 * Public depoya girecek her şey. Liste tam: burada olmayan hiçbir şey
 * kopyalanmıyor.
 */
const IZIN_LISTESI = [
  'site',
  'README.md',
  'LICENSE.md',
  // Tanıtım sayfasını GitHub Pages'e yayınlayan iş akışı. `ci.yml` bilerek
  // yok: derleme ve test kaynak kodu gerektiriyor, o da bu depoda değil.
  '.github/workflows/pages.yml',
  // Satır sonu kuralları iki depoda aynı olmalı: yoksa her yeniden üretim,
  // içerik değişmese bile bütün dosyaları değişmiş gösterir.
  '.gitattributes',
];

/**
 * Hedefte bulunmaları hata sayılan yollar.
 *
 * İzin listesi zaten bunları kopyalamıyor; bu ikinci kontrol, hedef klasörde
 * daha önce elle bir şey bırakılmış olma ihtimaline karşı. Kaynak kodun
 * public depoya sızması geri alınamaz bir olay (git geçmişi ve GitHub'ın
 * önbelleği kalıyor), o yüzden iki kapı da kapalı.
 */
const YASAKLI = ['src', 'src-tauri', 'docs', 'arac', 'CLAUDE.md', 'package.json'];

function kopyala(kaynak, hedef) {
  const durum = fs.statSync(kaynak);
  if (durum.isDirectory()) {
    fs.mkdirSync(hedef, { recursive: true });
    for (const ad of fs.readdirSync(kaynak)) {
      kopyala(path.join(kaynak, ad), path.join(hedef, ad));
    }
  } else {
    fs.mkdirSync(path.dirname(hedef), { recursive: true });
    fs.copyFileSync(kaynak, hedef);
  }
}

const hedef = process.argv[2];
if (!hedef) {
  console.error('Kullanım: node arac/vitrin-hazirla.mjs <hedef-klasor>');
  process.exit(1);
}
const hedefTam = path.resolve(hedef);
if (hedefTam === KOK) {
  console.error('Hedef, geliştirme deposunun kendisi olamaz.');
  process.exit(1);
}

fs.mkdirSync(hedefTam, { recursive: true });

let sayac = 0;
for (const yol of IZIN_LISTESI) {
  const kaynak = path.join(KOK, yol);
  if (!fs.existsSync(kaynak)) {
    console.error(`eksik: ${yol}`);
    process.exit(1);
  }
  kopyala(kaynak, path.join(hedefTam, yol));
  sayac += 1;
  console.log(`  ${yol}`);
}

// Son kontrol: hedefte kaynak kod var mı?
const sizanlar = YASAKLI.filter((y) => fs.existsSync(path.join(hedefTam, y)));
if (sizanlar.length > 0) {
  console.error(
    `\nDURDURULDU — hedef klasörde public depoya girmemesi gereken yollar var:\n` +
      sizanlar.map((s) => `  ${s}`).join('\n') +
      `\n\nBunları sil, sonra tekrar çalıştır (docs/DISTRIBUTION.md).`,
  );
  process.exit(1);
}

console.log(`\n${sayac} yol kopyalandı → ${hedefTam}`);
console.log('Kaynak kod kopyalanmadı (docs/DISTRIBUTION.md).');
