// Prueba E2E de "Importar desde Access…" en la app MIC REAL sobre Windows.
//
// Lanza el .exe compilado con la depuración remota de WebView2 encendida y se
// conecta por CDP (Playwright), igual que documenta Microsoft para probar apps
// WebView2. Hace clic en el botón de la bienvenida, abre el selector nativo (lo
// opera dialogo.ps1 con UI Automation) y espera a que la inspección termine o
// falle. Deja capturas (webview y escritorio), la consola JS, la salida del
// backend y la bitácora %TEMP%\mic-migracion.log en la carpeta de salida.
//
// Uso: node migracion-windows.mjs <app.exe> <archivo.mdb> <dir-salida>

import { chromium } from "playwright-core";
import { spawn, execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const [, , app, mdb, salida] = process.argv;
fs.mkdirSync(salida, { recursive: true });

const PLAZO_INSPECCION_MS = 240_000;
const PUERTO = 9222;
const aqui = path.dirname(fileURLToPath(import.meta.url));
const t0 = Date.now();
const lineas = [];
const log = (m) => {
  const l = `[+${((Date.now() - t0) / 1000).toFixed(1)}s] ${m}`;
  lineas.push(l);
  console.log(l);
};

let nCaptura = 0;
async function capturar(page, etiqueta) {
  const n = String(++nCaptura).padStart(2, "0");
  try {
    execFileSync("pwsh", ["-NoProfile", "-File", path.join(aqui, "captura.ps1"),
      "-Salida", path.join(salida, `${n}-escritorio-${etiqueta}.png`)]);
  } catch (e) {
    log(`captura de escritorio falló: ${e.message}`);
  }
  if (!page) return;
  try {
    await page.screenshot({ path: path.join(salida, `${n}-webview-${etiqueta}.png`), timeout: 10_000 });
  } catch (e) {
    log(`captura de webview falló (¿webview bloqueado?): ${e.message}`);
  }
}

const bitacora = path.join(os.tmpdir(), "mic-migracion.log");
fs.rmSync(bitacora, { force: true });

log(`app: ${app}`);
log(`mdb: ${mdb}`);
const proc = spawn(app, [], {
  env: {
    ...process.env,
    WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${PUERTO}`,
    RUST_LOG: "info",
    RUST_BACKTRACE: "1",
  },
  stdio: ["ignore", "pipe", "pipe"],
});
const backend = fs.createWriteStream(path.join(salida, "backend.log"));
proc.stdout.pipe(backend);
proc.stderr.pipe(backend);
proc.on("exit", (code) => log(`la app terminó con código ${code}`));

let browser;
let page;
let resultado = "desconocido";
try {
  // Esperar a que WebView2 abra el puerto de depuración.
  for (let i = 0; i < 60 && !browser; i++) {
    try {
      browser = await chromium.connectOverCDP(`http://127.0.0.1:${PUERTO}`);
    } catch {
      await new Promise((r) => setTimeout(r, 1_000));
    }
  }
  if (!browser) throw new Error("WebView2 nunca abrió el puerto de depuración");
  const ctx = browser.contexts()[0];
  for (let i = 0; i < 30 && !page; i++) {
    page = ctx.pages()[0];
    if (!page) await new Promise((r) => setTimeout(r, 500));
  }
  if (!page) throw new Error("sin página en el webview");
  log(`conectado al webview: ${page.url()}`);
  page.on("console", (m) => log(`[consola:${m.type()}] ${m.text()}`));
  page.on("pageerror", (e) => log(`[pageerror] ${e.message}`));
  page.on("framenavigated", (f) => log(`[navegación] ${f.url()}`));

  const importar = page.locator("button", { hasText: "Importar desde Access" }).first();
  await importar.waitFor({ timeout: 60_000 });
  log("app lista; clic en 'Importar desde Access…'");
  await importar.click();

  const examinar = page.locator(".modal button", { hasText: "Examinar" }).first();
  await examinar.waitFor({ timeout: 30_000 });
  await capturar(page, "dialogo-abierto");

  // El selector nativo lo opera un proceso aparte (CDP no lo ve).
  const helper = spawn("pwsh", ["-NoProfile", "-File", path.join(aqui, "dialogo.ps1"), "-Ruta", mdb],
    { stdio: ["ignore", "pipe", "pipe"] });
  helper.stdout.on("data", (d) => log(`[dialogo] ${String(d).trim()}`));
  helper.stderr.on("data", (d) => log(`[dialogo:err] ${String(d).trim()}`));

  log("clic en 'Examinar…'");
  await examinar.click({ noWaitAfter: true });
  log("clic devuelto");

  const fin = Date.now() + PLAZO_INSPECCION_MS;
  let ultimo = "";
  let siguienteCaptura = Date.now() + 5_000;
  while (Date.now() < fin) {
    let texto;
    try {
      texto = await page.evaluate(() => {
        const m = document.querySelector(".modal");
        return m ? m.innerText.replace(/\s+/g, " ") : "(sin modal)";
      });
    } catch (e) {
      texto = `(webview no respondió: ${e.message})`;
    }
    if (texto !== ultimo) {
      log(`modal: ${texto.slice(0, 500)}`);
      ultimo = texto;
    }
    if (texto.includes("Registros estimados")) {
      resultado = "exito";
      break;
    }
    if (await page.locator(".mg__error").count().catch(() => 0)) {
      resultado = "error-visible";
      break;
    }
    if (Date.now() >= siguienteCaptura) {
      await capturar(page, "esperando");
      siguienteCaptura = Date.now() + 20_000;
    }
    await new Promise((r) => setTimeout(r, 1_000));
  }
  if (resultado === "desconocido") {
    resultado = "COLGADO";
    // Sonda: ¿el IPC sigue vivo? Invoca un comando trivial directamente.
    const sonda = await page
      .evaluate(() =>
        Promise.race([
          window.__TAURI_INTERNALS__.invoke("migracion_verificar_mdbtools", {}).then((v) => `ipc vivo (${v})`),
          new Promise((r) => setTimeout(() => r("ipc SIN respuesta en 10 s"), 10_000)),
        ]),
      )
      .catch((e) => `sonda falló: ${e.message}`);
    log(`sonda IPC: ${sonda}`);
  }
  await capturar(page, `final-${resultado}`);
} catch (e) {
  resultado = `fallo-prueba: ${e.stack ?? e}`;
  await capturar(page, "fallo-prueba");
} finally {
  log(`RESULTADO: ${resultado}`);
  const contenido = fs.existsSync(bitacora) ? fs.readFileSync(bitacora, "utf8") : "(no existe)";
  console.log(`----- ${bitacora} -----\n${contenido}\n-----`);
  fs.writeFileSync(path.join(salida, "mic-migracion.log"), contenido);
  fs.writeFileSync(path.join(salida, "resultado.txt"), resultado);
  fs.writeFileSync(path.join(salida, "e2e.log"), lineas.join("\n"));
  try {
    await browser?.close();
  } catch {}
  proc.kill();
}
process.exit(resultado === "exito" ? 0 : 1);
