#!/usr/bin/env node
// review-pendientes.mjs — Revisión visual (temporal) de los pendientes de la
// tarea «Revisión visual en Tauri» del Handsoff, vía CDP de WebView2:
//   1. Modal de inspección de rentas (toggle Salida/Entrada)
//   2. Calendario
//   3. Panel del Agente SIMIT en /comparendos
// Por cada vista: captura PNG + audit de layout (desbordes, imágenes rotas) y
// errores de consola.
//
// Requisitos: app lanzada con
//   WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222
//
// Uso: node scripts/review-pendientes.mjs [--puerto 9222] [--pwd Admin123!] [--dir static/preview-shots]

import { mkdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function parseArgs(argv) {
	const opts = {
		puerto: Number(process.env.CDP_PORT || 9222),
		pwd: process.env.APP_PWD || 'Admin123!',
		dir: join(process.cwd(), 'static', 'preview-shots'),
		ayuda: false
	};
	for (let i = 0; i < argv.length; i++) {
		const a = argv[i];
		const val = () => argv[++i];
		if (a === '--ayuda' || a === '-h') opts.ayuda = true;
		else if (a === '--puerto') opts.puerto = Number(val());
		else if (a === '--pwd') opts.pwd = val();
		else if (a === '--dir') opts.dir = resolve(val());
	}
	return opts;
}

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
			ws.onerror = () => rej(new Error('error conectando WebSocket CDP'));
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
			awaitPromise: true
		});
		if (r.result?.exceptionDetails) {
			throw new Error('eval error: ' + JSON.stringify(r.result.exceptionDetails).slice(0, 300));
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

const Rellenar = `(sel, v) => {
  const el = document.querySelector(sel);
  if (!el) return false;
  const set = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
  set.call(el, v);
  el.dispatchEvent(new Event('input', { bubbles: true }));
  return true;
}`;

async function capturar(c, nombre, dir) {
	const r = await c.send('Page.captureScreenshot', { format: 'png' });
	if (!r.result?.data) throw new Error(`captureScreenshot sin datos (${nombre})`);
	const path = join(dir, nombre);
	writeFileSync(path, Buffer.from(r.result.data, 'base64'));
	return path;
}

/** Audit de layout de la página actual: desbordes, imágenes rotas, texto clave. */
async function audit(c) {
	return c.eval(`(() => {
		const doc = document.documentElement;
		const overX = doc.scrollWidth > window.innerWidth + 1;
		const overY = doc.scrollHeight > window.innerHeight + 1;
		const rotas = [...document.images].filter((i) => !i.complete || i.naturalWidth === 0).map((i) => i.src);
		return {
			ruta: location.pathname,
			viewport: [window.innerWidth, window.innerHeight],
			scroll: [doc.scrollWidth, doc.scrollHeight],
			desbordeX: overX,
			desbordeY: overY,
			imagenesRotas: rotas.slice(0, 5)
		};
	})()`);
}

async function main(opts) {
	const { puerto, pwd, dir } = opts;
	mkdirSync(dir, { recursive: true });
	console.log('== revisión visual pendientes (CDP WebView2) ==');

	// 1) Target de la app
	let ts = [];
	for (let i = 0; i < 90; i++) {
		try {
			ts = await targets(puerto);
			if (ts.some((t) => t.type === 'page')) break;
		} catch {
			/* app aún arrancando */
		}
		await sleep(1000);
	}
	const t = ts.find((x) => x.type === 'page');
	if (!t) throw new Error(`no hay target de la app en el puerto ${puerto}`);
	console.log('target:', t.url);

	const c = await CDP.connect(t.webSocketDebuggerUrl);
	await c.send('Page.enable');
	await c.send('Runtime.enable');
	await c.send('Log.enable');

	// Errores de consola durante toda la sesión.
	// IMPORTANTE: addEventListener — asignar c.ws.onmessage sobrescribiría el
	// dispatcher interno del CDP y colgaría todas las promesas de send().
	const erroresConsola = [];
	c.ws.addEventListener('message', (ev) => {
		const m = JSON.parse(ev.data);
		if (m.method === 'Runtime.consoleAPICalled' && m.params?.type === 'error') {
			const txt = (m.params.args || []).map((a) => a.value ?? a.description ?? '').join(' ');
			erroresConsola.push(`[console] ${txt.slice(0, 200)}`);
		}
		if (m.method === 'Log.entryAdded' && m.params?.entry?.level === 'error') {
			erroresConsola.push(`[log] ${(m.params.entry.text || '').slice(0, 200)}`);
		}
	});

	// 2) Login (o sesión activa)
	let ruta = await c.eval('location.pathname');
	if (ruta === '/login') {
		console.log('— autenticando admin…');
		await c.eval(`(() => { const set = ${Rellenar}; set('#username', 'admin'); set('#password', '${pwd}'); return true; })()`);
		await sleep(300);
		await c.eval(`document.querySelector('form button[type=submit]')?.click()`);
		await esperar(c, `location.pathname !== '/login'`, 20000, 'post-login');
		ruta = await c.eval('location.pathname');
		console.log('ruta tras login:', ruta);
	}
	if (ruta === '/cambiar-password') {
		console.log('— cambio forzado de contraseña (se usará Admin123!x)…');
		await esperar(c, `!!document.querySelector('#new')`, 10000, 'form-cambio');
		await c.eval(`(() => { const set = ${Rellenar}; set('#current', '${pwd}'); set('#new', 'Admin123!x'); set('#confirm', 'Admin123!x'); return true; })()`);
		await c.eval(`document.querySelector('form button[type=submit]')?.click()`);
		await esperar(c, `location.pathname !== '/cambiar-password'`, 15000, 'post-cambio');
		console.log('ruta:', await c.eval('location.pathname'));
	}

	const hallazgos = [];
	const paginas = [];

	// ── A) Modal de inspección de rentas ─────────────────────────────────────
	console.log('— /rentas: abriendo modal de inspección…');
	await c.eval(`document.querySelector('a[href="/rentas"]')?.click() || (location.href = '/rentas')`);
	await esperar(c, `location.pathname === '/rentas'`, 10000, 'ruta rentas');
	await esperar(c, `document.querySelectorAll('main table tbody tr').length > 0 || document.body.innerText.includes('No hay rentas')`, 20000, 'tabla rentas');
	const filas = await c.eval(`document.querySelectorAll('main table tbody tr').length`);
	console.log('rentas en la tabla:', filas);
	paginas.push({ vista: 'rentas-listado', audit: await audit(c) });

	if (filas === 0) {
		hallazgos.push('NO HAY rentas en la BD dev — no se pudo revisar el modal de inspección');
	} else {
		await c.eval(`document.querySelector('button[title="Registrar inspección"]')?.click()`);
		await esperar(c, `!!document.querySelector('[role="tablist"]')`, 10000, 'modal inspección');
		const tituloSalida = await c.eval(`document.querySelector('.card h2')?.textContent`);
		console.log('modal abierto:', tituloSalida);
		paginas.push({ vista: 'inspeccion-salida', audit: await audit(c) });
		await capturar(c, 'revision-1-inspeccion-salida.png', dir);

		// Toggle a Entrada
		await c.eval(`[...document.querySelectorAll('button[role="tab"]')].find((b) => b.textContent.trim() === 'Entrada')?.click()`);
		await sleep(400);
		const tituloEntrada = await c.eval(`document.querySelector('.card h2')?.textContent`);
		console.log('toggle Entrada:', tituloEntrada);
		hallazgos.push(tituloEntrada?.includes('Entrada') ? null : `el toggle a Entrada no actualizó el título del modal (queda: ${tituloEntrada})`);
		paginas.push({ vista: 'inspeccion-entrada', audit: await audit(c) });
		await capturar(c, 'revision-2-inspeccion-entrada.png', dir);

		// Cerrar modal
		await c.eval(`document.querySelector('button[aria-label="Cerrar"]')?.click()`);
		await esperar(c, `!document.querySelector('[role="tablist"]')`, 5000, 'cierre modal');
	}

	// ── B) Calendario ────────────────────────────────────────────────────────
	console.log('— /calendario…');
	await c.eval(`document.querySelector('a[href="/calendario"]')?.click() || (location.href = '/calendario')`);
	await esperar(c, `location.pathname === '/calendario'`, 10000, 'ruta calendario');
	await esperar(c, `document.body.innerText.includes('lun') || document.body.innerText.includes('Lun') || document.querySelectorAll('main .grid').length > 0`, 15000, 'grid calendario');
	const textoCal = await c.eval(`document.body.innerText.slice(0, 300)`);
	console.log('calendario OK — texto:', textoCal.replace(/\s+/g, ' ').slice(0, 120));
	paginas.push({ vista: 'calendario', audit: await audit(c) });
	await capturar(c, 'revision-3-calendario.png', dir);

	// ── C) Panel del Agente SIMIT en /comparendos ────────────────────────────
	console.log('— /comparendos: panel Agente SIMIT…');
	await c.eval(`document.querySelector('a[href="/comparendos"]')?.click() || (location.href = '/comparendos')`);
	await esperar(c, `location.pathname === '/comparendos'`, 10000, 'ruta comparendos');
	await esperar(c, `document.body.innerText.includes('Agente SIMIT') || document.body.innerText.includes('Sincronizar ahora')`, 15000, 'panel SIMIT');
	const panelTexto = await c.eval(`document.body.innerText.includes('Agente SIMIT')`);
	console.log('panel Agente SIMIT visible:', panelTexto);
	const infoPanel = await c.eval(`(() => {
		const card = [...document.querySelectorAll('.card')].find((el) => el.textContent.includes('Agente SIMIT'));
		if (!card) return 'sin card';
		const botones = [...card.querySelectorAll('button')].map((b) => ({ txt: b.textContent.trim().replace(/\\s+/g, ' ').slice(0, 40), disabled: b.disabled }));
		const parrafos = [...card.querySelectorAll('p')].map((p) => p.textContent.trim().replace(/\\s+/g, ' ').slice(0, 90));
		return { parrafos: parrafos.slice(0, 6), botones };
	})()`);
	console.log('panel:', JSON.stringify(infoPanel, null, 1));
	paginas.push({ vista: 'panel-simit', audit: await audit(c) });
	await capturar(c, 'revision-4-panel-simit.png', dir);

	// ── Resumen ──────────────────────────────────────────────────────────────
	const fallos = [];
	for (const p of paginas) {
		const a = p.audit;
		if (a.desbordeX) fallos.push(`${p.vista}: desborde horizontal (scroll ${a.scroll[0]} > viewport ${a.viewport[0]})`);
		if (a.desbordeY) fallos.push(`${p.vista}: desborde vertical de página (scroll ${a.scroll[1]} > viewport ${a.viewport[1]})`);
		if (a.imagenesRotas?.length) fallos.push(`${p.vista}: ${a.imagenesRotas.length} imagen(es) rota(s): ${a.imagenesRotas.join(', ')}`);
	}
	console.log('\n=== AUDIT ===');
	for (const p of paginas) {
		const a = p.audit;
		console.log(`${p.vista}: viewport=${a.viewport.join('x')} scroll=${a.scroll.join('x')} desbordeX=${a.desbordeX} desbordeY=${a.desbordeY} imágenesRotas=${a.imagenesRotas?.length ?? 0}`);
	}
	console.log('errores de consola:', erroresConsola.length ? erroresConsola : 'ninguno');
	console.log('hallazgos:', hallazgos.filter(Boolean).length ? hallazgos.filter(Boolean) : 'ninguno');
	console.log('fallos de layout:', fallos.length ? fallos : 'ninguno');

	c.close();
	process.exit(fallos.length ? 1 : 0);
}

const opts = parseArgs(process.argv.slice(2));
if (opts.ayuda) {
	console.log('Uso: node scripts/review-pendientes.mjs [--puerto 9222] [--pwd Admin123!] [--dir dir]');
	process.exit(0);
}
main(opts).catch((e) => {
	console.error('✗ FALLO:', e.message);
	process.exit(1);
});
