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
import { copyFileSync, mkdirSync, existsSync } from "node:fs";
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

const dizin = join(srcTauri, "binaries");
mkdirSync(dizin, { recursive: true });
const varis = join(dizin, `muifly-olcum-${hedef}.exe`);
copyFileSync(kaynak, varis);

console.log(`hazır: ${varis}`);
console.log(
  "not: kod imzalama yapılacaksa bu ikili de imzalanmalı — ana ikiliyle\n" +
    "birlikte kuruluyor ve imzasız bir yardımcı SmartScreen uyarısını geri\n" +
    "getirir (ROADMAP M3)."
);
