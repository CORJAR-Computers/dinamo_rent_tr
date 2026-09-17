#!/usr/bin/env node
// smoke-dev.mjs — Smoke E2E en dev con BD AISLADA y VACÍA de rentas.
//
// A diferencia de `smoke:app` (que corre contra la BD que haya), este
// orquestador garantiza el escenario completo del flujo de cobro:
//
//   1. Crea un data_dir temporal (`scripts/.tmp-smoke-data`, ignorado por git).
//   2. Lo siembra con `seed_ci <dir>`: config.ini + BD con admin/autos/
//      clientes, pero SIN rentas → la tabla de /rentas arranca vacía y el
//      smoke entra al branch de "renta de prueba" (el único que conoce la
//      base monetaria y puede asertar los totales de la extensión).
//   3. Lanza `tauri dev` con DINAMO_DATA_DIR apuntando al dir temporal y CDP
//      en 9222 (WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS).
//   4. Corre `smoke-test-app.mjs`: login → renta de prueba (5 días ×
//      $150.000, sin IVA) → pago → extensión decimal (+2 h × $25.000,5 =
//      $50.001 → total $800.001) → segunda extensión acumulativa (+1 día ×
//      $50.000 → total $850.001) → orden → contrato → gate anti-[devGuard].
//   5. Limpia procesos (node/npm/tauri/dinamo-rent) y reporta si quedaron
//      residuos del data_dir (Windows puede sostener el .fdb unos segundos
//      tras matar el proceso).
//
// Uso: npm run smoke:dev
//      node scripts/smoke-dev.mjs [--mantener]  (conserva el data_dir para inspección)

import { spawn, spawnSync } from 'node:child_process';
import { existsSync, rmSync } from 'node:fs';
import { join, resolve } from 'node:path';

const MANTENER = process.argv.includes('--mantener');
const RAIZ = resolve(import.meta.dirname, '..');
const DATA_DIR = join(RAIZ, 'scripts', '.tmp-smoke-data');
const FDB = join(DATA_DIR, 'dinamo_rent_v3.fdb');
const PUERTO_CDP = process.env.CDP_PORT || '9222';

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function matarArbol(pid) {
	// taskkill /T mata el árbol (cargo → tauri → vite/app), /F forzado.
	spawnSync('taskkill', ['/PID', String(pid), '/T', '/F'], { stdio: 'ignore' });
}

/** Mata los procesos que escuchan en el puerto dado (limpieza defensiva
 *  por si dinamo-rent.exe sobrevive al taskkill /T del árbol principal). */
async function matarPuerto(puerto) {
	const r = spawnSync('netstat', ['-ano'], { encoding: 'utf8' });
	const pids = new Set();
	for (const l of (r.stdout || '').split('\n')) {
		if (l.includes(`:${puerto} `) && l.includes('LISTENING')) {
			const pid = l.trim().split(/\s+/).pop();
			if (/^\d+$/.test(pid) && pid !== '0') pids.add(pid);
		}
	}
	for (const pid of pids) spawnSync('taskkill', ['/PID', pid, '/T', '/F'], { stdio: 'ignore' });
}

async function esperarPuertoLibre(puerto, ms) {
	const fin = Date.now() + ms;
	while (Date.now() < fin) {
		const r = spawnSync('netstat', ['-ano'], { encoding: 'utf8' });
		const ocupado = (r.stdout || '')
			.split('\n')
			.some((l) => l.includes(`:${puerto} `) && l.includes('LISTENING'));
		if (!ocupado) return true;
		await sleep(500);
	}
	return false;
}

/** Espera a que el puerto CDP suba (polling 1 s). Retorna null si subió,
 *  'timeout' si expiró el tiempo de espera. */
async function esperarCdp(puerto, ms) {
	const fin = Date.now() + ms;
	while (Date.now() < fin) {
		const r = spawnSync('netstat', ['-ano'], { encoding: 'utf8' });
		const ok = (r.stdout || '')
			.split('\n')
			.some((l) => l.includes(`:${puerto} `) && l.includes('LISTENING'));
		if (ok) return null;
		await sleep(1000);
	}
	return 'timeout';
}

