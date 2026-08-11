#!/usr/bin/env node
// scripts/watch-simit.mjs — Vigilante del portal SIMIT
//
// Re-ejecuta `check-simit.mjs` cada N horas (default: 2 h, el mismo intervalo
// de sincronización del Agente SIMIT) para detectar cuándo el portal vuelve a
// estar operativo — sin intervención manual. Nació para el corte del 10-08 en
// que los subdominios qxcaptcha/consultasimit dejaron de resolver.
//
// Desde el 11-08 `check-simit` usa el flujo de la Fase 1: un 401 SIN token ya
// NO se considera caída (el gateway exige captcha PoW) — ante ese 401 corre una
// sonda E2E con token real de UNA solución y solo reporta operativo si la
// consulta responde 200. Este vigilante hereda ese flujo automáticamente y
// registra además la firma del micro (down/gateway/red/indefinido/up) en el
// log de sesión.
//
// Comportamiento:
//   - Cada intento corre `node scripts/check-simit.mjs --json --multas` y lo
//     registra (timestamp + resultado + firma del micro + total pendiente) en
//     el log de sesión.
//   - Mientras el portal siga caído (o el DNS sin resolver), sigue esperando.
//   - En cuanto `check-simit` devuelva 0 (SIMIT operativo: sonda E2E con token
//     real responde 200), imprime el aviso, deja el log en el sitio y termina
//     con código 0.
//   - `--continuo`: no termina al detectar el portal operativo — sigue
//     monitoreando cada intervalo para ALERTAR cuando el total pendiente de la
//     flota cambia (persistido en ultimo_total.json) y cuando el portal cae o
//     vuelve. Termina con Ctrl+C o al alcanzar `--max-horas` (exit 0).
//   - Ctrl+C detiene el vigilante en cualquier momento (sin tocar nada).
//
// Uso:
//   npm run watch:simit
//   node scripts/watch-simit.mjs [--interval 2] [--max-horas 168] [--json]
//
// Opciones:
//   --interval <horas>   Intervalo entre chequeos (default: 2, env SIMIT_WATCH_INTERVAL_H)
//   --max-horas <n>      Tiempo máximo total de vigilancia (default: 168 = 7 días,
//                        env SIMIT_WATCH_MAX_H). 0 = sin límite.
//   --json               Solo salida JSON del último intento (para CI/scripts).

