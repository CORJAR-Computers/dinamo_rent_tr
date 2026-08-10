#!/usr/bin/env node
// scripts/watch-simit.mjs — Vigilante del portal SIMIT
//
// Re-ejecuta `check-simit.mjs` cada N horas (default: cada 2 h, el mismo
// intervalo de sincronización del Agente SIMIT) para detectar CUANDO el portal
// vuelva a estar operativo — sin intervención manual. Pensado para el corte
// del 10-08 en que los subdominios qxcaptcha/consultasimit dejaron de resolver.
//
// Comportamiento:
//   - Cada intento corre `node scripts/check-simit.mjs --json` y lo registra
//     (timestamp + resultado) en el log de sesión.
//   - Mientras el portal siga caído o el DNS sin resolver, sigue esperando.
//   - En cuanto `check-simit` devuelva 0 (SIMIT operativo), imprime el aviso,
//     deja el log en el sitio y termina con código 0.
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

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const ahora = () => new Date().toLocaleString('es-CO');

function parseArgs(argv) {
	const opts = {
		intervalH: Number(process.env.SIMIT_WATCH_INTERVAL_H || 2),
		maxH: Number(process.env.SIMIT_WATCH_MAX_H || 168),
		json: false
	};
	for (let i = 0; i < argv.length; i++) {
		const a = argv[i];
		const val = () => Number(argv[++i]);
		if (a === '--interval') opts.intervalH = val();
		else if (a === '--max-horas') opts.maxH = val();
		else if (a === '--json') opts.json = true;
		else if (a === '--ayuda' || a === '-h') {
			console.log(`Uso: node scripts/watch-simit.mjs [--interval 2] [--max-horas 168] [--json]

Re-ejecuta check-simit.mjs cada N horas hasta que el portal SIMIT vuelva
(sale con 0 cuando está operativo; Ctrl+C para detener).

Opciones:
  --interval <horas>   Intervalo entre chequeos (default: 2, env SIMIT_WATCH_INTERVAL_H)
  --max-horas <n>      Tiempo máximo total (default: 168 = 7 días, env SIMIT_WATCH_MAX_H). 0 = ilimitado
  --json               Solo salida JSON del último intento`);
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

/** Ejecuta un chequeo y devuelve { codigo, reporte } (reporte = JSON o null). */
function ejecutarChequeo() {
	return new Promise((resolve) => {
		const child = spawn(process.execPath, [CHECK_SCRIPT, '--json'], {
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

	for (;;) {
		if (Date.now() - inicio >= limiteMs) {
			if (!opts.json) console.log(`✗ Límite de ${opts.maxH} h alcanzado sin recuperación del portal.`);
			process.exitCode = 1;
			return;
		}
		intentos += 1;
		const t0 = Date.now();
		const { codigo, reporte, stderr } = await ejecutarChequeo();
		const ms = Math.round(Date.now() - t0);
		const resultado = reporte?.resultado ?? (codigo === 0 ? 'operativo' : `codigo_${codigo}`);
		const linea = `${new Date().toISOString()} | intento #${intentos} | ${resultado} | ${ms} ms${reporte?.dns && reporte.dns.ok === false ? ' | DNS sin resolver' : ''}${stderr ? ` | stderr: ${stderr.slice(0, 120)}` : ''}`;
		registrarLog(linea);

		if (codigo === 0) {
			if (!opts.json) {
				console.log(`\n🎉 ${ahora()} — ¡El portal SIMIT VOLVIÓ! (intento #${intentos})`);
				console.log(`   Último chequeo: SIMIT operativo — la E2E se puede reintentar.`);
				console.log(`   Log completo: ${LOG_FILE}`);
			} else {
				console.log(JSON.stringify({ fecha: new Date().toISOString(), operativo: true, intentos, resultado }));
			}
			process.exitCode = 0;
			return;
		}

		if (!opts.json) {
			console.log(`  ${ahora()} · #${intentos} ${resultado}${reporte?.dns && reporte.dns.ok === false ? ' (DNS sin resolver)' : ''} — sigue caído, próximo en ${opts.intervalH} h`);
		}
		await sleep(intervaloMs);
	}
}

main().catch((e) => {
	console.error(`Error inesperado: ${e.message}`);
	process.exitCode = 1;
});
