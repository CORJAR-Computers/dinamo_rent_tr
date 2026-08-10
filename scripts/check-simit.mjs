#!/usr/bin/env node
// scripts/check-simit.mjs — Diagnóstico del portal SIMIT en un comando
//
// Comprueba el estado de los dos servicios que usa el Agente SIMIT
// (src-tauri/src/services/simit.rs) para saber cuándo reintentar la
// verificación end-to-end SIN recrear tests temporales:
//
//   1. Captcha Proof-of-Work   → POST qxcaptcha.fcm.org.co/api.php y resuelve
//                                el PoW (mismo algoritmo que el backend).
//   2. Microservicio de consulta → página principal + POST del endpoint de
//                                consulta; si no está claramente caído, hace
//                                una SONDA E2E completa (token real + placa).
//
// Uso:
//   node scripts/check-simit.mjs [--placa AAA000] [--timeout 15000]
//                                 [--solo-captcha | --solo-micro] [--json]
//   npm run check:simit
//
// Códigos de salida: 0 = todo operativo · 1 = error técnico · 2 = SIMIT caído.

import { createHash } from 'node:crypto';

// ─── Contrato del SIMIT (espejo de services/simit.rs) ────────────────────────

const CAPTCHA_URL = 'https://qxcaptcha.fcm.org.co/api.php';
const CONSULTA_URL =
	'https://consultasimit.fcm.org.co/simit/microservices/estado-cuenta-simit/estadocuenta/consulta';
const PAGINA_URL = 'https://consultasimit.fcm.org.co/';
const USER_AGENT =
	'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36';
// Sin Origin/Referer el servidor rechaza la petición con 401 (verificado 10-08
// y documentado en la referencia manavarrp/SimitConsulta).
const HEADERS_BROWSER = {
	'User-Agent': USER_AGENT,
	Origin: 'https://www.fcm.org.co',
	Referer: 'https://www.fcm.org.co/',
	Accept: '*/*',
	'Accept-Language': 'es-ES,es;q=0.9'
};
const DIFICULTAD_DEFECTO = 2;
const MAX_ITERACIONES = 10_000_000;

// ─── Utilidades HTTP ──────────────────────────────────────────────────────────

async function request(url, { method = 'GET', headers = {}, body, timeoutMs }) {
	const ctrl = new AbortController();
	const timer = setTimeout(() => ctrl.abort(), timeoutMs);
	try {
		const res = await fetch(url, { method, headers, body, signal: ctrl.signal, redirect: 'follow' });
		const text = await res.text();
		return { status: res.status, statusText: res.statusText, text };
	} finally {
		clearTimeout(timer);
	}
}

// true = OK · null = advertencia · false = fallo
let QUIET = false; // true en modo --json (solo salida JSON)
function statusLine(label, estado, detalle) {
	if (QUIET) return estado === true;
	const marca = estado === true ? '\u2713' : estado === null ? '\u26A0' : '\u2717';
	console.log(`  ${marca} ${label}${detalle ? ` — ${detalle}` : ''}`);
	return estado === true;
}

// ─── Captcha PoW (mismo algoritmo que resolver_pow en simit.rs) ──────────────

function esPrimo(n) {
	if (n < 2) return false;
	if (n === 2) return true;
	if (n % 2 === 0) return false;
	for (let i = 3; i * i <= n; i += 2) if (n % i === 0) return false;
	return true;
}

function sha256Hex(s) {
	return createHash('sha256').update(s, 'utf8').digest('hex');
}

function jsonVerificacion(question, time, nonce) {
	return `{"question":"${question}","time":${time},"nonce":${nonce}}`;
}

function resolverIteracion(question, time, inicio) {
	for (let nonce = inicio + 1; nonce < MAX_ITERACIONES; nonce++) {
		if (!esPrimo(nonce)) continue;
		if (sha256Hex(jsonVerificacion(question, time, nonce)).startsWith('0000')) return nonce;
	}
	throw new Error('No se pudo resolver el captcha Proof-of-Work del SIMIT.');
}

