// devGuard.ts — Guardrail de desarrollo para el contrato de tipos IPC.
//
// Contrato de los DTOs de entrada (`src-tauri/src/repositories/*.rs`):
// - Campos monetarios y de kilometraje: `String` en Rust / `string` en TS. El
//   backend los acepta también como números JSON (red de seguridad
//   `core::decimal_string`), pero si un binding de la UI los envía como
//   `number` es una regresión de tipado — la misma clase de bug que produjo el
//   incidente de `extender_renta` (`bind:value` sobre `type="number"`
//   coerciona el valor).
// - Campos numéricos enteros: `i64` en Rust / `number` en TS
//   (diasCalculados, horasExtras, cantidad, kilometraje del auto, kms
//   próximos de mantenimiento, ids). Si viajan como `string`, serde los
//   rechaza con `invalid type: string, expected i64` y la operación falla
//   sin red de seguridad.
//
// El mapa de contratos NO se mantiene a mano: lo genera
// `cargo run --features dev --bin gen_devguard` en
// `devGuard.generated.ts` clasificando cada campo con sondas serde reales
// (¿el DTO acepta 999? ¿acepta "1"?). El test `devGuard.sync.test.ts`
// verifica que el archivo generado esté al día con las fuentes Rust.
//
// Este módulo advierte en consola (solo `import.meta.env.DEV`) cuando un
// campo viaja con el tipo de la otra dirección, para detectar la regresión en
// desarrollo y no en producción (ni, peor, en el backend). Nunca lanza ni
// muta el payload: el comando siempre se ejecuta.

import { MAPA_GENERADO } from './devGuard.generated';

/** Comandos `actualizar_*` que comparten contrato con su `crear_*`. */
export const ALIAS_COMANDO: Record<string, string> = {
	actualizar_renta: 'crear_renta',
	actualizar_reserva: 'crear_reserva',
	actualizar_gasto: 'crear_gasto',
	actualizar_comparendo: 'crear_comparendo',
	actualizar_mantenimiento: 'crear_mantenimiento',
	actualizar_auto: 'crear_auto',
	actualizar_cliente: 'crear_cliente'
};

/** Contrato resuelto para un comando (alias incluidos). */
export function contratoDe(comando: string): {
	numeros: readonly string[];
	strings: readonly string[];
} {
	const c = MAPA_GENERADO[ALIAS_COMANDO[comando] ?? comando];
	return c ?? { numeros: [], strings: [] };
}

/**
 * Advierte en consola si algún campo del DTO `datos` viaja con el tipo de la
 * dirección contraria a su declaración en Rust:
 * - decimal_string declarado → viaja como `number` (tolerado por el backend).
 * - i64/f64 declarado → viaja como `string` (el backend lo rechaza).
 * Solo activo en desarrollo; en producción y en tests es inoperante (los
 * tests lo activan con `vi.stubEnv('DEV', ...)`).
 */
export function devTypeCheckArgs(command: string, args?: Record<string, unknown>): void {
	if (!import.meta.env.DEV) return;
	if (!args) return;

	const datos = args.datos;
	if (!datos || typeof datos !== 'object' || Array.isArray(datos)) return;
	const registro = datos as Record<string, unknown>;
	const contrato = contratoDe(command);

	for (const campo of contrato.strings) {
		const valor = registro[campo];
		if (typeof valor === 'number') {
			console.warn(
				`[devGuard] ${command}: el campo "${campo}" viaja como number ` +
					`(${String(valor)}) pero el DTO lo declara string. ` +
					`Conviértelo con String(valor) o usa inputmode="decimal" en el input ` +
					`(un bind sobre type="number" coerciona). El backend lo tolera por la ` +
					`red de seguridad decimal_string, pero es una regresión de tipado.`
			);
		}
	}

	for (const campo of contrato.numeros) {
		const valor = registro[campo];
		if (typeof valor === 'string') {
			console.warn(
				`[devGuard] ${command}: el campo "${campo}" viaja como string ` +
					`("${valor}") pero el DTO lo declara i64. Conviértelo con Number(valor) ` +
					`o usa type="number" en el input. El backend NO lo tolera: serde lo ` +
					`rechazará con "invalid type: string, expected i64" y la operación fallará.`
			);
		}
	}
}
