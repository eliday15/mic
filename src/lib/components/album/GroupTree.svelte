<!--
  GroupTree — panel de grupos jerárquicos. Lista los grupos definidos
  (`grupos_listar`), y al elegir uno carga su árbol de valores por nivel
  (`grupo_arbol`) en un TreeView. Seleccionar un nodo fija `grupoSel` en el
  estado (filtrando la grilla). Permite crear/editar/eliminar grupos.
-->
<script lang="ts">
  import { Plus, Pencil, Trash2, FolderTree } from "lucide-svelte";
  import {
    TreeView,
    Select,
    Button,
    TextInput,
    Modal,
    ConfirmDialog,
    IconButton,
    EmptyState,
  } from "$lib/components/ui";
  import {
    gruposListar,
    grupoArbol,
    grupoGuardar,
    grupoEliminar,
  } from "$lib/ipc/commands";
  import { ui } from "$lib/stores/ui.svelte";
  import { t } from "$lib/i18n/es";
  import type { AlbumState } from "$lib/stores/albumState.svelte";
  import type { Grupo, NodoGrupo } from "$lib/domain/types";

  interface Props {
    estado: AlbumState;
  }

  let { estado }: Props = $props();

  let grupos = $state<Grupo[]>([]);
  let grupoActivoId = $state<number | null>(null);
  let arbol = $state<NodoGrupo[]>([]);
  let expandidos = $state(new Set<string>());
  let seleccionado = $state<string | null>(null);
  let cargando = $state(false);

  // Edición de grupos.
  let editorAbierto = $state(false);
  let borrador = $state<Grupo>(grupoVacio());
  let guardando = $state(false);
  let confirmar = $state(false);
  let idBorrar = $state<number | null>(null);

  const opcionesCampo = $derived(
    estado.campos
      .filter((c) => c.tipo !== "multidato")
      .sort((a, b) => a.ordenVisible - b.ordenVisible)
      .map((c) => ({ valor: c.nombre, etiqueta: c.nombre })),
  );

  function grupoVacio(): Grupo {
    return {
      id: 0,
      nombre: "",
      por: estado.campos[0]?.nombre ?? "",
      luego1: null,
      luego2: null,
    };
  }

  $effect(() => {
    void estado.albumId;
    cargarGrupos();
  });

  async function cargarGrupos(): Promise<void> {
    try {
      grupos = await gruposListar(estado.albumId);
    } catch {
      grupos = [];
    }
  }

  async function activarGrupo(id: number): Promise<void> {
    grupoActivoId = id;
    seleccionado = null;
    expandidos = new Set();
    cargando = true;
    try {
      arbol = await grupoArbol(estado.albumId, id);
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
      arbol = [];
    } finally {
      cargando = false;
    }
  }

  // --- Adaptadores del TreeView (id por ruta de valores) -----------------
  function idDe(nodo: NodoGrupo): string {
    return rutaDe(nodo);
  }

  // Mapa nodo→ruta calculado por recorrido (el TreeView pasa el nodo, no la
  // ruta). Como los valores pueden repetirse entre ramas, indexamos por
  // identidad de objeto.
  const rutas = new WeakMap<NodoGrupo, string>();
  function indexar(nodos: NodoGrupo[], prefijo: string[]): void {
    for (const n of nodos) {
      const ruta = [...prefijo, n.valor];
      rutas.set(n, ruta.join(" › "));
      indexar(n.hijos, ruta);
    }
  }
  $effect(() => {
    indexar(arbol, []);
  });

  function rutaDe(nodo: NodoGrupo): string {
    return rutas.get(nodo) ?? nodo.valor;
  }

  function valoresDeRuta(ruta: string): (string | null)[] {
    return ruta.split(" › ");
  }

  function seleccionarNodo(nodo: NodoGrupo): void {
    const ruta = rutaDe(nodo);
    seleccionado = ruta;
    if (grupoActivoId === null) return;
    estado.setGrupoSel({
      grupoId: grupoActivoId,
      valores: valoresDeRuta(ruta),
    });
  }

  function alternar(id: string): void {
    const s = new Set(expandidos);
    if (s.has(id)) s.delete(id);
    else s.add(id);
    expandidos = s;
  }

  function limpiarGrupo(): void {
    grupoActivoId = null;
    arbol = [];
    seleccionado = null;
    estado.setGrupoSel(null);
  }

  // --- CRUD de grupos ----------------------------------------------------
  function abrirAlta(): void {
    borrador = grupoVacio();
    editorAbierto = true;
  }

  function abrirEdicion(g: Grupo): void {
    borrador = { ...g };
    editorAbierto = true;
  }

  async function guardarGrupo(): Promise<void> {
    if (borrador.nombre.trim() === "" || borrador.por === "") {
      ui.aviso(t.grupos.nombre);
      return;
    }
    guardando = true;
    try {
      await grupoGuardar(estado.albumId, {
        ...borrador,
        nombre: borrador.nombre.trim(),
      });
      await cargarGrupos();
      ui.exito(t.mensaje.guardado);
      editorAbierto = false;
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    } finally {
      guardando = false;
    }
  }

  function pedirBorrado(id: number): void {
    idBorrar = id;
    confirmar = true;
  }

  async function borrarGrupo(): Promise<void> {
    if (idBorrar === null) return;
    try {
      await grupoEliminar(estado.albumId, idBorrar);
      if (grupoActivoId === idBorrar) limpiarGrupo();
      await cargarGrupos();
      ui.exito(t.mensaje.eliminado);
      confirmar = false;
    } catch (e) {
      ui.error(typeof e === "string" ? e : t.error.generico);
    } finally {
      idBorrar = null;
    }
  }

  function opcionesConNinguno() {
    return [{ valor: "", etiqueta: t.orden.ninguno }, ...opcionesCampo];
  }
