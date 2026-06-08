<!--
  App.svelte — componente raíz. Monta el armazón (AppShell), garantiza la
  aplicación del tema y registra los atajos de teclado globales:
    ⌘/Ctrl+K  → paleta de comandos
    ⌘/Ctrl+O  → abrir álbum
    ⌘/Ctrl+N  → nuevo álbum
    ⌘/Ctrl+F  → buscar (si hay álbum activo)
-->
<script lang="ts">
  import AppShell from "$lib/components/shell/AppShell.svelte";
  import { tema } from "$lib/stores/theme.svelte";
  import { albumes } from "$lib/stores/albums.svelte";
  import { ejecutarAccion } from "$lib/acciones";

  // Visibilidad de la paleta de comandos (atajo global ⌘K).
  let paletaAbierta = $state(false);

  // Asegura que el tema persistido quede aplicado al documento.
  $effect(() => {
    tema.inicializar();
  });

  function esModificador(e: KeyboardEvent): boolean {
    return e.metaKey || e.ctrlKey;
  }

  function onKeydown(e: KeyboardEvent): void {
    if (!esModificador(e)) return;
    const k = e.key.toLowerCase();
    switch (k) {
      case "k":
        e.preventDefault();
        paletaAbierta = !paletaAbierta;
        break;
      case "o":
        e.preventDefault();
        ejecutarAccion("abrir");
        break;
      case "n":
        e.preventDefault();
        ejecutarAccion("nuevo-album");
        break;
      case "f":
        if (albumes.activo) {
          e.preventDefault();
          ejecutarAccion("buscar");
        }
        break;
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<AppShell bind:paletaAbierta />
