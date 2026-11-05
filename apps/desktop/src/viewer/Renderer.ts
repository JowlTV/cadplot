// Renderizador WebGL2 + sobreposição de textos em Canvas 2D.
//
// Precisão: os vértices chegam em f32 RELATIVOS à origem da cena (calculada
// em f64 no Rust). A câmera é mantida em f64 (números do JS) também relativa
// à origem, e só a matriz final vira f32. Assim coordenadas grandes não
// tremem nem perdem precisão. Coordenadas exibidas ao usuário somam a origem
// em f64.

import type { Scene, SceneText } from "../scene";

const VS = `#version 300 es
in vec2 aPos;
uniform vec2 uScale;
uniform vec2 uOffset;
uniform float uPointSize;
void main() {
  gl_Position = vec4(aPos * uScale + uOffset, 0.0, 1.0);
  gl_PointSize = uPointSize;
}`;

const FS = `#version 300 es
precision mediump float;
uniform vec3 uColor;
out vec4 outColor;
void main() { outColor = vec4(uColor, 1.0); }`;

export interface Camera {
  /** Centro da vista, relativo à origem da cena (unidades do desenho). */
  cx: number;
  cy: number;
  /** Pixels (CSS) por unidade do desenho. */
  scale: number;
}

function compile(gl: WebGL2RenderingContext, type: number, src: string): WebGLShader {
  const s = gl.createShader(type)!;
  gl.shaderSource(s, src);
  gl.compileShader(s);
  if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s) ?? "shader");
  return s;
}

export class Renderer {
  private gl: WebGL2RenderingContext;
  private prog: WebGLProgram;
  private vbo: WebGLBuffer;
  private vao: WebGLVertexArrayObject;
  private loc: { scale: WebGLUniformLocation; offset: WebGLUniformLocation; color: WebGLUniformLocation; point: WebGLUniformLocation };
  private ctx2d: CanvasRenderingContext2D;
  private scene: Scene | null = null;
  private batchVisible: boolean[] = [];
  private textVisible: boolean[] = [];
  private frame = 0;
  cam: Camera = { cx: 0, cy: 0, scale: 1 };
  darkBackground = true;
  showTexts = true;
  /** Retângulo de zoom janela em pixels CSS, desenhado por cima. */
  rubber: { x0: number; y0: number; x1: number; y1: number } | null = null;
  onChange: (() => void) | null = null;

