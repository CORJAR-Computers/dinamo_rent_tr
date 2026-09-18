#!/usr/bin/env node
// smoke-dev.mjs — Smoke E2E en dev con BD AISLADA y VACÍA de rentas.
//
// A diferencia de `smoke:app` (que corre contra la BD que haya), este
// orquestador garantiza el escenario completo del flujo de cobro:
//
//   1. Compila app + seed_ci (`cargo build --features dev --bins`; el
//      feature `dev` es vacío y solo excluye esos binarios del bundle de
//      release — sin él `cargo build` no produce seed_ci.exe, corrida 287).
//   2. Crea un data_dir temporal (`scripts/.tmp-smoke-data`, ignorado por
//      git) y lo siembra con `seed_ci <dir>`: config.ini + BD con
//      admin/autos/clientes, pero SIN rentas → /rentas arranca vacía y el
//      smoke entra al branch de "renta de prueba" (el único que conoce la
//      base monetaria y puede asertar los totales de la extensión).
//   3. Habilita la política HKLM de WebView2 para CDP (puerto 9222) en
//      procesos elevados de Windows.
//      (En procesos elevados de Windows, WebView2 150+ ignora variables de
//      entorno de CDP por seguridad; la política oficial de Microsoft en
//      HKLM\SOFTWARE\Policies\Microsoft\Edge\WebView2\AdditionalBrowserArguments
//      habilita el puerto 9222 sin necesidad de tokens restringidos que
//      rompen la memoria compartida de Firebird Embedded).
//   4. Levanta vite (dev server) y lanza el EXE compilado con DINAMO_DATA_DIR.
//   5. Corre `smoke-test-app.mjs`: login → renta de prueba (5 días ×
//      $150.000, sin IVA) → pago → extensión decimal (+2 h × $25.000,5 =
//      $50.001 → total $800.001) → segunda extensión acumulativa (+1 día ×
//      $50.000 → total $850.001) → orden → contrato → gate anti-[devGuard].
//   6. Limpia procesos y archivos temporales.
//
// Uso: npm run smoke:dev
//      node scripts/smoke-dev.mjs [--mantener]  (conserva el data_dir para inspección)

import { spawn, spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, rmSync } from 'node:fs';
import { join, resolve } from 'node:path';

const MANTENER = process.argv.includes('--mantener');
const RAIZ = resolve(import.meta.dirname, '..');
const DATA_DIR = join(RAIZ, 'scripts', '.tmp-smoke-data');
const FDB = join(DATA_DIR, 'dinamo_rent_v3.fdb');
const PUERTO_CDP = process.env.CDP_PORT || '9222';
const EXE_APP = join(RAIZ, 'src-tauri', 'target', 'debug', 'dinamo-rent.exe');
const EXE_SEED = join(RAIZ, 'src-tauri', 'target', 'debug', 'seed_ci.exe');

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function matarArbol(pid) {
	spawnSync('taskkill', ['/PID', String(pid), '/T', '/F'], { stdio: 'ignore' });
}

/** true si algún proceso escucha en el puerto dado (netstat). */
function puertoEscuchando(puerto) {
	const r = spawnSync('netstat', ['-ano'], { encoding: 'utf8' });
	return (r.stdout || '')
		.split('\n')
		.some((l) => l.includes(`:${puerto} `) && l.includes('LISTENING'));
}

/** Mata los procesos que escuchan en el puerto dado. */
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
		if (!puertoEscuchando(puerto)) return true;
		await sleep(500);
	}
	return false;
}

async function esperarPuertoOcupado(puerto, ms) {
	const fin = Date.now() + ms;
	while (Date.now() < fin) {
		if (puertoEscuchando(puerto)) return true;
		await sleep(1000);
	}
	return false;
}

/** En Windows elevado (CI de GitHub), WebView2 ignora las variables de
 *  entorno para el debugging port. La directiva oficial en HKLM instruye
 *  a WebView2 a abrir el puerto sin requerir runas ni tokens degradados. */
