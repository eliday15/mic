<!--
  PrintSheet — renderiza un reporte completo a HTML con CSS de impresión.

  Dos modos (ex clsReporteCI / clsReporteSI del VB6):
    - "ci": catálogo con imágenes (cuadrícula de N por línea, campos debajo).
    - "si": tabla densa; con `agrupacion` agrupa filas con subtotales por grupo
      y, si `ponTotales`, una fila de totales alineada a las columnas.
  En ambos modos, `ponTotales` añade al final un bloque de totales con el número
  de registros y la suma de TODOS los campos totalizables del álbum, se impriman
  o no como columna.

  La hoja se imprime con window.print() del webview. Mientras la clase
  print-sheet--imprimiendo esté presente, un bloque de estilo inyectado en el
  head define el @page (papel/orientación dinámicos) y oculta el resto de la app
  en @media print, sin tocar src/app.css.
-->
<script lang="ts">
  import { thumbUrl } from "$lib/ipc/thumbnails";
  import {
    formatearPorTipo,
    formatearEntero,
    formatearFecha,
    hoyIso,
    esCampoFecha,
  } from "$lib/utils/format";
  import { t } from "$lib/i18n/es";
  import { papelCss } from "./tipos";
  import type {
    CampoDef,
    RegistroLigero,
    Tabla,
    Valor,
  } from "$lib/domain/types";
  import type { ConfigReporte } from "./tipos";

  interface Props {
    config: ConfigReporte;
    registros: RegistroLigero[];
    /** Definiciones de campos del álbum (para tipo, decimales, totalizable). */
    campos: CampoDef[];
    albumId: number;
    tabla: Tabla;
    nombreAlbum: string;
    /** Marca activa la ocultación del resto de la app en `@media print`. */
    imprimiendo?: boolean;
  }

  let {
    config,
    registros,
    campos,
    albumId,
    tabla,
    nombreAlbum,
    imprimiendo = false,
  }: Props = $props();

  // Definición de un campo por su nombre visible.
  const defPorNombre = $derived(
    new Map(campos.map((c) => [c.nombre, c] as const)),
  );

  // Campos a imprimir, resueltos a su definición (los que existan, en orden).
  const camposImpr = $derived(
    config.campos
      .map((nombre) => defPorNombre.get(nombre))
      .filter((c): c is CampoDef => c !== undefined),
  );

  // True si el campo es de alineación derecha (numérico). Un calculado con
  // resultado fecha no es numérico (ni se totaliza).
  function esNum(c: CampoDef): boolean {
    return (
      c.tipo === "numerico" ||
      c.tipo === "moneda" ||
      (c.tipo === "calculado" && !esCampoFecha(c))
    );
  }

  // Campos totalizables entre los impresos (numéricos/moneda totalizables).
  const totalizables = $derived(
    camposImpr.filter((c) => c.totalizable && esNum(c)),
  );

  // Todos los totalizables de la tabla del reporte, en su orden visible.
  const totalizablesAlbum = $derived(
    campos
      .filter((c) => c.tabla === tabla && c.totalizable && esNum(c))
      .slice()
      .sort((a, b) => a.ordenVisible - b.ordenVisible),
  );

  const titulo = $derived(config.titulo.trim() || nombreAlbum);

  /** Formatea un valor del registro según el tipo de su campo. */
  function fmt(reg: RegistroLigero, c: CampoDef): string {
    return formatearPorTipo(reg.valores[c.nombre] ?? null, c.tipo, c.decimales, c.formato);
  }

  /** Suma numérica de un valor (para totales/subtotales). */
  function aNumero(v: Valor): number {
    if (typeof v === "number") return Number.isFinite(v) ? v : 0;
    if (typeof v === "boolean") return v ? 1 : 0;
    if (typeof v === "string") {
      const n = Number(v.replace(/,/g, ""));
      return Number.isNaN(n) ? 0 : n;
    }
    return 0;
  }

  /** Suma de un campo totalizable sobre un conjunto de registros. */
  function suma(regs: RegistroLigero[], c: CampoDef): number {
    return regs.reduce(
      (acc, r) => acc + aNumero(r.valores[c.nombre] ?? null),
      0,
    );
  }

  /** Formatea una suma como su campo (moneda/numérico). */
  function fmtSuma(c: CampoDef, valor: number): string {
    return formatearPorTipo(valor, c.tipo, c.decimales, c.formato);
  }

  // Ancho relativo de cada columna de la tabla (layout fijo al ancho del
  // papel): el texto libre recibe algo más de espacio que números y fechas,
  // pero estos conservan lo suficiente para no partirse.
  function pesoColumna(c: CampoDef): number {
    if (c.tipo === "texto") return 1.5;
    if (c.tipo === "multidato") return 1.3;
    return 1.2;
  }
  const pesoTotal = $derived(camposImpr.reduce((acc, c) => acc + pesoColumna(c), 0) || 1);

  // Columna donde va la etiqueta "Subtotal"/"Total": la primera sin suma (si
  // la primera columna es totalizable, la etiqueta no debe pisar su valor).
  const colEtiqueta = $derived(
    camposImpr.findIndex((c) => !totalizables.some((x) => x.id === c.id)),
  );

  /** Campo totalizable que corresponde a una columna (o null). */
  function totalDeColumna(c: CampoDef): CampoDef | null {
    return totalizables.find((t) => t.id === c.id) ?? null;
  }

  // --- Agrupación (solo "si") --------------------------------------------
  interface GrupoFilas {
    clave: string;
    registros: RegistroLigero[];
  }

  const grupos = $derived.by<GrupoFilas[]>(() => {
    if (config.tipo !== "si" || !config.agrupacion) return [];
    const campo = config.agrupacion;
    const def = defPorNombre.get(campo);
    const mapa = new Map<string, RegistroLigero[]>();
    for (const r of registros) {
      const raw = r.valores[campo];
      const clave =
        raw === null || raw === undefined || raw === ""
          ? "—"
          : def
            ? formatearPorTipo(raw, def.tipo, def.decimales, def.formato) || String(raw)
            : String(raw);
      const lista = mapa.get(clave);
      if (lista) lista.push(r);
      else mapa.set(clave, [r]);
    }
    return [...mapa.entries()]
      .sort((a, b) => a[0].localeCompare(b[0], "es"))
      .map(([clave, registros]) => ({ clave, registros }));
  });

  const fechaHoy = $derived(formatearFecha(hoyIso()));

  // Cuadrícula de imágenes (tipo "ci"): ancho de columna en % por nº por línea.
  const anchoColCi = $derived(`${100 / config.imagenesPorLinea}%`);

  // CSS de impresión inyectado en el head solo al imprimir. El @page con papel
  // y orientación dinámicos no puede expresarse de forma estática en el bloque
  // de estilos; se inyecta imperativamente en document.head (y se retira al salir).
  const cssImpresion = $derived(
    `@page { size: ${papelCss(config.papel)} ${
      config.orientacion === "horizontal" ? "landscape" : "portrait"
    }; margin: 14mm;${
      config.ponPagina
        ? ` @bottom-right { content: "${t.reportes.pagina} " counter(page); font-size: 9px; color: #555; }`
        : ""
    } }
