<script lang="ts">
	// OrdenComparendo.svelte — Documento imprimible A4 para Comparendos
	import type { Comparendo } from '$lib/api';
	import { formatCOP, formatContrato, formatDate } from '$lib/utils/format';

	let { comparendo }: { comparendo: Comparendo } = $props();

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
</script>

<div class="print-area bg-white text-slate-900">
	<!-- Encabezado -->
	<div class="border-b-2 border-slate-800 pb-4 mb-5 flex items-start justify-between gap-4">
		<div class="flex items-center gap-3">
			<div class="w-16 h-16 rounded-lg bg-red-900 text-white flex items-center justify-center shrink-0 p-1.5">
				<img src="/LogoDinamo.png" alt="Logo" class="w-12 h-12 object-contain rounded-md" />
			</div>
			<div>
				<p class="text-lg font-black tracking-tight">DINAMO RENT A CAR</p>
				<p class="text-[11px] text-slate-600">Gestión de Infracciones y Comparendos</p>
			</div>
		</div>
		<div class="text-right">
			<p class="text-2xl font-black tracking-wide text-red-900">NOTIFICACIÓN DE COMPARENDO</p>
			<p class="text-sm font-semibold mt-1">Ref. <span class="tabular-nums">{String(comparendo.id).padStart(4, '0')}</span></p>
			<p class="text-[11px] text-slate-600 mt-0.5">Emitida: {hoy}</p>
		</div>
	</div>

	<!-- Estado -->
	<div class="flex items-center gap-2 mb-5">
		<span class="text-[11px] font-bold uppercase tracking-wider text-slate-500">Estado:</span>
		<span
			class="inline-flex items-center rounded-full border px-3 py-0.5 text-xs font-bold uppercase tracking-wide
			{comparendo.estado === 'Pendiente' ? 'border-orange-700 text-orange-700' :
			comparendo.estado === 'Pagado' ? 'border-green-700 text-green-700' :
			'border-slate-700 text-slate-700'}"
		>
			{comparendo.estado}
		</span>
	</div>

	<!-- Vehículo y Renta -->
	<div class="grid grid-cols-2 gap-4 mb-5">
		<div class="rounded-lg border border-slate-300 p-3">
			<p class="text-[10px] font-bold uppercase tracking-wider text-slate-500 mb-1">Vehículo Implicado</p>
			<p class="font-bold">{comparendo.vehiculo || 'No especificado'}</p>
			<p class="text-xs text-slate-600">Placa: <span class="font-semibold text-slate-800">{comparendo.placa}</span></p>
		</div>
		<div class="rounded-lg border border-slate-300 p-3">
			<p class="text-[10px] font-bold uppercase tracking-wider text-slate-500 mb-1">Responsable (Cliente / Renta)</p>
			{#if comparendo.responsable}
				<p class="font-bold">{comparendo.responsable.nombreCliente || 'Cliente sin nombre'}</p>
				<p class="text-xs text-slate-600">
					Renta {formatContrato(comparendo.responsable.anioContrato, comparendo.responsable.noContrato)}
					<span class="text-slate-400">· {formatDate(comparendo.responsable.fechaRecogida)} → {formatDate(comparendo.responsable.fechaRetorno)}</span>
				</p>
				<p class="text-[10px] text-slate-500 mt-0.5">Tenía el vehículo el día de la infracción</p>
			{:else if comparendo.idRenta}
				<p class="font-bold">Renta #{comparendo.idRenta}</p>
				{#if comparendo.idCliente}
					<p class="text-xs text-slate-600">Cliente ID: {comparendo.idCliente}</p>
				{/if}
				<p class="text-[10px] text-slate-500 mt-0.5">Vínculo registrado al importar</p>
			{:else}
				<p class="font-bold text-slate-500 italic">No asociado a renta</p>
				<p class="text-[10px] text-slate-500 mt-0.5">El vehículo no estaba rentado el día de la infracción.</p>
			{/if}
		</div>
	</div>

	<!-- Detalles de la Infracción -->
	<div class="rounded-lg border border-slate-300 p-3 mb-5">
		<p class="text-[10px] font-bold uppercase tracking-wider text-slate-500 mb-2">Detalles de la Infracción</p>
		<div class="space-y-2 text-sm">
			<div class="grid grid-cols-2">
				<p><span class="text-slate-500 inline-block w-20">Fecha:</span> <span class="font-semibold">{formatDate(comparendo.fechaInfraccion)}</span></p>
				<p><span class="text-slate-500 inline-block w-20">Hora:</span> <span class="font-semibold">{hora(comparendo.horaInfraccion)}</span></p>
			</div>
			<div class="pt-2 border-t border-slate-100 flex justify-between items-center bg-slate-50 p-2 rounded">
				<span class="text-slate-700 font-bold uppercase text-xs">Monto a Pagar:</span>
				<span class="font-black text-lg tabular-nums">{formatCOP(comparendo.monto)}</span>
			</div>
		</div>
	</div>

	<!-- Observaciones -->
	{#if comparendo.observaciones}
		<div class="rounded-lg border border-slate-300 p-3 mb-5">
			<p class="text-[10px] font-bold uppercase tracking-wider text-slate-500 mb-1">Observaciones</p>
			<p class="text-sm whitespace-pre-wrap">{comparendo.observaciones}</p>
		</div>
	{/if}

	<!-- Condiciones -->
	<div class="text-[10px] text-slate-600 space-y-0.5 mb-8">
		<p>• El cliente es responsable por daños, multas y comparendos ocurridos durante el periodo de la renta estipulado en el contrato.</p>
		<p>• La presente notificación informa del registro de una infracción de tránsito asociada al vehículo y periodo de renta correspondientes.</p>
		<p>• El monto indicado deberá ser cancelado según lo estipulado en los términos de servicio.</p>
	</div>

	<!-- Firmas -->
	<div class="grid grid-cols-2 gap-8 mt-10">
		<div class="text-center">
			<div class="border-t border-slate-400 pt-2 text-xs font-semibold">Recibí conforme (Cliente)</div>
			<p class="text-[10px] text-slate-500 mt-0.5">Firma / Cédula</p>
		</div>
		<div class="text-center">
			<div class="border-t border-slate-400 pt-2 text-xs font-semibold">Dinamo Rent a Car</div>
			<p class="text-[10px] text-slate-500 mt-0.5">Gestión Administrativa</p>
		</div>
	</div>

	<!-- Pie -->
	<p class="text-center text-[9px] text-slate-400 mt-8">
		Dinamo Rent a Car · Comparendo Ref. {String(comparendo.id).padStart(4, '0')} · Impreso el {hoy}
	</p>
</div>