</script>

<div class="gt">
  <div class="gt__barra">
    <div class="gt__sel">
      <Select
        value={grupoActivoId ?? 0}
        opciones={[
          { valor: 0, etiqueta: t.grupos.todos },
          ...grupos.map((g) => ({ valor: g.id, etiqueta: g.nombre })),
        ]}
        onCambio={(v) => (v === 0 ? limpiarGrupo() : activarGrupo(v))}
        etiqueta={t.grupos.titulo}
      />
    </div>
    <IconButton etiqueta={t.grupos.nuevo} tamano="sm" onclick={abrirAlta}>
      <Plus size={14} />
    </IconButton>
  </div>

  {#if grupos.length === 0}
    <EmptyState titulo={t.grupos.sinGrupos}>
      {#snippet icono()}
        <FolderTree size={24} />
      {/snippet}
      {#snippet accion()}
        <Button variante="secundario" tamano="sm" onclick={abrirAlta}>
          <Plus size={14} />
          {t.grupos.nuevo}
        </Button>
      {/snippet}
    </EmptyState>
  {:else if grupoActivoId !== null}
    <div class="gt__activo">
      {#each grupos.filter((g) => g.id === grupoActivoId) as g (g.id)}
        <span class="gt__nombre">{g.nombre}</span>
        <IconButton etiqueta={t.accion.editar} tamano="sm" onclick={() => abrirEdicion(g)}>
          <Pencil size={13} />
        </IconButton>
        <IconButton etiqueta={t.grupos.eliminarGrupo} tamano="sm" onclick={() => pedirBorrado(g.id)}>
          <Trash2 size={13} />
        </IconButton>
      {/each}
    </div>

    {#if cargando}
      <p class="gt__estado">{t.app.cargando}</p>
    {:else}
      <div class="gt__arbol">
        <TreeView
          nodos={arbol}
          {idDe}
          etiquetaDe={(n) => n.valor || "(vacío)"}
          hijosDe={(n) => n.hijos}
          conteoDe={(n) => n.conteo}
          {expandidos}
          {seleccionado}
          onSeleccionar={seleccionarNodo}
          onAlternar={alternar}
        />
      </div>
    {/if}
  {/if}
</div>

<!-- Editor de grupo -->
{#if editorAbierto}
  <Modal
    bind:abierto={editorAbierto}
    titulo={borrador.id === 0 ? t.grupos.nuevo : t.accion.editar}
    ancho="sm"
  >
    <div class="gform">
      <label class="gform__campo">
        <span class="gform__etq">{t.grupos.nombre}</span>
        <TextInput bind:value={borrador.nombre} />
      </label>
      <label class="gform__campo">
        <span class="gform__etq">{t.grupos.por}</span>
        <Select bind:value={borrador.por} opciones={opcionesCampo} />
      </label>
      <label class="gform__campo">
        <span class="gform__etq">{t.grupos.luego1}</span>
        <Select
          value={borrador.luego1 ?? ""}
          opciones={opcionesConNinguno()}
          onCambio={(v) => (borrador.luego1 = v === "" ? null : v)}
        />
      </label>
      <label class="gform__campo">
        <span class="gform__etq">{t.grupos.luego2}</span>
        <Select
          value={borrador.luego2 ?? ""}
          opciones={opcionesConNinguno()}
          onCambio={(v) => (borrador.luego2 = v === "" ? null : v)}
        />
      </label>
    </div>

    {#snippet pie()}
      <Button variante="fantasma" onclick={() => (editorAbierto = false)}>
        {t.accion.cancelar}
      </Button>
      <Button variante="primario" cargando={guardando} onclick={guardarGrupo}>
        {t.accion.guardar}
      </Button>
    {/snippet}
  </Modal>
{/if}

{#if confirmar}
  <ConfirmDialog
    bind:abierto={confirmar}
    titulo={t.confirmar.eliminarGrupo}
    textoConfirmar={t.accion.eliminar}
    peligro
    onConfirmar={borrarGrupo}
    onCancelar={() => (idBorrar = null)}
  />
{/if}

<style>
  .gt {
    display: flex;
    flex-direction: column;
    gap: var(--esp-2);
    height: 100%;
  }
  .gt__barra {
    display: flex;
    gap: var(--esp-2);
    align-items: center;
  }
  .gt__sel {
    flex: 1;
  }
  .gt__activo {
    display: flex;
    align-items: center;
    gap: var(--esp-1);
    padding: var(--esp-1) 0;
  }
  .gt__nombre {
    flex: 1;
    font-size: var(--tam-fuente-sm);
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .gt__arbol {
    flex: 1;
    overflow-y: auto;
  }
  .gt__estado {
    margin: 0;
    color: var(--color-texto-tenue);
    font-size: var(--tam-fuente-sm);
    padding: var(--esp-2);
  }

  .gform {
    display: flex;
    flex-direction: column;
    gap: var(--esp-3);
  }
  .gform__campo {
    display: flex;
    flex-direction: column;
    gap: var(--esp-1);
  }
  .gform__etq {
    font-size: var(--tam-fuente-xs);
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--color-texto-secundario);
  }
</style>
