// Prueba E2E de "Importar desde Access…" en la app MIC REAL sobre Windows.
//
// Arranca el .exe compilado vía tauri-driver (WebDriver sobre WebView2), hace
// clic en el botón de la bienvenida, abre el selector nativo (lo opera
// dialogo.ps1 con UI Automation) y espera a que la inspección termine o falle.
// Deja capturas (webview y escritorio), el texto del diálogo y la bitácora
// %TEMP%\mic-migracion.log en la carpeta de salida.
//
// Uso: node migracion-windows.mjs <app.exe> <archivo.mdb> <dir-salida>

import { Builder, By, until } from "selenium-webdriver";
import { spawn, execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const [, , app, mdb, salida] = process.argv;
fs.mkdirSync(salida, { recursive: true });

const PLAZO_INSPECCION_MS = 240_000;
const aqui = path.dirname(new URL(import.meta.url).pathname).replace(/^\/([A-Za-z]:)/, "$1");
const t0 = Date.now();
const log = (m) => console.log(`[+${((Date.now() - t0) / 1000).toFixed(1)}s] ${m}`);

let nCaptura = 0;
async function capturar(driver, etiqueta) {
  const n = String(++nCaptura).padStart(2, "0");
  try {
    execFileSync("pwsh", ["-NoProfile", "-File", path.join(aqui, "captura.ps1"),
      "-Salida", path.join(salida, `${n}-escritorio-${etiqueta}.png`)]);
  } catch (e) {
    log(`captura de escritorio falló: ${e.message}`);
  }
  try {
    const png = await driver.takeScreenshot();
    fs.writeFileSync(path.join(salida, `${n}-webview-${etiqueta}.png`), png, "base64");
  } catch (e) {
    log(`captura de webview falló (¿webview bloqueado?): ${e.message}`);
  }
}

async function textoModal(driver) {
  const els = await driver.findElements(By.css(".modal"));
  return els.length ? (await els[0].getText()).replace(/\s+/g, " ") : "(sin modal)";
}

const bitacora = path.join(os.tmpdir(), "mic-migracion.log");
fs.rmSync(bitacora, { force: true });

log(`app: ${app}`);
log(`mdb: ${mdb}`);
const driver = await new Builder()
  .usingServer("http://127.0.0.1:4444/")
  .withCapabilities({ browserName: "wry", "tauri:options": { application: app } })
  .build();

let resultado = "desconocido";
try {
  const importar = await driver.wait(
    until.elementLocated(By.xpath("//button[contains(., 'Importar desde Access')]")),
    60_000,
  );
  log("app lista; clic en 'Importar desde Access…'");
  await importar.click();

  const examinar = await driver.wait(
    until.elementLocated(By.xpath("//div[contains(@class,'modal')]//button[contains(., 'Examinar')]")),
    30_000,
  );
  await capturar(driver, "dialogo-abierto");

  // El selector nativo lo opera un proceso aparte (WebDriver no lo ve).
  const helper = spawn("pwsh", ["-NoProfile", "-File", path.join(aqui, "dialogo.ps1"), "-Ruta", mdb],
    { stdio: ["ignore", "pipe", "pipe"] });
  helper.stdout.on("data", (d) => process.stdout.write(`  [dialogo] ${d}`));
  helper.stderr.on("data", (d) => process.stdout.write(`  [dialogo:err] ${d}`));

  log("clic en 'Examinar…'");
  await examinar.click();
  log("clic devuelto");

  const fin = Date.now() + PLAZO_INSPECCION_MS;
  let ultimo = "";
  let siguienteCaptura = Date.now() + 5_000;
  while (Date.now() < fin) {
    let texto;
    try {
      texto = await textoModal(driver);
    } catch (e) {
      texto = `(webdriver no respondió: ${e.message})`;
    }
    if (texto !== ultimo) {
      log(`modal: ${texto.slice(0, 400)}`);
      ultimo = texto;
    }
    if (texto.includes("Registros estimados")) {
      resultado = "exito";
      break;
    }
    if ((await driver.findElements(By.css(".mg__error"))).length > 0) {
      resultado = "error-visible";
      break;
    }
    if (Date.now() >= siguienteCaptura) {
      await capturar(driver, "esperando");
      siguienteCaptura = Date.now() + 20_000;
    }
    await new Promise((r) => setTimeout(r, 1_000));
  }
  if (resultado === "desconocido") resultado = "COLGADO";
  await capturar(driver, `final-${resultado}`);
  fs.writeFileSync(path.join(salida, "modal.txt"), ultimo);
} catch (e) {
  resultado = `fallo-prueba: ${e.stack ?? e}`;
  await capturar(driver, "fallo-prueba");
} finally {
  log(`RESULTADO: ${resultado}`);
  const contenido = fs.existsSync(bitacora) ? fs.readFileSync(bitacora, "utf8") : "(no existe)";
  console.log(`----- ${bitacora} -----\n${contenido}\n-----`);
  fs.writeFileSync(path.join(salida, "mic-migracion.log"), contenido);
  fs.writeFileSync(path.join(salida, "resultado.txt"), resultado);
  try {
    await driver.quit();
  } catch {}
}
process.exit(resultado === "exito" ? 0 : 1);
