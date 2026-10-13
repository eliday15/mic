/**
 * Mock de álbumes ligados para el modo navegador (espejo de
 * `crates/mic-tauri/src/commands/ligados.rs`).
 *
 * El álbum ligado es otro álbum abierto en el mock con esa ruta o, si no lo
 * hay, uno sintético derivado del demo con estructura distinta: sin
 * "Descripción", con dos campos propios ("Proveedor", "Peso kg"), precios
 * distintos, fotos propias en 2 de cada 3 registros y 4 llaves que el demo no
 * tiene. Así se ejercitan todas las opciones de la liga.
 *
 * SOLO se usa en desarrollo cuando la app corre en un navegador sin Tauri.
 */

import { emit } from "@tauri-apps/api/event";
import type { CampoDef, Valores } from "$lib/domain/types";
import { crearAlbumDemo, type AlbumMock, type RegistroMock } from "./datos";
import { recalcular } from "./motor";

/** Una liga del mock (misma forma que `Liga` del contrato). */
interface LigaMock {
  id: number;
  rutaAlbum: string;
  llave: string;
  crearFaltantes: boolean;
  fotos: "actual" | "ligado";
  importarCamposNuevos: boolean;
}

/** Ligas del mock (compartidas entre álbumes), con una liga demo inicial. */
const ligasMock = new Map<number, LigaMock>([
  [
    1,
    {
      id: 1,
      rutaAlbum: "/demo/Bodegas.micdb",
      llave: "Clave",
      crearFaltantes: false,
      fotos: "actual",
      importarCamposNuevos: false,
    },
  ],
]);
let proximaLigaId = 2;

const espera = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** ¿El campo puede crearse en el actual? (no calculados ni multidatos). */
function esImportable(c: CampoDef): boolean {
  return c.tabla === "principal" && c.tipo !== "calculado" && c.tipo !== "multidato";
}

function nombresPrincipales(campos: CampoDef[]): Set<string> {
  return new Set(campos.filter((c) => c.tabla === "principal").map((c) => c.nombre));
}

/** Clave textual de la llave (null = sin llave útil), como `clave_de`. */
function claveDe(valores: Valores, llave: string): string | null {
  const v = valores[llave];
  if (v === null || v === undefined) return null;
  const txt = String(v).trim();
  return txt === "" ? null : txt;
}

/** Álbum ligado sintético (ver cabecera del módulo). */
function albumLigadoSintetico(ruta: string): AlbumMock {
  const a = crearAlbumDemo(0, ruta);
  const orden = a.campos.length;
  const extra = (id: number, nombre: string, tipo: CampoDef["tipo"], decimales: number): CampoDef => ({
    id,
    nombre,
    colFisica: `f_${id}`,
    tabla: "principal",
    tipo,
    decimales,
    totalizable: false,
    formula: null,
    visible: true,
    modificable: true,
    ordenVisible: orden + id,
    formato: null,
  });
  a.campos = [
    ...a.campos.filter((c) => c.nombre !== "Descripción"),
    extra(101, "Proveedor", "texto", 0),
    extra(102, "Peso kg", "numerico", 3),
  ];
  const proveedores = ["Bodega Norte", "Bodega Sur", "Importadora Centro"];
  a.principal = a.principal.slice(0, 40).map((r, i) => ({
    ...r,
    imagen: i % 3 === 2 ? null : `imagenes/ligado_${r.id}.jpg`,
    imagenVersion: 7,
    valores: {
      ...r.valores,
      Precio: Math.round(Number(r.valores.Precio) * 110) / 100,
      Proveedor: proveedores[i % proveedores.length],
      "Peso kg": Math.round((0.25 + i * 0.137) * 1000) / 1000,
    },
  }));
  for (let n = 1; n <= 4; n++) {
    a.principal.push({
      id: 9000 + n,
      idPrincipal: null,
      imagen: `imagenes/ligado_nuevo_${n}.jpg`,
      imagenVersion: 7,
      oculto: false,
      valores: {
        Clave: `SKU-${9000 + n}`,
        Nombre: `Artículo de bodega ${n}`,
        Cantidad: n * 5,
        Precio: n * 100,
        Proveedor: proveedores[n % proveedores.length],
        "Peso kg": n * 1.5,
      },
      multidatos: { Etiquetas: [] },
    });
  }
  return a;
}

/** Lo que devuelve `liga_inspeccionar`. */
function inspeccionar(actual: AlbumMock, ligado: AlbumMock) {
  const nombres = nombresPrincipales(actual.campos);
  const r = {
    camposComunes: [] as string[],
    camposSoloLigado: [] as string[],
    camposNoImportables: [] as string[],
    totalRegistros: ligado.principal.length,
    conImagen: ligado.principal.filter((x) => x.imagen).length,
  };
  for (const c of ligado.campos.filter((x) => x.tabla === "principal")) {
    if (nombres.has(c.nombre)) r.camposComunes.push(c.nombre);
    else if (esImportable(c)) r.camposSoloLigado.push(c.nombre);
    else r.camposNoImportables.push(c.nombre);
  }
  return r;
}

/** Copia "de archivo" de una foto del ligado: nombre propio y versión nueva. */
function copiarFoto(r: RegistroMock, origen: string): void {
  const base = origen.split("/").pop() ?? "imagen.jpg";
  r.imagen = `imagenes/${base.replace(/(\.\w+)?$/, `_${Date.now().toString(16)}$1`)}`;
  r.imagenVersion += 1;
}