async function main() {
	console.log('== smoke:dev — flujo completo con BD aislada y vacía ==');
	if (!existsSync(FDB)) {
		console.log('— sembrando BD aislada (seed_ci)…');
		const r = spawnSync('cargo', ['run', '--features', 'dev', '--bin', 'seed_ci', '--', DATA_DIR], {
			cwd: join(RAIZ, 'src-tauri'),
			stdio: 'inherit'
		});
		if (r.status !== 0) throw new Error('seed_ci falló');
	} else {
		console.log(
			'— BD aislada ya sembrada (reutilizando; borra scripts/.tmp-smoke-data para renovar)'
		);
	}
	if (existsSync(FDB)) console.log('   BD aislada:', FDB);

	console.log('— lanzando tauri dev (BD aislada + CDP ' + PUERTO_CDP + ')…');
	// Vite regenera .svelte-kit en el arranque (sync). Borrarlo obliga a vite
	// a recrearlo con su propia propiedad; en local es un directorio generado,
	// sin costo.
	rmSync(join(RAIZ, '.svelte-kit'), { recursive: true, force: true });
	// Node moderno rechaza spawn de .cmd sin shell (EINVAL, mitigación CVE): en
	// Windows pasamos por cmd.exe explícito con shell:true.
	// NOTA: la estrategia anterior de degradar con runas /trustlevel:0x20000
	// (para que Chromium/WebView2 acepte --remote-debugging-port en runners
	// elevados) fue descartada porque runas retorna inmediatamente sin esperar
	// al proceso hijo, haciendo que el PID del orquestador sea inútil para
	// taskkill /T y el árbol quede huérfano. En la práctica, los runners de CI
	// heredan WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS al proceso cargo→tauri→app
	// incluso corriendo elevados, porque la variable llega por herencia de
	// entorno del proceso padre (no por la línea de comandos, que sí bloquea
	// Chromium con integridad alta). El lanzamiento directo funciona en CI.
	const esWin = process.platform === 'win32';
	let ejecutable, argv;
	if (!esWin) {
		ejecutable = 'npm';
		argv = ['run', 'tauri', 'dev'];
	} else {
		ejecutable = 'cmd.exe';
		argv = ['/d', '/s', '/c', 'npm run tauri dev'];
	}
	console.log('   lanzamiento: directo (env heredado)');
	const app = spawn(ejecutable, argv, {
		cwd: RAIZ,
		stdio: ['ignore', 'pipe', 'pipe'],
		env: {
			...process.env,
			DINAMO_DATA_DIR: DATA_DIR,
			WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${PUERTO_CDP}`
		}
	});
	let salida = '';
	app.stdout.on('data', (d) => (salida += d));
	app.stderr.on('data', (d) => (salida += d));
	const fin = () => {
		try {
			if (app.pid) matarArbol(app.pid);
		} catch {
			/* noop */
		}
	};

	try {
		// CDP arriba = la ventana WebView2 de la app ya existe (tras compilar).
		const causa = await esperarCdp(PUERTO_CDP, 420000);
		if (causa) {
			const detalle = salida.slice(-1500);
			throw new Error(`CDP ${PUERTO_CDP} no subió en 7 min (${causa}):\n` + detalle);
		}
		await sleep(2000); // margen para que el target page esté servido

		console.log('— corriendo smoke-test-app.mjs…');
		const r = spawnSync('node', [join(RAIZ, 'scripts', 'smoke-test-app.mjs')], {
			cwd: RAIZ,
			stdio: 'inherit',
			env: { ...process.env, CDP_PORT: PUERTO_CDP }
		});
		if (r.status !== 0) {
			// La salida de vite/cargo ayuda a distinguir crash de una carrera
			// de compilación on-demand: incluirla siempre en el error.
			throw new Error(`smoke falló (exit ${r.status}):
--- cola del dev server ---\n${salida.slice(-1000)}`);
		}
	} finally {
		console.log('— cerrando la app y el dev server…');
		fin();
		// El binario de la app puede sobrevivir al árbol npm→cargo: matarlo por
		// nombre si sigue vivo (target/debug solo existe en desarrollo).
		spawnSync('taskkill', ['/IM', 'dinamo-rent.exe', '/F'], { stdio: 'ignore' });
		await matarPuerto(PUERTO_CDP);
		await matarPuerto('5173');
		await esperarPuertoLibre(PUERTO_CDP, 10000);
		await esperarPuertoLibre('5173', 10000);
	}
	if (MANTENER) {
		console.log('--mantener: data_dir conservado en ' + DATA_DIR);
	} else {
		rmSync(DATA_DIR, { recursive: true, force: true });
		if (!existsSync(DATA_DIR)) console.log('✓ limpieza: data_dir aislado eliminado');
		else console.log('⚠ el data_dir quedó bloqueado (BORRAR A MANO): ' + DATA_DIR);
	}
	console.log('LISTO — smoke:dev OK');
}

main()
	.then(() => process.exit(0))
	.catch((e) => {
		console.error('✗ FALLO smoke:dev:', e.message);
		process.exit(1);
	});
