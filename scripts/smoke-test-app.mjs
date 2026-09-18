#!/usr/bin/env node
// smoke-test-app.mjs — Humo-test de la app compilada (Tauri + WebView2) vía CDP.
//
// Controla el binario real de la app usando el protocolo CDP de WebView2,
// recorre el flujo de negocio (login → rentas → pago → extensión → orden →
// contrato), verifica el aviso de impresión y captura los PDFs reales con
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
extensión → orden → contrato), verifica el aviso de impresión y captura los
PDFs reales.

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
		ayuda: false
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

// Directorio de capturas de diagnóstico (mismo lugar que los PDFs del smoke).
let dirCapturas = '.tmp-print';

async function targets(puerto) {
	const r = await fetch(`http://127.0.0.1:${puerto}/json`);
	return r.json();
}

class CDP {
	constructor(ws) {
		this.ws = ws;
		this.id = 0;
		this.pend = new Map();
		/** Consola de la página capturada (consoleAPICalled + Log.entryAdded). */
		this.eventos = [];
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
				return;
			}
			if (m.method === 'Runtime.consoleAPICalled') {
				const texto = (m.params.args || [])
					.map((a) => a.value ?? a.description ?? JSON.stringify(a.preview?.properties ?? []))
					.join(' ');
				c.eventos.push(`console.${m.params.type}: ${texto}`);
			} else if (m.method === 'Log.entryAdded') {
				c.eventos.push(`${m.params.entry.source}: ${m.params.entry.text}`);
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

/** Espera a que alguna fila de la tabla principal muestre el monto dado
 *  (formato es-CO con puntos de miles, p. ej. '800.001'). La frontera evita
 *  falsos positivos: '$ 1.800.001' NO contiene '$ 800.001'. */
async function esperaFila(c, texto, ms, etiqueta) {
	await esperar(
		c,
		`[...document.querySelectorAll('main table tbody tr')].some((tr) => {
      const t = tr.innerText;
      let i = t.indexOf('${texto}');
      while (i !== -1) {
        const prev = i === 0 ? ' ' : t[i - 1];
        if (prev !== '.' && !(prev >= '0' && prev <= '9')) return true;
        i = t.indexOf('${texto}', i + 1);
      }
      return false;
    })`,
		ms,
		etiqueta
	);
	return texto;
}

/** Click sobre el botón cuyo title/aria-label coincide, con reintentos. En dev,
 *  las recargas de Vite (optimización de deps) pueden tragar el click; reintentar
 *  hasta que el modal aparezca (y no solo hasta que el click se dispare).
 *  Devuelve true si el modal apareció; false si se agotaron los reintentos. */
async function clickConReintento(c, title, selectorModal, ms, etiqueta) {
	const fin = Date.now() + ms;
	while (Date.now() < fin) {
		await c.eval(`document.querySelector('button[title="${title}"]')?.click(); true`);
		try {
			await esperar(c, selectorModal, 5000, etiqueta);
			return true;
		} catch {
			/* reintentar */
		}
	}
	return false;
}

/** Captura de pantalla de diagnóstico ante un fallo (queda en .tmp-print). */
async function capturaFallo(c, nombre) {
	try {
		const r = await c.send('Page.captureScreenshot', { format: 'png' });
		const path = join(dirCapturas, `${nombre}.png`);
		writeFileSync(path, Buffer.from(r.result.data, 'base64'));
		console.error('   captura de diagnóstico: ' + path);
	} catch {
		/* noop */
	}
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
		displayHeaderFooter: false
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
	dirCapturas = dir;
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
	// Captura la consola real de la app: el guardrail [devGuard] (solo dev)
	// advierte allí si un payload IPC viaja con tipos incorrectos.
	await c.send('Log.enable').catch(() => {});

	// 2) Login determinista: las sesiones viven en memoria del backend y pueden
	// estar expiradas aunque localStorage diga lo contrario (p. ej. tras relanzar
	// la app en dev). Siempre limpiamos estado y autenticamos de cero.
	// Primero esperar el origen real de la app (Vite en dev): sobre about:blank
	// (ventana antes de la primera navegación) localStorage lanza SecurityError.
	await esperar(
		c,
		`document.readyState === 'complete' && location.protocol === 'http:'`,
		30000,
		'origen de la app (http listo)'
	);
	console.log('— sesión fresca: limpiando estado y autenticando…');
	await c
		.eval(`(() => { localStorage.clear(); sessionStorage.clear(); return true; })()`)
		.catch(() => {});
	// En dev el primer salto a /login dispara la compilación on-demand de la ruta
	// (Vite recién arrancado y, tras borrar .svelte-kit, el grafo entero): puede
	// tardar más de un minuto en una máquina fría — la página queda montada a
	// medias (body vacío) mientras compila. Reintentamos hasta 2 min.
	const finLogin = Date.now() + 120000;
	let ultimoErrLogin;
	while (Date.now() < finLogin) {
		await c.eval(`if (location.pathname !== '/login') location.href = '/login'; true`);
		try {
			await esperar(
				c,
				`location.pathname === '/login' && !!document.querySelector('#username')`,
				12000,
				'form login'
			);
			ultimoErrLogin = null;
			break;
		} catch (e) {
			ultimoErrLogin = e;
		}
	}
	if (ultimoErrLogin) {
		// Diagnóstico: ¿en qué quedó la página? (overlay de vite, error 500, otra ruta…)
		try {
			const d = await c.eval(
				`({ href: location.href, titulo: document.title, cuerpo: (document.body?.innerText || '').slice(0, 300) })`
			);
			console.error(
				'   diagnóstico login:',
				JSON.stringify({ href: d.href, titulo: d.titulo }),
				'\n   cuerpo:',
				d.cuerpo.replace(/\n/g, ' | '),
				'\n   consola (últimas):',
				c.eventos.slice(-5).join(' || ') || '(vacía)'
			);
			await capturaFallo(c, 'fallo-login');
		} catch {
			/* sin diagnóstico */
		}
		throw ultimoErrLogin;
	}
	// Época de la página: si vite hace full-reload (optimización de deps en
	// arranque frío), `window.__smokeEpoca` se regenera y detectamos que el
	// formulario que rellenamos fue arrasado (fallo del run #300).
	const epocaLogin = await c
		.eval(`(window.__smokeEpoca = window.__smokeEpoca || Date.now())`)
		.catch(() => 0);
	let passUsada = pwd;
	const intentarLogin = async (pass) => {
		await c.eval(`(() => {
      const set = ${Rellenar};
      set('#username', 'admin');
      set('#password', '${pass}');
      document.querySelector('#username')?.dispatchEvent(new Event('change', { bubbles: true }));
      document.querySelector('#password')?.dispatchEvent(new Event('change', { bubbles: true }));
      return true;
    })()`);
		await sleep(500);
		const verif = await c
			.eval(
				`(() => ({ epoca: window.__smokeEpoca, u: document.querySelector('#username')?.value, p: !!document.querySelector('#password')?.value }))()`
			)
			.catch(() => ({}));
		if (verif.epoca !== epocaLogin || !verif.u || !verif.p) {
			console.log('   la página se recargó a mitad del llenado; rellenando de nuevo…');
			await c.eval(`(() => {
      const set = ${Rellenar};
      set('#username', 'admin');
      set('#password', '${pass}');
      document.querySelector('#username')?.dispatchEvent(new Event('change', { bubbles: true }));
      document.querySelector('#password')?.dispatchEvent(new Event('change', { bubbles: true }));
      return true;
    })()`);
			await sleep(500);
		}
		await c.eval(`document.querySelector('form button[type=submit]')?.click()`);
		try {
			await esperar(c, `location.pathname !== '/login'`, 15000, 'post-login');
			passUsada = pass;
			return true;
		} catch {
			const diag = await c.eval(`(() => {
				const alertEl = document.querySelector('[role=alert]') || document.querySelector('.text-peligro') || document.querySelector('.text-alerta');
				return {
					uVal: document.querySelector('#username')?.value,
					pVal: !!document.querySelector('#password')?.value,
					alerta: alertEl?.innerText || '(sin alerta)',
					btnDisabled: document.querySelector('form button[type=submit]')?.disabled,
					body: (document.body?.innerText || '').slice(0, 300)
				};
			})()`).catch(() => ({}));
			console.log(`   diagnóstico intento login (${pass}):`, JSON.stringify(diag));
			if (c.eventos.length > 0) {
				console.log('   consola reciente:', c.eventos.slice(-5).join(' || '));
			}
			return false;
		}
	};
	if (!(await intentarLogin(pwd))) {
		console.log('— contraseña por defecto rechazada; probando inicial de fábrica (admin123)…');
		if (!(await intentarLogin('admin123'))) {
			console.log('— admin123 rechazada; probando la del cambio forzado (Admin123!x)…');				if (!(await intentarLogin('Admin123!x'))) {
					// Última carta: los intentos previos pueden haberse perdido por un
					// reload de vite en arranque frío (run #300); repetir la contraseña
					// sembrada ya con la página estable.
					console.log('— reintentando la contraseña sembrada (posible reload de vite)…');
					if (!(await intentarLogin(pwd)))
						throw new Error('login fallido con todas las contraseñas conocidas');
				}
			}
		}
	let ruta = await c.eval(`location.pathname`);
	console.log('ruta tras login:', ruta);

	if (ruta === '/cambiar-password') {
		console.log('— cambio de contraseña forzado…');
		await esperar(c, `!!document.querySelector('#new')`, 10000, 'form-cambio');
		await c.eval(`(() => {
      const set = ${Rellenar};
      set('#current', '${passUsada}');
      set('#new', 'Admin123!x');
      set('#confirm', 'Admin123!x');
      document.querySelector('#current')?.dispatchEvent(new Event('change', { bubbles: true }));
      document.querySelector('#new')?.dispatchEvent(new Event('change', { bubbles: true }));
      document.querySelector('#confirm')?.dispatchEvent(new Event('change', { bubbles: true }));
      return true;
    })()`);
		await sleep(300);
		await c.eval(`document.querySelector('form button[type=submit]')?.click()`);
		await esperar(c, `location.pathname !== '/cambiar-password'`, 15000, 'post-cambio');
		console.log('ruta:', await c.eval(`location.pathname`));
		// El smoke puede rellenar el form de nueva renta con el usuario antiguo;
		// cerrar sesión para que todo corra como 'admin' (el usuario sembrado).
		await c.eval(`localStorage.clear(); sessionStorage.clear();`);
		await c.eval(`location.href = '/login'`);
		await esperar(c, `!!document.querySelector('#username')`, 15000, 're-login');
		if (!(await intentarLogin('Admin123!x')))
			throw new Error('re-login fallido tras el cambio forzado');
		console.log('ruta:', await c.eval(`location.pathname`));
	}

	// 3) Rentas
	// En `tauri dev` Vite puede recargar la página (optimización de deps) y perder
	// el click; reintentamos la navegación hasta que la ruta cambie.
	console.log('— navegando a /rentas…');
	const navegarA = async (href, ms) => {
		const fin = Date.now() + ms;
		let ultimoError;
		while (Date.now() < fin) {
			await c.eval(`document.querySelector('a[href="${href}"]')?.click()`);
			try {
				await esperar(c, `location.pathname === '${href}'`, 5000, `ruta ${href}`);
				return;
			} catch (e) {
				ultimoError = e;
			}
		}
		throw ultimoError;
	};
	await navegarA('/rentas', 45000);
	await esperar(
		c,
		`document.querySelectorAll('main table tbody tr').length > 0 || document.body.innerText.includes('No hay rentas')`,
		20000,
		'tabla rentas'
	);

	const filas = await c.eval(
		`[...document.querySelectorAll('main table tbody tr')].filter((tr) => !tr.innerText.includes('No hay rentas')).length`
	);
	console.log('rentas en la tabla:', filas);

	// true cuando el smoke creó la renta: es la única forma de conocer la base
	// (5 días × $150.000, sin IVA) y poder asertar los totales de la extensión.
	let rentaDePrueba = false;

	if (filas === 0) {
		rentaDePrueba = true;
		console.log('— creando una renta de prueba…');
		await c.eval(
			`[...document.querySelectorAll('button')].find((b) => b.textContent.includes('Nueva Renta'))?.click()`
		);
		await esperar(
			c,
			`!!document.querySelector('input[placeholder="Nombre para la renta"]')`,
			10000,
			'modal renta'
		);
		// Placa: combobox SearchSelect — focus + query filtra; la opción se
		// consulta en OTRO eval (con espera) para dejar a Svelte renderizar el
		// dropdown: en el mismo bloque sincrónico el <li> aún no existe.
		await c.eval(`(() => {
      const combo = document.querySelector('[role="dialog"] input[role="combobox"][aria-label="Placa"]');
      if (!combo) return 'sin-combo';
      combo.focus();
      const setVal = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
      setVal.call(combo, 'ABC123');
      combo.dispatchEvent(new Event('input', { bubbles: true }));
      return true;
    })()`);
		await sleep(500);
		const placaOk = await c.eval(`(() => {
      const opt = document.querySelector('[role="dialog"] li[role="option"]#opt-0');
      if (!opt) return false;
      opt.click();
      return true;
    })()`);
		if (!placaOk) {
			await capturaFallo(c, 'fallo-placa');
			throw new Error('no se pudo seleccionar la placa en el combobox (dropdown sin opciones)');
		}
		await c.eval(`(() => {
      const set = ${Rellenar};
      set('[role="dialog"] input[placeholder="Nombre para la renta"]', 'Cliente Prueba Final');
      // Itinerario: los DOS inputs de fecha comparten type; seleccionar por índice.
      const setVal = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
      const fechas = document.querySelectorAll('[role="dialog"] input[type="date"]');
      const horas = document.querySelectorAll('[role="dialog"] input[type="time"]');
      setVal.call(fechas[0], '2026-08-08');
      fechas[0].dispatchEvent(new Event('input', { bubbles: true }));
      setVal.call(horas[0], '09:00');
      horas[0].dispatchEvent(new Event('input', { bubbles: true }));
      setVal.call(fechas[1], '2026-08-13');
      fechas[1].dispatchEvent(new Event('input', { bubbles: true }));
      setVal.call(horas[1], '09:00');
      horas[1].dispatchEvent(new Event('input', { bubbles: true }));
      // Tarifas base + días (5 días × $150.000, sin IVA) y km de salida.
      set('[role="dialog"] input[inputmode="decimal"][placeholder="150000"]', '150000');
      set('[role="dialog"] input[inputmode="decimal"][placeholder="10000"]', '10000');
      set('[role="dialog"] input[type="number"][min="0"]', '5');
      set('[role="dialog"] input[inputmode="numeric"][placeholder="Ej: 42000"]', '42000');
      const tanque = document.querySelector('[role="dialog"] select.input');
      tanque.value = 'Lleno';
      tanque.dispatchEvent(new Event('change', { bubbles: true }));
      return true;
    })()`);
		await sleep(400);
		await c.eval(
			`[...document.querySelectorAll('button')].find((b) => b.textContent.includes('Crear renta'))?.click()`
		);
		// Esperar una fila REAL de renta (la fila de estado vacío «No hay rentas»
		// también cuenta como <tr>: no sirve como señal de éxito).
		const finCreacion = Date.now() + 25000;
		let creada = false;
		while (Date.now() < finCreacion) {
			creada = await c.eval(
				`[...document.querySelectorAll('main table tbody tr')].some((tr) => tr.innerText.toLowerCase().includes('cliente prueba final'))`
			);
			if (creada) break;
			await sleep(400);
		}
		if (!creada) {
			await capturaFallo(c, 'fallo-crear-renta');
			const visible = await c.eval(`document.body.innerText.slice(0, 700)`);
			console.error('   texto visible en pantalla:', JSON.stringify(visible));
			throw new Error('la renta de prueba no se creó (posible validación rechazada)');
		}
		console.log('renta creada OK');
	}

	// 4) Pago (para que la orden muestre la tabla de pagos)
	console.log('— registrando un pago…');
	if (
		!(await clickConReintento(
			c,
			'Registrar pago',
			`!!document.querySelector('input[placeholder="Ej: 200000"]')`,
			30000,
			'modal pago'
		))
	) {
		await capturaFallo(c, 'fallo-modal-pago');
		const diag = await c.eval(`({
      botonesPago: document.querySelectorAll('button[title="Registrar pago"]').length,
      filas: document.querySelectorAll('main table tbody tr').length,
      fila: document.querySelector('main table tbody tr')?.innerText?.slice(0, 250) ?? null
    })`);
		console.error('   diagnóstico:', JSON.stringify(diag));
		if (c.eventos.length)
			console.error('   consola de la app (últimos):', c.eventos.slice(-8).join(' | '));
		throw new Error('el modal de pago no abrió tras los reintentos');
	}
	await c.eval(
		`(() => { const set = ${Rellenar}; set('input[placeholder="Ej: 200000"]', '100000'); return true; })()`
	);
	await sleep(200);
	await c.eval(
		`[...document.querySelectorAll('button')].find((b) => b.textContent.includes('Registrar pago'))?.click()`
	);
	await esperar(
		c,
		`!document.querySelector('input[placeholder="Ej: 200000"]')`,
		10000,
		'pago cerrado'
	);
	console.log('pago registrado');

	// 4.5) EXTENSIÓN con valor decimal (regresión del incidente extender_renta)
	// Se ejecuta SOLO con la renta de prueba (base conocida: 5 días × $150.000,
	// sin IVA) — es el escenario garantizado de `npm run smoke:dev`, que siembra
	// la BD con seed_ci SIN rentas. Contra una BD con datos (smoke:app) se omite:
	// la base monetaria sería desconocida y los totales no serían asertables.
	// Extiende +2 h con $25.000,5 (el valor decimal que en el incidente del
	// 2026-09-16 viajaba como número y era rechazado) y verifica:
	//   - el backend acepta el comando y el modal se cierra;
	//   - el historial persiste la extensión con su total exacto ($ 50.001);
	//   - el total en la tabla queda consistente SIN doble cobro (la renta de
	//     prueba se crea sin IVA):
	//       base 5×150.000 = 750.000 + extensión 50.001 = 800.001;
	//   - una segunda extensión (+1 día × $50.000) ACUMULA en valor_dia_extra
	//     en vez de sobrescribir: 750.000 + 50.001 + 50.000 = $ 850.001.
	if (rentaDePrueba) {
		console.log('— extendiendo la renta (+2 h, $25.000,5)…');
		if (
			!(await clickConReintento(
				c,
				'Extender renta (agregar horas/días)',
				`!!document.querySelector('input[placeholder="$0"]')`,
				30000,
				'modal extender'
			))
		) {
			await capturaFallo(c, 'fallo-modal-extender');
			throw new Error('el modal de extensión no abrió tras los reintentos');
		}
		await c.eval(`(() => {
    const set = ${Rellenar};
    // Cantidad: 2 horas (input number → bind:number del modal)
    set('[role="dialog"] input[type="number"][min="1"]', '2');
    // Valor unitario DECIMAL (el caso del incidente extender_renta)
    set('input[placeholder="$0"]', '25000.5');
    return true;
  })()`);
		await sleep(200);
		await c.eval(
			`[...document.querySelectorAll('button')].find((b) => b.textContent.includes('Aplicar extensión'))?.click()`
		);
		await esperar(
			c,
			`!document.querySelector('input[placeholder="$0"]')`,
			15000,
			'extensión aplicada'
		);
		console.log('✓ extensión aplicada (valor decimal aceptado por el backend)');

		// El total de la tabla queda consistente SIN doble cobro:
		// 5×150.000 (base) + 50.001 (extensión) = $ 800.001
		const total1 = await esperaFila(
			c,
			'800.001',
			15000,
			'total 750.000 + extensión 50.001 (sin doble cobro)'
		);
		console.log('✓ total tras la extensión:', total1);

		// Reabrir el modal: el historial debe mostrar la extensión persistida
		if (
			!(await clickConReintento(
				c,
				'Extender renta (agregar horas/días)',
				`!!document.querySelector('input[placeholder="$0"]')`,
				30000,
				'modal extender (historial)'
			))
		) {
			await capturaFallo(c, 'fallo-modal-extender-historial');
			throw new Error('el modal de extensión no reabrió tras los reintentos');
		}
		await esperar(
			c,
			`[...document.querySelectorAll('[role="dialog"] span')].some((s) => s.textContent.trim() === '+2h')`,
			10000,
			'historial +2h'
		);
		const extOk = await c.eval(
			`[...document.querySelectorAll('[role="dialog"] span')].some((s) => { const m = s.textContent.match(/\\$\\s?([\\d.]+)/); return !!m && Math.abs(Number(m[1].replace(/\\./g, '')) - 50001) < 0.02; })`
		);
		if (!extOk)
			throw new Error(
				'el historial no muestra el total de la extensión (2 h × $25.000,5 = $ 50.001)'
			);
		console.log('✓ historial de extensiones: +2h con total $ 50.001');
		const shotExt = await c.send('Page.captureScreenshot', { format: 'png' });
		writeFileSync(join(dir, '3-modal-extender.png'), Buffer.from(shotExt.result.data, 'base64'));

		// Segunda extensión (+1 día × $50.000): debe ACUMULAR en valor_dia_extra
		await c.eval(`(() => {
    const sel = document.querySelector('[role="dialog"] select');
    sel.value = 'dias';
    sel.dispatchEvent(new Event('change', { bubbles: true }));
    const set = ${Rellenar};
    set('input[placeholder="$0"]', '50000');
    return true;
  })()`);
		await sleep(200);
		await c.eval(
			`[...document.querySelectorAll('button')].find((b) => b.textContent.includes('Aplicar extensión'))?.click()`
		);
		await esperar(
			c,
			`!document.querySelector('input[placeholder="$0"]')`,
			15000,
			'segunda extensión aplicada'
		);

		// La segunda extensión ACUMULA en valor_dia_extra en vez de sobrescribir:
		// 750.000 + 50.001 + 50.000 = $ 850.001 (si sobrescribiera sería $ 800.000)
		const total2 = await esperaFila(c, '850.001', 15000, 'total con 2 extensiones acumuladas');
		console.log('✓ total con las 2 extensiones acumuladas:', total2);
	} // fin if (rentaDePrueba)

	// 5) ORDEN
	console.log('— abriendo modal de orden…');
	await c.eval(`document.querySelector('button[title="Imprimir orden de renta"]')?.click()`);
	await esperar(c, `!!document.querySelector('.print-area.orden-carta')`, 10000, 'modal orden');
	const aviso = await c.eval(`document.body.innerText.includes('Encabezados y pies de página')`);
	if (!aviso)
		throw new Error('el modal de orden no muestra el aviso «Encabezados y pies de página»');
	console.log('✓ aviso «Encabezados y pies» visible en el modal');
	const shot1 = await c.send('Page.captureScreenshot', { format: 'png' });
	writeFileSync(join(dir, '1-modal-orden.png'), Buffer.from(shot1.result.data, 'base64'));

	await c.eval(
		`(() => { window.__printOriginal = window.print; window.print = () => { window.__printLlamado = true; }; return true; })()`
	);
	await c.eval(
		`[...document.querySelectorAll('button')].find((b) => b.textContent.includes('Imprimir orden'))?.click()`
	);
	await esperar(c, `!!document.getElementById('print-clone')`, 10000, 'clon orden');
	const { path: pdfOrden } = await capturarPDF(c, 'orden-real.pdf', dir);
	console.log('✓ orden: PDF capturado (' + statSync(pdfOrden).size + ' B)');

	// 6) CONTRATO
	console.log('— abriendo modal de contrato…');
	await esperar(c, `!document.getElementById('print-clone')`, 15000, 'limpieza clon');
	await c.eval(
		`[...document.querySelectorAll('button')].find((b) => b.textContent.includes('Ver contrato'))?.click()`
	);
	await esperar(
		c,
		`!!document.querySelector('.print-area.contrato-carta')`,
		10000,
		'modal contrato'
	);
	const shot2 = await c.send('Page.captureScreenshot', { format: 'png' });
	writeFileSync(join(dir, '2-modal-contrato.png'), Buffer.from(shot2.result.data, 'base64'));

	await c.eval(
		`(() => { window.__printOriginal = window.print; window.print = () => { window.__printLlamado = true; }; return true; })()`
	);
	await c.eval(
		`[...document.querySelectorAll('button')].find((b) => b.textContent.includes('Imprimir contrato'))?.click()`
	);
	await esperar(c, `!!document.getElementById('print-clone')`, 10000, 'clon contrato');
	const { path: pdfContrato } = await capturarPDF(c, 'contrato-real.pdf', dir);
	console.log('✓ contrato: PDF capturado (' + statSync(pdfContrato).size + ' B)');

	// 7) Restaurar el entorno de la app (window.print original)
	await c
		.eval(
			`(() => {
      if (window.__printOriginal) window.print = window.__printOriginal;
      delete window.__printLlamado;
      delete window.__printOriginal;
      return true;
    })()`
		)
		.catch(() => {});
	// 8) Guardrail [devGuard]: en flujos reales no debe emitir NINGÚN aviso.
	const devguard = c.eventos.filter((e) => e.includes('[devGuard]'));
	if (devguard.length > 0) {
		console.error(`✗ [devGuard] avisos de contrato en flujos reales (${devguard.length}):`);
		for (const e of devguard) console.error('   ' + e);
		process.exit(1);
	}
	console.log('✓ guardrail [devGuard]: 0 avisos de contrato en todo el flujo');

	c.close();

	console.log('LISTO — smoke test OK');
	console.log('PDFs:', pdfOrden);
	console.log('      ', pdfContrato);
	console.log('Capturas: 1-modal-orden.png · 2-modal-contrato.png · 3-modal-extender.png');
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
