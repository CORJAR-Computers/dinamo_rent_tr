<script lang="ts">
	// OrdenReserva.svelte — Documento imprimible en papel Carta.
	// Tipografía amplia y aire entre secciones para que llene la hoja y se lea
	// cómodo. No lleva firmas: las firma las lleva el contrato de renta que se
	// firma al momento de la entrega (nota al pie). Se imprime con
	// window.print(); las reglas @media print de app.css muestran solo el área
	// .print-area cuando el body tiene la clase .printing.
	import { onMount } from 'svelte';
	import type { Reserva } from '$lib/api';
	import { formatCOP, formatDate } from '$lib/utils/format';
	import { empresa } from '$lib/stores/empresa.svelte';
	import { sid } from '$lib/stores/session.svelte';

	// Datos + logo de la empresa (setup inicial); fallback estático si no hay config.
	onMount(() => {
		void empresa.cargarCompleta(sid());
	});

	let { reserva }: { reserva: Reserva } = $props();

	const hoy = new Date().toLocaleDateString('es-CO', {
		year: 'numeric',
		month: 'long',
		day: 'numeric'
	});

	function hora(h: string | null): string {
		if (!h) return '—';
		const [hh, mm] = h.split(':');
		const hora12 = ((Number(hh) + 11) % 12) + 1;
		return `${hora12}:${mm} ${Number(hh) >= 12 ? 'PM' : 'AM'}`;
	}

	const saldoPendiente = $derived(
		Math.max(0, (parseFloat(reserva.total) || 0) - (parseFloat(reserva.abono) || 0))
	);
</script>

