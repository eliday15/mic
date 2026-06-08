<!--
  FormulaEditor — editor de la fórmula de un campo calculado. Permite insertar
  nombres de campo en el cursor y prueba la expresión en vivo con `formula_probar`
  (debounced) usando valores de ejemplo (1 para numéricos, fecha de hoy).
-->
<script lang="ts">
  import { Calculator, Delete, Eraser } from "lucide-svelte";
  import { formulaProbar } from "$lib/ipc/commands";
  import { debounce } from "$lib/utils/debounce";
  import { hoyIso, formatearNumero } from "$lib/utils/format";
  import { t } from "$lib/i18n/es";
  import type { CampoDef, Valor, Valores } from "$lib/domain/types";

  interface Props {
    albumId: number;
    /** Fórmula enlazada bidireccionalmente. */
    formula?: string;
    /** Campos disponibles para insertar (excluye el propio si se indica). */
    campos: CampoDef[];
  }

  let { albumId, formula = $bindable(""), campos }: Props = $props();

  let area = $state<HTMLTextAreaElement | null>(null);
  let resultado = $state<Valor>(null);
  let error = $state<string | null>(null);

  // Solo se pueden referenciar campos numéricos, moneda, fecha y calculados.
  const referenciables = $derived(
    campos.filter(
      (c) =>
        c.tipo === "numerico" ||
        c.tipo === "moneda" ||
        c.tipo === "fecha" ||
        c.tipo === "calculado",
    ),
  );

  function valoresEjemplo(): Valores {
    const v: Valores = {};
    for (const c of referenciables) {
      v[c.nombre] = c.tipo === "fecha" ? hoyIso() : 1;
    }
    return v;
  }

  const probar = debounce(async (expr: string) => {
    if (expr.trim() === "") {
      resultado = null;
      error = null;
      return;
    }
    try {
      resultado = await formulaProbar(albumId, expr, valoresEjemplo());
      error = null;
    } catch (e) {
      error = typeof e === "string" ? e : t.formulas.error;
      resultado = null;
    }
  }, 350);

  $effect(() => {
    probar(formula);
  });

  /** Inserta un token en la posición del cursor (con espacios alrededor). */
  function insertarToken(token: string): void {
    const conEspacios =
      formula === "" || formula.endsWith(" ") ? token : ` ${token}`;
    if (!area) {
      formula += conEspacios;
      return;
    }
    const inicio = area.selectionStart ?? formula.length;
    const fin = area.selectionEnd ?? formula.length;
    formula = formula.slice(0, inicio) + conEspacios + formula.slice(fin);
    queueMicrotask(() => {
      if (!area) return;
      const pos = inicio + conEspacios.length;
      area.focus();
      area.setSelectionRange(pos, pos);
    });
  }

  function insertar(nombre: string): void {
    // Forma nativa del motor de cálculo (port de Module5.bas): el nombre tal
    // cual, con los espacios escritos como guion bajo (`Precio Venta` →
    // `Precio_Venta`). Sin llaves: el lexer no las reconoce.
    insertarToken(nombre.replace(/ /g, "_"));
  }

  /** Operadores disponibles para construir la fórmula con clics. */
  const OPERADORES: { ver: string; token: string }[] = [
    { ver: "+", token: "+" },
    { ver: "−", token: "-" },
    { ver: "×", token: "*" },
    { ver: "÷", token: "/" },
    { ver: "(", token: "(" },
    { ver: ")", token: ")" },
  ];

  /** Número a insertar con el mini teclado (constructor sin escritura). */
  let numero = $state<string>("");

  function insertarNumero(): void {
    const n = numero.trim().replace(",", ".");
    if (n === "" || isNaN(Number(n))) return;
    insertarToken(n);
    numero = "";
  }

  /** Quita el último token de la fórmula (deshacer del constructor). */
  function quitarUltimo(): void {
    formula = formula.replace(/\s*\S+\s*$/, "");
  }

  function vistaResultado(): string {
    if (resultado === null) return "—";
    if (typeof resultado === "number") return formatearNumero(resultado, 2);
    return String(resultado);
  }
</script>

