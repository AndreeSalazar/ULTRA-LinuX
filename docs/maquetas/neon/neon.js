// Piezas de las maquetas BMO-X / Neon. Usa APP, MINI, app() y mini() de ../comun.js.
const AM = '#fcee0a', RO = '#ff2a4f', CY = '#00f0ff', TX = '#ece9dc', T2 = '#a19eae';

function barra(o = {}) {
  const esc = (o.escritorios || [[1, 'ocupado'], [2, 'activo'], [3, 'ocupado'], [4, ''], [5, '']])
    .map(([n, c]) => `<div class="esc ${c}">${String(n).padStart(2, '0')}</div>`).join('');
  return `<div class="barra">
  <div class="grupo"><div class="inicio">${mini('prompt')}BMO-X</div>
    <div class="escritorios">${esc}</div>
    <div class="titulo-ventana">${o.titulo || '<b>foot</b>~/proyectos/ULTRA-LinuX'}</div></div>
  <div class="centro"><span class="reloj">${o.hora || '23<i>:</i>19'}</span><span class="fecha">${o.fecha || 'VIE 23 OCT · LIMA'}</span></div>
  <div class="grupo der">
    <div class="mod"><span class="k">GPU</span><span class="v">12%</span><span class="t">41°</span></div>
    <div class="mod"><span class="k">CPU</span><span class="v">8%</span><span class="t">46°</span></div>
    <div class="mod"><span class="k">RAM</span><span class="v">5.8G</span></div>
    <div class="mod"><span class="k">NET</span><span class="v">↓2.1M</span></div>
    <div class="mod"><span class="k">VOL</span><span class="v">100</span></div>
    <div class="mod"><span class="v" style="color:${T2}">LATAM</span></div>
    <div class="mod" style="color:${CY}">${mini('luna')}</div>
    <div class="boton-fin">${mini('fin')}</div>
  </div></div>`;
}

function dock(o = {}) {
  const abiertas = o.abiertas ?? ['navegador', 'terminal', 'archivos'];
  const foco = o.foco ?? 'terminal';
  const lista = [['navegador', CY], ['juegos', AM], ['archivos', CY], ['terminal', AM], ['editor', CY]];
  const b = lista.map(([n, c]) => `<div class="app ${abiertas.includes(n) ? 'abierta' : ''} ${foco === n ? 'foco' : ''}">${app(n, foco === n ? AM : c)}</div>`).join('');
  return `<div class="dock">${b}<div class="linea"></div><div class="app">${app('equipo', RO)}</div><div class="app">${app('ayuda', T2)}</div></div>`;
}

function sello(texto) {
  if (location.hash === '#limpio') return '';
  return `<div class="sello">Maqueta Neón · ${texto} · datos de ejemplo</div>`;
}