function resolverPow(question, time, difficulty) {
	const partes = [];
	let nonce = 1;
	for (let i = 0; i < difficulty; i++) {
		nonce = resolverIteracion(question, time, nonce);
		partes.push(jsonVerificacion(question, time, nonce));
	}
	return `[${partes.join(',')}]`;
}

// Pide la pregunta del captcha y resuelve el PoW. Devuelve
// { ok: true, token, dificultad } o { ok: false, motivo }.
async function obtenerTokenCaptcha(timeoutMs) {
	let res;
	try {
		res = await request(CAPTCHA_URL, {
			method: 'POST',
			headers: { ...HEADERS_BROWSER, 'Content-Type': 'application/x-www-form-urlencoded' },
			body: 'endpoint=question',
			timeoutMs
		});
	} catch (e) {
		return { ok: false, motivo: `error de red: ${e.cause?.code ?? e.message}` };
	}
	if (res.status !== 200) {
		return { ok: false, motivo: `HTTP ${res.status} ${res.statusText}` };
	}
	let data;
	try {
		data = JSON.parse(res.text);
	} catch {
		return { ok: false, motivo: 'respuesta no JSON' };
	}
	// error puede venir como bool (false/true) o int (0/1)
	const errValue = data.error;
	const conError = errValue === true || (typeof errValue === 'number' && errValue !== 0);
	if (conError || !data.data?.question) {
		return { ok: false, motivo: `servidor rechazó la consulta (error=${JSON.stringify(errValue)})` };
	}
	const dificultad = Number(data.data.recommended_difficulty) > 0
		? Number(data.data.recommended_difficulty)
		: DIFICULTAD_DEFECTO;
	const time = Math.floor(Date.now() / 1000);
	try {
		return { ok: true, token: resolverPow(data.data.question, time, dificultad), dificultad };
	} catch (e) {
		return { ok: false, motivo: `PoW falló: ${e.message}` };
	}
}

async function comprobarCaptcha(timeoutMs) {
	if (!QUIET) console.log(`\n[Captcha] qxcaptcha.fcm.org.co`);
	const t0 = performance.now();
	const r = await obtenerTokenCaptcha(timeoutMs);
	const ms = Math.round(performance.now() - t0);
	if (!r.ok) return { ok: false, motivo: r.motivo };
	return {
		ok: true,
		detalle: `HTTP 200 · PoW (dificultad ${r.dificultad}) resuelto en ${ms} ms · token ${r.token.length} B`
	};
}

// ─── Microservicio de consulta ────────────────────────────────────────────────

// Clasifica una respuesta del endpoint de consulta.
// Firma conocida de gateway caído: 401 {"codigo":5,"descripcion":"Autenticación
// fallida: Acceso denegado..."} — el mismo para cualquier petición (con o sin
// token), verificado el 09-08 y 10-08 contra el portal real.
function clasificarConsulta(res) {
	if (res.status === 503 || /server-unavailable/i.test(res.text)) {
		return { up: false, firma: 'down', texto: `HTTP 503 — Server-unavailable!` };
	}
	if (res.status === 401 || /autenticación fallida|acceso denegado|politica de seguridad/i.test(res.text)) {
		return { up: false, firma: 'gateway', texto: 'HTTP 401 — gateway de seguridad bloquea (portal caído)' };
	}
	if (res.status === 200) {
		return { up: true, firma: 'up', texto: 'HTTP 200 — endpoint responde' };
	}
	return { up: null, firma: 'indefinido', texto: `HTTP ${res.status} — estado indefinido, sonda E2E` };
}