<div class="print-area bg-white text-slate-900">
	<!-- Encabezado -->
	<div class="border-b-2 border-slate-800 pb-5 mb-6 flex items-start justify-between gap-5">
		<div class="flex items-center gap-3">
			<div class="w-20 h-20 rounded-lg bg-blue-900 text-white flex items-center justify-center shrink-0 p-2">
				<img src={empresa.logoSrc} alt={empresa.nombreMostrar} class="w-14 h-14 object-contain rounded-md" />
			</div>
			<div>
				<p class="text-xl font-black tracking-tight">{empresa.nombreMostrar.toUpperCase()}</p>
				<p class="text-xs text-slate-600">Renta de vehículos · Reservas</p>
			</div>
		</div>
		<div class="text-right">
			<p class="text-3xl font-black tracking-wide text-blue-900">ORDEN DE RESERVA</p>
			<p class="text-base font-semibold mt-1.5">No. <span class="tabular-nums">{String(reserva.id).padStart(4, '0')}</span></p>
			<p class="text-xs text-slate-600 mt-1">Emitida: {hoy}</p>
		</div>
	</div>

	<!-- Estado -->
	<div class="flex items-center gap-3 mb-6">
		<span class="text-xs font-bold uppercase tracking-wider text-slate-500">Estado:</span>
		<span
			class="inline-flex items-center rounded-full border px-4 py-1 text-sm font-bold uppercase tracking-wide
			{reserva.estado === 'Cancelada' ? 'border-red-700 text-red-700' :
			reserva.estado === 'Completada' ? 'border-green-700 text-green-700' :
			reserva.estado === 'Pendiente' ? 'border-orange-700 text-orange-700' :
			'border-blue-900 text-blue-900'}"
		>
			{reserva.estado}
		</span>
	</div>

	<!-- Cliente -->
	<div class="grid grid-cols-2 gap-5 mb-6">
		<div class="rounded-lg border border-slate-300 p-4">
			<p class="text-[11px] font-bold uppercase tracking-wider text-slate-500 mb-1.5">Cliente</p>
			<p class="text-lg font-bold">{reserva.nombreCliente}</p>
			{#if reserva.nacionalidad}
				<p class="text-sm text-slate-600">{reserva.nacionalidad}</p>
			{/if}
		</div>
		<div class="rounded-lg border border-slate-300 p-4">
			<p class="text-[11px] font-bold uppercase tracking-wider text-slate-500 mb-1.5">Vehículo</p>
			<p class="text-lg font-bold">{reserva.categoriaVehiculo || 'Por definir'}</p>
			<p class="text-sm text-slate-600">Placa: <span class="font-semibold">{reserva.placaAsignada || 'Por asignar'}</span></p>
		</div>
	</div>

	<!-- Itinerario -->
	<div class="grid grid-cols-2 gap-5 mb-6">
		<div class="rounded-lg border border-slate-300 p-4">
			<p class="text-[11px] font-bold uppercase tracking-wider text-slate-500 mb-2">Recogida</p>
			<div class="space-y-1.5 text-base">
				<p class="flex justify-between"><span class="text-slate-500">Fecha</span><span class="font-semibold tabular-nums">{formatDate(reserva.fechaRecogida)}</span></p>
				<p class="flex justify-between"><span class="text-slate-500">Hora</span><span class="font-semibold tabular-nums">{hora(reserva.horaRecogida)}</span></p>
				<p class="flex justify-between gap-2"><span class="text-slate-500 shrink-0">Lugar</span><span class="font-semibold text-right">{reserva.ubicacionRecogida || '—'}</span></p>
			</div>
		</div>
		<div class="rounded-lg border border-slate-300 p-4">
			<p class="text-[11px] font-bold uppercase tracking-wider text-slate-500 mb-2">Retorno</p>
			<div class="space-y-1.5 text-base">
				<p class="flex justify-between"><span class="text-slate-500">Fecha</span><span class="font-semibold tabular-nums">{formatDate(reserva.fechaRetorno)}</span></p>
				<p class="flex justify-between"><span class="text-slate-500">Hora</span><span class="font-semibold tabular-nums">{hora(reserva.horaRetorno)}</span></p>
				<p class="flex justify-between gap-2"><span class="text-slate-500 shrink-0">Lugar</span><span class="font-semibold text-right">{reserva.ubicacionRetorno || '—'}</span></p>
			</div>
		</div>
	</div>

	<!-- Tarifas -->
	<div class="rounded-lg border border-slate-300 overflow-hidden mb-6">
		<table class="w-full text-base">
			<thead>
				<tr class="bg-slate-100 text-left">
					<th class="px-4 py-2.5 text-[11px] font-bold uppercase tracking-wider text-slate-500">Concepto</th>
					<th class="px-4 py-2.5 text-right text-[11px] font-bold uppercase tracking-wider text-slate-500">Valor</th>
				</tr>
			</thead>
			<tbody class="divide-y divide-slate-200">
				<tr>
					<td class="px-4 py-2.5">Valor del día × {reserva.diasCalculados} día{reserva.diasCalculados === 1 ? '' : 's'}</td>
					<td class="px-4 py-2.5 text-right font-semibold tabular-nums">{formatCOP((parseFloat(reserva.valorDia) || 0) * reserva.diasCalculados)}</td>
				</tr>
				<tr>
					<td class="px-4 py-2.5">Horas extras ({reserva.horasExtras} × {formatCOP(reserva.valorHoraAdic)})</td>
					<td class="px-4 py-2.5 text-right font-semibold tabular-nums">{formatCOP((parseFloat(reserva.valorHoraAdic) || 0) * reserva.horasExtras)}</td>
				</tr>
				<tr class="bg-slate-50">
					<td class="px-4 py-2.5 font-bold">Total reserva</td>
					<td class="px-4 py-2.5 text-right font-black tabular-nums">{formatCOP(reserva.total)}</td>
				</tr>
				<tr>
					<td class="px-4 py-2.5">Abono recibido</td>
					<td class="px-4 py-2.5 text-right font-semibold text-green-700 tabular-nums">- {formatCOP(reserva.abono)}</td>
				</tr>
				<tr>
					<td class="px-4 py-2.5 font-bold">Saldo pendiente</td>
					<td class="px-4 py-2.5 text-right font-black tabular-nums">{formatCOP(saldoPendiente)}</td>
				</tr>
			</tbody>
		</table>
	</div>

	<!-- Observaciones -->
	{#if reserva.observaciones}
		<div class="rounded-lg border border-slate-300 p-4 mb-6">
			<p class="text-[11px] font-bold uppercase tracking-wider text-slate-500 mb-1.5">Observaciones</p>
			<p class="text-base whitespace-pre-wrap">{reserva.observaciones}</p>
		</div>
	{/if}

	<!-- Condiciones -->
	<div class="text-xs text-slate-600 space-y-1 mb-6">
		<p>• El vehículo debe devolverse en las mismas condiciones en que fue entregado y con el tanque en el nivel pactado.</p>
		<p>• La reserva es confirmada una vez se reciba el abono indicado. Los datos de la orden son vinculantes para el cliente.</p>
		<p>• Esta orden no constituye contrato de renta; el contrato se firma al momento de la entrega del vehículo.</p>
	</div>

	<!-- El contrato de renta asociado lleva las firmas de las partes -->
	<p class="text-center text-[10px] italic text-slate-400 mt-6">
		Documento informativo: las firmas de las partes constan en el contrato de renta correspondiente.
	</p>

	<!-- Pie -->
	<p class="border-t border-slate-200 pt-3 text-center text-[10px] text-slate-400 mt-8">
		{empresa.nombreMostrar} · Reserva No. {String(reserva.id).padStart(4, '0')} · Impresa el {hoy}
	</p>
</div>