function configurarWebView2Policy() {
	if (process.platform !== 'win32') return;
	console.log('— configurando política HKLM de WebView2 para CDP…');
	try {
		spawnSync(
			'reg',
			[
				'add',
				'HKLM\\SOFTWARE\\Policies\\Microsoft\\Edge\\WebView2\\AdditionalBrowserArguments',
				'/v',
				'*',
				'/t',
				'REG_SZ',
				'/d',
				`--remote-debugging-port=${PUERTO_CDP} --remote-allow-origins=*`,
				'/f'
			],
			{ stdio: 'ignore' }
		);
	} catch {
		/* best-effort */
	}
}

function limpiarWebView2Policy() {
	if (process.platform !== 'win32') return;
	try {
		spawnSync(
			'reg',
			[
				'delete',
				'HKLM\\SOFTWARE\\Policies\\Microsoft\\Edge\\WebView2\\AdditionalBrowserArguments',
				'/v',
				'*',
				'/f'
			],
			{ stdio: 'ignore' }
		);
	} catch {
		/* best-effort */
	}
}

/** Siembra la BD aislada con seed_ci. */
function sembrarBd() {
	console.log('— sembrando BD aislada (seed_ci)…');
	mkdirSync(DATA_DIR, { recursive: true });
	const rs = spawnSync(EXE_SEED, [DATA_DIR], {
		cwd: RAIZ,
		stdio: 'inherit',
		env: {
			...process.env,
			DINAMO_DATA_DIR: DATA_DIR
		}
	});
	if (rs.status !== 0) throw new Error(`seed_ci falló (exit ${rs.status})`);
	if (!existsSync(FDB)) throw new Error('seed_ci no produjo la BD: ' + FDB);
	console.log('   BD aislada:', FDB);
}

/** Levanta vite como hijo del orquestador. */
async function levantarVite() {
	console.log('— lanzando vite (dev server)…');
	const vite = spawn('cmd.exe', ['/d', '/s', '/c', 'npm run dev'], {
		cwd: RAIZ,
		stdio: ['ignore', 'pipe', 'pipe'],
		env: { ...process.env }
	});
	let salida = '';
	vite.stdout.on('data', (d) => (salida += d));
	vite.stderr.on('data', (d) => (salida += d));
	const fin = () => {
		try {
			if (vite.pid) matarArbol(vite.pid);
		} catch {
			/* noop */
		}
	};
	if (!(await esperarPuertoOcupado('5173', 120000))) {
		fin();
		throw new Error('vite (5173) no subió en 2 min:\n' + salida.slice(-1500));
	}
	return { fin, cola: () => salida.slice(-1000) };
}

/** Ejecuta smoke-test-app.mjs contra el CDP ya arriba. */
function correrSmoke() {
	return new Promise((resolveP, rejectP) => {
		const r = spawnSync('node', [join(RAIZ, 'scripts', 'smoke-test-app.mjs')], {
			cwd: RAIZ,
			stdio: 'inherit',
			env: { ...process.env, CDP_PORT: PUERTO_CDP }
		});
		if (r.status !== 0) rejectP(new Error(`smoke falló (exit ${r.status})`));
		else resolveP();
	});
}

