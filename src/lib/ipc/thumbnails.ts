/**
 * Construcción de URLs para el protocolo custom `thumb` que sirve miniaturas
 * (y la imagen original con `size=0`) desde el backend Rust.
 *
 * La forma de la URL depende de la plataforma, igual que hace `convertFileSrc`
 * de Tauri: en Windows se usa el esquema `http://thumb.localhost/...`; en
 * macOS y Linux el esquema personalizado `thumb://localhost/...`.
 */

import type { TamanoThumb, Tabla } from "$lib/domain/types";

/** True si el webview corre sobre Windows (detección por userAgent). */
function esWindows(): boolean {
  if (typeof navigator === "undefined") return false;
  return navigator.userAgent.includes("Windows");
}

/** True en modo navegador de desarrollo (mock IPC instalado, sin Tauri). */
function esMock(): boolean {
  return (
    typeof window !== "undefined" &&
    (window as { __MIC_MOCK__?: boolean }).__MIC_MOCK__ === true
  );
}

/** Formas geométricas de los placeholders del modo mock. */
const FORMAS_MOCK = [
  '<circle cx="50" cy="42" r="24" fill="rgba(255,255,255,.85)"/>',
  '<rect x="28" y="20" width="44" height="44" rx="6" fill="rgba(255,255,255,.85)"/>',
  '<polygon points="50,16 78,64 22,64" fill="rgba(255,255,255,.85)"/>',
  '<polygon points="50,14 80,32 80,58 50,76 20,58 20,32" fill="rgba(255,255,255,.85)"/>',
  '<ellipse cx="50" cy="42" rx="30" ry="18" fill="rgba(255,255,255,.85)"/>',
];

/**
 * Miniatura sintética determinista (SVG data-URI) para el modo navegador:
 * tono según el id, forma variable y el id visible — suficiente para revisar
 * la UI con imágenes "reales" sin backend.
 */
function thumbMock(
  tabla: Tabla,
  id: number,
  size: TamanoThumb,
  version: number,
): string {
  const lado = size === 0 ? 512 : size;
  // La versión entra en la derivación para que "Elegir imagen…" (que sube la
  // versión) cambie visiblemente el color y la forma en modo navegador.
  const hue = (id * 47 + version * 53 + (tabla === "variantes" ? 180 : 0)) % 360;
  const forma = FORMAS_MOCK[(id + version) % FORMAS_MOCK.length];
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" width="${lado}" height="${lado}" viewBox="0 0 100 100">` +
    `<defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1">` +
    `<stop offset="0" stop-color="hsl(${hue},42%,38%)"/>` +
    `<stop offset="1" stop-color="hsl(${(hue + 40) % 360},48%,24%)"/>` +
    `</linearGradient></defs>` +
    `<rect width="100" height="100" fill="url(#g)"/>${forma}` +
    `<text x="50" y="88" font-family="system-ui" font-size="11" fill="rgba(255,255,255,.9)" text-anchor="middle">#${id}</text>` +
    `</svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

/**
 * Devuelve la URL de la miniatura de un registro.
 *
 * @param albumId Id de sesión del álbum.
 * @param tabla   Tabla del registro ('principal' | 'variantes').
 * @param id      Id del registro.
 * @param size    Tamaño del lado mayor (0 = imagen original a tamaño completo).
 * @param version Versión de la imagen (mtime) para invalidar la caché del webview.
 * @returns URL servible por el protocolo `thumb`.
 */
export function thumbUrl(
  albumId: number,
  tabla: Tabla,
  id: number,
  size: TamanoThumb,
  version: number,
): string {
  if (esMock()) return thumbMock(tabla, id, size, version);
  const ruta = `${albumId}/${tabla}/${id}?size=${size}&v=${version}`;
  return esWindows()
    ? `http://thumb.localhost/${ruta}`
    : `thumb://localhost/${ruta}`;
}

/**
 * URL de la imagen original a tamaño completo (atajo de `thumbUrl` con size=0),
 * usada por el visor al 100 %.
 */
export function imagenOriginalUrl(
  albumId: number,
  tabla: Tabla,
  id: number,
  version: number,
): string {
  return thumbUrl(albumId, tabla, id, 0, version);
}
