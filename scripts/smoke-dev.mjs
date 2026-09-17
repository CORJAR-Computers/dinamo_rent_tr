#!/usr/bin/env node
// smoke-dev.mjs — Smoke E2E en dev con BD AISLADA y VACÍA de rentas.
//
// A diferencia de `smoke:app` (que corre contra la BD que haya), este
// orquestador garantiza el escenario completo del flujo de cobro:
//
//   1. Compila app + seed_ci (`cargo build --features dev --bins`).
//   2. Crea un data_dir temporal (`scripts/.tmp-smoke-data`, ignorado por
//      git) y lo siembra con `seed_ci <dir>`: config.ini + BD con
//      admin/autos/clientes, pero SIN rentas → /rentas arranca vacía y el
//      smoke entra al branch de "renta de prueba" (el único que conoce la
//      base monetaria y puede asertar los totales de la extensión).
//   3. Levanta vite (dev server) y lanza el EXE ya compilado con
//      DINAMO_DATA_DIR apuntando al dir temporal y CDP en 9222
//      (WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS).
//
//      ⚠️ EJECUCIÓN DESDE UN PROCESO ELEVADO (runners de CI): tres
//      restricciones descubiertas a fuerza de corridas:
//      a) WebView2 IGNORA --remote-debugging-port en un proceso ELEVADO
//         (probado con UAC: el mismo exe sin elevar abre 9222 en 1 s y
//         elevado nunca lo abre, con y sin la variable en el entorno;
//         CI #285). Y degradar la compilación/servido con runas
//         /trustlevel:0x20000 rompe TODO (CI #273-#277): el token
//         restringido pierde los ACE de Administrators →
//         .cargo-build-lock denegado y EPERM de vite sobre .svelte-kit.
//      b) Firebird embedded exige que TODOS los procesos que tocan la BD
//         compartan el MISMO token: sembrar elevado y abrir la app
//         degradada falla ("Wrong file for memory mapping", run #286), y
//         sembrar degradado vía runas falla IGUAL (runs #287-#288): el
//         token RESTRINGIDO de Basic User no puede mapear las secciones
//         de memoria compartida de Firebird (firebird.msg, tablas de
//         locks), pase lo que pase antes.
//      c) runas y `schtasks /Run` retornan de inmediato: no se puede
//         esperar al hijo por PID; el batch deja marcas de fase
//         (SEED-EXIT:<code> / APP-EXIT) al final de su log y el
//         orquestador las sondea.
//      SOLUCIÓN: compilar y servir ELEVADOS (vite borra .svelte-kit y lo
//      recrea con su propia propiedad), y correr TODO lo que toca Firebird
//      o WebView2 (seed_ci + app) en un token de usuario NORMAL vía tarea
//      normal: `explorer.exe <batch>` + un batch que fija el env del humo
//      (explorer NO hereda el entorno del orquestador) y corre seed → app
//      con logs en archivo para el diagnóstico. El token del shell es el
//      equivalente exacto a una sesión dev normal (donde todo esto
//      funciona de punta a punta), sin las restricciones de Basic User que
//      runas impone a Firebird ni la sesión aparte de schtasks.
//
//   4. Corre `smoke-test-app.mjs`: login → renta de prueba (5 días ×
//      $150.000, sin IVA) → pago → extensión decimal (+2 h × $25.000,5 =
//      $50.001 → total $800.001) → segunda extensión acumulativa (+1 día ×
//      $50.000 → total $850.001) → orden → contrato → gate anti-[devGuard].
//   5. Limpia procesos (vite/app/seed por árbol e imagen, CDP por puerto)
//      y reporta si quedaron residuos del data_dir (Windows puede sostener
//      el .fdb unos segundos tras matar el proceso).
//
// Uso: npm run smoke:dev
//      node scripts/smoke-dev.mjs [--mantener]  (conserva el data_dir para inspección)

