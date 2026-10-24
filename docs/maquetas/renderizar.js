// Renderiza cada maqueta NN-*.html a png/NN-*.png a 1920x1080 con Chromium.
// Uso: node docs/maquetas/renderizar.js [nombre...]
const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');
(async () => {
  const dir = __dirname;
  const pedidos = process.argv.slice(2);
  const paginas = fs.readdirSync(dir).filter(f => /^\d\d-.*\.html$/.test(f))
    .filter(f => !pedidos.length || pedidos.some(p => f.includes(p)));
  fs.mkdirSync(path.join(dir, 'png'), { recursive: true });
  const navegador = await chromium.launch();
  const pagina = await navegador.newPage({ viewport: { width: 1920, height: 1080 } });
  // Base sin sello para las maquetas que se apoyan en el escritorio.
  await pagina.goto('file://' + path.join(dir, '01-escritorio.html#limpio'));
  await pagina.evaluate(() => document.fonts.ready);
  await pagina.waitForTimeout(300);
  await pagina.screenshot({ path: path.join(dir, 'png', 'base-escritorio.png') });
  for (const f of paginas) {
    await pagina.goto('file://' + path.join(dir, f));
    await pagina.evaluate(() => document.fonts.ready);
    await pagina.waitForTimeout(300);
    const salida = path.join(dir, 'png', f.replace('.html', '.png'));
    await pagina.screenshot({ path: salida });
    console.log('VERDE [MAQUETA]: ' + path.relative(process.cwd(), salida));
  }
  await navegador.close();
})();