@media print {
  body > *:not(:has(.print-sheet--imprimiendo)) { display: none !important; }
  body:has(.print-sheet--imprimiendo) { background: #fff !important; }
}`,
  );

  $effect(() => {
    if (!imprimiendo) return;
    const el = document.createElement("style");
    el.setAttribute("data-print-sheet", "");
    el.textContent = cssImpresion;
    document.head.appendChild(el);
    return () => el.remove();
  });
</script>

<div
  class="print-sheet print-sheet--{config.papel} print-sheet--{config.orientacion}"
  class:print-sheet--imprimiendo={imprimiendo}
>
  <header class="ps__cab">
    <h1 class="ps__titulo">{titulo}</h1>
    {#if config.ponFecha}
      <span class="ps__fecha">{fechaHoy}</span>
    {/if}
  </header>

  {#if config.tipo === "ci"}
    <!-- Catálogo con imágenes -->
    <div class="ps__rejilla">
      {#each registros as reg (reg.id)}
        <div class="ps__celda" style:width={anchoColCi}>
          <div class="ps__img-marco">
            {#if reg.imagen}
              <img
                class="ps__img"
                src={thumbUrl(
                  albumId,
                  tabla,
                  reg.id,
                  512,
                  reg.imagenVersion ?? 0,
                )}
                alt=""
              />
            {:else}
              <div class="ps__img-vacia"></div>
            {/if}
          </div>
          <dl class="ps__datos">
            {#each camposImpr as c (c.id)}
              <div class="ps__dato">
                <dt class="ps__etq">{c.nombre}:</dt>
                <dd class="ps__val">{fmt(reg, c)}</dd>
              </div>
            {/each}
          </dl>
        </div>
      {/each}
    </div>

  {:else}
    <!-- Tabla densa sin imágenes -->
    <table
      class="ps__tabla"
      class:ps__tabla--densa={camposImpr.length > 8}
      class:ps__tabla--muy-densa={camposImpr.length > 11}
    >
      <colgroup>
        {#each camposImpr as c (c.id)}
          <col style:width="{(pesoColumna(c) / pesoTotal) * 100}%" />
        {/each}
      </colgroup>
      <thead>
        <tr>
          {#each camposImpr as c (c.id)}
            <th class="ps__th" class:ps__th--num={esNum(c)}>{c.nombre}</th>
          {/each}
        </tr>
      </thead>
      <tbody>
        {#if config.agrupacion}
          {#each grupos as g (g.clave)}
            <tr class="ps__grupo">
              <td class="ps__grupo-cab" colspan={camposImpr.length}>
                {config.agrupacion}: {g.clave}
                <span class="ps__grupo-conteo">({g.registros.length})</span>
              </td>
            </tr>
            {#each g.registros as reg (reg.id)}
              <tr>
                {#each camposImpr as c (c.id)}
                  <td class="ps__td" class:ps__td--num={esNum(c)}>{fmt(reg, c)}</td>
                {/each}
              </tr>
            {/each}
            {#if totalizables.length > 0}
              <tr class="ps__subtotal">
                {#each camposImpr as c, i (c.id)}
                  {@const tot = totalDeColumna(c)}
                  <td
                    class="ps__td ps__td--subtotal"
                    class:ps__td--num={tot !== null}
                  >
                    {#if tot}
                      {fmtSuma(tot, suma(g.registros, tot))}
                    {:else if i === colEtiqueta}
                      Subtotal
                    {/if}
                  </td>
                {/each}
              </tr>
            {/if}
          {/each}
        {:else}
          {#each registros as reg (reg.id)}
            <tr>
              {#each camposImpr as c (c.id)}
                <td class="ps__td" class:ps__td--num={esNum(c)}>{fmt(reg, c)}</td>
              {/each}
            </tr>
          {/each}
        {/if}
      </tbody>
      {#if config.ponTotales && totalizables.length > 0}
        <tfoot>
          <tr class="ps__total-grl">
            {#each camposImpr as c, i (c.id)}
              {@const tot = totalDeColumna(c)}
              <td class="ps__td ps__td--total" class:ps__td--num={tot !== null}>
                {#if tot}
                  {fmtSuma(tot, suma(registros, tot))}
                {:else if i === colEtiqueta}
                  {t.reportes.total}
                {/if}
              </td>
            {/each}
          </tr>
        </tfoot>
      {/if}
    </table>
  {/if}

  {#if config.ponTotales}
    <section class="ps__totales" aria-label={t.reportes.totales}>
      <h2 class="ps__totales-titulo">{t.reportes.totales}</h2>
      <dl class="ps__totales-lista">
        <div class="ps__total">
          <dt>{t.reportes.totalRegistros}</dt>
          <dd>{formatearEntero(registros.length)}</dd>
        </div>
        {#each totalizablesAlbum as c (c.id)}
          <div class="ps__total">
            <dt>{c.nombre}</dt>
            <dd>{fmtSuma(c, suma(registros, c))}</dd>
          </div>
        {/each}
      </dl>
      {#if totalizablesAlbum.length === 0}
        <p class="ps__totales-nota">{t.reportes.sinTotalizables}</p>
      {/if}
    </section>
  {/if}
</div>

<style>
  /* Hoja en pantalla (vista previa): aspecto de papel con sombra suave. */
  .print-sheet {
    box-sizing: border-box;
    width: 216mm; /* carta vertical por defecto */
    min-height: 279mm;
    margin: 0 auto;
    padding: 14mm;
    background: #fff;
    color: #111;
    font-family: system-ui, sans-serif;
    font-size: 11px;
    line-height: 1.35;
    box-shadow: 0 2px 16px rgba(0, 0, 0, 0.25);
  }
  .print-sheet--oficio {
    min-height: 356mm;
  }
  .print-sheet--a4 {
    width: 210mm;
    min-height: 297mm;
  }
  .print-sheet--horizontal.print-sheet--carta {
    width: 279mm;
    min-height: 216mm;
  }
  .print-sheet--horizontal.print-sheet--oficio {
    width: 356mm;
    min-height: 216mm;
  }
  .print-sheet--horizontal.print-sheet--a4 {
    width: 297mm;
    min-height: 210mm;
  }

  .ps__cab {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8mm;
    padding-bottom: 4mm;
    margin-bottom: 6mm;
    border-bottom: 1.5px solid #111;
  }
  .ps__titulo {
    margin: 0;
    font-size: 18px;
    font-weight: 700;
  }
  .ps__fecha {
    font-size: 11px;
    color: #444;
    white-space: nowrap;
  }

  /* --- Catálogo con imágenes --- */
  .ps__rejilla {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
  }
  .ps__celda {
    box-sizing: border-box;
    padding: 3mm;
    page-break-inside: avoid;
    break-inside: avoid;
  }
  .ps__img-marco {
    width: 100%;
    aspect-ratio: 1 / 1;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    background: #f3f3f3;
    border: 1px solid #ddd;
  }
  .ps__img {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
  }
  .ps__img-vacia {
    width: 100%;
    height: 100%;
    background: repeating-linear-gradient(
      45deg,
      #f3f3f3,
      #f3f3f3 6px,
      #eaeaea 6px,
      #eaeaea 12px
    );
  }
  .ps__datos {
    margin: 2mm 0 0;
  }
  .ps__dato {
    display: flex;
    gap: 4px;
    font-size: 10px;
  }
  .ps__etq {
    margin: 0;
    font-weight: 600;
    color: #333;
  }
  .ps__val {
    margin: 0;
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* --- Bloque de totales (ambos modos) --- */
  .ps__totales {
    margin-top: 6mm;
    padding-top: 3mm;
    border-top: 1.5px solid #111;
    page-break-inside: avoid;
    break-inside: avoid;
  }
  .ps__totales-titulo {
    margin: 0 0 2mm;
    font-size: 12px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .ps__totales-lista {
    display: flex;
    flex-wrap: wrap;
    gap: 3mm 10mm;
    margin: 0;
  }
  .ps__total {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .ps__total dt {
    font-size: 10px;
    color: #555;
  }
  .ps__total dd {
    margin: 0;
    font-size: 13px;
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .ps__totales-nota {
    margin: 2mm 0 0;
    font-size: 10px;
    color: #666;
  }

  /* --- Tabla densa --- */
  /* Layout fijo: la tabla nunca rebasa el ancho del papel; el texto largo se
     parte en varias líneas en lugar de desbordar. */
  .ps__tabla {
    width: 100%;
    table-layout: fixed;
    border-collapse: collapse;
    font-size: 10px;
  }
  .ps__tabla--densa {
    font-size: 8.5px;
  }
  .ps__tabla--muy-densa {
    font-size: 7.5px;
  }
  .ps__tabla--densa .ps__th,
  .ps__tabla--densa .ps__td {
    padding: 2px 3px;
  }
  .ps__th {
    padding: 2px 6px;
    text-align: left;
    font-weight: 700;
    border-bottom: 1.5px solid #111;
    background: #f4f4f4;
    vertical-align: bottom;
    overflow-wrap: anywhere;
  }
  .ps__th--num {
    text-align: right;
  }
  .ps__td {
    padding: 2px 6px;
    border-bottom: 0.5px solid #ddd;
    vertical-align: top;
    overflow-wrap: anywhere;
  }
  .ps__td--num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .ps__grupo-cab {
    padding: 4px 6px 2px;
    font-weight: 700;
    background: #ececec;
    border-bottom: 1px solid #bbb;
  }
  .ps__grupo-conteo {
    font-weight: 400;
    color: #666;
  }
  .ps__subtotal .ps__td--subtotal {
    font-weight: 600;
    border-top: 1px solid #999;
    border-bottom: 1px solid #999;
    background: #f7f7f7;
  }
  .ps__total-grl .ps__td {
    font-weight: 700;
    border-top: 1.5px solid #111;
    background: #f0f0f0;
  }
  .ps__grupo,
  .ps__subtotal,
  .ps__total-grl {
    page-break-inside: avoid;
    break-inside: avoid;
  }

  /* En impresión, la hoja ocupa la página completa sin sombra ni márgenes. */
  @media print {
    .print-sheet {
      box-shadow: none;
      margin: 0;
      width: auto;
      min-height: 0;
      padding: 0;
    }
  }
</style>
