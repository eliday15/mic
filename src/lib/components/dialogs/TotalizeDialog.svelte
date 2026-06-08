<!--
  TotalizeDialog — panel de totales y estadísticas (ex-frmTotalizar ampliado).
  Para los campos numéricos elegidos muestra Cuenta, Suma, Media, Mediana,
  Moda, Mín y Máx sobre el conjunto filtrado actual. Práctico:
    - clic en el encabezado de una columna → ordena la tabla por esa medida
    - clic en cualquier valor → lo copia al portapapeles
    - botón ⇅ por fila → ordena la grilla del álbum por ese campo (asc/desc)
-->
<script lang="ts">
  import { ArrowDownUp, ArrowDown, ArrowUp } from "lucide-svelte";
  import { Modal, Button, Spinner, EmptyState, IconButton } from "$lib/components/ui";
  import { registrosEstadisticas } from "$lib/ipc/commands";
  import { formatearEntero, formatearPorTipo } from "$lib/utils/format";
  import { ui } from "$lib/stores/ui.svelte";
  import { t } from "$lib/i18n/es";
  import type { AlbumState } from "$lib/stores/albumState.svelte";
  import type { CampoDef, Estadisticas, EstadisticaCampo } from "$lib/domain/types";

  interface Props {
    abierto?: boolean;
    estado: AlbumState;
    onCerrar?: () => void;
  }

  let { abierto = $bindable(true), estado, onCerrar }: Props = $props();

  // Campos analizables: numéricos de la tabla activa.
  const numericos = $derived(
    estado.campos
      .filter(
        (c) =>
          c.tabla === estado.tabla &&
          (c.tipo === "numerico" || c.tipo === "moneda" || c.tipo === "calculado"),
      )
      .sort((a, b) => a.ordenVisible - b.ordenVisible),
  );

  /** Selección inicial: los totalizables; si no hay, todos los numéricos. */
  function seleccionInicial(): string[] {
    const nums = estado.campos.filter(
      (c) =>
        c.tabla === estado.tabla &&
        (c.tipo === "numerico" || c.tipo === "moneda" || c.tipo === "calculado"),
    );
    const tot = nums.filter((c) => c.totalizable).map((c) => c.nombre);
    return tot.length > 0 ? tot : nums.map((c) => c.nombre);
  }

  // svelte-ignore state_referenced_locally
  let seleccion = $state<string[]>(seleccionInicial());
  let cargando = $state(true);
  let resultado = $state<Estadisticas | null>(null);

  function alternarCampo(nombre: string): void {
    seleccion = seleccion.includes(nombre)
      ? seleccion.filter((n) => n !== nombre)
      : [...seleccion, nombre];
  }

  // Recarga al abrir y al cambiar la selección de campos.
  $effect(() => {
    if (!abierto) return;
    const campos = [...seleccion];
    cargando = true;
    registrosEstadisticas(estado.albumId, estado.construirQuery(0, 1), campos)
      .then((r) => (resultado = r))
      .catch((e) => {
        ui.error(typeof e === "string" ? e : t.error.generico);
        cerrar();
      })
      .finally(() => (cargando = false));
  });

  // --- Orden de la tabla de estadísticas ---------------------------------
  type Medida =
    | "campo"
    | "cuenta"
    | "suma"
    | "media"
    | "mediana"
    | "moda"
    | "minimo"
    | "maximo";
  let ordenPor = $state<Medida>("campo");
  let ordenDesc = $state(false);

  const COLUMNAS: { id: Medida; etiqueta: string }[] = [
    { id: "cuenta", etiqueta: t.totalizar.cuenta },
    { id: "suma", etiqueta: t.totalizar.suma },
    { id: "media", etiqueta: t.totalizar.promedio },
    { id: "mediana", etiqueta: t.totalizar.mediana },
    { id: "moda", etiqueta: t.totalizar.moda },
    { id: "minimo", etiqueta: t.totalizar.minimo },
    { id: "maximo", etiqueta: t.totalizar.maximo },
  ];

  function ordenarPor(m: Medida): void {
    if (ordenPor === m) ordenDesc = !ordenDesc;
    else {
      ordenPor = m;
      ordenDesc = m !== "campo"; // medidas: primero lo mayor
    }
  }

  const filas = $derived.by(() => {
    const lista = resultado?.campos.slice() ?? [];
    lista.sort((a, b) => {
      let c: number;
      if (ordenPor === "campo") c = a.campo.localeCompare(b.campo, "es");
      else c = ((a[ordenPor] ?? -Infinity) as number) - ((b[ordenPor] ?? -Infinity) as number);
      return ordenDesc ? -c : c;
    });
    return lista;
  });

  // --- Formato y acciones prácticas --------------------------------------
  function defDe(nombre: string): CampoDef | undefined {
    return estado.campos.find((c) => c.nombre === nombre);
  }

  function fmt(fila: EstadisticaCampo, m: Medida): string {
    if (m === "campo") return fila.campo;
    if (m === "cuenta") return formatearEntero(fila.cuenta);
    const v = fila[m];
    if (v === null || v === undefined) return "—";
    const def = defDe(fila.campo);
    return formatearPorTipo(
      v,
      def?.tipo ?? "numerico",
      def?.decimales ?? 2,
      def?.formato ?? null,
    );
  }

  async function copiar(texto: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(texto);
      ui.exito(`${texto} — ${t.mensaje.copiado}`);
    } catch {
      /* portapapeles no disponible */
    }
  }

  /** Ordena la grilla del álbum por el campo (asc ⇄ desc en clics sucesivos). */
  function ordenarGrilla(campo: string): void {
    const actual = estado.orden[0];
    const direccion =
      actual?.campo === campo && actual.direccion === "desc" ? "asc" : "desc";
    estado.setOrden([{ campo, direccion }]);
    ui.exito(`${t.totalizar.ordenarGrilla}: ${campo} (${direccion})`);
  }

  function cerrar(): void {
    abierto = false;
    onCerrar?.();
  }