async function comprobarMicroservicio(timeoutMs, placa, conCaptcha) {
	if (!QUIET) console.log(`\n[Microservicio] consultasimit.fcm.org.co`);

	// 1) Página principal (2xx = OK · 4xx/5xx = advertencia · red = fallo)
	let pagina = 'error de red';
	let estadoPagina = false;
	try {
		const res = await request(PAGINA_URL, { headers: HEADERS_BROWSER, timeoutMs });
		pagina = `HTTP ${res.status}${res.statusText ? ` ${res.statusText}` : ''}`;
		estadoPagina = res.status < 400 ? true : null;
	} catch (e) {
		pagina = `error de red: ${e.cause?.code ?? e.message}`;
	}
	statusLine('Página principal', estadoPagina, pagina);

	// 2) Consulta SIN token (firma rápida de gateway)
	let probe;
	try {
		const res = await request(CONSULTA_URL, {
			method: 'POST',
			headers: { ...HEADERS_BROWSER, 'Content-Type': 'application/json' },
			body: JSON.stringify({ filtro: placa, reCaptchaDTO: { response: '[]', consumidor: '1' } }),
			timeoutMs
		});
		probe = clasificarConsulta(res);
	} catch (e) {
		probe = { up: null, firma: 'red', texto: `error de red: ${e.cause?.code ?? e.message}` };
	}
	statusLine(`POST consulta (sin token) — ${probe.texto}`, probe.up === true);

	// 3) Sonda E2E con token real solo si el servicio parece vivo (ni caído ni
	//    con error de red previo: tras un fallo de red no tiene sentido resolver)
	if ((probe.up === true || probe.firma === 'indefinido') && conCaptcha) {
		const captcha = await obtenerTokenCaptcha(timeoutMs);
		if (!captcha.ok) {
			statusLine('Sonda E2E', false, captcha.motivo);
			return { ok: false, up: probe.up, firma: probe.firma };
		}
		const token = captcha.token;
		try {
			const res = await request(CONSULTA_URL, {
				method: 'POST',
				headers: { ...HEADERS_BROWSER, 'Content-Type': 'application/json' },
				body: JSON.stringify({ filtro: placa, reCaptchaDTO: { response: token, consumidor: '1' } }),
				timeoutMs
			});
			const cls = clasificarConsulta(res);
			if (cls.up === true) {
				let multas = 0;
				let total = 0;
				try {
					const dto = JSON.parse(res.text);
					multas = Array.isArray(dto.multas) ? dto.multas.length : 0;
					total = dto.multas?.reduce((s, m) => s + (Number(m.valorPagar) || 0), 0) || 0;
				} catch { /* sin JSON: se reporta el código */ }
				const detalle = multas > 0
					? `HTTP 200 · ${multas} multa(s) · $${total.toLocaleString('es-CO')}`
					: 'HTTP 200 · sin multas';
				statusLine(`Sonda E2E (placa ${placa})`, true, detalle);
				return { ok: true, up: true, firma: 'up' };
			}
			statusLine(`Sonda E2E (placa ${placa})`, false, cls.texto);
			return { ok: false, up: false, firma: cls.firma };
		} catch (e) {
			statusLine('Sonda E2E', false, `error de red: ${e.cause?.code ?? e.message}`);
			return { ok: false, up: null, firma: 'red' };
		}
	}

	return { ok: probe.up === true, up: probe.up, firma: probe.firma };
}

// ─── CLI ──────────────────────────────────────────────────────────────────────

function ayuda() {
	console.log(`Uso: node scripts/check-simit.mjs [opciones]

Comprueba el estado del portal SIMIT (captcha PoW + microservicio) para
saber cuándo reintentar la verificación end-to-end del Agente SIMIT.

Opciones:
  --placa <PLACA>     Placa de prueba para la sonda E2E (default: AAA000, env SIMIT_PLACA)
  --timeout <ms>      Timeout por petición (default: 15000, env SIMIT_TIMEOUT_MS)
  --solo-captcha      Solo comprueba el captcha
  --solo-micro        Solo comprueba el microservicio (sin sonda E2E)
  --json              Salida JSON (para scripts/CI)
  --ayuda, -h         Muestra esta ayuda

Códigos de salida: 0 = operativo · 1 = error técnico · 2 = SIMIT caído`);
}

