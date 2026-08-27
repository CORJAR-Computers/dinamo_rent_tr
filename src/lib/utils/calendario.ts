// calendario.ts — Utilidades puras del calendario de rentas/reservas.
// Sin dependencias del DOM: testeables de forma aislada.

export interface RangoFechas {
	id: number;
	/** Placa del vehículo (si está asignado) */
	placa: string | null;
	/** Fecha de inicio en formato ISO (YYYY-MM-DD) */
	inicio: string;
	/** Fecha de fin en formato ISO (YYYY-MM-DD) */
	fin: string;
}

export interface Solapamiento {
	a: number;
	b: number;
	placa: string;
}

/** ¿Un rango [inicio, fin] cubre el día `dia` (ISO)? */
export function rangoCubreDia(inicio: string, fin: string, dia: string): boolean {
	if (!inicio || !fin) return false;
	const i = new Date(inicio + 'T00:00:00').getTime();
	const f = new Date(fin + 'T00:00:00').getTime();
	const d = new Date(dia + 'T00:00:00').getTime();
	if ([i, f, d].some(Number.isNaN)) return false;
	return i <= d && d <= f;
}

/**
 * Detecta solapamientos de vehículos: dos items con la misma placa cuyos
 * rangos [inicio, fin] se cruzan (inclusivo). Devuelve pares únicos (a < b).
 */
export function detectarSolapamientos(items: RangoFechas[]): Solapamiento[] {
	const resultados: Solapamiento[] = [];
	for (let i = 0; i < items.length; i++) {
		for (let j = i + 1; j < items.length; j++) {
			const a = items[i];
			const b = items[j];
			if (!a.placa || a.placa !== b.placa) continue;
			if (a.inicio <= b.fin && b.inicio <= a.fin) {
				resultados.push({ a: a.id, b: b.id, placa: a.placa });
			}
		}
	}
	return resultados;
}

/** Nombre corto del día (lun..dom) para la cabecera */
export function diasSemanaCorto(): string[] {
	return ['lun', 'mar', 'mié', 'jue', 'vie', 'sáb', 'dom'];
}

/**
 * Celdas del mes visible (semana empieza en lunes). Devuelve una matriz de
 * semanas; cada celda tiene `dia` (ISO), `enMes` (bool) y `hoy` (bool).
 */
export function celdasDelMes(
	anio: number,
	mes: number
): { dia: string; enMes: boolean; hoy: boolean }[][] {
	const hoy = new Date();
	const hoyISO = `${hoy.getFullYear()}-${String(hoy.getMonth() + 1).padStart(2, '0')}-${String(hoy.getDate()).padStart(2, '0')}`;

	const primerDia = new Date(anio, mes, 1);
	// Desplazamiento: 0 = domingo... convertir a semana iniciando lunes
	const offset = (primerDia.getDay() + 6) % 7;
	const inicioCelda = new Date(anio, mes, 1 - offset);

	const semanas: { dia: string; enMes: boolean; hoy: boolean }[][] = [];
	const celdas: { dia: string; enMes: boolean; hoy: boolean }[] = [];
	for (let i = 0; i < 42; i++) {
		const d = new Date(
			inicioCelda.getFullYear(),
			inicioCelda.getMonth(),
			inicioCelda.getDate() + i
		);
		const iso = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
		celdas.push({ dia: iso, enMes: d.getMonth() === mes, hoy: iso === hoyISO });
	}
	for (let i = 0; i < 42; i += 7) {
		semanas.push(celdas.slice(i, i + 7));
	}
	return semanas;
}

/** Devuelve el nombre del mes en español */
export function nombreMes(anio: number, mes: number): string {
	return new Date(anio, mes, 1).toLocaleDateString('es-CO', { month: 'long', year: 'numeric' });
}
