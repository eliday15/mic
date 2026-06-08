/**
 * Motor de consultas del mock: replica en memoria la semántica del backend
 * Rust (query_builder + calc) lo suficiente para revisar la UI en navegador.
 *
 * - Filtros avanzados: evaluación secuencial izquierda→derecha (sin
 *   precedencia), igual que el original VB6.
 * - Búsqueda libre: subcadena sin acentos sobre todos los valores de texto.
 * - Orden: hasta 3 niveles, con comparación según tipo de campo.
 * - Fórmulas: sustitución de nombres de campo y evaluación aritmética simple.
 */

import type {
  CampoDef,
  CondicionFiltro,
  NodoGrupo,
  OrdenCampo,
  QueryReq,
  Valor,
  Valores,
} from "$lib/domain/types";
import type { AlbumMock, RegistroMock } from "./datos";

/** Quita acentos y pasa a minúsculas (equivale a unicode61 remove_diacritics). */
export function normalizar(s: string): string {
  return s
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase();
}

/** Convierte un Valor a número si es posible (para comparaciones). */
function aNumero(v: Valor): number | null {
  if (typeof v === "number") return v;
  if (typeof v === "string" && v.trim() !== "" && !isNaN(Number(v))) {
    return Number(v);
  }
  return null;
}

/** Compara dos valores con semántica por tipo (números, fechas ISO, texto). */
function comparar(a: Valor, b: Valor): number {
  if (a == null && b == null) return 0;
  if (a == null) return -1;
  if (b == null) return 1;
  const na = aNumero(a);
  const nb = aNumero(b);
  if (na != null && nb != null) return na - nb;
  return normalizar(String(a)).localeCompare(normalizar(String(b)), "es");
}

/** Evalúa una condición de filtro contra un registro. */
function cumpleCondicion(
  reg: RegistroMock,
  c: CondicionFiltro,
  campos: CampoDef[],
): boolean {
  const def = campos.find((d) => d.nombre === c.campo);
  // Campos multidato filtran sobre la lista de valores, no sobre el conteo.
  const crudo: Valor =
    def?.tipo === "multidato"
      ? (reg.multidatos[c.campo] ?? []).join(" ")
      : (reg.valores[c.campo] ?? null);
  const cmp = comparar(crudo, c.valor);
  const texto = normalizar(String(crudo ?? ""));
  const buscado = normalizar(c.valor);
  switch (c.opComp) {
    case "igual":
      return cmp === 0;
    case "distinto":
      return cmp !== 0;
    case "mayor":
      return cmp > 0;
    case "menor":
      return cmp < 0;
    case "mayor_igual":
      return cmp >= 0;
    case "menor_igual":
      return cmp <= 0;
    case "contiene":
      return texto.includes(buscado);
    case "empieza":
      return texto.startsWith(buscado);
  }
}

/** Evalúa la cadena de condiciones secuencialmente (Y/O sin precedencia). */
export function cumpleCondiciones(
  reg: RegistroMock,
  condiciones: CondicionFiltro[],
  campos: CampoDef[],
): boolean {
  if (condiciones.length === 0) return true;
  let acc = cumpleCondicion(reg, condiciones[0], campos);
  for (let i = 1; i < condiciones.length; i++) {
    const c = condiciones[i];
    const v = cumpleCondicion(reg, c, campos);
    acc = c.opRel === "o" ? acc || v : acc && v;
  }
  return acc;
}

/** True si algún valor del registro contiene la búsqueda (sin acentos). */
function coincideBusqueda(reg: RegistroMock, busqueda: string): boolean {
  const q = normalizar(busqueda);
  for (const v of Object.values(reg.valores)) {
    if (v != null && normalizar(String(v)).includes(q)) return true;
  }
  for (const lista of Object.values(reg.multidatos)) {
    if (lista.some((x) => normalizar(x).includes(q))) return true;
  }
  return false;
}

