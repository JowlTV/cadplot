# Arquitetura do CadPlot

Documento vivo. Registra as decisões que valem para todos os milestones.

## Visão geral

```
┌──────────────────────── app Tauri 2 (M1+) ────────────────────────┐
│ interface TypeScript (React) — painel de camadas, inspetor, plot   │
│ viewport WebGL2 ← buffers de vértices por bloco (relativos a uma   │
│                   origem local, ver "Precisão")                    │
└───────────────▲────────────────────────────────────────────────────┘
                │ comandos Tauri (IPC)
┌───────────────┴────────────────────────────────────────────────────┐
│ cadplot-core (Rust)                                                │
│   source   abertura somente-leitura, detecção de formato, SHA-256  │
│   import   acadrust → CadDocument + relatório de importação        │
│   analyze  contagens, camadas, blocos, layouts, fontes, xrefs      │
│   (M1) geom      tesselação f64: arcos, bulge, spline, elipse      │
│   (M1) index     índice espacial (R-tree) por entidade/bloco       │
│   (M3) state     regras de limpeza + desenho.cadplot.json          │
│   (M4) plot      mundo → papel em f64; PDF (printpdf), PNG/TIFF    │
└────────────────────────────────────────────────────────────────────┘
```

## Regras de fidelidade e como são garantidas

| Regra | Garantia |
|---|---|
| Origem somente leitura | `source::open_read_only` usa só `OpenOptions::read(true)`, lê tudo para a memória e fecha. O teste `somente_leitura.rs` varre o código de produção e falha se aparecer `DwgWriter`, `DxfWriter`, `File::create`, `.write(true)` etc. A CLI recusa saídas iguais à origem ou com extensão `.dwg`/`.dxf`. O gerador de fixtures (único código que grava DWG/DXF) é só `dev-dependency`. |
| Coordenadas em `f64` | O documento do acadrust já é `f64`. Toda conversão mundo → papel é `f64`. Ver "Precisão" para a GPU. |
| Nada descartado em silêncio | Cada tipo de entidade tem um nível em `support.rs` (renderiza, parcial, estrutural, fora do escopo, desconhecido). Tudo que não for "renderiza" gera aviso no relatório. Entidades `UNKNOWN`, proxies e objetos desconhecidos são contados por nome. |
| Acentos | Fixtures com `PA-COMENTÁRIO`, `PISO-DIVISÃO`, `HID-ÁGUA FRIA` em DWG 2000–2018, DXF ASCII/binário e DXF R12 em Windows-1252 escrito byte a byte. O relatório conta strings com `U+FFFD`. |

## Precisão na renderização (decisão para o M1)

A GPU trabalha em `float32` (~7 dígitos). Um desenho georreferenciado em milímetros (coordenadas como
6 700 000 000) perderia precisão se fosse mandado direto. Por isso:

- a geometria fica em `f64` no Rust; o índice espacial divide o desenho em blocos (tiles);
- cada bloco é enviado à GPU **relativo à sua própria origem** (subtraída em `f64`), e a matriz de
  câmera é montada em `f64` e só então convertida, por bloco;
- seleção, medição, inspetor e plotagem usam sempre os valores `f64` originais, nunca os da GPU.

Resultado: o erro visual fica abaixo de 1 pixel em qualquer zoom, e nenhuma medida ou plot passa pela GPU.

## Plotagem (decisão do M0)

Gerador próprio com **printpdf** (ver `spikes/pdf/RESULTADOS.md`): página exata, saída vetorial,
espessuras corretas. O `PrintToPdf` do WebView2 foi descartado: página 0,2 mm maior em A3 e canvas/WebGL
rasterizado. PNG/TIFF no M4 por rasterização própria (tiny-skia) no DPI escolhido
(A3 a 300 dpi = 4961 × 3508 px, arredondamento para o pixel mais próximo).

## Comandos e desfazer (previsto, não implementado)

A Fase 1 já altera **estado de visualização** (camadas ligadas/desligadas, regras de limpeza). Esse estado
muda só por comandos:

```rust
trait Command {
    fn apply(&mut self, state: &mut ProjectState) -> Result<()>;
    fn revert(&mut self, state: &mut ProjectState) -> Result<()>;
    fn label(&self) -> &str; // "Ocultar camada PA-COMENTÁRIO"
}
```

com uma pilha de desfazer/refazer. Na Fase 2 os comandos de geometria (mover, apagar, trim…) entram no
mesmo mecanismo, operando sobre uma cópia em memória — o arquivo de origem continua intocado.

## Dependências

- `acadrust` 0.6.8 (MPL-2.0), fixado com `=`; sem modificações. Mudanças vão upstream ou para um fork
  público sob MPL.
- `printpdf` 0.12 (MIT) para PDF.
- CADCraft é só referência de arquitetura; nenhum código copiado.

## Rede e telemetria

Nenhuma. O núcleo não tem dependência de rede. Auto-update: decisão pendente com o usuário antes do M5.
