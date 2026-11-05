import type { SceneMeta } from "../scene";
import { fmtNum, t } from "../i18n";

export function ReportPanel({ meta, onClose }: { meta: SceneMeta; onClose: () => void }) {
  const r = meta.report;
  const types = Object.entries(meta.stats.by_type).sort((a, b) => b[1].skipped + b[1].approximated - (a[1].skipped + a[1].approximated));
  return (
    <aside className="report">
      <div className="panel-head">
        <strong>{t("report.title")}</strong>
        <button onClick={onClose}>{t("report.close")}</button>
      </div>
      {r && (
        <section>
          <div className="kv">
            <span>{t("report.file")}</span>
            <span title={r.file.path}>{r.file.file_name}</span>
          </div>
          <div className="kv">
            <span>Versão</span>
            <span>
              {r.file.version_code} — {r.file.version_name}
            </span>
          </div>
          <div className="kv">
            <span>Tempo de leitura</span>
            <span>{fmtNum(r.timing.total_ms, 0)} ms</span>
          </div>
          <div className="kv">
            <span>Unidade</span>
            <span>
              {r.drawing.insunits_name}
              {r.drawing.suggested_unit ? ` (parece ${r.drawing.suggested_unit})` : ""}
            </span>
          </div>
        </section>
      )}
      {r && r.warnings.length > 0 && (
        <section>
          <h4>{t("report.warnings")}</h4>
          <ul className="warnings">
            {r.warnings.map((w, i) => (
              <li key={i} className={w.level}>
                {w.message}
              </li>
            ))}
          </ul>
        </section>
      )}
      <section>
        <h4>{t("report.render")}</h4>
        <table>
          <thead>
            <tr>
              <th>{t("report.type")}</th>
              <th>{t("report.drawn")}</th>
              <th>{t("report.approx")}</th>
              <th>{t("report.skipped")}</th>
            </tr>
          </thead>
          <tbody>
            {types.map(([name, s]) => (
              <tr key={name} title={Object.entries(s.notes).map(([k, v]) => `${k}: ${v}`).join("\n")}>
                <td>{name}</td>
                <td>{s.drawn}</td>
                <td className={s.approximated ? "approx" : ""}>{s.approximated || ""}</td>
                <td className={s.skipped ? "skipped" : ""}>{s.skipped || ""}</td>
              </tr>
            ))}
          </tbody>
        </table>
        <ul className="notes">
          {types.flatMap(([name, s]) =>
            Object.entries(s.notes).map(([k, v]) => (
              <li key={name + k}>
                <b>{name}</b>: {k} ({v})
              </li>
            )),
          )}
        </ul>
      </section>
      <section>
        <h4>{t("report.general")}</h4>
        <ul className="notes">
          {meta.stats.global_notes.map((n) => (
            <li key={n}>{n}</li>
          ))}
        </ul>
      </section>
    </aside>
  );
}
