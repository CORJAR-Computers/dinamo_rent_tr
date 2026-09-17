// devGuard.sync.test.ts — Vigilancia de obsolescencia del mapa generado.
//
// El mapa de contratos del guardrail NO es manual: lo genera
// `cargo run --features dev --bin gen_devguard` en `devGuard.generated.ts`,
// clasificando cada campo de los DTOs de entrada con sondas serde reales
// (¿acepta 999? ¿acepta "1?"). Este test NO compara listas campo a campo
// (eso ya lo hace el generador contra el código Rust): verifica que el
// archivo generado esté AL DÍA y que el guardrail siga funcionando:
//
// 1. Integridad estructural: el generado trae los comandos y campos
//    representativos que definimos como contrato mínimo.
// 2. Frescura: si alguna fuente Rust (DTOs) es más nueva que el generado,
//    alguien editó el backend sin regenerar → corre `gen_devguard`.
// 3. Coherencia de aliases y smoke del guardrail (warning del incidente).
import { describe, it, expect, vi, beforeEach, afterEach, type MockInstance } from 'vitest';
import { readFileSync, statSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';
import { devTypeCheckArgs, ALIAS_COMANDO, contratoDe } from './devGuard';

const AQUI = dirname(fileURLToPath(import.meta.url));
const RAIZ = join(AQUI, '..', '..', '..');
const GENERADO = join(AQUI, 'devGuard.generated.ts');

/** Fuentes Rust que alimentan el generador (los archivos con DTOs de entrada). */
const FUENTES = [
	'src-tauri/src/repositories/renta.rs',
	'src-tauri/src/repositories/reserva.rs',
	'src-tauri/src/repositories/auto.rs',
	'src-tauri/src/repositories/cliente.rs',
	'src-tauri/src/repositories/gasto.rs',
	'src-tauri/src/repositories/comparendo.rs',
	'src-tauri/src/repositories/mantenimiento.rs',
	'src-tauri/src/repositories/empresa.rs',
	'src-tauri/src/services/usuario.rs'
].map((p) => join(RAIZ, p));

const COMANDO_REGEN = 'bun run gen:devguard';

describe('devGuard.generated.ts — mapa generado al día', () => {
	it('existe el archivo generado con el mapa exportado', () => {
		const contenido = readFileSync(GENERADO, 'utf8');
		expect(contenido).toContain('MAPA_GENERADO');
		expect(contenido).toContain('NO EDITAR A MANO');
	});

	it('cubre los comandos con DTO y sus campos de contrato mínimo', () => {
		const contenido = readFileSync(GENERADO, 'utf8');
		// Un comando por cada módulo con DTO de entrada (los actualizar_* son alias):
		for (const cmd of [
			'crear_renta',
			'cerrar_renta',
			'editar_renta_cerrada',
			'extender_renta',
			'registrar_pago_renta',
			'registrar_inspeccion_renta',
			'crear_reserva',
			'crear_gasto',
			'crear_comparendo',
			'crear_mantenimiento',
			'crear_auto',
			'crear_cliente',
			'guardar_empresa',
			'crear_usuario'
		]) {
			expect(contenido, `falta el comando ${cmd}`).toContain(`\t${cmd}: {`);
		}
		// Campos representativos (si un renombre los rompe, actualizar aquí):
		expect(contenido).toContain("'valorDia'");
		expect(contenido).toContain("'kmSalida'");
		expect(contenido).toContain("'diasCalculados'");
		expect(contenido).toContain("'cantidad'");
		expect(contenido).toContain("'kilometraje'");
	});

	it('el contrato de extender_renta reproduce el incidente (valor/valorDia)', () => {
		const ext = contratoDe('extender_renta');
		expect(ext.strings).toContain('valor');
		expect(ext.numeros).toContain('cantidad');
	});

	it('está al día: ninguna fuente Rust es más nueva que el generado', () => {
		const mtimeGenerado = statSync(GENERADO).mtimeMs;
		const nuevas = FUENTES.filter((f) => statSync(f).mtimeMs > mtimeGenerado);
		if (nuevas.length > 0) {
			const lista = nuevas
				.map((f) => f.replace(RAIZ + '\\', '').replace(RAIZ + '/', ''))
				.join(', ');
			expect.fail(
				`devGuard.generated.ts está obsoleto (fuentes más nuevas: ${lista}). ` +
					`Regenera con: ${COMANDO_REGEN}`
			);
		}
	});

	it('los aliases actualizar_* resuelven al mismo contrato que su crear_*', () => {
		for (const [actualizar, crear] of Object.entries(ALIAS_COMANDO)) {
			expect(contratoDe(actualizar), `${actualizar} ≠ ${crear}`).toBe(contratoDe(crear));
		}
	});

	describe('smoke del guardrail (sigue vivo con el mapa generado)', () => {
		let warn: MockInstance<typeof console.warn>;
		beforeEach(() => {
			vi.stubEnv('DEV', true);
			warn = vi.spyOn(console, 'warn').mockImplementation(() => {});
		});
		afterEach(() => {
			warn.mockRestore();
			vi.unstubAllEnvs();
		});

		it('avisa con el payload del incidente (valor numérico) y calla con el correcto', () => {
			devTypeCheckArgs('extender_renta', {
				datos: { tipo: 'horas', cantidad: 1, valor: 150000 }
			});
			expect(warn).toHaveBeenCalledTimes(1);
			expect(String(warn.mock.calls[0]?.[0])).toContain('[devGuard]');
			expect(String(warn.mock.calls[0]?.[0])).toContain('"valor"');

			warn.mockClear();
			devTypeCheckArgs('extender_renta', {
				datos: { tipo: 'horas', cantidad: 1, valor: '150000' }
			});
			expect(warn).not.toHaveBeenCalled();
		});

		it('avisa en dirección inversa (i64 viajando como string)', () => {
			devTypeCheckArgs('extender_renta', {
				datos: { tipo: 'horas', cantidad: '2', valor: '25000' }
			});
			expect(warn).toHaveBeenCalledTimes(1);
			expect(String(warn.mock.calls[0]?.[0])).toContain('"cantidad"');
			expect(String(warn.mock.calls[0]?.[0])).toContain('declara i64');
		});
	});
});
