export function inTauri(): boolean {
  return "__TAURI_INTERNALS__" in window;
}