import { spawn, spawnSync } from 'node:child_process';
import { existsSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

const MANTENER = process.argv.includes('--mantener');
const RAIZ = resolve(import.meta.dirname, '..');
const DATA_DIR = join(RAIZ, 'scripts', '.tmp-smoke-data');
const FDB = join(DATA_DIR, 'dinamo_rent_v3.fdb');
const PUERTO_CDP = process.env.CDP_PORT || '9222';
const EXE_APP = join(RAIZ, 'src-tauri', 'target', 'debug', 'dinamo-rent.exe');
const EXE_SEED = join(RAIZ, 'src-tauri', 'target', 'debug', 'seed_ci.exe');
const BAT_HUMO = join(RAIZ, 'scripts', '.tmp-smoke-humo.cmd');
const LOG_HUMO = join(RAIZ, 'scripts', '.tmp-smoke-humo.log');

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** true si el proceso actual corre con integridad alta (elevado). Los runners
 *  de CI de Windows ejecutan elevados y ahí WebView2 ignora --remote-debugging-port. */
function esElevado() {
	if (process.platform !== 'win32') return false;
	const r = spawnSync('whoami', ['/groups'], { encoding: 'utf8' });
	return /S-1-16-12288/.test(r.stdout || '');
}

function matarArbol(pid) {
	// taskkill /T mata el árbol (npm → vite, etc.), /F forzado.
	spawnSync('taskkill', ['/PID', String(pid), '/T', '/F'], { stdio: 'ignore' });
}

/** true si algún proceso escucha en el puerto dado (netstat). */
function puertoEscuchando(puerto) {
	const r = spawnSync('netstat', ['-ano'], { encoding: 'utf8' });
	return (r.stdout || '')
		.split('\n')
		.some((l) => l.includes(`:${puerto} `) && l.includes('LISTENING'));
}

/** Mata los procesos que escuchan en el puerto dado (limpieza defensiva:
 *  la app lanzada fuera del orquestador escapa a su árbol de PID). */
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

/** Espera a que el puerto dado esté en LISTENING (polling 1 s). */
async function esperarPuertoOcupado(puerto, ms) {
	const fin = Date.now() + ms;
	while (Date.now() < fin) {
		if (puertoEscuchando(puerto)) return true;
		await sleep(1000);
	}
	return false;
}

/** Espera a que el CDP suba. null = arriba; 'app-murio' = el batch escribió
 *  APP-EXIT (la app murió antes de abrir el puerto); 'timeout' = venció. */
async function esperarCdp(puerto, ms) {
	const fin = Date.now() + ms;
	while (Date.now() < fin) {
		if (puertoEscuchando(puerto)) return null;
		if (/APP-EXIT\s*$/.test(colaLog(LOG_HUMO, 200))) return 'app-murio';
		await sleep(1000);
	}
	return 'timeout';
}

/** Cola de un archivo de log para adjuntar al diagnóstico de fallo. */
function colaLog(ruta, n = 1500) {
	try {
		return readFileSync(ruta, 'utf8').slice(-n);
	} catch {
		return '(sin log en ' + ruta + ')';
	}
}

/** Escribe el batch del humo para la fase dada. Fija el env del humo (las
 *  tareas programadas NO heredan el entorno del orquestador), redirige todo
 *  al log y deja marcas de corte de fase (SEED-EXIT / APP-EXIT) que el
 *  orquestador sondea, porque runas, schtasks /Run y el lanzamiento vía
 *  explorer retornan de inmediato. whoami registra el token real de la
 *  fase (integridad + sesión) para el diagnóstico. */
function escribirBatchHumo(fase) {
	const lineas = [
		'@echo off',
		`set "DINAMO_DATA_DIR=${DATA_DIR}"`,
		`set "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=${PUERTO_CDP}"`,
		`cd /d "${RAIZ}"`,
		`echo TOKEN-DE-LA-FASE >> "${LOG_HUMO}" 2>&1`,
		`whoami /groups >> "${LOG_HUMO}" 2>&1`
	];
	if (fase === 'seed') {
		lineas.push(
			`"${EXE_SEED}" "${DATA_DIR}" >> "${LOG_HUMO}" 2>&1`,
			'set "EC=%ERRORLEVEL%"',
			`>> "${LOG_HUMO}" echo SEED-EXIT:%EC%`
		);
	} else {
		lineas.push(`"${EXE_APP}" >> "${LOG_HUMO}" 2>&1`, `>> "${LOG_HUMO}" echo APP-EXIT`);
	}
	writeFileSync(BAT_HUMO, lineas.join('\r\n'));
}

/** Lanza el batch del humo en token de usuario NORMAL delegando en
 *  explorer.exe: el shell de la sesión interactiva corre con el token medio
 *  del usuario y ShellExecute hace que el hijo herede ESE token — ni
 *  elevado ni restringido (corridas #273-#289: runas/Basic User no puede
 *  mapear la memoria compartida de Firebird, y bajo schtasks /RL LIMITED
 *  /IT el WebView2 de la app no abrió CDP pese a que el seed funcionó).
 *  explorer retorna de inmediato: el orquestador espera por las marcas del
 *  log, no por PID. */
function lanzarHumo() {
	const r = spawn('cmd.exe', ['/d', '/s', '/c', `start "" explorer.exe "${BAT_HUMO}"`], {
		cwd: RAIZ,
		stdio: 'ignore',
		windowsVerbatimArguments: true
	});
	if (!r.pid) throw new Error('no se pudo lanzar explorer.exe');
}

/** Espera la marca SEED-EXIT del batch degradado (polling 1 s). */
async function esperarMarcaSeed(ms) {
	const fin = Date.now() + ms;
	while (Date.now() < fin) {
		const m = /\r?\nSEED-EXIT:(\d+)\s*$/.exec(colaLog(LOG_HUMO, 200));
		if (m) return Number(m[1]);
		await sleep(1000);
	}
	spawnSync('taskkill', ['/IM', 'seed_ci.exe', '/F'], { stdio: 'ignore' });
	throw new Error('seed_ci (fase humo) no terminó en 5 min:\n' + colaLog(LOG_HUMO));
}

/** Levanta vite como hijo del orquestador: su salida sirve de diagnóstico y
 *  muere con taskkill /T. (Elevado NO tiene el EPERM de antes: ese lo
 *  causaba escribir .svelte-kit desde un proceso de baja integridad.) */
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
	// Temporales de corridas abortadas fuera del camino.
	rmSync(BAT_HUMO, { force: true });
	rmSync(LOG_HUMO, { force: true });

	const esWin = process.platform === 'win32';
	const elevado = esWin && esElevado();

	// ── Rama no-Windows: sin concepto de elevación, `tauri dev` completo. ──
	if (!esWin) {
		console.log('— sembrando BD aislada (seed_ci)…');
		const rs = spawnSync('cargo', ['run', '--features', 'dev', '--bin', 'seed_ci', '--', DATA_DIR], {
			cwd: join(RAIZ, 'src-tauri'),
			stdio: 'inherit'
		});
		if (rs.status !== 0) throw new Error('seed_ci falló');
		if (existsSync(FDB)) console.log('   BD aislada:', FDB);

		// Vite regenera .svelte-kit en el arranque (sync). Borrarlo obliga a
		// vite a recrearlo con su propia propiedad; en local es un directorio
		// generado, sin costo.
		rmSync(join(RAIZ, '.svelte-kit'), { recursive: true, force: true });

		console.log('— lanzando tauri dev (BD aislada + CDP ' + PUERTO_CDP + ')…');
		const dev = spawn('npm', ['run', 'tauri', 'dev'], {
			cwd: RAIZ,
			stdio: ['ignore', 'pipe', 'pipe'],
			env: {
				...process.env,
				DINAMO_DATA_DIR: DATA_DIR,
				WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${PUERTO_CDP}`
			}
		});
		let salida = '';
		dev.stdout.on('data', (d) => (salida += d));
		dev.stderr.on('data', (d) => (salida += d));

		try {
			if (!(await esperarPuertoOcupado(PUERTO_CDP, 420000))) {
				throw new Error(`CDP ${PUERTO_CDP} no subió en 7 min (timeout):\n` + salida.slice(-1500));
			}
			await sleep(2000); // margen para que el target page esté servido
			await correrSmoke();
		} finally {
			try {
				if (dev.pid) matarArbol(dev.pid);
			} catch {
				/* noop */
			}
		}
	} else {
		// ── Rama Windows (CI elevado y dev local). ──
		// Compilar primero: seed_ci tiene required-features=["dev"] (feature
		// vacío que solo lo excluye del bundle de release) — sin --features
		// dev el build lo salta y el exe no existe (corrida 287). --bins
		// acota el build a los binarios; la app se compila aquí también
		// porque el lanzamiento por tarea/batch no compila nada.
		console.log('— compilando app + seed_ci (cargo build --features dev --bins)…');
		const rc = spawnSync('cargo', ['build', '--features', 'dev', '--bins'], {
			cwd: join(RAIZ, 'src-tauri'),
			stdio: 'inherit'
		});
		if (rc.status !== 0) throw new Error(`cargo build falló (exit ${rc.status})`);
		if (!existsSync(EXE_APP)) throw new Error('no existe el binario compilado: ' + EXE_APP);
		if (!existsSync(EXE_SEED)) throw new Error('no existe el binario seed_ci: ' + EXE_SEED);

		// Vite regenera .svelte-kit en el arranque (sync). Borrarlo obliga a
		// vite a recrearlo con su propia propiedad (crítico si el checkout lo
		// creó elevado y el humo corre con un token distinto).
		rmSync(join(RAIZ, '.svelte-kit'), { recursive: true, force: true });

		// ── Semilla ──
		if (elevado) {
			// En token de usuario normal vía explorer: MISMO token que la app.
			// Firebird embedded mapea la BD y sus locks en memoria compartida;
			// si el seed corre con un token distinto al de la app, el otro
			// proceso no puede mapearlos → "Wrong file for memory mapping"
			// (runs #286-#288).
			console.log('— sembrando BD aislada (seed_ci en token de usuario normal vía explorer)…');
			rmSync(LOG_HUMO, { force: true });
			escribirBatchHumo('seed');
			lanzarHumo();
			const code = await esperarMarcaSeed(300000);
			if (code !== 0) throw new Error(`seed_ci falló (exit ${code}):\n` + colaLog(LOG_HUMO));
		} else {
			console.log('— sembrando BD aislada (seed_ci)…');
			const rs = spawnSync(EXE_SEED, [DATA_DIR], { stdio: 'inherit' });
			if (rs.status !== 0) throw new Error('seed_ci falló');
		}
		if (existsSync(FDB)) console.log('   BD aislada:', FDB);

		// ── Vite (elevado, hijo del orquestador) ──
		const vite = await levantarVite();

		const finApp = () => {
			// La app puede sobrevivir a su árbol (tarea/runas sueltan el PID):
			// matarla por nombre (target/debug solo existe en desarrollo).
			spawnSync('taskkill', ['/IM', 'dinamo-rent.exe', '/F'], { stdio: 'ignore' });
		};

		// ── App ──
		if (elevado) {
			console.log('— lanzando la app (token de usuario normal vía explorer, CDP ' + PUERTO_CDP + ')…');
			rmSync(LOG_HUMO, { force: true }); // log limpio para la fase app
			escribirBatchHumo('app');
			lanzarHumo();
		} else {
			console.log('— lanzando la app (directa, CDP ' + PUERTO_CDP + ')…');
			escribirBatchHumo('app');
			spawn('cmd.exe', ['/d', '/s', '/c', BAT_HUMO], { cwd: RAIZ, stdio: 'ignore' });
		}

		try {
			const razon = await esperarCdp(PUERTO_CDP, 420000);
			if (razon === 'timeout') {
				throw new Error(
					`CDP ${PUERTO_CDP} no subió en 7 min (timeout):\n` +
						`--- log del humo ---\n${colaLog(LOG_HUMO)}\n` +
						`--- cola del dev server ---\n${vite.cola()}`
				);
			}
			if (razon === 'app-murio') {
				throw new Error('la app murió antes de abrir CDP:\n' + colaLog(LOG_HUMO));
			}
			await sleep(2000); // margen para que el target page esté servido
			await correrSmoke();
		} finally {
			finApp();
			await matarPuerto(PUERTO_CDP);
			await matarPuerto('5173');
			await esperarPuertoLibre(PUERTO_CDP, 10000);
			await esperarPuertoLibre('5173', 10000);
			vite.fin();
		}
	}

	// Limpieza defensiva por imagen: seed_ci puede quedar colgado si la
	// espera por marca venció, y el .fdb quedaría retenido.
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
	rmSync(BAT_HUMO, { force: true });
	rmSync(LOG_HUMO, { force: true });
	console.log('LISTO — smoke:dev OK');
}

main()
	.then(() => process.exit(0))
	.catch((e) => {
		console.error('✗ FALLO smoke:dev:', e.message);
		process.exit(1);
	});