import { spawn } from 'node:child_process';
import { mkdirSync, writeFileSync, existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const CHECK_SCRIPT = join(__dirname, 'check-simit.mjs');
// El log de sesión se guarda en data/ (gitignored), como los informes SIMIT.
const LOG_DIR = join(__dirname, '..', 'data', 'simit_watch');
const LOG_FILE = join(LOG_DIR, 'watch.log');
// Último total pendiente observado por la sonda E2E (para alertar cambios
// del total de la flota entre corridas).
const TOTAL_FILE = join(LOG_DIR, 'ultimo_total.json');

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const ahora = () => new Date().toLocaleString('es-CO');

function parseArgs(argv) {
	const opts = {
		intervalH: Number(process.env.SIMIT_WATCH_INTERVAL_H || 2),
		maxH: Number(process.env.SIMIT_WATCH_MAX_H || 168),
		json: false,
		continuo: false
	};
	for (let i = 0; i < argv.length; i++) {
		const a = argv[i];
		const val = () => Number(argv[++i]);
		if (a === '--interval') opts.intervalH = val();
		else if (a === '--max-horas') opts.maxH = val();
		else if (a === '--continuo') opts.continuo = true;
		else if (a === '--json') opts.json = true;
		else if (a === '--ayuda' || a === '-h') {
			console.log(`Uso: node scripts/watch-simit.mjs [--interval 2] [--max-horas 168] [--json] [--continuo]

Re-ejecuta check-simit.mjs cada N horas hasta que el portal SIMIT vuelva
(sale con 0 cuando está operativo: la sonda E2E con token real responde 200;
Ctrl+C para detener). Con --continuo sigue vigilando tras estar operativo.

Opciones:
  --interval <horas>   Intervalo entre chequeos (default: 2, env SIMIT_WATCH_INTERVAL_H)
  --max-horas <n>      Tiempo máximo total (default: 168 = 7 días, env SIMIT_WATCH_MAX_H). 0 = ilimitado
  --continuo           Vigilancia continua: no termina al detectar el portal operativo;
                       alerta cambios del total pendiente de la flota y caídas/vueltas
                       del portal. Termina con Ctrl+C o al alcanzar --max-horas (exit 0)
  --json               Solo salida JSON por chequeo`);
			process.exit(0);
		}
	}
	if (!Number.isFinite(opts.intervalH) || opts.intervalH <= 0) opts.intervalH = 2;
	if (!Number.isFinite(opts.maxH) || opts.maxH < 0) opts.maxH = 168;
	return opts;
}

const opts = parseArgs(process.argv.slice(2));
const intervaloMs = opts.intervalH * 3600_000;
const limiteMs = opts.maxH === 0 ? Infinity : opts.maxH * 3600_000;

/** Lee el último total pendiente persistido (o null si no existe). */
function leerUltimoTotal() {
	try {
		return JSON.parse(readFileSync(TOTAL_FILE, 'utf8'));
	} catch {
		return null;
	}
}

/**
 * Persiste el total pendiente observado por la sonda E2E y devuelve el cambio
 * vs. el previo: null si no cambió (o no había previo), si no {desde, hasta}.
 */
function registrarTotal(e2e) {
	const actual = {
		totalPendiente: e2e.totalPendiente,
		multas: e2e.multas,
		placa: e2e.placa,
		fecha: new Date().toISOString()
	};
	const previo = leerUltimoTotal();
	writeFileSync(TOTAL_FILE, JSON.stringify(actual, null, 2));
	if (!previo || previo.totalPendiente === actual.totalPendiente) return null;
	return { desde: previo.totalPendiente, hasta: actual.totalPendiente };
}

/** Ejecuta un chequeo y devuelve { codigo, reporte } (reporte = JSON o null). */
function ejecutarChequeo() {
	return new Promise((resolve) => {
		const child = spawn(process.execPath, [CHECK_SCRIPT, '--json', '--multas'], {
			stdio: ['ignore', 'pipe', 'pipe']
		});
		let stdout = '';
		let stderr = '';
		child.stdout.on('data', (d) => (stdout += d));
		child.stderr.on('data', (d) => (stderr += d));
		child.on('close', (codigo) => {
			let reporte = null;
			try {
				reporte = JSON.parse(stdout);
			} catch {
				/* sin JSON: se reporta el código */
			}
			resolve({ codigo: codigo ?? 1, reporte, stderr: stderr.trim() });
		});
	});
}

function registrarLog(linea) {
	mkdirSync(LOG_DIR, { recursive: true });
	const historial = existsSync(LOG_FILE) ? readFileSync(LOG_FILE, 'utf8') : '';
	writeFileSync(LOG_FILE, historial + linea + '\n');
}

async function main() {
	if (!opts.json) {
		console.log(`─ Vigilante SIMIT ────────────────────────────────`);
		console.log(`⏱  ${ahora()} · chequeo cada ${opts.intervalH} h · máx ${opts.maxH} h · log: ${LOG_FILE}`);
	}
	const inicio = Date.now();
	let intentos = 0;
	// Estado previo para avisar transiciones en vigilancia continua
	// (null = sin estado aún · 'operativo' · 'caido').
	let estadoPrevio = null;

	for (;;) {
		if (Date.now() - inicio >= limiteMs) {
			if (!opts.json) {
				console.log(
					opts.continuo
						? `⏹ Vigilancia continua finalizada (${opts.maxH} h).`
						: `✗ Límite de ${opts.maxH} h alcanzado sin recuperación del portal.`
				);
			}
			process.exitCode = opts.continuo ? 0 : 1;
			return;
		}
		intentos += 1;
		const t0 = Date.now();
		const { codigo, reporte, stderr } = await ejecutarChequeo();
		const ms = Math.round(Date.now() - t0);
		const resultado = reporte?.resultado ?? (codigo === 0 ? 'operativo' : `codigo_${codigo}`);
		// Firma del micro del nuevo check-simit (sonda E2E con token real):
		// up = operativo · down = 503 · gateway = 401 con token (bloqueado/caído)
		// red = error de red · indefinido = requiere token (sin confirmar).
		const firma = reporte?.microservicio?.firma;
		const detalleFirma = firma ? ` | micro:${firma}` : '';

		// Sonda E2E con multas (--multas): total pendiente de la flota, para
		// alertar cuando cambie entre corridas (persistido en ultimo_total.json).
		const e2e = reporte?.microservicio?.e2e;
		let cambioTotal = null;
		let textoCambio = '';
		if (e2e && Number.isFinite(e2e.totalPendiente)) {
			cambioTotal = registrarTotal(e2e);
			if (cambioTotal) {
				const delta = cambioTotal.hasta - cambioTotal.desde;
				textoCambio = `⚠ Total pendiente de la flota CAMBIÓ: $${cambioTotal.desde.toLocaleString('es-CO')} → $${cambioTotal.hasta.toLocaleString('es-CO')} (delta ${delta >= 0 ? '+' : ''}${delta.toLocaleString('es-CO')})`;
				registrarLog(`${new Date().toISOString()} | ${textoCambio}`);
				if (!opts.json) console.log(`\n  ${textoCambio}`);
			}
		}
		const totalTexto = e2e ? ` | multas:${e2e.multas} · $${e2e.totalPendiente.toLocaleString('es-CO')}` : '';
		const linea = `${new Date().toISOString()} | intento #${intentos} | ${resultado}${detalleFirma}${totalTexto} | ${ms} ms${reporte?.dns && reporte.dns.ok === false ? ' | DNS sin resolver' : ''}${stderr ? ` | stderr: ${stderr.slice(0, 120)}` : ''}`;
		registrarLog(linea);

		if (codigo === 0) {
			const recienOperativo = estadoPrevio !== 'operativo';
			estadoPrevio = 'operativo';
			if (!opts.json) {
				if (recienOperativo) {
					console.log(`\n🎉 ${ahora()} — ¡El portal SIMIT VOLVIÓ! (intento #${intentos})`);
					console.log(`   Último chequeo: SIMIT operativo — la E2E se puede reintentar.`);
					if (e2e) {
						console.log(`   Total pendiente de la flota (placa ${e2e.placa}): $${e2e.totalPendiente.toLocaleString('es-CO')} · ${e2e.multas} multa(s)`);
					}
					if (cambioTotal) console.log(`   ${textoCambio}`);
					console.log(`   Log completo: ${LOG_FILE}`);
				} else if (opts.continuo) {
					// Ya operativos en vigilancia continua: resumen breve por chequeo
					console.log(`  ${ahora()} · #${intentos} operativo${totalTexto} — ${ms} ms`);
				}
			} else {
				console.log(
					JSON.stringify({
						fecha: new Date().toISOString(),
						operativo: true,
						intentos,
						resultado,
						firma,
						...(e2e ? { totalPendiente: e2e.totalPendiente, multas: e2e.multas } : {}),
						...(cambioTotal ? { cambioTotal } : {})
					})
				);
			}
			if (!opts.continuo) {
				process.exitCode = 0;
				return;
			}
			await sleep(intervaloMs);
			continue;
		}

		// No operativo: avisar la transición operativo → caído (modo continuo)
		if (estadoPrevio === 'operativo') {
			estadoPrevio = 'caido';
			registrarLog(`${new Date().toISOString()} | ⚠ el portal dejó de responder (${resultado})`);
			if (!opts.json) console.log(`\n  ⚠ ${ahora()} — el portal dejó de responder (${resultado}); se sigue vigilando.`);
		}
		if (!opts.json) {
			const sufijoFirma = firma ? ` · micro:${firma}` : '';
			console.log(`  ${ahora()} · #${intentos} ${resultado}${sufijoFirma}${reporte?.dns && reporte.dns.ok === false ? ' (DNS sin resolver)' : ''} — sigue caído, próximo en ${opts.intervalH} h`);
		}
		await sleep(intervaloMs);
	}
}

main().catch((e) => {
	console.error(`Error inesperado: ${e.message}`);
	process.exitCode = 1;
});
