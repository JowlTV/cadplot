# CadPlot

Visualizador e plotter técnico para arquivos **DWG** e **DXF** de arquitetura e paisagismo, para Windows.
Funciona como uma "mesa" de organização: abrir o desenho, ocultar camadas e elementos que não interessam
e exportar uma imagem em escala (PDF/PNG/TIFF) para trabalhar no Photoshop.

**O CadPlot não grava DWG nem DXF.** O arquivo de origem é somente leitura; o estado do projeto fica em
um arquivo separado (`desenho.cadplot.json`).

## Estado

| Milestone | Conteúdo | Estado |
|---|---|---|
| M0 | Spike de leitura (relatório de importação) e escolha do gerador de PDF | em andamento |
| M1 | Visualizador e painel de camadas | — |
| M2 | Inspetor e busca | — |
| M3 | Regras de limpeza e arquivo de estado JSON | — |
| M4 | Plotagem em PDF e PNG em escala | — |
| M5 | Empacotamento, instalador assinado, testes Win10/11 | — |

## Estrutura

```
crates/cadplot-core       núcleo: leitura somente-leitura, relatório de importação
crates/cadplot-inspect    CLI: cadplot-inspect <arquivo.dwg|dxf>
crates/cadplot-fixtures   gerador de desenhos sintéticos para testes (nunca usado pelo app)
spikes/pdf                comparação de geradores de PDF (M0)
docs/                     relatórios por milestone e instruções de teste
samples/, tests/corpus/   arquivos reais locais — ignorados pelo git
```

## Arquivos de clientes

Nunca commitar DWG/DXF/PDF/PNG/TIFF de clientes. Coloque-os em `samples/` ou `tests/corpus/`;
o `.gitignore` bloqueia essas extensões nessas pastas. Os fixtures do repositório são gerados por código.

## Dependências e licenças

- [acadrust](https://github.com/hakanaktt/acadrust) 0.6.8 — MPL-2.0, usada sem modificações.
- Licença do CadPlot: **a definir** (decisão pendente).
