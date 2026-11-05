import { useCallback, useEffect, useRef, useState } from "react";
import { isTauri, initialPath, onFileDrop, openPath, openSceneFile, openSceneUrl, pickAndOpen } from "./backend";
import type { Scene } from "./scene";
import { Viewer, type ViewerHandle } from "./components/Viewer";
import { LayerPanel } from "./components/LayerPanel";
import { ReportPanel } from "./components/ReportPanel";
import { t } from "./i18n";

export default function App() {
  const [scene, setScene] = useState<Scene | null>(null);
  const [on, setOn] = useState<boolean[]>([]);
  const [frozen, setFrozen] = useState<boolean[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [showReport, setShowReport] = useState(false);
  const [windowMode, setWindowMode] = useState(false);
  const [dark, setDark] = useState(true);
  const [texts, setTexts] = useState(true);
  const viewer = useRef<ViewerHandle>(null);
  const fileInput = useRef<HTMLInputElement>(null);

  const load = useCallback(async (p: Promise<Scene | null>) => {
    setLoading(true);
    setError(null);
    try {
      const s = await p;
      if (s) {
        setScene(s);
        setOn(s.meta.layers.map((l) => l.on));
        setFrozen(s.meta.layers.map((l) => l.frozen));
        const name = s.meta.report?.file.file_name;
        document.title = name ? `${name} — CadPlot` : "CadPlot";
      }
    } catch (e) {
      setError(`${t("error.open")}: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    onFileDrop((path) => load(openPath(path))).then((u) => (unlisten = u));
    initialPath().then((p) => {
      if (p) load(openPath(p));
    });
    const q = new URLSearchParams(location.search).get("cena");
    if (!isTauri && q) load(openSceneUrl(q));
    return () => unlisten?.();
  }, [load]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "o") {
        e.preventDefault();
        openClick();
      }
      if (e.key === "Escape") setWindowMode(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  const openClick = () => {
    if (isTauri) load(pickAndOpen());
    else fileInput.current?.click();
  };

  const stats = scene?.meta.stats;
  const skipped = stats ? Object.values(stats.by_type).reduce((a, s) => a + s.skipped, 0) : 0;

  return (
    <div className={"app" + (dark ? " dark" : " light")}>
      <header className="toolbar">
        <span className="brand">
          {t("app.title")} <small>{t("app.subtitle")}</small>
        </span>
        <button className="primary" onClick={openClick} title="Ctrl+O">
          {t("toolbar.open")}
        </button>
        <button disabled={!scene} onClick={() => viewer.current?.zoomExtents()}>
          {t("toolbar.extents")}
        </button>
        <button disabled={!scene} className={windowMode ? "active" : ""} onClick={() => setWindowMode(!windowMode)} title="Shift + arrastar">
          {t("toolbar.window")}
        </button>
        <button
          onClick={() => {
            viewer.current?.setDark(!dark);
            setDark(!dark);
          }}
        >
          {t("toolbar.background")}: {dark ? "escuro" : "claro"}
        </button>
        <button
          className={texts ? "active" : ""}
          onClick={() => {
            viewer.current?.setShowTexts(!texts);
            setTexts(!texts);
          }}
        >
          {t("toolbar.texts")}
        </button>
        <span className="spacer" />
        {scene && skipped > 0 && (
          <button className="warn" onClick={() => setShowReport(true)}>
            ⚠ {t("warn.notDrawn", { n: skipped })}
          </button>
        )}
        <button disabled={!scene} className={showReport ? "active" : ""} onClick={() => setShowReport(!showReport)}>
          {t("toolbar.report")}
        </button>
        <input
          ref={fileInput}
          type="file"
          accept=".cps"
          style={{ display: "none" }}
          onChange={(e) => {
            const f = e.target.files?.[0];
            if (f) load(openSceneFile(f));
            e.target.value = "";
          }}
        />
      </header>
      <main>
        {scene && <LayerPanel layers={scene.meta.layers} on={on} frozen={frozen} setOn={setOn} setFrozen={setFrozen} />}
        <section className="canvas-area">
          <Viewer ref={viewer} scene={scene} on={on} frozen={frozen} windowMode={windowMode} onWindowModeEnd={() => setWindowMode(false)} />
          {!scene && !loading && (
            <div className="empty-state">
              <h2>{t("empty.title")}</h2>
              <p>{isTauri ? t("empty.hint") : t("empty.hintBrowser")}</p>
              <button className="primary" onClick={openClick}>
                {t("toolbar.open")}
              </button>
            </div>
          )}
          {loading && <div className="overlay">{t("loading")}</div>}
          {error && (
            <div className="error-banner" onClick={() => setError(null)}>
              {error}
            </div>
          )}
        </section>
        {scene && showReport && <ReportPanel meta={scene.meta} onClose={() => setShowReport(false)} />}
      </main>
    </div>
  );
}
