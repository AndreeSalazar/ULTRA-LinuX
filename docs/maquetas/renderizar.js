// Renderiza las maquetas NN-*.html a PNG de 1920x1080 con Chromium.
//   Nucleo 2: docs/maquetas/NN-*.html      -> docs/maquetas/png/
//   Neon:     docs/maquetas/neon/NN-*.html -> docs/maquetas/png/neon/
// Uso: node docs/maquetas/renderizar.js [filtro...]   (ej.: neon/03 o 05)
const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');

const raiz = __dirname;
const sets = [
  { nombre: '', fuente: raiz, salida: path.join(raiz, 'png') },
  { nombre: 'neon/', fuente: path.join(raiz, 'neon'), salida: path.join(raiz, 'png', 'neon') },
];

async function capturar(pagina, url, destino) {
  await pagina.goto(url);
  await pagina.evaluate(() => document.fonts.ready);
  await pagina.waitForTimeout(400);
  await pagina.screenshot({ path: destino });
}

(async () => {
  const filtros = process.argv.slice(2);
  const navegador = await chromium.launch();
  const pagina = await navegador.newPage({ viewport: { width: 1920, height: 1080 } });
  for (const set of sets) {
    const paginas = fs.readdirSync(set.fuente).filter(f => /^\d\d-.*\.html$/.test(f))
      .filter(f => !filtros.length || filtros.some(x => (set.nombre + f).includes(x)));
    if (!paginas.length) continue;
    fs.mkdirSync(set.salida, { recursive: true });
    // Base sin sello para las maquetas que se apoyan en el escritorio.
    await capturar(pagina, 'file://' + path.join(set.fuente, '01-escritorio.html#limpio'), path.join(set.salida, 'base-escritorio.png'));
    for (const f of paginas) {
      const destino = path.join(set.salida, f.replace('.html', '.png'));
      await capturar(pagina, 'file://' + path.join(set.fuente, f), destino);
      console.log('VERDE [MAQUETA]: ' + path.relative(process.cwd(), destino));
    }
  }
  await navegador.close();
})();
