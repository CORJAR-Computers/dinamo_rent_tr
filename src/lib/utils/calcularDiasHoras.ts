// calcularDiasHoras.ts — Cálculo unificado de días y horas extras para rentas
// y reservas (espejo de la regla del backend en el cierre de renta).
//
// Regla de negocio:
//   - Cada 24 h desde la recogida = 1 día.
//   - El excedente de hasta 3 h se cobra como horas extras (redondeadas hacia
//     arriba).
//   - Si el excedente supera 3 h se cobra el día completo (horas extras = 0).
//
// Si falta la hora de recogida o de retorno, se cae al cálculo por fecha
// (diferencia de días calendario, redondeada) — comportamiento histórico de
// los formularios, donde la hora es opcional.

/** Umbral: un excedente mayor a 3 h cuenta como día completo. */
export const HORAS_TOLERANCIA_DIA_COMPLETO = 3;

export interface DiasHoras {
	dias: number;
	horas: number;
}

function parseFechaHora(fecha?: string, hora?: string): Date | null {
	if (!fecha) return null;
	const [hh = '00', mm = '00'] = (hora || '00:00').split(':');
	const d = new Date(`${fecha}T${hh}:${mm}:00`);
	return Number.isNaN(d.getTime()) ? null : d;
}

/**
 * Calcula días y horas extras entre recogida y retorno aplicando la regla de
 * negocio. Si faltan fechas devuelve (0, 0).
 */
export function calcularDiasHoras(
	fechaRecogida?: string,
	horaRecogida?: string,
	fechaRetorno?: string,
	horaRetorno?: string
): DiasHoras {
	if (!fechaRecogida || !fechaRetorno) {
		return { dias: 0, horas: 0 };
	}

	const a = parseFechaHora(fechaRecogida, horaRecogida);
	const b = parseFechaHora(fechaRetorno, horaRetorno);

	// Sin horas (o fechas inválidas): diferencia de días calendario.
	if (!a || !b) {
		const aD = new Date(`${fechaRecogida}T00:00:00`);
		const bD = new Date(`${fechaRetorno}T00:00:00`);
		const d = Math.round((bD.getTime() - aD.getTime()) / 86_400_000);
		return { dias: Math.max(0, d), horas: 0 };
	}

	const minutos = Math.max(0, (b.getTime() - a.getTime()) / 60_000);
	const diaMin = 24 * 60;
	const dias = Math.floor(minutos / diaMin);
	const rem = minutos % diaMin;

	if (rem > HORAS_TOLERANCIA_DIA_COMPLETO * 60) {
		return { dias: dias + 1, horas: 0 };
	}
	return { dias, horas: Math.ceil(rem / 60) };
}
