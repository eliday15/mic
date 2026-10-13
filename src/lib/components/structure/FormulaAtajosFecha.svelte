<!--
  FormulaAtajosFecha — atajos de fórmulas con fechas para FormulaEditor:
  "Hoy" (@HOY), "Días entre fechas" (Fin − Inicio) y "Sumar días" (Fecha + 30).
  Los atajos usan los campos fecha disponibles; el editor inserta el texto.
  Cada atajo indica el tipo de resultado que produce (`onResultado`).
-->
<script lang="ts">
  import { CalendarDays } from "lucide-svelte";
  import { t } from "$lib/i18n/es";
  import { esCampoFecha } from "$lib/utils/format";
  import type { CampoDef } from "$lib/domain/types";

  interface Props {
    /** Campos referenciables por la fórmula. */
    campos: CampoDef[];
    /** Inserta un fragmento de fórmula en el cursor. */
    onInsertar: (fragmento: string) => void;
    /** Tipo de resultado del atajo usado: fecha o número (días). */
    onResultado?: (tipo: "fecha" | "numero") => void;
  }

  let { campos, onInsertar, onResultado }: Props = $props();

  /** Nombres en forma de fórmula (espacios → `_`) de los campos fecha. */
  const fechas = $derived(
    campos.filter(esCampoFecha).map((c) => c.nombre.replace(/ /g, "_")),
  );

  /** Fin − Inicio con los dos primeros campos fecha; con uno solo, hasta hoy. */
  function diasEntre(): void {
    const [inicio, fin] = fechas;
    onInsertar(fin ? `${fin} - ${inicio}` : `@HOY - ${inicio}`);
    onResultado?.("numero");
  }

  /** Fecha + 30: el resultado es otra fecha. */
  function sumarDias(): void {
    onInsertar(`${fechas[0]} + 30`);
    onResultado?.("fecha");
  }
</script>

<div class="fa">
  <span class="fa__sub"><CalendarDays size={12} /> {t.formulas.fechas}</span>
  <div class="fa__botones">
    <button
      type="button"
      class="fa__btn"
      title={t.formulas.hoyTitulo}
      onclick={() => onInsertar("@HOY")}
    >
      {t.formulas.hoy}
    </button>
    <button
      type="button"
      class="fa__btn"
      title={t.formulas.diasEntreTitulo}
      disabled={fechas.length === 0}
      onclick={diasEntre}
    >
      {t.formulas.diasEntre}
    </button>
    <button
      type="button"
      class="fa__btn"
      title={t.formulas.sumarDiasTitulo}
      disabled={fechas.length === 0}
      onclick={sumarDias}
    >
      {t.formulas.sumarDias}
    </button>
  </div>
  <p class="fa__ayuda">{t.formulas.ayudaFechas} {t.formulas.ejemploHoy}</p>
</div>

<style>
  .fa {
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
  }
  .fa__sub {
    display: inline-flex;
    align-items: center;
    gap: var(--esp-1);
    font-size: var(--tam-fuente-xs);
    color: var(--color-texto-tenue);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .fa__botones {
    display: flex;
    flex-wrap: wrap;
    gap: var(--esp-1);
  }
  .fa__btn {
    height: var(--alto-control-sm);
    padding: 0 var(--esp-2);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio-sm);
    background: var(--color-superficie);
    color: var(--color-texto-secundario);
    font-size: var(--tam-fuente-sm);
    cursor: pointer;
    transition:
      color var(--transicion),
      border-color var(--transicion);
  }
  .fa__btn:hover:not(:disabled) {
    color: var(--color-acento);
    border-color: var(--color-acento);
  }
  .fa__btn:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .fa__ayuda {
    margin: 0;
    font-size: var(--tam-fuente-xs);
    color: var(--color-texto-tenue);
  }
</style>
