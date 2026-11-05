// Ponte com o núcleo Rust. No app (Tauri) chama os comandos nativos; no
// navegador (desenvolvimento) lê arquivos .cps gerados pela CLI.

import { parseScene, type Scene } from "./scene";

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export async function pickAndOpen(): Promise<Scene | null> {
  if (!isTauri) return null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const path = await open({
    multiple: false,
    directory: false,
    filters: [
      { name: "Desenhos (DWG, DXF)", extensions: ["dwg", "dxf", "DWG", "DXF"] },
      { name: "Todos os arquivos", extensions: ["*"] },
    ],
  });
  if (!path || Array.isArray(path)) return null;
  return openPath(path);
}

export async function openPath(path: string): Promise<Scene> {
  const { invoke } = await import("@tauri-apps/api/core");
  const buf = await invoke<ArrayBuffer>("open_drawing", { path });
  return parseScene(buf);
}

export async function initialPath(): Promise<string | null> {
  if (!isTauri) return null;
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string | null>("initial_file");
}

/** Registra soltar arquivos na janela. Devolve a função de cancelamento. */
export async function onFileDrop(cb: (path: string) => void): Promise<() => void> {
  if (!isTauri) return () => {};
  const { getCurrentWebview } = await import("@tauri-apps/api/webview");
  return getCurrentWebview().onDragDropEvent((ev) => {
    if (ev.payload.type === "drop" && ev.payload.paths.length > 0) cb(ev.payload.paths[0]);
  });
}

export async function openSceneFile(file: File): Promise<Scene> {
  return parseScene(await file.arrayBuffer());
}

export async function openSceneUrl(url: string): Promise<Scene> {
  const r = await fetch(url);
  if (!r.ok) throw new Error(`HTTP ${r.status} ao carregar ${url}`);
  return parseScene(await r.arrayBuffer());
}