  constructor(private glCanvas: HTMLCanvasElement, private textCanvas: HTMLCanvasElement) {
    const gl = glCanvas.getContext("webgl2", { antialias: true, preserveDrawingBuffer: true });
    if (!gl) throw new Error("WebGL2 indisponível neste computador");
    this.gl = gl;
    const p = gl.createProgram()!;
    gl.attachShader(p, compile(gl, gl.VERTEX_SHADER, VS));
    gl.attachShader(p, compile(gl, gl.FRAGMENT_SHADER, FS));
    gl.linkProgram(p);
    if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(p) ?? "link");
    this.prog = p;
    this.vbo = gl.createBuffer()!;
    this.vao = gl.createVertexArray()!;
    gl.bindVertexArray(this.vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.vbo);
    const aPos = gl.getAttribLocation(p, "aPos");
    gl.enableVertexAttribArray(aPos);
    gl.vertexAttribPointer(aPos, 2, gl.FLOAT, false, 0, 0);
    this.loc = {
      scale: gl.getUniformLocation(p, "uScale")!,
      offset: gl.getUniformLocation(p, "uOffset")!,
      color: gl.getUniformLocation(p, "uColor")!,
      point: gl.getUniformLocation(p, "uPointSize")!,
    };
    this.ctx2d = textCanvas.getContext("2d")!;
  }

  setScene(scene: Scene) {
    this.scene = scene;
    const gl = this.gl;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.vbo);
    gl.bufferData(gl.ARRAY_BUFFER, scene.vertices, gl.STATIC_DRAW);
    this.batchVisible = scene.meta.batches.map(() => true);
    this.textVisible = scene.meta.texts.map(() => true);
    this.zoomExtents();
  }

  /** Recalcula a visibilidade dos lotes a partir do estado das camadas. */
  setLayerState(on: boolean[], frozen: boolean[]) {
    const s = this.scene;
    if (!s) return;
    const hostOk = s.meta.host_sets.map((hs) => hs.every((l) => !frozen[l]));
    const vis = (layer: number, hosts: number) => on[layer] && !frozen[layer] && hostOk[hosts];
    this.batchVisible = s.meta.batches.map((b) => vis(b.layer, b.hosts));
    this.textVisible = s.meta.texts.map((tx) => vis(tx.layer, tx.hosts));
    this.requestDraw();
  }

  size(): { w: number; h: number } {
    const r = this.glCanvas.getBoundingClientRect();
    return { w: Math.max(1, r.width), h: Math.max(1, r.height) };
  }

  zoomExtents() {
    const b = this.scene?.meta.bounds;
    const o = this.scene?.meta.origin ?? [0, 0];
    if (!b) {
      this.cam = { cx: 0, cy: 0, scale: 1 };
    } else {
      this.fitRect(b[0] - o[0], b[1] - o[1], b[2] - o[0], b[3] - o[1]);
    }
    this.requestDraw();
  }

  /** Enquadra um retângulo (relativo à origem), com 4 % de margem. */
  fitRect(x0: number, y0: number, x1: number, y1: number) {
    const { w, h } = this.size();
    const dx = Math.max(Math.abs(x1 - x0), 1e-9);
    const dy = Math.max(Math.abs(y1 - y0), 1e-9);
    const scale = Math.min(w / dx, h / dy) * 0.96;
    this.cam = { cx: (x0 + x1) / 2, cy: (y0 + y1) / 2, scale: isFinite(scale) && scale > 0 ? scale : 1 };
    this.requestDraw();
  }

  screenToWorld(px: number, py: number): [number, number] {
    const { w, h } = this.size();
    return [this.cam.cx + (px - w / 2) / this.cam.scale, this.cam.cy - (py - h / 2) / this.cam.scale];
  }

  /** Coordenada absoluta (com a origem somada em f64). */
  screenToAbsolute(px: number, py: number): [number, number] {
    const [x, y] = this.screenToWorld(px, py);
    const o = this.scene?.meta.origin ?? [0, 0];
    return [x + o[0], y + o[1]];
  }

  zoomAt(px: number, py: number, factor: number) {
    const [wx, wy] = this.screenToWorld(px, py);
    const scale = Math.min(Math.max(this.cam.scale * factor, 1e-12), 1e12);
    const { w, h } = this.size();
    this.cam = { scale, cx: wx - (px - w / 2) / scale, cy: wy + (py - h / 2) / scale };
    this.requestDraw();
  }

  panBy(dxPx: number, dyPx: number) {
    this.cam = { ...this.cam, cx: this.cam.cx - dxPx / this.cam.scale, cy: this.cam.cy + dyPx / this.cam.scale };
    this.requestDraw();
  }

  zoomWindow(x0: number, y0: number, x1: number, y1: number) {
    if (Math.abs(x1 - x0) < 4 || Math.abs(y1 - y0) < 4) return;
    const a = this.screenToWorld(Math.min(x0, x1), Math.max(y0, y1));
    const b = this.screenToWorld(Math.max(x0, x1), Math.min(y0, y1));
    this.fitRect(a[0], a[1], b[0], b[1]);
  }

  requestDraw() {
    if (this.frame) return;
    this.frame = requestAnimationFrame(() => {
      this.frame = 0;
      this.draw();
      this.onChange?.();
    });
  }

  private resizeCanvases(): { w: number; h: number; dpr: number } {
    const dpr = window.devicePixelRatio || 1;
    const { w, h } = this.size();
    for (const c of [this.glCanvas, this.textCanvas]) {
      const W = Math.round(w * dpr);
      const H = Math.round(h * dpr);
      if (c.width !== W || c.height !== H) {
        c.width = W;
        c.height = H;
      }
    }
    return { w, h, dpr };
  }

  private fg(c: [number, number, number]): [number, number, number] {
    // Cor 7 (branco) vira preto no fundo claro, como no AutoCAD.
    if (!this.darkBackground && c[0] > 235 && c[1] > 235 && c[2] > 235) return [0, 0, 0];
    return c;
  }

  draw() {
    const { w, h, dpr } = this.resizeCanvases();
    const gl = this.gl;
    gl.viewport(0, 0, this.glCanvas.width, this.glCanvas.height);
    const bg = this.darkBackground ? [0.13, 0.14, 0.15] : [1, 1, 1];
    gl.clearColor(bg[0], bg[1], bg[2], 1);
    gl.clear(gl.COLOR_BUFFER_BIT);
    const ctx = this.ctx2d;
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.clearRect(0, 0, this.textCanvas.width, this.textCanvas.height);
    const s = this.scene;
    if (!s) return;

    const sx = (2 * this.cam.scale) / w;
    const sy = (2 * this.cam.scale) / h;
    gl.useProgram(this.prog);
    gl.bindVertexArray(this.vao);
    gl.uniform2f(this.loc.scale, sx, sy);
    gl.uniform2f(this.loc.offset, -this.cam.cx * sx, -this.cam.cy * sy);
    gl.uniform1f(this.loc.point, Math.max(2, 2 * dpr));
    const batches = s.meta.batches;
    for (let i = 0; i < batches.length; i++) {
      if (!this.batchVisible[i]) continue;
      const b = batches[i];
      const c = this.fg(b.color);
      gl.uniform3f(this.loc.color, c[0] / 255, c[1] / 255, c[2] / 255);
      const mode = b.kind === "lines" ? gl.LINES : b.kind === "triangles" ? gl.TRIANGLES : gl.POINTS;
      gl.drawArrays(mode, b.offset / 2, b.len / 2);
    }

    if (this.showTexts) this.drawTexts(s.meta.texts, w, h, dpr);

    if (this.rubber) {
      const r = this.rubber;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.strokeStyle = this.darkBackground ? "#7cc4ff" : "#0060c0";
      ctx.setLineDash([5, 4]);
      ctx.lineWidth = 1;
      ctx.strokeRect(Math.min(r.x0, r.x1) + 0.5, Math.min(r.y0, r.y1) + 0.5, Math.abs(r.x1 - r.x0), Math.abs(r.y1 - r.y0));
      ctx.setLineDash([]);
    }
  }

  private drawTexts(texts: SceneText[], w: number, h: number, dpr: number) {
    const ctx = this.ctx2d;
    const sc = this.cam.scale;
    const margin = 200;
    for (let i = 0; i < texts.length; i++) {
      if (!this.textVisible[i]) continue;
      const t = texts[i];
      const hp = t.height * sc; // altura das maiúsculas em px
      if (hp < 1.2) continue;
      const px = w / 2 + (t.x - this.cam.cx) * sc;
      const py = h / 2 - (t.y - this.cam.cy) * sc;
      const reach = hp * (t.text.length + 2) + margin;
      if (px < -reach || px > w + reach || py < -reach || py > h + reach) continue;
      const lines = t.text.split("\n");
      const fontPx = hp / 0.716; // altura de maiúscula da Arial ≈ 0,716 em
      const c = this.fg(t.color);
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.translate(px, py);
      ctx.rotate(-t.rotation);
      ctx.scale(t.width_factor, 1);
      ctx.font = `${fontPx}px Arial, "Segoe UI", sans-serif`;
      ctx.fillStyle = `rgb(${c[0]},${c[1]},${c[2]})`;
      ctx.textBaseline = "alphabetic";
      const lineStep = hp * (5 / 3);
      const blockH = hp + (lines.length - 1) * lineStep;
      // Deslocamento da primeira linha de base em relação ao ponto de inserção (y para baixo).
      // MTEXT: ay 1 = topo do bloco, 0,5 = meio, 0 = base do bloco.
      // TEXT: ay 0 = linha de base, 0,5 = meio, 1 = topo, −0,25 = embaixo (descendentes).
      const y0 = t.multiline
        ? t.ay === 1 ? hp : t.ay === 0.5 ? hp - blockH / 2 : hp - blockH
        : t.ay === 1 ? hp : t.ay === 0.5 ? hp / 2 : t.ay < 0 ? -hp * 0.25 : 0;
      for (let k = 0; k < lines.length; k++) {
        const line = lines[k];
        const tw = ctx.measureText(line).width;
        ctx.fillText(line, -t.ax * tw, y0 + k * lineStep);
      }
    }
  }
}