</script>

<Modal bind:abierto titulo={t.totalizar.titulo} ancho="lg" onCerrar={cerrar}>
  <div class="tot">
    <!-- Selector de campos a analizar -->
    {#if numericos.length === 0}
      <EmptyState
        titulo={t.totalizar.sinTotalizables}
        descripcion={t.totalizar.sinTotalizablesDesc}
      />
    {:else}
      <div class="tot__sel">
        <span class="tot__etq">{t.totalizar.elegirCampos}</span>
        <div class="tot__chips">
          {#each numericos as c (c.id)}
            <button
              type="button"
              class="tot__chip"
              class:tot__chip--on={seleccion.includes(c.nombre)}
              aria-pressed={seleccion.includes(c.nombre)}
              onclick={() => alternarCampo(c.nombre)}
            >
              {c.nombre}
            </button>
          {/each}
        </div>
      </div>

      {#if cargando}
        <div class="tot__centro"><Spinner /></div>
      {:else if resultado}
        <div class="tot__fila tot__fila--conteo">
          <span>{t.totalizar.registros}</span>
          <strong class="tabular">{formatearEntero(resultado.registros)}</strong>
        </div>

        <div class="tot__tabla" role="table">
          <div class="tot__head" role="row">
            <button
              type="button"
              class="tot__th tot__th--campo"
              onclick={() => ordenarPor("campo")}
            >
              {t.totalizar.campo}
              {#if ordenPor === "campo"}
                {#if ordenDesc}<ArrowDown size={11} />{:else}<ArrowUp size={11} />{/if}
              {/if}
            </button>
            {#each COLUMNAS as col (col.id)}
              <button
                type="button"
                class="tot__th"
                onclick={() => ordenarPor(col.id)}
              >
                {col.etiqueta}
                {#if ordenPor === col.id}
                  {#if ordenDesc}<ArrowDown size={11} />{:else}<ArrowUp size={11} />{/if}
                {/if}
              </button>
            {/each}
            <span class="tot__th tot__th--acc"></span>
          </div>

          {#each filas as fila (fila.campo)}
            <div class="tot__row" role="row">
              <span class="tot__celda tot__celda--campo" title={fila.campo}>
                {fila.campo}
              </span>
              {#each COLUMNAS as col (col.id)}
                <button
                  type="button"
                  class="tot__celda tabular"
                  title={col.id === "moda" && fila.moda !== null
                    ? `${formatearEntero(fila.modaConteo)} ${t.totalizar.veces}`
                    : t.mensaje.copiado}
                  onclick={() => copiar(fmt(fila, col.id))}
                >
                  {fmt(fila, col.id)}
                </button>
              {/each}
              <span class="tot__celda tot__celda--acc">
                <IconButton
                  etiqueta={t.totalizar.ordenarGrilla}
                  tamano="sm"
                  title={t.totalizar.ordenarGrilla}
                  activo={estado.orden[0]?.campo === fila.campo}
                  onclick={() => ordenarGrilla(fila.campo)}
                >
                  <ArrowDownUp size={13} />
                </IconButton>
              </span>
            </div>
          {/each}
        </div>

        <p class="tot__nota">
          {t.totalizar.clicCopia}{estado.hayFiltros ? ` · ${t.totalizar.nota}` : ""}
        </p>
      {/if}
    {/if}
  </div>

  {#snippet pie()}
    <Button variante="primario" onclick={cerrar}>{t.accion.cerrar}</Button>
  {/snippet}
</Modal>

<style>
  .tot {
    display: flex;
    flex-direction: column;
    gap: var(--esp-3);
  }
  .tot__centro {
    display: flex;
    justify-content: center;
    padding: var(--esp-5) 0;
  }
  .tot__sel {
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
  }
  .tot__etq {
    font-size: var(--tam-fuente-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-texto-secundario);
  }
  .tot__chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--esp-1);
  }
  .tot__chip {
    height: var(--alto-control-sm);
    padding: 0 var(--esp-2);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio-pill);
    background: var(--color-superficie);
    color: var(--color-texto-secundario);
    font-size: var(--tam-fuente-sm);
    cursor: pointer;
    transition:
      color var(--transicion),
      border-color var(--transicion),
      background var(--transicion);
  }
  .tot__chip:hover {
    border-color: var(--color-acento);
    color: var(--color-texto);
  }
  .tot__chip--on {
    background: var(--color-acento);
    border-color: var(--color-acento);
    color: #fff;
  }

  .tot__fila--conteo {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    padding: var(--esp-1) var(--esp-2);
    border-bottom: 1px solid var(--color-borde);
    color: var(--color-texto-secundario);
  }

  .tot__tabla {
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow-x: auto;
  }
  .tot__head,
  .tot__row {
    display: grid;
    grid-template-columns: minmax(110px, 1.3fr) repeat(7, minmax(86px, 1fr)) 34px;
    align-items: center;
    gap: var(--esp-1);
    min-width: 780px;
  }
  .tot__th {
    display: inline-flex;
    align-items: center;
    justify-content: flex-end;
    gap: 3px;
    padding: var(--esp-1) var(--esp-1);
    border: none;
    background: transparent;
    color: var(--color-texto-tenue);
    font-size: var(--tam-fuente-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    cursor: pointer;
  }
  .tot__th:hover {
    color: var(--color-acento);
  }
  .tot__th--campo {
    justify-content: flex-start;
  }
  .tot__row {
    padding: 2px var(--esp-1);
    border-radius: var(--radio-sm);
  }
  .tot__row:nth-child(even) {
    background: var(--color-superficie);
  }
  .tot__celda {
    padding: 2px var(--esp-1);
    border: none;
    background: transparent;
    color: var(--color-texto);
    font-size: var(--tam-fuente-sm);
    text-align: right;
    border-radius: 3px;
    cursor: copy;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tot__celda:hover {
    background: var(--color-hover);
  }
  .tot__celda--campo {
    font-weight: 600;
    text-align: left;
    cursor: default;
  }
  .tot__celda--campo:hover {
    background: transparent;
  }
  .tot__celda--acc {
    cursor: default;
    text-align: center;
  }
  .tot__celda--acc:hover {
    background: transparent;
  }
  .tot__nota {
    margin: 0;
    font-size: var(--tam-fuente-xs);
    color: var(--color-texto-tenue);
  }
</style>
