// Ölçüm yardımcısını (`muifly-olcum.exe`) Tauri'nin sidecar düzenine hazırlar.
//
// ÜRETEÇ. Çıktısı sürüm kontrolünde değil; `tauri build` öncesi çalıştırılmalı.
//
// # Neden gerekiyor
//
// Kare ölçümü yükseltilmiş yetki istiyor (karar #27) ve bu yüzden ana
// uygulamanın dışında, ayrı bir ikilide yapılıyor. `cargo build` o ikiliyi
// üretiyor ama `tauri build` yalnızca ana ikiliyi paketliyor — yardımcı
// kurulumla birlikte gitmezse özellik yayın sürümünde ÖLÜ olur ve bunu
// yalnızca kullanıcı fark eder.
//
// Tauri sidecar'ları hedef üçlüsü ekiyle adlandırılmış bekliyor:
//   binaries/muifly-olcum-x86_64-pc-windows-msvc.exe
// Kurulumda eki düşürüp ana ikilinin yanına koyuyor — `olcum::yardimci_yolu`
// tam da oraya bakıyor.

import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, existsSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const kok = join(dirname(fileURLToPath(import.meta.url)), "..");
const srcTauri = join(kok, "src-tauri");

function ucluk() {
  const cikti = execFileSync("rustc", ["-vV"], { encoding: "utf8" });
  const satir = cikti.split("\n").find((s) => s.startsWith("host:"));
  if (!satir) throw new Error("rustc -vV çıktısında 'host:' yok");
  return satir.slice("host:".length).trim();
}

const hedef = ucluk();
console.log(`hedef üçlü: ${hedef}`);

const dizin = join(srcTauri, "binaries");
const varis = join(dizin, `muifly-olcum-${hedef}.exe`);

// Yumurta-tavuk: yardımcıyı derlemek de `tauri-build`i çalıştırıyor ve o,
// `externalBin` listesindeki sidecar'ın VAR OLMASINI arıyor. Dosya üretilmiş
// bir çıktı (`.gitignore`), yani temiz bir klonda ya da CI'da yok — derleme
// daha başlamadan "resource path ... doesn't exist" ile kırılıyor.
//
// Çözüm boş bir yer tutucu: `tauri-build` yalnızca varlığa bakıyor, içeriğe
// değil. Aşağıda gerçek ikiliyle üzerine yazılıyor, yani yer tutucu yalnızca
// derleme boyunca yaşıyor.
mkdirSync(dizin, { recursive: true });
if (!existsSync(varis)) {
  console.log("yer tutucu yazılıyor (ilk derleme)...");
  writeFileSync(varis, "");
}

console.log("yardımcı derleniyor (release)...");
// `olcum-yardimcisi` özelliği şart: yardımcı ikili `required-features`
// arkasında duruyor ki normal `cargo build` onu üretmesin. Üretseydi
// `tauri build` hem cargo'nun ikilisini hem sidecar'ı kuruluma koyar,
// ikisi aynı ada yazılır ve MSI hedefi ICE30 ile kırılırdı.
execFileSync(
  "cargo",
  ["build", "--release", "--bin", "muifly-olcum", "--features", "olcum-yardimcisi"],
  { cwd: srcTauri, stdio: "inherit" },
);

const kaynak = join(srcTauri, "target", "release", "muifly-olcum.exe");
if (!existsSync(kaynak)) {
  throw new Error(`derleme çıktısı bulunamadı: ${kaynak}`);
}

copyFileSync(kaynak, varis);

console.log(`hazır: ${varis}`);
console.log(
  "not: kod imzalama yapılacaksa bu ikili de imzalanmalı — ana ikiliyle\n" +
    "birlikte kuruluyor ve imzasız bir yardımcı SmartScreen uyarısını geri\n" +
    "getirir (ROADMAP M3)."
);
