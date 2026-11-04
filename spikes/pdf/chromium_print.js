// Imprime folha.html e folha-canvas.html com o print-to-PDF do Chromium.
// O WebView2 usa o mesmo motor (Chromium/Skia PDF); este script reproduz
// CoreWebView2.PrintToPdfAsync com PageWidth/PageHeight em polegadas,
// margens 0, escala 1 e "preferCSSPageSize".
// Uso: node spikes/pdf/chromium_print.js spikes/pdf/out
const { chromium } = require('playwright');
const path = require('path');
(async () => {
  const out = path.resolve(process.argv[2] || 'spikes/pdf/out');
  const browser = await chromium.launch();
  const page = await browser.newPage();
  console.log('chromium', browser.version());
  for (const [html, pdf, prefer] of [
    ['folha.html', 'chromium-svg.pdf', true],
    ['folha.html', 'chromium-svg-polegadas.pdf', false],
    ['folha-canvas.html', 'chromium-canvas.pdf', true],
  ]) {
    await page.goto('file://' + path.join(out, html));
    await page.pdf({
      path: path.join(out, pdf),
      // Igual ao WebView2: tamanho em polegadas (420 mm = 16.5354…").
      width: (420 / 25.4) + 'in',
      height: (297 / 25.4) + 'in',
      margin: { top: 0, right: 0, bottom: 0, left: 0 },
      printBackground: true,
      preferCSSPageSize: prefer,
      scale: 1,
    });
    console.log('gerado', pdf);
  }
  await browser.close();
})();
