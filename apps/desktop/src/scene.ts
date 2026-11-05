// Formato da cena gerado pelo núcleo em Rust (crates/cadplot-core/src/scene.rs):
// "CPS1" | u32 LE tamanho do JSON | JSON | preenchimento até múltiplo de 4 | f32 LE (x, y relativos à origem).

export type PrimKind = "lines" | "triangles" | "points";

export interface SceneLayer {
  name: string;
  color: [number, number, number];
  color_label: string;
  linetype: string;
  lineweight: string;
  on: boolean;
  frozen: boolean;
  locked: boolean;
  plottable: boolean;
  primitives: number;
}

export interface BatchMeta {
  layer: number;
  hosts: number;
  color: [number, number, number];
  kind: PrimKind;
  offset: number;
  len: number;
}

export interface SceneText {
  layer: number;
  hosts: number;
  color: [number, number, number];
  x: number;
  y: number;
  height: number;
  rotation: number;
  width_factor: number;
  ax: number;
  ay: number;
  multiline: boolean;
  text: string;
}

export interface TypeStats {
  drawn: number;
  approximated: number;
  skipped: number;
  notes: Record<string, number>;
}

export interface RenderStats {
  by_type: Record<string, TypeStats>;
  inserts_expanded: number;
  missing_blocks: Record<string, number>;
  segments: number;
  triangles: number;
  texts: number;
  global_notes: string[];
}

export interface ReportWarning {
  level: "info" | "warning" | "error";
  key: string;
  message: string;
}

export interface ImportReportLite {
  file: { path: string; file_name: string; version_code: string; version_name: string; size_bytes: number };
  timing: { total_ms: number };
  read: { recovery_mode: boolean };
  drawing: { insunits_name: string; suggested_unit: string | null };
  warnings: ReportWarning[];
  [k: string]: unknown;
}

export interface SceneMeta {
  schema: number;
  space: string;
  origin: [number, number];
  bounds: [number, number, number, number] | null;
  layers: SceneLayer[];
  host_sets: number[][];
  batches: BatchMeta[];
  texts: SceneText[];
  stats: RenderStats;
  report?: ImportReportLite;
}

export interface Scene {
  meta: SceneMeta;
  vertices: Float32Array;
}

export function parseScene(buf: ArrayBuffer): Scene {
  const bytes = new Uint8Array(buf);
  const magic = String.fromCharCode(bytes[0], bytes[1], bytes[2], bytes[3]);
  if (magic !== "CPS1") throw new Error("arquivo de cena inválido (assinatura " + magic + ")");
  const jsonLen = new DataView(buf).getUint32(4, true);
  const json = new TextDecoder("utf-8").decode(bytes.subarray(8, 8 + jsonLen));
  const meta = JSON.parse(json) as SceneMeta;
  let off = 8 + jsonLen;
  while (off % 4 !== 0) off++;
  // Cópia alinhada (o ArrayBuffer da IPC pode não começar alinhado).
  const vertices = new Float32Array(buf.slice(off));
  return { meta, vertices };
}
