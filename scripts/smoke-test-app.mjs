#!/usr/bin/env node
// smoke-test-app.mjs — Humo-test de la app compilada (Tauri + WebView2) vía CDP.
//
// Controla el binario real de la app usando el protocolo CDP de WebView2,
// recorre el flujo de negocio (login → rentas → pago → orden → contrato),
// verifica el aviso de impresión y captura los PDFs reales con
// `Page.printToPDF` — el mismo pipeline de renderizado que usa el diálogo de
// impresión, dentro del runtime WebView2 real de la app.
//
// Requisitos:
//   - La app debe estar lanzada con depuración remota de WebView2:
//       WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222 \
//         ./src-tauri/target/release/dinamo-rent.exe
//   - La BD debe tener un usuario admin activo (dev: `dev_reset_admin`).
//   - La UI usa selectores del flujo de rentas/impresión; si cambian los
//     componentes (selectores de botones, ids del login, clases .print-area)
//     hay que actualizar este script.
//
// Uso:
//   node scripts/smoke-test-app.mjs [--puerto 9222] [--pwd Admin123!] [--dir .tmp-print]
//
// Códigos de salida: 0 = OK · 1 = fallo del humo-test.

import { mkdirSync, writeFileSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';

const AYUDA = `Humo-test de la app compilada (Tauri + WebView2) vía CDP.

Controla la app real, recorre el flujo de negocio (login → rentas → pago →
orden → contrato), verifica el aviso de impresión y captura los PDFs reales.

Requisito: la app debe estar lanzada con depuración remota de WebView2:
  WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222 \\
    ./src-tauri/target/release/dinamo-rent.exe

Opciones:
  --puerto <n>   puerto CDP de WebView2 (default 9222, env CDP_PORT)
  --pwd <pass>   contraseña del admin (default 'Admin123!', env APP_PWD)
  --dir <dir>    directorio de salida de PDFs y capturas (default .tmp-print)
  --ayuda        muestra esta ayuda y sale

Códigos de salida: 0 = OK · 1 = fallo del humo-test.
`;

function parseArgs(argv) {
  const opts = {
    puerto: Number(process.env.CDP_PORT || 9222),
    pwd: process.env.APP_PWD || 'Admin123!',
    dir: join(process.cwd(), '.tmp-print'),
    ayuda: false,
  };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    const val = () => argv[++i];
    if (a === '--ayuda' || a === '-h' || a === '--help') opts.ayuda = true;
    else if (a === '--puerto' || a === '--port') opts.puerto = Number(val());
    else if (a === '--pwd' || a === '--password') opts.pwd = val();
    else if (a === '--dir' || a === '--out') opts.dir = resolve(val());
    else if (a.startsWith('--puerto=')) opts.puerto = Number(a.split('=')[1]);
    else if (a.startsWith('--pwd=')) opts.pwd = a.split('=')[1];
    else if (a.startsWith('--dir=')) opts.dir = resolve(a.split('=')[1]);
    else {
      console.error(`Opción desconocida: ${a}\n\n${AYUDA}`);
      process.exit(1);
    }
  }
  return opts;
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function targets(puerto) {
  const r = await fetch(`http://127.0.0.1:${puerto}/json`);
  return r.json();
}

class CDP {
  constructor(ws) {
    this.ws = ws;
    this.id = 0;
    this.pend = new Map();
  }
  static async connect(url) {
    const ws = new WebSocket(url);
    await new Promise((res, rej) => {
      ws.onopen = res;
      ws.onerror = () => rej(new Error('error de conexión WebSocket CDP'));
    });
    const c = new CDP(ws);
    ws.onmessage = (ev) => {
      const m = JSON.parse(ev.data);
      if (m.id && c.pend.has(m.id)) {
        c.pend.get(m.id)(m);
        c.pend.delete(m.id);
      }
    };
    return c;
  }
  send(method, params = {}) {
    const id = ++this.id;
    return new Promise((res) => {
      this.pend.set(id, res);
      this.ws.send(JSON.stringify({ id, method, params }));
    });
  }
  async eval(expression) {
    const r = await this.send('Runtime.evaluate', {
      expression,
      returnByValue: true,
      awaitPromise: true,
    });
    if (r.result?.exceptionDetails) {
      throw new Error(
        'eval error: ' + JSON.stringify(r.result.exceptionDetails).slice(0, 300)
      );
    }
    return r.result?.result?.value;
  }
  close() {
    try {
      this.ws.close();
    } catch {
      /* noop */
    }
  }
}

async function esperar(c, expr, ms, etiqueta) {
  const fin = Date.now() + ms;
  while (Date.now() < fin) {
    const v = await c.eval(expr);
    if (v) return v;
    await sleep(250);
  }
  throw new Error(`timeout esperando: ${etiqueta} (${expr})`);
}

// Setter compatible con inputs bind:value de Svelte (dispara el evento input).
const Rellenar = `(sel, v) => {
  const el = document.querySelector(sel);
  if (!el) return false;
  const set = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
  set.call(el, v);
  el.dispatchEvent(new Event('input', { bubbles: true }));
  return true;
}`;

async function capturarPDF(c, nombre, dir) {
  const r = await c.send('Page.printToPDF', {
    printBackground: true,
    preferCSSPageSize: true,
    displayHeaderFooter: false,
  });
  if (!r.result?.data) throw new Error(`printToPDF no devolvió datos (${nombre})`);
  const path = join(dir, nombre);
  writeFileSync(path, Buffer.from(r.result.data, 'base64'));
  const bytes = statSync(path).size;
  if (bytes < 1000) throw new Error(`PDF sospechosamente pequeño (${bytes} B): ${path}`);
  return { path, bytes };
}

async function main(opts) {
  const { puerto, pwd, dir } = opts;
  mkdirSync(dir, { recursive: true });
  console.log(`== smoke test de la app compilada ==`);
  console.log(`   puerto CDP: ${puerto} · salida: ${dir}`);

  // 1) Esperar el target de la app
  let ts = [];
  for (let i = 0; i < 60; i++) {
    try {
      ts = await targets(puerto);
      if (ts.some((t) => t.type === 'page')) break;
    } catch {
      /* app aún arrancando */
    }
    await sleep(1000);
  }
  const t = ts.find((x) => x.type === 'page');
  if (!t) throw new Error(`no se encontró el target de la app en el puerto ${puerto}`);
  console.log('target:', t.url);

  const c = await CDP.connect(t.webSocketDebuggerUrl);
  await c.send('Page.enable');
  await c.send('Runtime.enable');

  // 2) Login (o sesión activa)
  console.log('— comprobando sesión…');
  let yaLogueado = false;
  try {
    await esperar(c, `!!document.querySelector('#username')`, 3000, 'login');
  } catch {
    yaLogueado = true;
  }
  let ruta = await c.eval(`location.pathname`);
  console.log('ruta actual:', ruta, '| ya logueado:', yaLogueado);
  if (!yaLogueado) {
    console.log('— página de login: autenticando…');
    await c.eval(`(() => {
      const set = ${Rellenar};
      set('#username', 'admin');
      set('#password', '${pwd}');
      return true;
    })()`);
    await sleep(300);
    await c.eval(`document.querySelector('form button[type=submit]')?.click()`);
    await esperar(c, `location.pathname !== '/login'`, 20000, 'post-login');
    ruta = await c.eval(`location.pathname`);
    console.log('ruta tras login:', ruta);
  }

  if (ruta === '/cambiar-password') {
    console.log('— cambio de contraseña forzado…');
    await esperar(c, `!!document.querySelector('#new')`, 10000, 'form-cambio');
    await c.eval(`(() => {
      const set = ${Rellenar};
      set('#current', '${pwd}');
      set('#new', 'Admin123!x');
      set('#confirm', 'Admin123!x');
      return true;
    })()`);
    await c.eval(`document.querySelector('form button[type=submit]')?.click()`);
    await esperar(c, `location.pathname !== '/cambiar-password'`, 15000, 'post-cambio');
    console.log('ruta:', await c.eval(`location.pathname`));
  }

  // 3) Rentas
  console.log('— navegando a /rentas…');
  await c.eval(`document.querySelector('a[href="/rentas"]')?.click()`);
  await esperar(c, `location.pathname === '/rentas'`, 10000, 'ruta rentas');
  await esperar(
    c,
    `document.querySelectorAll('main table tbody tr').length > 0 || document.body.innerText.includes('No hay rentas')`,
    20000,
    'tabla rentas'
  );

  const filas = await c.eval(`document.querySelectorAll('main table tbody tr').length`);
  console.log('rentas en la tabla:', filas);

  if (filas === 0) {
    console.log('— creando una renta de prueba…');
    await c.eval(`[...document.querySelectorAll('button')].find((b) => b.textContent.includes('Nueva Renta'))?.click()`);
    await esperar(c, `!!document.querySelector('input[placeholder="Nombre para la renta"]')`, 10000, 'modal renta');
    await c.eval(`(() => {
      const set = ${Rellenar};
      set('input[placeholder="Nombre para la renta"]', 'Cliente Prueba Final');
      set('input[type="date"]', '2026-08-08');
      set('input[type="date"]', '2026-08-13');
      set('input[inputmode="decimal"][placeholder^="Ej: 150"]', '150000');
      set('input[inputmode="decimal"][placeholder^="Ej: 100"]', '100000');
      set('input[type="number"][min="0"]', '5');
      return true;
    })()`);
    await sleep(300);
    await c.eval(`[...document.querySelectorAll('button')].find((b) => b.textContent.includes('Crear renta'))?.click()`);
    await esperar(c, `document.querySelectorAll('main table tbody tr').length > 0`, 20000, 'renta creada');
    console.log('renta creada OK');
  }

  // 4) Pago (para que la orden muestre la tabla de pagos)
  console.log('— registrando un pago…');
  await c.eval(`document.querySelector('button[title="Registrar pago"]')?.click()`);
  await esperar(c, `!!document.querySelector('input[placeholder="Ej: 200000"]')`, 10000, 'modal pago');
  await c.eval(`(() => { const set = ${Rellenar}; set('input[placeholder="Ej: 200000"]', '100000'); return true; })()`);
  await sleep(200);
  await c.eval(`[...document.querySelectorAll('button')].find((b) => b.textContent.includes('Registrar pago'))?.click()`);
  await esperar(c, `!document.querySelector('input[placeholder="Ej: 200000"]')`, 10000, 'pago cerrado');
  console.log('pago registrado');

  // 5) ORDEN
  console.log('— abriendo modal de orden…');
  await c.eval(`document.querySelector('button[title="Imprimir orden de renta"]')?.click()`);
  await esperar(c, `!!document.querySelector('.print-area.orden-carta')`, 10000, 'modal orden');
  const aviso = await c.eval(`document.body.innerText.includes('Encabezados y pies de página')`);
  if (!aviso) throw new Error('el modal de orden no muestra el aviso «Encabezados y pies de página»');
  console.log('✓ aviso «Encabezados y pies» visible en el modal');
  const shot1 = await c.send('Page.captureScreenshot', { format: 'png' });
  writeFileSync(join(dir, '1-modal-orden.png'), Buffer.from(shot1.result.data, 'base64'));

  await c.eval(`(() => { window.__printOriginal = window.print; window.print = () => { window.__printLlamado = true; }; return true; })()`);
  await c.eval(`[...document.querySelectorAll('button')].find((b) => b.textContent.includes('Imprimir orden'))?.click()`);
  await esperar(c, `!!document.getElementById('print-clone')`, 10000, 'clon orden');
  const { path: pdfOrden } = await capturarPDF(c, 'orden-real.pdf', dir);
  console.log('✓ orden: PDF capturado (' + statSync(pdfOrden).size + ' B)');

  // 6) CONTRATO
  console.log('— abriendo modal de contrato…');
  await esperar(c, `!document.getElementById('print-clone')`, 15000, 'limpieza clon');
  await c.eval(`[...document.querySelectorAll('button')].find((b) => b.textContent.includes('Ver contrato'))?.click()`);
  await esperar(c, `!!document.querySelector('.print-area.contrato-carta')`, 10000, 'modal contrato');
  const shot2 = await c.send('Page.captureScreenshot', { format: 'png' });
  writeFileSync(join(dir, '2-modal-contrato.png'), Buffer.from(shot2.result.data, 'base64'));

  await c.eval(`(() => { window.__printOriginal = window.print; window.print = () => { window.__printLlamado = true; }; return true; })()`);
  await c.eval(`[...document.querySelectorAll('button')].find((b) => b.textContent.includes('Imprimir contrato'))?.click()`);
  await esperar(c, `!!document.getElementById('print-clone')`, 10000, 'clon contrato');
  const { path: pdfContrato } = await capturarPDF(c, 'contrato-real.pdf', dir);
  console.log('✓ contrato: PDF capturado (' + statSync(pdfContrato).size + ' B)');

  // 7) Restaurar el entorno de la app (window.print original)
  await c
    .eval(`(() => {
      if (window.__printOriginal) window.print = window.__printOriginal;
      delete window.__printLlamado;
      delete window.__printOriginal;
      return true;
    })()`)
    .catch(() => {});
  c.close();

  console.log('LISTO — smoke test OK');
  console.log('PDFs:', pdfOrden);
  console.log('      ', pdfContrato);
  console.log('Capturas: 1-modal-orden.png · 2-modal-contrato.png');
}

const opts = parseArgs(process.argv.slice(2));
if (opts.ayuda) {
  console.log(AYUDA);
  process.exit(0);
}

main(opts)
  .then(() => process.exit(0))
  .catch((e) => {
    console.error('✗ FALLO del humo-test:', e.message);
    process.exit(1);
  });