<div class="fe">
  <label class="fe__etq" for="fe-area">{t.formulas.expresion}</label>
  <textarea
    bind:this={area}
    id="fe-area"
    class="fe__area"
    rows="3"
    bind:value={formula}
    placeholder="Precio * Cantidad"
  ></textarea>

  <p class="fe__ayuda">{t.formulas.ayuda}</p>

  <!-- Constructor por selección: campos, operadores y números con clic. -->
  {#if referenciables.length > 0}
    <div class="fe__bloque">
      <span class="fe__sub">{t.formulas.campos}</span>
      <div class="fe__campos">
        {#each referenciables as c (c.id)}
          <button
            type="button"
            class="fe__campo"
            onclick={() => insertar(c.nombre)}
          >
            {c.nombre}
          </button>
        {/each}
      </div>
    </div>
  {/if}

  <div class="fe__fila">
    <div class="fe__bloque">
      <span class="fe__sub">{t.formulas.operadores}</span>
      <div class="fe__ops">
        {#each OPERADORES as op (op.token)}
          <button
            type="button"
            class="fe__op"
            onclick={() => insertarToken(op.token)}
          >
            {op.ver}
          </button>
        {/each}
      </div>
    </div>

    <div class="fe__bloque">
      <span class="fe__sub">{t.formulas.numero}</span>
      <div class="fe__numfila">
        <input
          class="fe__numinput"
          type="number"
          step="any"
          bind:value={numero}
          placeholder="0"
          onkeydown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              insertarNumero();
            }
          }}
        />
        <button
          type="button"
          class="fe__op"
          title={t.formulas.insertarNumero}
          disabled={numero === "" || isNaN(Number(String(numero).replace(",", ".")))}
          onclick={insertarNumero}
        >
          ↵
        </button>
      </div>
    </div>

    <div class="fe__bloque fe__bloque--der">
      <span class="fe__sub">&nbsp;</span>
      <div class="fe__ops">
        <button
          type="button"
          class="fe__op"
          title={t.formulas.quitarUltimo}
          disabled={formula.trim() === ""}
          onclick={quitarUltimo}
        >
          <Delete size={13} />
        </button>
        <button
          type="button"
          class="fe__op"
          title={t.formulas.limpiarFormula}
          disabled={formula.trim() === ""}
          onclick={() => (formula = "")}
        >
          <Eraser size={13} />
        </button>
      </div>
    </div>
  </div>

  <div class="fe__prueba" class:fe__prueba--error={error !== null}>
    <Calculator size={14} />
    {#if error !== null}
      <span class="fe__err">{error}</span>
    {:else}
      <span class="fe__vista">{t.formulas.vistaPrevia}:</span>
      <span class="fe__res tabular">{vistaResultado()}</span>
    {/if}
  </div>
</div>

<style>
  .fe {
    display: flex;
    flex-direction: column;
    gap: var(--esp-2);
  }
  .fe__etq {
    font-size: var(--tam-fuente-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-texto-secundario);
  }
  .fe__area {
    width: 100%;
    padding: var(--esp-2);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio);
    background: var(--color-superficie);
    color: var(--color-texto);
    font-family: var(--fuente-mono);
    font-size: var(--tam-fuente-sm);
    resize: vertical;
    outline: none;
  }
  .fe__area:focus {
    border-color: var(--color-acento);
    box-shadow: 0 0 0 2px var(--color-acento-tenue);
  }
  .fe__ayuda {
    margin: 0;
    font-size: var(--tam-fuente-xs);
    color: var(--color-texto-tenue);
  }
  .fe__campos {
    display: flex;
    flex-wrap: wrap;
    gap: var(--esp-1);
  }

  .fe__bloque {
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
  }
  .fe__bloque--der {
    margin-left: auto;
  }
  .fe__sub {
    font-size: var(--tam-fuente-xs);
    color: var(--color-texto-tenue);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .fe__fila {
    display: flex;
    align-items: flex-end;
    gap: var(--esp-4);
    flex-wrap: wrap;
  }
  .fe__ops {
    display: flex;
    gap: var(--esp-1);
  }
  .fe__op {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 28px;
    height: var(--alto-control-sm);
    padding: 0 var(--esp-1);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio-sm);
    background: var(--color-superficie);
    color: var(--color-texto);
    font-size: var(--tam-fuente-sm);
    font-family: var(--fuente-mono);
    cursor: pointer;
    transition:
      color var(--transicion),
      border-color var(--transicion);
  }
  .fe__op:hover:not(:disabled) {
    color: var(--color-acento);
    border-color: var(--color-acento);
  }
  .fe__op:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .fe__numfila {
    display: flex;
    gap: var(--esp-1);
  }
  .fe__numinput {
    width: 90px;
    height: var(--alto-control-sm);
    padding: 0 var(--esp-2);
    border: 1px solid var(--color-borde);
    border-radius: var(--radio-sm);
    background: var(--color-superficie);
    color: var(--color-texto);
    font-size: var(--tam-fuente-sm);
    outline: none;
  }
  .fe__numinput:focus {
    border-color: var(--color-acento);
  }
  .fe__campo {
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
      border-color var(--transicion);
  }
  .fe__campo:hover {
    color: var(--color-acento);
    border-color: var(--color-acento);
  }
  .fe__prueba {
    display: flex;
    align-items: center;
    gap: var(--esp-2);
    padding: var(--esp-2);
    border-radius: var(--radio);
    background: var(--color-panel);
    color: var(--color-texto-secundario);
    font-size: var(--tam-fuente-sm);
  }
  .fe__prueba--error {
    color: var(--color-peligro);
  }
  .fe__res {
    color: var(--color-texto);
    font-weight: 600;
  }
</style>
