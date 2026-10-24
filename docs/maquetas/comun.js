// Piezas comunes de las maquetas BMO-X / Nucleo 2. Solo dibujo estatico.
// Los simbolos de apps son los mismos trazos de src/rootfs/crear-tema.py.
const APP = {
  terminal: '<path d="m15 18 9 6-9 6m14 1h8"/>',
  archivos: '<path d="M10 17h12l4 4h13v15H10Z"/><path d="M10 17v-4h12l4 4"/>',
  equipo: '<rect x="11" y="12" width="27" height="20" rx="2"/><path d="M24 32v6m-8 0h16"/>',
  ajustes: '<circle cx="24" cy="24" r="9"/><circle cx="24" cy="24" r="3"/><path d="M24 10v5m0 18v5M10 24h5m18 0h5M14 14l4 4m12 12 4 4m0-20-4 4M18 30l-4 4"/>',
  navegador: '<circle cx="24" cy="24" r="13"/><ellipse cx="24" cy="24" rx="6" ry="13"/><path d="M11 24h26m-23-7h20m-20 14h20"/>',
  juegos: '<path d="M14 18h20l6 17-5 2-8-8h-6l-8 8-5-2Z"/><path d="M15 22v8m-4-4h8m12-3h1m3 5h1"/>',
  ayuda: '<path d="M18 18a6 6 0 1 1 9 5c-3 1-3 3-3 5m0 5v1"/>',
  editor: '<path d="m15 31 16-18 5 5-16 18-7 2Z"/><path d="m28 17 5 5M12 39h25"/>',
  audio: '<path d="M11 20h6l8-7v23l-8-7h-6Z"/><path d="M30 19q6 5 0 10m5-15q11 10 0 20"/>',
  pantalla: '<rect x="10" y="12" width="29" height="22" rx="2"/><path d="m18 26 5-8 6 11 4-6m-9 11v5m-7 0h14"/>',
  discos: '<ellipse cx="24" cy="14" rx="13" ry="5"/><path d="M11 14v20c0 3 6 5 13 5s13-2 13-5V14"/><path d="M11 24c0 3 6 5 13 5s13-2 13-5"/>',
  ambiente: '<path d="M30 10a14 14 0 1 0 8 22 11 11 0 0 1-8-22Z"/>',
  captura: '<path d="M10 16v-4h4m20 0h4v4M10 32v4h4m20 0h4v-4"/><circle cx="24" cy="24" r="6"/>',
  portapapeles: '<rect x="13" y="11" width="22" height="28" rx="2"/><path d="M19 9h10v5H19Z M18 22h12m-12 6h12m-12 6h7"/>',
  candado: '<rect x="13" y="22" width="22" height="16" rx="2"/><path d="M17 22v-5a7 7 0 0 1 14 0v5m-7 7v4"/>'
};
const MINI = {
  cpu: '<rect x="6" y="6" width="12" height="12" rx="1"/><path d="M9 2v4m6-4v4M9 18v4m6-4v4M2 9h4m-4 6h4m12-6h4m-4 6h4"/>',
  ram: '<rect x="2" y="7" width="20" height="10" rx="1"/><path d="M6 17v3m4-3v3m4-3v3m4-3v3M6 10v4m4-4v4m4-4v4m4-4v4"/>',
  gpu: '<rect x="2" y="6" width="20" height="12" rx="1"/><circle cx="9" cy="12" r="3"/><path d="M15 10h4m-4 4h4M4 18v3"/>',
  red: '<rect x="9" y="2" width="6" height="5"/><rect x="2" y="16" width="6" height="5"/><rect x="16" y="16" width="6" height="5"/><path d="M12 7v4M5 16v-3h14v3"/>',
  vol: '<path d="M4 9h4l5-4v14l-5-4H4Z"/><path d="M16 9q3 3 0 6m3-9q6 6 0 12"/>',
  luna: '<path d="M16 3a9 9 0 1 0 5 13 7 7 0 0 1-5-13Z"/>',
  fin: '<path d="M12 3v8"/><path d="M6.5 6.5a8 8 0 1 0 11 0"/>',
  prompt: '<path d="m4 7 5 5-5 5m8 1h8"/>',
  temp: '<path d="M10 14V5a2 2 0 0 1 4 0v9a4 4 0 1 1-4 0Z"/>',
  teclado: '<rect x="2" y="6" width="20" height="12" rx="2"/><path d="M6 10h1m3 0h1m3 0h1m3 0h1M7 14h10"/>'
};
const mini = (n, color) => `<svg class="i" viewBox="0 0 24 24" ${color ? `style="color:${color}"` : ''}>${MINI[n]}</svg>`;
const app = (n, color, extra = '') => `<svg viewBox="0 0 48 48" fill="none" style="stroke:${color};color:${color}" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" ${extra}>${APP[n]}</svg>`;

function barra(o = {}) {
  const esc = (o.escritorios || [[1, 'ocupado'], [2, 'activo'], [3, 'ocupado'], [4, ''], [5, '']])
    .map(([n, c]) => `<div class="esc ${c}">${n}</div>`).join('');
  return `<div class="barra">
  <div class="grupo"><div class="inicio">${mini('prompt')}BMO-X</div>
    <div class="escritorios">${esc}</div>
    <div class="titulo-ventana">${o.titulo || 'foot — ~/proyectos/ULTRA-LinuX'}</div></div>
  <div class="centro"><span class="reloj">${o.hora || '23:19'}</span><span class="fecha">${o.fecha || 'vie 23 oct'}</span></div>
  <div class="grupo der">
    <div class="mod" style="color:var(--morado)">${mini('gpu')}RTX 3060 <span>12 %</span><small>41 °C</small></div>
    <div class="mod sep" style="color:var(--cyan)">${mini('cpu')}8 %<small>46 °C</small></div>
    <div class="mod sep" style="color:var(--verde)">${mini('ram')}5,8 GiB</div>
    <div class="mod sep" style="color:var(--acero)">${mini('red')}enp5s0 <small>↓ 2,1 MB/s</small></div>
    <div class="mod sep" style="color:var(--cyan-claro)">${mini('vol')}100 %</div>
    <div class="mod sep" style="color:var(--texto-2)">${mini('teclado')}LATAM</div>
    <div class="mod sep" style="color:var(--cyan)">${mini('luna')}</div>
    <div class="mod boton-fin">${mini('fin')}</div>
  </div></div>`;
}

function dock(o = {}) {
  const abiertas = o.abiertas ?? ['navegador', 'terminal', 'archivos'];
  const foco = o.foco ?? 'terminal';
  const lista = [['navegador', '#b496e8'], ['juegos', '#8dcdaa'], ['archivos', '#66e1eb'], ['terminal', '#8dcdaa'], ['editor', '#ba9bea']];
  const botones = lista.map(([n, c]) => `<div class="app ${abiertas.includes(n) ? 'abierta' : ''} ${foco === n ? 'foco' : ''}">${app(n, c)}</div>`).join('');
  return `<div class="dock">${botones}<div class="linea"></div><div class="app">${app('equipo', '#a987e6')}</div><div class="app">${app('ayuda', '#93b2b6')}</div></div>`;
}

function sello(texto) {
  if (location.hash === '#limpio') return '';
  return `<div class="sello">Maqueta · ${texto} · datos de ejemplo</div>`;
}
