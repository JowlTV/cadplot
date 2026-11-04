# Spike M0 — gerador de PDF

Mesma folha de teste (A3 paisagem, 1:100, desenho em mm) gerada por dois caminhos e medida
pela geometria vetorial do PDF (`medir.py`, que aplica a CTM e lê espessuras de `w` e de `ExtGState /LW`).

| Critério | printpdf 0.12.8 (gerador próprio) | Chromium print-to-PDF (motor do WebView2), SVG | Chromium print-to-PDF, `<canvas>` |
|---|---|---|---|
| Tamanho da página (alvo 420 × 297 mm) | **420,0000 × 297,0000 mm** (MediaBox 1190,5513 × 841,8898 pt) | 420,2007 × 297,0107 mm (1191,12 × 841,92 pt) — **+0,20 mm** na largura | igual ao SVG |
| Linha de 10 m a 1:100 (alvo 100 mm) | 100,0000 mm | 99,9993 mm | — |
| Linha de 5 m (alvo 50 mm) | 50,0000 mm | 49,9997 mm | — |
| Círculo r = 2 m (alvo Ø 40 mm) | 40,0000 mm | 39,9997 mm | — |
| Espessuras 0,13 … 0,70 mm | exatas | exatas | — |
| Vetorial | sim (12 segmentos, 1 curva, 0 imagens) | sim | **não**: 1 imagem raster |

Ambiente: Chromium 153 (Playwright, Linux). O WebView2 usa o mesmo pipeline de impressão do Chromium
(Skia/PDF); `CoreWebView2PrintSettings.PageWidth/PageHeight` são em polegadas, como no teste.

## Conclusões

1. **O print-to-PDF não atende "tamanho de página exato"**: a página sai 0,2 mm mais larga em A3. O erro
   vem do arredondamento do tamanho do papel no Chromium e não some com `preferCSSPageSize` nem passando
   o tamanho em polegadas.
2. **O print-to-PDF rasteriza canvas/WebGL.** Para sair vetorial pelo WebView2 seria preciso montar um
   segundo renderizador em SVG/DOM só para impressão, que é praticamente o mesmo trabalho de um gerador
   próprio, com menos controle.
3. **printpdf atende os três critérios** (página exata, vetorial, espessuras corretas). Coordenadas no PDF
   são `f32`; como a conversão mundo → papel é feita em `f64` antes, o erro de representação em A0 fica
   abaixo de 0,0002 mm.

**Decisão do M0: gerador próprio com printpdf.** O PrintToPdf do WebView2 fica descartado para a plotagem.
Confirmação no Windows real no M4 (o mesmo `medir.py` roda lá).

## Como reproduzir

```
cargo run -p cadplot-pdf-spike -- spikes/pdf/out
node spikes/pdf/chromium_print.js spikes/pdf/out     # precisa do Playwright
python3 -I spikes/pdf/medir.py spikes/pdf/out/*.pdf
```
