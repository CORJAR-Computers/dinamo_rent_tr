<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import {
		autoApi,
		comparendoApi,
		rentaApi,
		ApiError,
		type Auto
	} from '$lib/api';
	import { sid } from '$lib/stores/session.svelte';
	import { toast } from '$lib/stores/toast.svelte';
	import { formatCOP, formatContrato, formatDate } from '$lib/utils/format';
	import { guardSesion } from '$lib/utils/guards';
	import {
		construirTimelineVehiculo,
		type TimelineVehiculo
	} from '$lib/utils/timelineVehiculo';

	const placa = $derived(String(page.params.placa ?? '').toUpperCase());

	let vehiculo = $state<Auto | null>(null);
	let timeline = $state<TimelineVehiculo | null>(null);
	let loading = $state(true);

	onMount(async () => {
		if (!guardSesion()) return;
		await cargar();
	});

	async function cargar() {
		loading = true;
		try {
			const [auto, rentas, comparendos] = await Promise.all([
				autoApi.obtener(sid(), placa),
				rentaApi.listar(sid(), undefined, undefined, placa),
				comparendoApi.listar(sid(), undefined, placa, undefined)
			]);
			vehiculo = auto;
			timeline = construirTimelineVehiculo(rentas, comparendos);
		} catch (e) {
			toast.error(
				e instanceof ApiError ? `No se pudo cargar el historial: ${e.message}` : 'No se pudo cargar el historial del vehículo.'
			);
		} finally {
			loading = false;
		}
	}

	function estadoMultaClases(estado: string): string {
		if (estado === 'Pagado') return 'bg-exito/10 text-exito border-exito/25';
		return 'bg-alerta/10 text-alerta border-alerta/25';
	}

	function estadoRentaClases(estado: string): string {
		const e = estado.toLowerCase();
		if (e === 'cancelada') return 'bg-peligro/10 text-peligro border-peligro/25';
		if (e === 'cerrada') return 'bg-text-secondary/10 text-text-secondary border-text-secondary/25';
		return 'bg-primary/10 text-primary border-primary/25';
	}
</script>

<svelte:head>
	<title>Historial {placa} — Dinamo Rent ERP</title>
</svelte:head>

