/**
 * Punto de reexportación de los primitivos de interfaz, para importarlos en
 * bloque desde las vistas: `import { Button, Modal } from "$lib/components/ui"`.
 */

export { default as Button } from "./Button.svelte";
export { default as IconButton } from "./IconButton.svelte";
export { default as TextInput } from "./TextInput.svelte";
export { default as NumberInput } from "./NumberInput.svelte";
export { default as Select } from "./Select.svelte";
export { default as Combobox } from "./Combobox.svelte";
export { default as Chip } from "./Chip.svelte";
export { default as Tabs } from "./Tabs.svelte";
export { default as Modal } from "./Modal.svelte";
export { default as ConfirmDialog } from "./ConfirmDialog.svelte";
export { default as SplitPane } from "./SplitPane.svelte";
export { default as TreeView } from "./TreeView.svelte";
export { default as ContextMenu } from "./ContextMenu.svelte";
export { default as Tooltip } from "./Tooltip.svelte";
export { default as Toast } from "./Toast.svelte";
export { default as Spinner } from "./Spinner.svelte";
export { default as EmptyState } from "./EmptyState.svelte";

export type { OpcionMenu } from "./ContextMenu.svelte";