/** Aplica todos los criterios de un QueryReq y devuelve registros ordenados. */
export function ejecutarQuery(
  album: AlbumMock,
  req: QueryReq,
): RegistroMock[] {
  let regs =
    req.tabla === "variantes"
      ? album.variantes.filter(
          (r) => req.idPrincipal == null || r.idPrincipal === req.idPrincipal,
        )
      : [...album.principal];

  if (!req.incluirOcultos) {
    regs = regs.filter((r) => !r.oculto);
  }

  if (req.grupo) {
    const g = album.grupos.find((x) => x.id === req.grupo!.grupoId);
    if (g) {
      const niveles = [g.por, g.luego1, g.luego2];
      regs = regs.filter((r) =>
        req.grupo!.valores.every((sel, i) => {
          const campo = niveles[i];
          if (sel == null || !campo) return true;
          return String(r.valores[campo] ?? "") === sel;
        }),
      );
    }
  }

  if (req.filtroRapido) {
    const { campo, valor } = req.filtroRapido;
    regs = regs.filter((r) =>
      cumpleCondicion(r, { opRel: null, campo, opComp: "igual", valor }, album.campos),
    );
  }

  if (req.condiciones.length > 0) {
    regs = regs.filter((r) => cumpleCondiciones(r, req.condiciones, album.campos));
  }

  if (req.busqueda && req.busqueda.trim() !== "") {
    regs = regs.filter((r) => coincideBusqueda(r, req.busqueda!));
  }

  if (req.orden.length > 0) {
    regs.sort((a, b) => {
      for (const o of req.orden as OrdenCampo[]) {
        const c = comparar(a.valores[o.campo] ?? null, b.valores[o.campo] ?? null);
        if (c !== 0) return o.direccion === "desc" ? -c : c;
      }
      return a.id - b.id;
    });
  }

  return regs;
}

/** Construye el árbol de valores distintos (con conteos) para un grupo. */
export function construirArbol(album: AlbumMock, grupoId: number): NodoGrupo[] {
  const g = album.grupos.find((x) => x.id === grupoId);
  if (!g) return [];
  const niveles = [g.por, g.luego1, g.luego2].filter(
    (x): x is string => x != null,
  );

  function nivel(regs: RegistroMock[], idx: number): NodoGrupo[] {
    if (idx >= niveles.length) return [];
    const campo = niveles[idx];
    const porValor = new Map<string, RegistroMock[]>();
    for (const r of regs) {
      const v = String(r.valores[campo] ?? "(vacío)");
      const lista = porValor.get(v);
      if (lista) lista.push(r);
      else porValor.set(v, [r]);
    }
    return [...porValor.entries()]
      .sort((a, b) => a[0].localeCompare(b[0], "es"))
      .map(([valor, lista]) => ({
        valor,
        conteo: lista.length,
        hijos: nivel(lista, idx + 1),
      }));
  }

  return nivel(album.principal, 0);
}

/**
 * Evalúa una fórmula de campo calculado: sustituye nombres de campo (el más
 * largo primero, para que "Precio Var" gane a "Precio") por sus valores
 * numéricos y evalúa la aritmética restante. Solo permite dígitos y
 * operadores tras la sustitución — suficiente para el mock.
 */
export function evaluarFormula(
  formula: string,
  valores: Valores,
  campos: CampoDef[],
): Valor {
  let expr = formula;
  // Acepta el nombre visible y su forma de fórmula (espacios → `_`), igual
  // que el motor real (normaliza_nombre del port de Module5.bas).
  const alias: [string, string][] = campos.flatMap((c) => {
    const formas: [string, string][] = [[c.nombre, c.nombre]];
    const conGuion = c.nombre.replace(/ /g, "_");
    if (conGuion !== c.nombre) formas.push([conGuion, c.nombre]);
    return formas;
  });
  alias.sort((a, b) => b[0].length - a[0].length);
  for (const [forma, nombre] of alias) {
    if (!expr.includes(forma)) continue;
    const v = aNumero(valores[nombre] ?? null) ?? 0;
    expr = expr.split(forma).join(`(${v})`);
  }
  if (!/^[\d\s+\-*/().,]+$/.test(expr)) return null;
  try {
    // Mock de desarrollo: la expresión ya quedó saneada por la regex previa.
    const r = new Function(`return (${expr.replace(/,/g, ".")});`)() as unknown;
    if (typeof r !== "number" || !isFinite(r)) return null;
    return Math.round(r * 100) / 100;
  } catch {
    return null;
  }
}

/** Recalcula los campos calculados de un registro (tras crear/editar). */
export function recalcular(
  reg: RegistroMock,
  campos: CampoDef[],
  tabla: "principal" | "variantes",
): void {
  for (const c of campos) {
    if (c.tabla !== tabla || c.tipo !== "calculado" || !c.formula) continue;
    reg.valores[c.nombre] = evaluarFormula(c.formula, reg.valores, campos);
  }
}
