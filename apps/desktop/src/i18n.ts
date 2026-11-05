// Textos da interface. Estrutura pronta para outros idiomas: adicionar um
// dicionário com as mesmas chaves e trocar `current`.

const ptBR = {
  "app.title": "CadPlot",
  "app.subtitle": "visualizador DWG/DXF — somente leitura",
  "toolbar.open": "Abrir…",
  "toolbar.extents": "Zoom extensão",
  "toolbar.window": "Zoom janela",
  "toolbar.background": "Fundo",
  "toolbar.report": "Relatório",
  "toolbar.texts": "Textos",
  "empty.title": "Abra um desenho DWG ou DXF",
  "empty.hint": "Clique em “Abrir…” ou arraste o arquivo para esta janela. O arquivo nunca é alterado.",
  "empty.hintBrowser": "Modo navegador: abra um arquivo .cps gerado por “cadplot-inspect --cena”.",
  "loading": "Lendo o desenho…",
  "layers.title": "Camadas",
  "layers.search": "Buscar camada…",
  "layers.groupByPrefix": "Agrupar por prefixo",
  "layers.showAll": "Mostrar todas",
  "layers.hideAll": "Ocultar todas",
  "layers.isolate": "Isolar",
  "layers.isolateHint": "Mostrar só esta camada",
  "layers.on": "Ligada / desligada",
  "layers.frozen": "Congelada (esconde também os blocos inseridos nela)",
  "layers.locked": "Travada (no AutoCAD)",
  "layers.empty": "Sem entidades no espaço modelo",
  "layers.count": "{visible} de {total} visíveis",
  "layers.noPrefix": "(sem prefixo)",
  "report.title": "Relatório de importação",
  "report.close": "Fechar",
  "report.warnings": "Avisos",
  "report.render": "O que esta versão desenha",
  "report.type": "Tipo",
  "report.drawn": "Desenhadas",
  "report.approx": "Aproximadas",
  "report.skipped": "Não desenhadas",
  "report.notes": "Observações",
  "report.general": "Limitações gerais",
  "report.file": "Arquivo",
  "status.units": "unid.",
  "status.zoom": "zoom",
  "error.open": "Não foi possível abrir o desenho",
  "warn.notDrawn": "{n} entidade(s) não desenhada(s) — veja o relatório",
  "warn.approx": "{n} aproximada(s)",
} as const;

export type Key = keyof typeof ptBR;
const current: Record<Key, string> = ptBR;

export function t(key: Key, vars?: Record<string, string | number>): string {
  let s: string = current[key] ?? key;
  if (vars) for (const [k, v] of Object.entries(vars)) s = s.replace(`{${k}}`, String(v));
  return s;
}

export function fmtNum(v: number, digits = 3): string {
  return v.toLocaleString("pt-BR", { maximumFractionDigits: digits, minimumFractionDigits: 0 });
}
