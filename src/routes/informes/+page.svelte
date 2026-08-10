<script lang="ts">
	import { onMount } from 'svelte';
	import { informeApi, type InformeMensual, type UtilidadVehiculo } from '$lib/api';
	import { session } from '$lib/stores/session.svelte';
	import { toast } from '$lib/stores/toast.svelte';
	import { formatCOP, formatDate } from '$lib/utils/format';
	import { guardSesion } from '$lib/utils/guards';
	import { construirLibroInforme } from '$lib/utils/informeExcel';
	import { imprimirDocumento } from '$lib/utils/imprimir';

	const sid = () => session.token ?? '';

	let informe = $state<InformeMensual | null>(null);
	let loading = $state(true);
	let error = $state('');

	// Selector de rango de fechas
	const ahora = new Date();
	const primerDiaMes = new Date(ahora.getFullYear(), ahora.getMonth(), 1);
	
	let fechaInicio = $state(primerDiaMes.toISOString().split('T')[0]);
	let fechaFin = $state(ahora.toISOString().split('T')[0]);

	async function cargar() {
		if (!guardSesion()) return;
		loading = true;
		error = '';
		try {
			informe = await informeApi.mensual(sid(), fechaInicio, fechaFin);
		} catch {
			error = 'No se pudo calcular el informe del mes.';
			informe = null;
		} finally {
			loading = false;
		}
	}

	onMount(cargar);



	const balancePositivo = $derived(informe !== null && parseFloat(informe.balance) >= 0);

	function imprimir() {
		imprimirDocumento();
	}

	// MIGRACIÓN (G-C2): reemplazado `xlsx` (SheetJS CE descontinuado + CVEs) por
	// `exceljs`. La serialización es async y descarga vía Blob (en Tauri desktop el
	// guardado directo de archivos requiere el plugin fs/dialog; aquí seguimos usando
	// el flujo de descarga del navegador embebido, que WebView2 cumple bien).
	async function exportarExcel() {
		if (!informe) return;
		try {
			const wb = construirLibroInforme(informe, rangoTexto);
			const buffer = await wb.xlsx.writeBuffer();
			const blob = new Blob([buffer], {
				type: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'
			});
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			a.download = `informe_${fechaInicio}_al_${fechaFin}.xlsx`;
			document.body.appendChild(a);
			a.click();
			document.body.removeChild(a);
			URL.revokeObjectURL(url);
			toast.success('Informe exportado a Excel.');
		} catch {
			toast.error('No se pudo exportar el informe a Excel.');
		}
	}

	const rangoTexto = $derived(`${fechaInicio} al ${fechaFin}`);

	// Utilidad por vehículo: barra proporcional al mayor valor absoluto
	const maxUtilidad = $derived(
		informe?.utilidadPorVehiculo.reduce((m, v) => Math.max(m, Math.abs(parseFloat(v.utilidad)) || 0), 0) ?? 0
	);

	const vehiculosRentables = $derived(
		informe?.utilidadPorVehiculo.filter((v) => parseFloat(v.utilidad) >= 0).length ?? 0
	);

	function utilidadClases(v: UtilidadVehiculo): string {
		const u = parseFloat(v.utilidad);
		if (u > 0) return 'text-exito';
		if (u < 0) return 'text-peligro';
		return 'text-text-secondary';
	}

	function barraClases(v: UtilidadVehiculo): string {
		return parseFloat(v.utilidad) >= 0 ? 'bg-exito/70' : 'bg-peligro/70';
	}
</script>

<svelte:head>
	<title>Informes — Dinamo Rent ERP</title>
</svelte:head>

