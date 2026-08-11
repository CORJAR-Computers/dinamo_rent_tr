// src/lib/utils/timelineVehiculo.ts — Línea de tiempo de un vehículo.
//
// Cruza las rentas del vehículo con sus comparendos para ver quién tenía el
// vehículo en cada fecha: cada renta lleva las multas que cayeron dentro de su
// rango (el cruce del backend ya trae `responsable.idRenta` en cada comparendo).
// Funciones puras, sin Tauri — testeable en vitest.

import type { Comparendo, Renta } from '$lib/api';

/** Renta en la línea de tiempo: rango efectivo [inicio, fin] + multas asociadas */
export interface RentaTimeline {
	renta: Renta;
	/** Fecha de recogida (AAAA-MM-DD) */
	inicio: string;
	/** Fecha efectiva de fin: devolución real si existe, si no el retorno */
	fin: string;
	/** Comparendos que cayeron dentro de esta renta (por responsable.idRenta) */
	multas: Comparendo[];
}

/** Comparendo en la línea de tiempo */
export interface MultaTimeline {
	comparendo: Comparendo;
}

export interface EventoTimelineRenta {
	tipo: 'renta';
	/** Inicio de la renta (para ordenar en la línea de tiempo) */
	fecha: string;
	renta: RentaTimeline;
}

export interface EventoTimelineMulta {
	tipo: 'multa';
	/** Fecha de la infracción */
	fecha: string;
	multa: MultaTimeline;
}

export type EventoTimeline = EventoTimelineRenta | EventoTimelineMulta;

export interface TimelineVehiculo {
	rentas: RentaTimeline[];
	multas: MultaTimeline[];
	/** Rentas y multas mezcladas y ordenadas por fecha */
	eventos: EventoTimeline[];
	totalMultas: number;
	/** Suma de montos de multas Pendiente (en pesos) */
	totalPendiente: number;
}

/** Construye la línea de tiempo de un vehículo a partir de sus rentas y comparendos. */
export function construirTimelineVehiculo(rentas: Renta[], comparendos: Comparendo[]): TimelineVehiculo {
	// Agrupa las multas por la renta que las cubre (cruce del backend)
	const porRenta = new Map<number, Comparendo[]>();
	for (const c of comparendos) {
		const id = c.responsable?.idRenta;
		if (id == null) continue;
		const arr = porRenta.get(id) ?? [];
		arr.push(c);
		porRenta.set(id, arr);
	}

	const rentasTl: RentaTimeline[] = rentas
		.map((r) => ({
			renta: r,
			inicio: r.fechaRecogida,
			fin: r.fechaDevolucionReal ?? r.fechaRetorno,
			multas: porRenta.get(r.id) ?? []
		}))
		.sort((a, b) => a.inicio.localeCompare(b.inicio) || a.fin.localeCompare(b.fin));

	const multasTl: MultaTimeline[] = comparendos
		.map((c) => ({ comparendo: c }))
		.sort((a, b) => a.comparendo.fechaInfraccion.localeCompare(b.comparendo.fechaInfraccion));

	const eventos: EventoTimeline[] = [
		...rentasTl.map((r) => ({ tipo: 'renta' as const, fecha: r.inicio, renta: r })),
		...multasTl.map((m) => ({ tipo: 'multa' as const, fecha: m.comparendo.fechaInfraccion, multa: m }))
	].sort((a, b) => a.fecha.localeCompare(b.fecha) || (a.tipo === 'renta' ? -1 : 1));

	const totalPendiente = comparendos
		.filter((c) => c.estado === 'Pendiente')
		.reduce((suma, c) => suma + (parseFloat(c.monto) || 0), 0);

	return {
		rentas: rentasTl,
		multas: multasTl,
		eventos,
		totalMultas: multasTl.length,
		totalPendiente
	};
}