/** Espejo de `sincronizar_liga_nucleo`. */
async function sincronizar(a: AlbumMock, ligado: AlbumMock, liga: LigaMock) {
  if (liga.rutaAlbum === a.ruta) throw "no se puede ligar un álbum consigo mismo";
  if (!nombresPrincipales(a.campos).has(liga.llave))
    throw `el campo llave '${liga.llave}' no existe en el álbum actual`;
  if (!nombresPrincipales(ligado.campos).has(liga.llave))
    throw `el campo llave '${liga.llave}' no existe en el álbum ligado`;

  const resultado = {
    actualizados: 0,
    creados: 0,
    sinCoincidencia: 0,
    fotosCopiadas: 0,
    camposCreados: [] as string[],
  };

  if (liga.importarCamposNuevos) {
    const nombres = nombresPrincipales(a.campos);
    let orden = Math.max(-1, ...a.campos.map((c) => c.ordenVisible));
    for (const c of ligado.campos.filter((x) => esImportable(x) && !nombres.has(x.nombre))) {
      const id = a.proximoCampoId++;
      a.campos.push({ ...c, id, colFisica: `f_${id}`, formula: null, modificable: true, ordenVisible: ++orden });
      for (const r of a.principal) r.valores[c.nombre] = null;
      resultado.camposCreados.push(c.nombre);
    }
  }

  const nombresLigado = nombresPrincipales(ligado.campos);
  const copia = a.campos.filter(
    (c) =>
      c.tabla === "principal" &&
      c.nombre !== liga.llave &&
      c.tipo !== "calculado" &&
      nombresLigado.has(c.nombre),
  );

  const indice = new Map<string, RegistroMock>();
  for (const r of ligado.principal) {
    const k = claveDe(r.valores, liga.llave);
    if (k !== null) indice.set(k, r);
  }
  const clavesActual = new Set(
    a.principal.map((r) => claveDe(r.valores, liga.llave)).filter((k) => k !== null),
  );
  const faltantes = [...indice.keys()].filter((k) => !clavesActual.has(k));
  const total = a.principal.length + (liga.crearFaltantes ? faltantes.length : 0);
  let hechas = 0;
  await emit("liga-progreso", { hechas, total });

  for (const r of a.principal) {
    const k = claveDe(r.valores, liga.llave);
    const origen = k !== null ? indice.get(k) : undefined;
    if (origen) {
      for (const c of copia) r.valores[c.nombre] = origen.valores[c.nombre] ?? null;
      recalcular(r, a.campos, "principal");
      resultado.actualizados++;
      if (origen.imagen && (!r.imagen || liga.fotos === "ligado")) {
        copiarFoto(r, origen.imagen);
        resultado.fotosCopiadas++;
      }
    }
    if (++hechas % 50 === 0) {
      await emit("liga-progreso", { hechas, total });
      await espera(60);
    }
  }

  if (liga.crearFaltantes) {
    for (const k of faltantes) {
      const origen = indice.get(k)!;
      const valores: Valores = { [liga.llave]: k };
      for (const c of copia) valores[c.nombre] = origen.valores[c.nombre] ?? null;
      const r: RegistroMock = {
        id: a.proximoId++,
        idPrincipal: null,
        imagen: null,
        imagenVersion: 1,
        oculto: false,
        valores,
        multidatos: {},
      };
      for (const c of a.campos) {
        if (c.tabla === "principal" && c.tipo === "multidato") {
          r.multidatos[c.nombre] = [];
          r.valores[c.nombre] = 0;
        }
      }
      recalcular(r, a.campos, "principal");
      if (origen.imagen) {
        copiarFoto(r, origen.imagen);
        resultado.fotosCopiadas++;
      }
      a.principal.push(r);
      resultado.creados++;
      hechas++;
    }
  } else {
    resultado.sinCoincidencia = faltantes.length;
  }

  await emit("liga-progreso", { hechas: total, total });
  await espera(200);
  return resultado;
}

/**
 * Atiende los comandos de ligados del mock; `undefined` si `cmd` no es de ligados.
 *
 * @param obtenerAlbum Álbum abierto por id (lanza si no existe).
 * @param abiertoPorRuta Álbum abierto en el mock con esa ruta, si lo hay.
 */
/* eslint-disable @typescript-eslint/no-explicit-any */
export async function manejarLigados(
  cmd: string,
  p: any,
  obtenerAlbum: (albumId: number) => AlbumMock,
  abiertoPorRuta: (ruta: string) => AlbumMock | undefined,
): Promise<unknown> {
  const ligado = (ruta: string) => abiertoPorRuta(ruta) ?? albumLigadoSintetico(ruta);
  switch (cmd) {
    case "ligados_listar":
      return [...ligasMock.values()];
    case "liga_guardar": {
      const liga = p.liga as LigaMock;
      if (liga.id === 0) {
        const nueva = { ...liga, id: proximaLigaId++ };
        ligasMock.set(nueva.id, nueva);
        return nueva.id;
      }
      ligasMock.set(liga.id, liga);
      return liga.id;
    }
    case "liga_eliminar":
      ligasMock.delete(p.ligaId);
      return null;
    case "liga_inspeccionar":
      await espera(250);
      return inspeccionar(obtenerAlbum(p.albumId), ligado(p.rutaAlbum));
    case "liga_actualizar": {
      const liga = ligasMock.get(p.ligaId);
      if (!liga) throw `No existe la liga id=${p.ligaId}`;
      return sincronizar(obtenerAlbum(p.albumId), ligado(liga.rutaAlbum), liga);
    }
    case "ligas_actualizar_todas": {
      const a = obtenerAlbum(p.albumId);
      const res = [];
      for (const liga of ligasMock.values()) {
        res.push(await sincronizar(a, ligado(liga.rutaAlbum), liga));
      }
      return res;
    }
    default:
      return undefined;
  }
}