<div class="space-y-5">
	<!-- Encabezado -->
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div class="flex items-center gap-3">
			<a
				href="/autos"
				class="p-2 rounded-lg text-text-secondary hover:text-primary hover:bg-primary/10 transition-colors"
				title="Volver a Autos"
				aria-label="Volver a Autos"
			>
				<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M10.5 19.5L3 12m0 0l7.5-7.5M3 12h18" /></svg>
			</a>
			<div>
				<h2 class="text-2xl font-bold text-text-primary font-mono">{placa}</h2>
				<p class="text-sm text-text-secondary mt-0.5">
					{vehiculo ? `${vehiculo.marca} ${vehiculo.modelo}` : 'Historial del vehículo'}
					{#if vehiculo}
						· <span class="inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[11px] font-semibold whitespace-nowrap {estadoRentaClases(vehiculo.estado)}">
							<span class="w-1.5 h-1.5 rounded-full bg-current opacity-70"></span>
							{vehiculo.estado}
						</span>
					{/if}
				</p>
			</div>
		</div>
	</div>

	{#if loading}
		<div class="card flex items-center justify-center py-16">
			<div class="flex flex-col items-center gap-3">
				<svg class="animate-spin h-8 w-8 text-primary" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				<p class="text-sm text-text-secondary">Cargando historial de {placa}...</p>
			</div>
		</div>
	{:else if timeline}
		<!-- Resumen -->
		<div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
			<div class="card p-4">
				<p class="text-xs text-text-secondary">Rentas registradas</p>
				<p class="text-2xl font-bold text-text-primary tabular-nums mt-1">{timeline.rentas.length}</p>
			</div>
			<div class="card p-4">
				<p class="text-xs text-text-secondary">Comparendos / multas</p>
				<p class="text-2xl font-bold text-text-primary tabular-nums mt-1">{timeline.totalMultas}</p>
			</div>
			<div class="card p-4">
				<p class="text-xs text-text-secondary">Pendiente por pagar</p>
				<p class="text-2xl font-bold text-text-primary tabular-nums mt-1">{formatCOP(String(timeline.totalPendiente))}</p>
			</div>
		</div>

		<!-- Línea de tiempo -->
		<div class="card p-5">
			<h3 class="font-semibold text-text-primary mb-1">Historial del vehículo</h3>
			<p class="text-xs text-text-secondary mb-4">
				Rentas (quién tuvo el vehículo y cuándo) y multas en orden cronológico. Cada multa indica quién la tenía el día de la infracción.
			</p>

			{#if timeline.eventos.length === 0}
				<div class="py-12 text-center">
					<p class="text-text-secondary text-sm">Sin rentas ni comparendos para {placa}.</p>
				</div>
			{:else}
				<ol class="relative border-l-2 border-primary/20 ml-3 pl-6 space-y-5">
					{#each timeline.eventos as ev}
						<li class="relative">
							<span
								class="absolute -left-[35px] top-4 w-3 h-3 rounded-full border-2 border-surface {ev.tipo === 'renta' ? 'bg-primary' : 'bg-alerta'} shadow"
								aria-hidden="true"
							></span>

							{#if ev.tipo === 'renta'}
								{@const r = ev.renta}
								<div class="rounded-xl border border-primary/20 bg-primary/5 p-4">
									<div class="flex flex-wrap items-center justify-between gap-2">
										<p class="font-semibold text-text-primary">{r.renta.nombreCliente || '—'}</p>
										<span class="inline-flex items-center gap-1 rounded-full border px-2.5 py-0.5 text-[11px] font-semibold whitespace-nowrap {estadoRentaClases(r.renta.estado)}">
											<span class="w-1.5 h-1.5 rounded-full bg-current opacity-70"></span>
											{r.renta.estado}
										</span>
									</div>
									<p class="text-xs text-text-secondary mt-1">
										Renta {formatContrato(r.renta.anioContrato, r.renta.noContrato)} ·
										{formatDate(r.inicio)} → {formatDate(r.fin)}
										{#if r.renta.fechaDevolucionReal}
											<span class="text-text-secondary/70">(devuelto {formatDate(r.renta.fechaDevolucionReal)})</span>
										{/if}
									</p>
									{#if r.multas.length > 0}
										<div class="mt-2 pt-2 border-t border-primary/10">
											<p class="text-[11px] font-semibold uppercase tracking-wide text-text-secondary">
												{r.multas.length} multa{r.multas.length === 1 ? '' : 's'} dentro de esta renta
											</p>
											<div class="mt-1.5 flex flex-wrap gap-1.5">
												{#each r.multas as m}
													<span class="inline-flex items-center gap-1 rounded-full border border-alerta/25 bg-alerta/10 px-2.5 py-0.5 text-[11px] font-semibold text-alerta whitespace-nowrap">
														{formatDate(m.fechaInfraccion)} · {formatCOP(m.monto)}
													</span>
												{/each}
											</div>
										</div>
									{/if}
								</div>
							{:else}
								{@const m = ev.multa.comparendo}
								<div class="rounded-xl border border-alerta/25 bg-alerta/5 p-4">
									<div class="flex flex-wrap items-center justify-between gap-2">
										<p class="font-bold text-text-primary tabular-nums">{formatCOP(m.monto)}</p>
										<span class="inline-flex items-center gap-1 rounded-full border px-2.5 py-0.5 text-[11px] font-semibold whitespace-nowrap {estadoMultaClases(m.estado)}">
											<span class="w-1.5 h-1.5 rounded-full bg-current opacity-70"></span>
											{m.estado}
										</span>
									</div>
									<p class="text-xs text-text-secondary mt-1">
										{formatDate(m.fechaInfraccion)} · {m.horaInfraccion}
										{#if m.numeroComparendo}
											· <span class="font-mono">{m.numeroComparendo}</span>
										{/if}
									</p>
									{#if m.responsable}
										<p class="text-xs mt-1.5">
											<span class="text-text-secondary">Tenía el vehículo:</span>
											<strong class="text-text-primary">{m.responsable.nombreCliente}</strong>
											<span class="text-text-secondary">
												(renta {formatContrato(m.responsable.anioContrato, m.responsable.noContrato)})
											</span>
										</p>
									{:else}
										<p class="text-xs mt-1.5 text-text-secondary">
											El vehículo no estaba rentado ese día.
										</p>
									{/if}
								</div>
							{/if}
						</li>
					{/each}
				</ol>
			{/if}
		</div>
	{/if}
</div>