<div class="space-y-5">
	<!-- Encabezado -->
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div>
			<h2 class="text-2xl font-bold text-text-primary">Informes</h2>
			<p class="text-sm text-text-secondary mt-0.5">Balance mensual: ingresos reales vs egresos de la operación</p>
		</div>
		<div class="flex items-center gap-2">
			<input
				class="input w-auto"
				type="date"
				bind:value={fechaInicio}
				onchange={cargar}
				aria-label="Fecha inicio"
			/>
			<span class="text-text-secondary text-sm">hasta</span>
			<input
				class="input w-auto"
				type="date"
				bind:value={fechaFin}
				onchange={cargar}
				aria-label="Fecha fin"
			/>
		</div>
	</div>

	<!-- Acciones -->
	<div class="flex flex-wrap items-center gap-2">
		<button class="btn-primary" onclick={imprimir} disabled={!informe}>
			<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M6.72 13.829c-.24.03-.48.062-.72.096m.72-.096a42.415 42.415 0 0110.56 0m-10.56 0L6.34 18m10.94-4.171c.24.03.48.062.72.096m-.72-.096L17.66 18m0 0l.229 2.523a1.125 1.125 0 01-1.12 1.227H7.231c-.662 0-1.18-.568-1.12-1.227L6.34 18m11.318 0h1.091A2.25 2.25 0 0021 15.75V9.456c0-1.081-.768-2.015-1.837-2.175a48.055 48.055 0 00-1.913-.247M6.34 18H5.25A2.25 2.25 0 013 15.75V9.456c0-1.081.768-2.015 1.837-2.175a48.041 48.041 0 011.913-.247m10.5 0a48.536 48.536 0 00-10.5 0m10.5 0V3.375c0-.621-.504-1.125-1.125-1.125h-8.25c-.621 0-1.125.504-1.125 1.125v3.659M18 10.5h.008v.008H18V10.5z" /></svg>
			Imprimir / PDF
		</button>
		<button class="btn-ghost" onclick={exportarExcel} disabled={!informe}>
			<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5M16.5 12L12 16.5m0 0L7.5 12m4.5 4.5V3" /></svg>
			Exportar Excel
		</button>
	</div>

	{#if error}
		<div class="rounded-lg bg-peligro/10 border border-peligro/30 px-4 py-3 text-sm text-peligro" role="alert">{error}</div>
	{/if}

	{#if loading}
		<div class="card flex items-center justify-center py-16">
			<div class="flex flex-col items-center gap-3">
				<svg class="animate-spin h-8 w-8 text-primary" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				<p class="text-sm text-text-secondary">Calculando balance...</p>
			</div>
		</div>
	{:else if informe}
		<div class="print-area bg-white">
		<!-- Resumen -->
		<div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
			<div class="card p-5 border-l-4 border-l-exito">
				<p class="text-[11px] font-bold uppercase tracking-wider text-text-secondary">Ingresos del mes</p>
				<p class="text-3xl font-black text-exito tabular-nums mt-1">{formatCOP(informe.totalIngresos)}</p>
				<p class="text-xs text-text-secondary mt-1 tabular-nums">
					Pagos de rentas {formatCOP(informe.ingresosPagos)} · Abonos {formatCOP(informe.ingresosReservas)}
				</p>
			</div>
			<div class="card p-5 border-l-4 border-l-peligro">
				<p class="text-[11px] font-bold uppercase tracking-wider text-text-secondary">Egresos del mes</p>
				<p class="text-3xl font-black text-peligro tabular-nums mt-1">{formatCOP(informe.totalEgresos)}</p>
				<p class="text-xs text-text-secondary mt-1 tabular-nums">
					Gastos {formatCOP(informe.egresosGastos)} · Mantenimiento {formatCOP(informe.egresosMantenimiento)} · Comparendos {formatCOP(informe.egresosComparendos)}
				</p>
			</div>
			<div class="card p-5 border-l-4 {balancePositivo ? 'border-l-primary' : 'border-l-peligro'}">
				<p class="text-[11px] font-bold uppercase tracking-wider text-text-secondary">Balance</p>
				<p class="text-3xl font-black {balancePositivo ? 'text-primary' : 'text-peligro'} tabular-nums mt-1">{formatCOP(informe.balance)}</p>
				<p class="text-xs text-text-secondary mt-1">{rangoTexto}</p>
			</div>
		</div>

		<div class="grid grid-cols-1 xl:grid-cols-2 gap-5">
			<!-- Gastos por categoría -->
			<section class="card">
				<h3 class="text-sm font-bold uppercase tracking-wider text-text-primary mb-3">Gastos por categoría</h3>
				{#if informe.gastosPorCategoria.length === 0}
					<p class="text-sm text-text-secondary py-4 text-center">Sin gastos registrados este mes.</p>
				{:else}
					<div class="space-y-2">
						{#each informe.gastosPorCategoria as [categoria, total]}
							{@const pct = Math.min(100, (parseFloat(total) / Math.max(1e-9, parseFloat(informe.egresosGastos))) * 100)}
							<div>
								<div class="flex justify-between text-sm mb-1">
									<span class="font-medium text-text-primary">{categoria}</span>
									<span class="font-bold text-text-primary tabular-nums">{formatCOP(total)}</span>
								</div>
								<div class="h-2 rounded-full bg-alt-row overflow-hidden">
									<div class="h-full rounded-full bg-primary/70 transition-all" style="width: {pct}%"></div>
								</div>
							</div>
						{/each}
					</div>
				{/if}
			</section>

			<!-- Rentas del mes -->
			<section class="card">
				<h3 class="text-sm font-bold uppercase tracking-wider text-text-primary mb-3">Rentas del mes ({informe.rentas.length})</h3>
				{#if informe.rentas.length === 0}
					<p class="text-sm text-text-secondary py-4 text-center">Sin rentas iniciadas este mes.</p>
				{:else}
					<div class="overflow-x-auto max-h-[360px] overflow-y-auto">
						<table class="w-full text-sm">
							<thead class="sticky top-0 bg-surface">
								<tr class="border-b border-border text-left">
									<th class="py-2 pr-3 font-semibold text-text-secondary">No.</th>
									<th class="py-2 pr-3 font-semibold text-text-secondary">Placa</th>
									<th class="py-2 pr-3 font-semibold text-text-secondary">Cliente</th>
									<th class="py-2 pr-3 font-semibold text-text-secondary">Fecha</th>
									<th class="py-2 pr-3 font-semibold text-text-secondary">Estado</th>
									<th class="py-2 font-semibold text-text-secondary text-right">Total</th>
								</tr>
							</thead>
							<tbody class="divide-y divide-border/60">
								{#each informe.rentas as r}
									<tr class="hover:bg-alt-row/50 transition-colors">
										<td class="py-2 pr-3 font-bold text-primary tabular-nums">#{String(r.id).padStart(4, '0')}</td>
										<td class="py-2 pr-3 font-mono text-xs">{r.placa || '—'}</td>
										<td class="py-2 pr-3 text-text-primary truncate max-w-[180px]">{r.nombreCliente}</td>
										<td class="py-2 pr-3 text-text-secondary tabular-nums whitespace-nowrap">{formatDate(r.fechaRecogida)}</td>
										<td class="py-2 pr-3">
											<span class="inline-flex rounded-full border px-2 py-0.5 text-[10px] font-semibold
												{r.estado === 'Cerrada' ? 'border-exito/30 text-exito' :
												r.estado === 'Cancelada' ? 'border-peligro/30 text-peligro' :
												'border-primary/30 text-primary'}">
												{r.estado}
											</span>
										</td>
										<td class="py-2 text-right font-bold text-text-primary tabular-nums whitespace-nowrap">{formatCOP(r.total)}</td>
									</tr>
								{/each}
							</tbody>
						</table>
					</div>
				{/if}
			</section>
		</div>

		<!-- Utilidad por vehículo -->
		<section class="card">
			<div class="flex flex-wrap items-center justify-between gap-2 mb-3">
				<div class="flex items-center gap-2">
					<span class="w-2 h-2 rounded-full bg-exito"></span>
					<h3 class="text-sm font-bold uppercase tracking-wider text-text-primary">
						Utilidad por vehículo ({informe.utilidadPorVehiculo.length})
					</h3>
				</div>
				<p class="text-xs text-text-secondary">
					{vehiculosRentables} rentable{vehiculosRentables === 1 ? '' : 's'} · {informe.utilidadPorVehiculo.length - vehiculosRentables} en pérdida
				</p>
			</div>

			{#if informe.utilidadPorVehiculo.length === 0}
				<p class="text-sm text-text-secondary py-4 text-center">Sin movimiento por vehículo este mes.</p>
			{:else}
				<div class="overflow-x-auto">
					<table class="w-full text-sm">
						<thead>
							<tr class="border-b border-border text-left">
								<th class="py-2 pr-3 font-semibold text-text-secondary">Placa</th>
								<th class="py-2 pr-3 font-semibold text-text-secondary">Vehículo</th>
								<th class="py-2 pr-3 font-semibold text-text-secondary text-right">Ingresos</th>
								<th class="py-2 pr-3 font-semibold text-text-secondary text-right">Costos</th>
								<th class="py-2 pr-3 font-semibold text-text-secondary text-right">Utilidad</th>
								<th class="py-2 font-semibold text-text-secondary w-[180px]">Rentabilidad</th>
							</tr>
						</thead>
						<tbody class="divide-y divide-border/60">
							{#each informe.utilidadPorVehiculo as v}
								{@const util = parseFloat(v.utilidad) || 0}
								{@const ancho = maxUtilidad > 0 ? Math.min(100, (Math.abs(util) / maxUtilidad) * 100) : 0}
								<tr class="hover:bg-alt-row/50 transition-colors">
									<td class="py-2.5 pr-3 font-mono text-xs font-bold text-primary">{v.placa}</td>
									<td class="py-2.5 pr-3 text-text-primary truncate max-w-[200px]">{v.vehiculo || '—'}</td>
									<td class="py-2.5 pr-3 text-right text-text-primary tabular-nums whitespace-nowrap">{formatCOP(v.ingresos)}</td>
									<td class="py-2.5 pr-3 text-right text-text-primary tabular-nums whitespace-nowrap">{formatCOP(v.costos)}</td>
									<td class="py-2.5 pr-3 text-right font-bold tabular-nums whitespace-nowrap {utilidadClases(v)}">
										{util >= 0 ? '' : '−'}{formatCOP(Math.abs(util))}
									</td>
									<td class="py-2.5">
										<div class="h-2 rounded-full bg-alt-row overflow-hidden">
											<div
													class="h-full rounded-full transition-all {barraClases(v)}"
													style="width: {ancho}%"
													title="{formatCOP(v.utilidad)}"
												></div>
										</div>
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{/if}
		</section>
		</div>
	{/if}
</div>
