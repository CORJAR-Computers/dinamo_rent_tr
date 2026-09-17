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
//   3. Levanta vite (dev server) y lanza el EXE ya compilado con
//      DINAMO_DATA_DIR apuntando al dir temporal y CDP en 9222
//      (WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS).
//
//      ⚠️ TOKENS DE WINDOWS EN CI (descubierto a fuerza de corridas):
//      a) En un proceso ELEVADO, WebView2 IGNORA --remote-debugging-port
//         (probado con UAC: el mismo exe sin elevar abre 9222 en 1 s y
//         elevado nunca lo abre; CI #285). Los runners de GitHub van más
//         lejos: UAC DESHABILITADO, así que NO existe token medio que
//         heredar — ni schtasks /RL LIMITED ni explorer.exe producen uno
//         (runs #289-#290: whoami mostró High Mandatory Level incluso bajo
//         la tarea / el shell).
//      b) La alternativa es `runas /trustlevel:0x20000`, que crea un token
//         de INTEGRIDAD MEDIA con SIDs RESTRINGIDOS (Basic User). WebView2
//         le honra la bandera CDP (integridad media), pero su check de
//         acceso a secciones de memoria mapeadas exige que la DACL del
//         archivo conceda algo a los SIDs restringidos: los archivos del
//         checkout (creados elevado) solo conceden a
//         Administrators/SYSTEM/Usuarios autentificados → Firebird muere
//         con "Wrong file for memory mapping" al mapear firebird.msg o la
//         BD (runs #286 y #288).
//      c) SOLUCIÓN: endurecer las ACLs de lo que Firebird mapea
//         (`icacls /grant *S-1-1-0:(OI)(CI)F` — Todos/Everyone — sobre el
//         data_dir y resources/firebird, con herencia para los archivos
//         que seed/app creen después). Con Everyone en la DACL, el
//         intersect con CUALQUIER conjunto de SIDs restringidos es no
//         vacío y el mapeo procede. En Windows el humo corre SIEMPRE vía
//         runas (local y CI comparten el mismo camino de código).
//      d) runas retorna de inmediato: no se puede esperar al hijo por PID;
//         el batch deja marcas de fase (SEED-EXIT:<code> / APP-EXIT) al
//         final de su log y el orquestador las sondea. Cada fase registra
//         además `whoami /groups` para diagnosticar el token real.
//
//   4. Corre `smoke-test-app.mjs`: login → renta de prueba (5 días ×
//      $150.000, sin IVA) → pago → extensión decimal (+2 h × $25.000,5 =
//      $50.001 → total $800.001) → segunda extensión acumulativa (+1 día ×
//      $50.000 → total $850.001) → orden → contrato → gate anti-[devGuard].
//   5. Limpia procesos (vite por árbol, app/seed por imagen, CDP por
//      puerto) y reporta si quedaron residuos del data_dir (Windows puede
//      sostener el .fdb unos segundos tras matar el proceso).
//
// Uso: npm run smoke:dev
//      node scripts/smoke-dev.mjs [--mantener]  (conserva el data_dir para inspección)

import { spawn, spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
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
const FIREBIRD_RES = join(RAIZ, 'src-tauri', 'resources', 'firebird');
// SID de Todos/Everyone (S-1-1-0): independiente del idioma del SO.
const GRANT_TODOS = '*S-1-1-0:(OI)(CI)F';

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

/** true si el proceso actual corre con integridad alta (elevado). Solo
 *  informativo: en Windows el humo corre SIEMPRE vía runas. */
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
 *  la app lanzada vía runas escapa al PID del orquestador). */
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

/** Concede Full Control a Everyone (con herencia a lo nuevo) sobre las
 *  rutas que Firebird mapea en memoria: el data_dir y los recursos
 *  embebidos (firebird.msg, ICU, tzdata, y el firebird.log que pueda
 *  crearse allí). Sin esto, el token restringido del runas no puede mapear
 *  archivos creados por el checkout elevado → "Wrong file for memory
 *  mapping" (runs #286/#288). Best-effort: si icacls falla en algún
 *  archivo puntual se continúa (el fallo real aparecería al mapear). */
