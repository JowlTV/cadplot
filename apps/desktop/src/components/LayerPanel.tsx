import { useMemo, useState } from "react";
import type { SceneLayer } from "../scene";
import { t } from "../i18n";

interface Props {
  layers: SceneLayer[];
  on: boolean[];
  frozen: boolean[];
  setOn: (next: boolean[]) => void;
  setFrozen: (next: boolean[]) => void;
}

function prefixOf(name: string): string | null {
  const m = name.match(/^([^-_]+)[-_]/);
  return m && m[1].length > 0 ? m[1].toUpperCase() : null;
}

function rgb(c: [number, number, number]) {
  return `rgb(${c[0]},${c[1]},${c[2]})`;
}

export function LayerPanel({ layers, on, frozen, setOn, setFrozen }: Props) {
  const [query, setQuery] = useState("");
  const [grouped, setGrouped] = useState(true);
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});

  const filtered = useMemo(() => {
    const q = query.trim().toLocaleLowerCase("pt-BR");
    return layers
      .map((l, i) => ({ l, i }))
      .filter(({ l }) => !q || l.name.toLocaleLowerCase("pt-BR").includes(q))
      .sort((a, b) => a.l.name.localeCompare(b.l.name, "pt-BR", { sensitivity: "base" }));
  }, [layers, query]);

  const groups = useMemo(() => {
    const map = new Map<string, { l: SceneLayer; i: number }[]>();
    for (const it of filtered) {
      const g = grouped ? prefixOf(it.l.name) ?? t("layers.noPrefix") : "";
      if (!map.has(g)) map.set(g, []);
      map.get(g)!.push(it);
    }
    return [...map.entries()].sort((a, b) => a[0].localeCompare(b[0], "pt-BR"));
  }, [filtered, grouped]);

  const visibleCount = layers.filter((_, i) => on[i] && !frozen[i]).length;

  const setMany = (idx: number[], value: boolean) => {
    const next = [...on];
    for (const i of idx) next[i] = value;
    setOn(next);
  };

  const isolate = (keep: number[]) => {
    const next = layers.map((_, i) => keep.includes(i));
    setOn(next);
    // Isolar também descongela a camada mantida, senão ela continuaria escondida.
    const fz = [...frozen];
    for (const i of keep) fz[i] = false;
    setFrozen(fz);
  };

  const row = ({ l, i }: { l: SceneLayer; i: number }) => {
    const hidden = !on[i] || frozen[i];
    return (
      <div key={i} className={"layer-row" + (hidden ? " hidden" : "") + (l.primitives === 0 ? " no-ents" : "")} title={l.primitives === 0 ? t("layers.empty") : `${l.name} · ${l.color_label} · ${l.linetype} · ${l.lineweight}`}>
        <button className="icon" title={t("layers.on")} aria-pressed={on[i]} onClick={() => setMany([i], !on[i])}>
          {on[i] ? "●" : "○"}
        </button>
        <button className={"icon" + (frozen[i] ? " active" : "")} title={t("layers.frozen")} aria-pressed={frozen[i]} onClick={() => {
          const next = [...frozen];
          next[i] = !next[i];
          setFrozen(next);
        }}>
          ❄
        </button>
        <span className="swatch" style={{ background: rgb(l.color) }} />
        <span className="name">{l.name}</span>
        {l.locked && <span className="lock" title={t("layers.locked")}>🔒</span>}
        <span className="count">{l.primitives || ""}</span>
        <button className="link" title={t("layers.isolateHint")} onClick={() => isolate([i])}>
          {t("layers.isolate")}
        </button>
      </div>
    );
  };

  return (
    <aside className="layers">
      <div className="panel-head">
        <strong>{t("layers.title")}</strong>
        <span className="muted">{t("layers.count", { visible: visibleCount, total: layers.length })}</span>
      </div>
      <input className="search" placeholder={t("layers.search")} value={query} onChange={(e) => setQuery(e.target.value)} />
      <div className="layer-actions">
        <button onClick={() => setMany(filtered.map((x) => x.i), true)}>{t("layers.showAll")}</button>
        <button onClick={() => setMany(filtered.map((x) => x.i), false)}>{t("layers.hideAll")}</button>
        <label>
          <input type="checkbox" checked={grouped} onChange={(e) => setGrouped(e.target.checked)} /> {t("layers.groupByPrefix")}
        </label>
      </div>
      <div className="layer-list">
        {groups.map(([g, items]) =>
          grouped ? (
            <div key={g} className="group">
              <div className="group-head">
                <button className="icon" onClick={() => setCollapsed({ ...collapsed, [g]: !collapsed[g] })}>
                  {collapsed[g] ? "▸" : "▾"}
                </button>
                <span className="name">{g}</span>
                <span className="muted">{items.length}</span>
                <button className="link" onClick={() => setMany(items.map((x) => x.i), true)}>{t("layers.showAll")}</button>
                <button className="link" onClick={() => setMany(items.map((x) => x.i), false)}>{t("layers.hideAll")}</button>
                <button className="link" onClick={() => isolate(items.map((x) => x.i))}>{t("layers.isolate")}</button>
              </div>
              {!collapsed[g] && items.map(row)}
            </div>
          ) : (
            items.map(row)
          ),
        )}
      </div>
    </aside>
  );
}
