import { forwardRef, useEffect, useImperativeHandle, useRef, useState } from "react";
import type { Scene } from "../scene";
import { Renderer } from "../viewer/Renderer";
import { fmtNum, t } from "../i18n";

export interface ViewerHandle {
  zoomExtents(): void;
  setWindowMode(on: boolean): void;
  setDark(dark: boolean): void;
  setShowTexts(show: boolean): void;
}

interface Props {
  scene: Scene | null;
  on: boolean[];
  frozen: boolean[];
  onWindowModeEnd?: () => void;
  windowMode: boolean;
}

export const Viewer = forwardRef<ViewerHandle, Props>(function Viewer({ scene, on, frozen, windowMode, onWindowModeEnd }, ref) {
  const glRef = useRef<HTMLCanvasElement>(null);
  const txRef = useRef<HTMLCanvasElement>(null);
  const boxRef = useRef<HTMLDivElement>(null);
  const r = useRef<Renderer | null>(null);
  const [cursor, setCursor] = useState<[number, number] | null>(null);
  const [zoom, setZoom] = useState(1);
  const [error, setError] = useState<string | null>(null);
  const drag = useRef<{ x: number; y: number; mode: "pan" | "window"; sx: number; sy: number } | null>(null);
  const windowModeRef = useRef(windowMode);
  windowModeRef.current = windowMode;

  useEffect(() => {
    try {
      r.current = new Renderer(glRef.current!, txRef.current!);
      r.current.onChange = () => setZoom(r.current!.cam.scale);
    } catch (e) {
      setError(String(e));
    }
    const ro = new ResizeObserver(() => r.current?.requestDraw());
    ro.observe(boxRef.current!);
    return () => ro.disconnect();
  }, []);

  useEffect(() => {
    if (scene && r.current) r.current.setScene(scene);
  }, [scene]);

  useEffect(() => {
    r.current?.setLayerState(on, frozen);
  }, [on, frozen, scene]);

  useImperativeHandle(ref, () => ({
    zoomExtents: () => r.current?.zoomExtents(),
    setWindowMode: () => {},
    setDark: (d) => {
      if (r.current) {
        r.current.darkBackground = d;
        r.current.requestDraw();
      }
    },
    setShowTexts: (s) => {
      if (r.current) {
        r.current.showTexts = s;
        r.current.requestDraw();
      }
    },
  }));

  const local = (e: { clientX: number; clientY: number }) => {
    const rect = glRef.current!.getBoundingClientRect();
    return [e.clientX - rect.left, e.clientY - rect.top] as const;
  };

  return (
    <div className="viewer" ref={boxRef}>
      <canvas ref={glRef} className="gl" />
      <canvas
        ref={txRef}
        className={"tx" + (windowMode ? " crosshair" : "")}
        onWheel={(e) => {
          const [x, y] = local(e);
          r.current?.zoomAt(x, y, Math.pow(1.0015, -e.deltaY));
        }}
        onMouseDown={(e) => {
          const [x, y] = local(e);
          const win = windowModeRef.current || e.shiftKey;
          if (e.button === 0 || e.button === 1) {
            e.preventDefault();
            drag.current = { x, y, sx: x, sy: y, mode: win && e.button === 0 ? "window" : "pan" };
          }
        }}
        onMouseMove={(e) => {
          const [x, y] = local(e);
          if (r.current) setCursor(r.current.screenToAbsolute(x, y));
          const d = drag.current;
          if (!d || !r.current) return;
          if (d.mode === "pan") {
            r.current.panBy(x - d.x, y - d.y);
            d.x = x;
            d.y = y;
          } else {
            r.current.rubber = { x0: d.sx, y0: d.sy, x1: x, y1: y };
            r.current.requestDraw();
          }
        }}
        onMouseUp={(e) => {
          const d = drag.current;
          drag.current = null;
          if (d?.mode === "window" && r.current) {
            const [x, y] = local(e);
            r.current.rubber = null;
            r.current.zoomWindow(d.sx, d.sy, x, y);
            onWindowModeEnd?.();
          }
        }}
        onMouseLeave={() => {
          drag.current = null;
          if (r.current?.rubber) {
            r.current.rubber = null;
            r.current.requestDraw();
          }
          setCursor(null);
        }}
        onDoubleClick={(e) => {
          if (e.button === 1 || e.button === 0) r.current?.zoomExtents();
        }}
        onAuxClick={(e) => e.preventDefault()}
        onContextMenu={(e) => e.preventDefault()}
      />
      {error && <div className="viewer-error">{error}</div>}
      <div className="statusbar">
        {cursor ? (
          <span>
            X {fmtNum(cursor[0], 4)} &nbsp; Y {fmtNum(cursor[1], 4)} {t("status.units")}
          </span>
        ) : (
          <span>&nbsp;</span>
        )}
        <span>
          {t("status.zoom")} {fmtNum(zoom, 4)} px/{t("status.units")}
        </span>
      </div>
    </div>
  );
});