function endurecerAcls() {
	console.log('— concediendo Full Control a Everyone en data_dir y recursos Firebird…');
	for (const ruta of [DATA_DIR, FIREBIRD_RES]) {
		const r = spawnSync(
			'icacls',
			[ruta, '/grant', GRANT_TODOS, '/t', '/c', '/q'],
			{ encoding: 'utf8' }
		);
		if (r.status !== 0) {
			console.warn('⚠ icacls terminó con errores en ' + ruta + ':\n' + (r.stderr || r.stdout || ''));
		}
	}
}

/** Escribe el batch del humo para la fase dada. Fija el env del humo (el
 *  runas NO hereda el entorno del orquestador), registra el token real de
 *  la fase (whoami /groups), redirige todo al log y deja marcas de corte
 *  de fase (SEED-EXIT / APP-EXIT) que el orquestador sondea. */
function escribirBatchHumo(fase) {
	const lineas = [
		'@echo off',
		`set "DINAMO_DATA_DIR=${DATA_DIR}"`,
		`set "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=${PUERTO_CDP}"`,
		`cd /d "${RAIZ}"`,
		`echo === TOKEN-DE-LA-FASE === >> "${LOG_HUMO}" 2>&1`,
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

/** Lanza el batch vía runas degradado (token de integridad media con SIDs
 *  restringidos: WebView2 honra la bandera CDP y Firebird puede mapear
 *  gracias al grant de Everyone). runas retorna de inmediato: el
 *  orquestador espera por las marcas del log, no por PID. */
function lanzarDegradado() {
	const r = spawn('runas', ['/trustlevel:0x20000', `cmd.exe /c "${BAT_HUMO}"`], {
		cwd: RAIZ,
		stdio: 'ignore'
	});
	if (!r.pid) throw new Error('no se pudo lanzar runas');
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
 *  muere con taskkill /T. El dev server corre con el token del orquestador
 *  (elevado en CI): no toca Firebird ni WebView2, y degradarlo rompía la
 *  compilación por los ACE de Administrators (CI #273-#277). */
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
		// ── Rama Windows (CI elevado y dev local): un solo camino. ──
		const elevado = esElevado();

		// Compilar con el token del orquestador: seed_ci tiene
		// required-features=["dev"] (feature vacío que solo lo excluye del
		// bundle de release) — sin --features dev el build lo salta y el exe
		// no existe (corrida 287). --bins acota el build a los binarios.
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
		// creó con otro token).
		rmSync(join(RAIZ, '.svelte-kit'), { recursive: true, force: true });

		// El data_dir debe existir ANTES del icacls para que la ACE con
		// herencia cubra todo lo que seed/app creen dentro.
		mkdirSync(DATA_DIR, { recursive: true });
		endurecerAcls();

		// ── Semilla (degradada: MISMO token que la app — Firebird embedded
		// mapea la BD y sus locks en memoria compartida y exige que ambos
		// procesos los puedan abrir) ──
		console.log('— sembrando BD aislada (seed_ci vía runas)…');
		rmSync(LOG_HUMO, { force: true });
		escribirBatchHumo('seed');
		lanzarDegradado();
		const code = await esperarMarcaSeed(300000);
		if (code !== 0) throw new Error(`seed_ci falló (exit ${code}):\n` + colaLog(LOG_HUMO));
		if (!existsSync(FDB)) throw new Error('seed_ci no produjo la BD: ' + FDB);
		console.log('   BD aislada:', FDB);

		// ── Vite (token del orquestador) ──
		const vite = await levantarVite();

		const finApp = () => {
			// La app puede sobrevivir a su árbol (runas suelta el PID):
			// matarla por nombre (target/debug solo existe en desarrollo).
			spawnSync('taskkill', ['/IM', 'dinamo-rent.exe', '/F'], { stdio: 'ignore' });
		};

		// ── App (degradada: WebView2 solo honra la bandera CDP con
		// integridad media; el token del runas la tiene) ──
		console.log(
			'— lanzando la app (' +
				(elevado ? 'orquestador elevado' : 'orquestador sin elevar') +
				', app vía runas, CDP ' +
				PUERTO_CDP +
				')…'
		);
		rmSync(LOG_HUMO, { force: true }); // log limpio para la fase app
		escribirBatchHumo('app');
		lanzarDegradado();

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