const args = process.argv.slice(2);
const valorTimeout = (v) => (Number.isFinite(v) && v > 0 ? v : 15000);
const opciones = { placa: process.env.SIMIT_PLACA || 'AAA000', timeoutMs: valorTimeout(Number(process.env.SIMIT_TIMEOUT_MS)), json: false, soloCaptcha: false, soloMicro: false };
for (let i = 0; i < args.length; i++) {
	const a = args[i];
	if (a === '--placa') opciones.placa = args[++i];
	else if (a === '--timeout') opciones.timeoutMs = valorTimeout(Number(args[++i]));
	else if (a === '--solo-captcha') opciones.soloCaptcha = true;
	else if (a === '--solo-micro') opciones.soloMicro = true;
	else if (a === '--json') opciones.json = true;
	else if (a === '--ayuda' || a === '-h') { ayuda(); process.exit(0); } // seguro: aún no hay sockets pendientes
}

QUIET = opciones.json; // en modo --json solo se imprime el JSON final
const reporte = { fecha: new Date().toISOString(), placa: opciones.placa, captcha: null, microservicio: null, resultado: null };

try {
	if (!opciones.json) {
		console.log('─ Dinamo Rent — Chequeo SIMIT ────────────────────────────');
		console.log(`⏱  ${new Date().toLocaleString('es-CO')}`);
	}

	const conCaptcha = !opciones.soloMicro;
	const conMicro = !opciones.soloCaptcha;

	if (conCaptcha) {
		reporte.captcha = await comprobarCaptcha(opciones.timeoutMs);
		if (!opciones.json && reporte.captcha.ok) {
			statusLine('Captcha operativo', true, reporte.captcha.detalle);
		} else if (!opciones.json) {
			statusLine('Captcha', false, reporte.captcha.motivo);
		}
	}

	if (conMicro) {
		const r = await comprobarMicroservicio(opciones.timeoutMs, opciones.placa, conCaptcha);
		reporte.microservicio = r;
	}

	// Resultado global
	const captchaOk = !conCaptcha || reporte.captcha?.ok === true;
	const microUp = reporte.microservicio?.up;
	const hayErrorTecnico =
		(conCaptcha && reporte.captcha?.ok === false && reporte.captcha?.motivo?.startsWith('error de red')) ||
		(conMicro && reporte.microservicio?.firma === 'red');

	// Orden importa: un error técnico de red es distinto de "servicio caído".
	if (hayErrorTecnico) {
		reporte.resultado = 'error_tecnico';
	} else if (conCaptcha && reporte.captcha?.ok === false) {
		reporte.resultado = 'captcha_caido';
	} else if (conMicro && microUp === false) {
		reporte.resultado = 'micro_caido';
	} else if (conMicro && microUp === true && captchaOk) {
		reporte.resultado = 'operativo';
	} else if (conCaptcha && !conMicro) {
		reporte.resultado = captchaOk ? 'operativo' : 'captcha_caido';
	} else {
		reporte.resultado = 'indefinido';
	}

	if (!opciones.json) {
		const ok = reporte.resultado === 'operativo';
		const mensaje = {
			operativo: '✓ SIMIT OPERATIVO — la E2E se puede reintentar',
			captcha_caido: '✗ Captcha NO operativo',
			micro_caido: '✗ Microservicio NO operativo (portal caído)',
			error_tecnico: '? Error técnico de red — reintentar más tarde',
			indefinido: '? Estado indefinido — revisar salida'
		}[reporte.resultado];
		console.log(`\nRESULTADO: ${mensaje}`);
	} else {
		console.log(JSON.stringify(reporte, null, 2));
	}

	// NOTA: se usa `process.exitCode` en lugar de `process.exit()`: en Windows,
	// salir con `process.exit()` mientras undici cierra los sockets del fetch
	// dispara una aserción de libuv (UV_HANDLE_CLOSING) que devuelve exit 127.
	// Con `process.exitCode` el event loop drena y el código de salida es exacto.
	process.exitCode = reporte.resultado === 'operativo' ? 0 : hayErrorTecnico ? 1 : 2;
} catch (e) {
	console.error(`Error inesperado: ${e.message}`);
	process.exitCode = 1;
}