async function main() {
	console.log('== smoke:dev — flujo completo con BD aislada y vacía ==');
	const esWin = process.platform === 'win32';

	if (!esWin) {
		sembrarBd();
		rmSync(join(RAIZ, '.svelte-kit'), { recursive: true, force: true });

		console.log('— lanzando tauri dev (BD aislada + CDP ' + PUERTO_CDP + ')…');
		const dev = spawn('npm', ['run', 'tauri', 'dev'], {
			cwd: RAIZ,
			stdio: ['ignore', 'pipe', 'pipe'],
			env: {
				...process.env,
				DINAMO_DATA_DIR: DATA_DIR,
				WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${PUERTO_CDP} --remote-allow-origins=*`
			}
		});
		let salida = '';
		dev.stdout.on('data', (d) => (salida += d));
		dev.stderr.on('data', (d) => (salida += d));

		try {
			if (!(await esperarPuertoOcupado(PUERTO_CDP, 420000))) {
				throw new Error(`CDP ${PUERTO_CDP} no subió en 7 min (timeout):\n` + salida.slice(-1500));
			}
			await sleep(2000);
			await correrSmoke();
		} finally {
			try {
				if (dev.pid) matarArbol(dev.pid);
			} catch {
				/* noop */
			}
		}
	} else {
		// ── Rama Windows (directa, sin runas) ──
		configurarWebView2Policy();

		console.log('— compilando app + seed_ci (cargo build --features dev --bins)…');
		const rc = spawnSync('cargo', ['build', '--features', 'dev', '--bins'], {
			cwd: join(RAIZ, 'src-tauri'),
			stdio: 'inherit'
		});
		if (rc.status !== 0) throw new Error(`cargo build falló (exit ${rc.status})`);
		if (!existsSync(EXE_APP)) throw new Error('no existe el binario compilado: ' + EXE_APP);
		if (!existsSync(EXE_SEED)) throw new Error('no existe el binario seed_ci: ' + EXE_SEED);

		rmSync(join(RAIZ, '.svelte-kit'), { recursive: true, force: true });

		sembrarBd();

		const vite = await levantarVite();

		console.log(`— lanzando la app (CDP ${PUERTO_CDP})…`);
		const app = spawn(EXE_APP, [], {
			cwd: RAIZ,
			stdio: ['ignore', 'pipe', 'pipe'],
			env: {
				...process.env,
				DINAMO_DATA_DIR: DATA_DIR,
				WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${PUERTO_CDP} --remote-allow-origins=*`
			}
		});
		let salidaApp = '';
		app.stdout.on('data', (d) => (salidaApp += d));
		app.stderr.on('data', (d) => (salidaApp += d));
		let appTermino = false;
		app.on('exit', (code) => {
			appTermino = true;
			salidaApp += `\n[APP EXITIO: code=${code}]`;
		});

		try {
			const finCdp = Date.now() + 420000;
			let cdpListo = false;
			while (Date.now() < finCdp) {
				if (puertoEscuchando(PUERTO_CDP)) {
					cdpListo = true;
					break;
				}
				if (appTermino) {
					throw new Error('la app murió antes de abrir CDP:\n' + salidaApp.slice(-2000));
				}
				await sleep(1000);
			}
			if (!cdpListo) {
				throw new Error(
					`CDP ${PUERTO_CDP} no subió en 7 min (timeout):\n` +
						`--- cola de la app ---\n${salidaApp.slice(-1500)}\n` +
						`--- cola de vite ---\n${vite.cola()}`
				);
			}

			await sleep(2000); // margen para que el target page esté servido
			await correrSmoke();
		} catch (err) {
			if (salidaApp) {
				console.error('--- Salida reciente de la app (dinamo-rent.exe) ---');
				console.error(salidaApp.slice(-3000));
				console.error('---------------------------------------------------');
			}
			throw err;
		} finally {
			try {
				if (app.pid) matarArbol(app.pid);
			} catch {
				/* noop */
			}
			spawnSync('taskkill', ['/IM', 'dinamo-rent.exe', '/F'], { stdio: 'ignore' });
			await matarPuerto(PUERTO_CDP);
			await matarPuerto('5173');
			await esperarPuertoLibre(PUERTO_CDP, 10000);
			await esperarPuertoLibre('5173', 10000);
			vite.fin();
			limpiarWebView2Policy();
		}
	}

	if (esWin) {
		spawnSync('taskkill', ['/IM', 'seed_ci.exe', '/F'], { stdio: 'ignore' });
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
		limpiarWebView2Policy();
		console.error('✗ FALLO smoke:dev:', e.message);
		process.exit(1);
	});
