<script lang="ts">
	// OrdenReserva.svelte — Documento imprimible A4 (puerto de orden_reserva_jinja.html)
	// Se imprime con window.print(); las reglas @media print de app.css muestran
	// solo el área .print-area cuando el body tiene la clase .printing.
	import type { Reserva } from '$lib/api';
	import { formatCOP, formatDate } from '$lib/utils/format';

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
	<div class="border-b-2 border-slate-800 pb-4 mb-5 flex items-start justify-between gap-4">
		<div class="flex items-center gap-3">
			<div class="w-16 h-16 rounded-lg bg-blue-900 text-white flex items-center justify-center shrink-0 p-1.5">
				<img src="/LogoDinamo.png" alt="Logo" class="w-12 h-12 object-contain rounded-md" />
			</div>
			<div>
				<p class="text-lg font-black tracking-tight">DINAMO RENT A CAR</p>
				<p class="text-[11px] text-slate-600">Renta de vehículos · Reservas</p>
			</div>
		</div>
		<div class="text-right">
			<p class="text-2xl font-black tracking-wide text-blue-900">ORDEN DE RESERVA</p>
			<p class="text-sm font-semibold mt-1">No. <span class="tabular-nums">{String(reserva.id).padStart(4, '0')}</span></p>
			<p class="text-[11px] text-slate-600 mt-0.5">Emitida: {hoy}</p>
		</div>
	</div>

	<!-- Estado -->
	<div class="flex items-center gap-2 mb-5">
		<span class="text-[11px] font-bold uppercase tracking-wider text-slate-500">Estado:</span>
		<span
			class="inline-flex items-center rounded-full border px-3 py-0.5 text-xs font-bold uppercase tracking-wide
			{reserva.estado === 'Cancelada' ? 'border-red-700 text-red-700' :
			reserva.estado === 'Completada' ? 'border-green-700 text-green-700' :
			reserva.estado === 'Pendiente' ? 'border-orange-700 text-orange-700' :
			'border-blue-900 text-blue-900'}"
		>
			{reserva.estado}
		</span>
	</div>

	<!-- Cliente -->
	<div class="grid grid-cols-2 gap-4 mb-5">
		<div class="rounded-lg border border-slate-300 p-3">
			<p class="text-[10px] font-bold uppercase tracking-wider text-slate-500 mb-1">Cliente</p>
			<p class="font-bold">{reserva.nombreCliente}</p>
			{#if reserva.nacionalidad}
				<p class="text-xs text-slate-600">{reserva.nacionalidad}</p>
			{/if}
		</div>
		<div class="rounded-lg border border-slate-300 p-3">
			<p class="text-[10px] font-bold uppercase tracking-wider text-slate-500 mb-1">Vehículo</p>
			<p class="font-bold">{reserva.categoriaVehiculo || 'Por definir'}</p>
			<p class="text-xs text-slate-600">Placa: <span class="font-semibold">{reserva.placaAsignada || 'Por asignar'}</span></p>
		</div>
	</div>

	<!-- Itinerario -->
	<div class="grid grid-cols-2 gap-4 mb-5">
		<div class="rounded-lg border border-slate-300 p-3">
			<p class="text-[10px] font-bold uppercase tracking-wider text-slate-500 mb-2">Recogida</p>
			<div class="space-y-1 text-sm">
				<p class="flex justify-between"><span class="text-slate-500">Fecha</span><span class="font-semibold tabular-nums">{formatDate(reserva.fechaRecogida)}</span></p>
				<p class="flex justify-between"><span class="text-slate-500">Hora</span><span class="font-semibold tabular-nums">{hora(reserva.horaRecogida)}</span></p>
				<p class="flex justify-between gap-2"><span class="text-slate-500 shrink-0">Lugar</span><span class="font-semibold text-right">{reserva.ubicacionRecogida || '—'}</span></p>
			</div>
		</div>
		<div class="rounded-lg border border-slate-300 p-3">
			<p class="text-[10px] font-bold uppercase tracking-wider text-slate-500 mb-2">Retorno</p>
			<div class="space-y-1 text-sm">
				<p class="flex justify-between"><span class="text-slate-500">Fecha</span><span class="font-semibold tabular-nums">{formatDate(reserva.fechaRetorno)}</span></p>
				<p class="flex justify-between"><span class="text-slate-500">Hora</span><span class="font-semibold tabular-nums">{hora(reserva.horaRetorno)}</span></p>
				<p class="flex justify-between gap-2"><span class="text-slate-500 shrink-0">Lugar</span><span class="font-semibold text-right">{reserva.ubicacionRetorno || '—'}</span></p>
			</div>
		</div>
	</div>

	<!-- Tarifas -->
	<div class="rounded-lg border border-slate-300 overflow-hidden mb-5">
		<table class="w-full text-sm">
			<thead>
				<tr class="bg-slate-100 text-left">
					<th class="px-3 py-2 text-[10px] font-bold uppercase tracking-wider text-slate-500">Concepto</th>
					<th class="px-3 py-2 text-right text-[10px] font-bold uppercase tracking-wider text-slate-500">Valor</th>
				</tr>
			</thead>
			<tbody class="divide-y divide-slate-200">
				<tr>
					<td class="px-3 py-2">Valor del día × {reserva.diasCalculados} día{reserva.diasCalculados === 1 ? '' : 's'}</td>
					<td class="px-3 py-2 text-right font-semibold tabular-nums">{formatCOP((parseFloat(reserva.valorDia) || 0) * reserva.diasCalculados)}</td>
				</tr>
				<tr>
					<td class="px-3 py-2">Horas extras ({reserva.horasExtras} × {formatCOP(reserva.valorHoraAdic)})</td>
					<td class="px-3 py-2 text-right font-semibold tabular-nums">{formatCOP((parseFloat(reserva.valorHoraAdic) || 0) * reserva.horasExtras)}</td>
				</tr>
				<tr class="bg-slate-50">
					<td class="px-3 py-2 font-bold">Total reserva</td>
					<td class="px-3 py-2 text-right font-black tabular-nums">{formatCOP(reserva.total)}</td>
				</tr>
				<tr>
					<td class="px-3 py-2">Abono recibido</td>
					<td class="px-3 py-2 text-right font-semibold text-green-700 tabular-nums">- {formatCOP(reserva.abono)}</td>
				</tr>
				<tr>
					<td class="px-3 py-2 font-bold">Saldo pendiente</td>
					<td class="px-3 py-2 text-right font-black tabular-nums">{formatCOP(saldoPendiente)}</td>
				</tr>
			</tbody>
		</table>
	</div>

	<!-- Observaciones -->
	{#if reserva.observaciones}
		<div class="rounded-lg border border-slate-300 p-3 mb-5">
			<p class="text-[10px] font-bold uppercase tracking-wider text-slate-500 mb-1">Observaciones</p>
			<p class="text-sm whitespace-pre-wrap">{reserva.observaciones}</p>
		</div>
	{/if}

	<!-- Condiciones -->
	<div class="text-[10px] text-slate-600 space-y-0.5 mb-8">
		<p>• El vehículo debe devolverse en las mismas condiciones en que fue entregado y con el tanque en el nivel pactado.</p>
		<p>• La reserva es confirmada una vez se reciba el abono indicado. Los datos de la orden son vinculantes para el cliente.</p>
		<p>• Esta orden no constituye contrato de renta; el contrato se firma al momento de la entrega del vehículo.</p>
	</div>

	<!-- Firmas -->
	<div class="grid grid-cols-2 gap-8 mt-10">
		<div class="text-center">
			<div class="border-t border-slate-400 pt-2 text-xs font-semibold">Firma del cliente</div>
			<p class="text-[10px] text-slate-500 mt-0.5">{reserva.nombreCliente}</p>
		</div>
		<div class="text-center">
			<div class="border-t border-slate-400 pt-2 text-xs font-semibold">Firma del representante</div>
			<p class="text-[10px] text-slate-500 mt-0.5">Dinamo Rent a Car</p>
		</div>
	</div>

	<!-- Pie -->
	<p class="text-center text-[9px] text-slate-400 mt-8">
		Dinamo Rent a Car · Reserva No. {String(reserva.id).padStart(4, '0')} · Impresa el {hoy}
	</p>
</div>
