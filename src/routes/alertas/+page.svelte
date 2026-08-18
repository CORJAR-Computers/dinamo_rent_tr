<script lang="ts">
	import { onMount } from 'svelte';
	import {
		autoApi,
		mantenimientoApi,
		rentaApi,
		comparendoApi,
		type AlertaVencimiento,
		type AlertaKm,
		type Renta,
		type Comparendo
	} from '$lib/api';
	import { session } from '$lib/stores/session.svelte';
	import { formatCOP, formatDate } from '$lib/utils/format';
	import { guardSesion } from '$lib/utils/guards';
	import Icon from '$lib/components/Icon.svelte';

	const sid = () => session.token ?? '';

	let vencimientos = $state<AlertaVencimiento[]>([]);
	let km = $state<AlertaKm[]>([]);
	let rentas = $state<Renta[]>([]);
	let comparendos = $state<Comparendo[]>([]);
	let loading = $state(true);
	let soloCriticas = $state(false);
	let ultimaActualizacion = $state<Date | null>(null);
	let error = $state('');

	const diasVencimiento = $derived.by(() => {
		const hoy = new Date();
		hoy.setHours(0, 0, 0, 0);
		return (fecha: string | null): number | null => {
			if (!fecha) return null;
			const d = new Date(fecha + 'T00:00:00');
			if (Number.isNaN(d.getTime())) return null;
			return Math.round((d.getTime() - hoy.getTime()) / 86_400_000);
		};
	});

	// Rentas activas por vencer (retorno dentro de 3 días o ya vencidas)
	const rentasPorVencer = $derived(
		rentas.filter((r) => {
			const dias = diasVencimiento(r.fechaRetorno);
			return dias !== null && dias <= 3;
		})
	);

	const totalAlertas = $derived(
		vencimientos.length + km.length + rentasPorVencer.length + comparendos.length
	);

	async function cargar() {
		loading = true;
		error = '';
		try {
			const [v, k, r, c] = await Promise.all([
				autoApi.alertas(sid()),
				mantenimientoApi.alertasKm(sid()),
				rentaApi.listar(sid(), undefined, 'Activo'),
				comparendoApi.listar(sid(), undefined, undefined, 'Pendiente')
			]);
			vencimientos = v;
			km = k;
			rentas = r;
			comparendos = c;
			ultimaActualizacion = new Date();
		} catch {
			error = 'No se pudieron cargar las alertas. Verifica la conexión con el backend.';
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		if (!guardSesion()) return;
		cargar();
	});

	// ── Presentación ──
	function badgeClases(critica: boolean): string {
		return critica
			? 'bg-peligro/10 text-peligro border-peligro/30'
			: 'bg-alerta/10 text-alerta border-alerta/30';
	}

	function diasLabel(dias: number | null): string {
		if (dias === null) return 'Sin fecha';
		if (dias < 0) return `Vencido hace ${Math.abs(dias)} día${Math.abs(dias) === 1 ? '' : 's'}`;
		if (dias === 0) return 'Vence hoy';
		return `Vence en ${dias} día${dias === 1 ? '' : 's'}`;
	}

	function diasClases(dias: number | null): string {
		if (dias === null) return 'text-text-secondary';
		if (dias < 0) return 'text-peligro font-bold';
		if (dias <= 3) return 'text-alerta font-semibold';
		return 'text-text-secondary';
	}

	const vencimientosFiltrados = $derived(
		soloCriticas ? vencimientos.filter((v) => v.critica) : vencimientos
	);
	const kmFiltrados = $derived(soloCriticas ? km.filter((k) => k.critica) : km);
	const rentasFiltradas = $derived(
		soloCriticas
			? rentasPorVencer.filter((r) => {
					const d = diasVencimiento(r.fechaRetorno);
					return d !== null && d < 0;
				})
			: rentasPorVencer
	);
	// Los comparendos ya vienen filtrados a Pendiente; "solo críticas" aplica
	// sobre los que superan el valor de un salario mínimo mensual (1.423.500 COP)
	const COMPARENDO_CRITICO = 1_423_500;
	const comparendosFiltrados = $derived(
		soloCriticas ? comparendos.filter((c) => parseFloat(c.monto) > COMPARENDO_CRITICO) : comparendos
	);
</script>

<svelte:head>
	<title>Alertas — Dinamo Rent ERP</title>
</svelte:head>

<div class="space-y-5">
	<!-- Encabezado -->
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div>
			<h2 class="text-2xl font-bold text-text-primary">Alertas</h2>
			<p class="text-sm text-text-secondary mt-0.5">
				{totalAlertas} alerta{totalAlertas === 1 ? '' : 's'} activa{totalAlertas === 1 ? '' : 's'}
				{#if ultimaActualizacion}
					· actualizado {ultimaActualizacion.toLocaleTimeString('es-CO', { hour: '2-digit', minute: '2-digit' })}
				{/if}
			</p>
		</div>
		<div class="flex items-center gap-2">
			<label class="inline-flex items-center gap-2 text-sm text-text-primary cursor-pointer select-none rounded-lg border border-border px-3 py-2 hover:bg-alt-row/60 transition-colors">
				<input type="checkbox" class="accent-primary" bind:checked={soloCriticas} />
				Solo críticas
			</label>
			<button class="btn-ghost" onclick={cargar} disabled={loading}>
				<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4 {loading ? 'animate-spin' : ''}" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" /></svg>
				Refrescar
			</button>
		</div>
	</div>

	{#if error}
		<div class="rounded-lg bg-peligro/10 border border-peligro/30 px-4 py-3 text-sm text-peligro" role="alert">
			{error}
		</div>
	{/if}

	{#if loading}
		<div class="card flex items-center justify-center py-16">
			<div class="flex flex-col items-center gap-3">
				<svg class="animate-spin h-8 w-8 text-primary" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				<p class="text-sm text-text-secondary">Calculando alertas...</p>
			</div>
		</div>
	{:else}
		<!-- Resumen -->
		<div class="grid grid-cols-2 sm:grid-cols-4 gap-3">
			<div class="card p-4 border-l-4 border-l-peligro">
				<p class="text-3xl font-black text-peligro tabular-nums">{vencimientos.length}</p>
				<p class="text-xs font-semibold uppercase tracking-wide text-text-secondary mt-1">Vencimientos de vehículos</p>
			</div>
			<div class="card p-4 border-l-4 border-l-alerta">
				<p class="text-3xl font-black text-alerta tabular-nums">{km.length}</p>
				<p class="text-xs font-semibold uppercase tracking-wide text-text-secondary mt-1">Mantenimiento por km</p>
			</div>
			<div class="card p-4 border-l-4 border-l-primary">
				<p class="text-3xl font-black text-primary tabular-nums">{rentasPorVencer.length}</p>
				<p class="text-xs font-semibold uppercase tracking-wide text-text-secondary mt-1">Rentas por vencer</p>
			</div>
			<div class="card p-4 border-l-4 border-l-alerta">
				<p class="text-3xl font-black text-alerta tabular-nums">{comparendos.length}</p>
				<p class="text-xs font-semibold uppercase tracking-wide text-text-secondary mt-1">Comparendos pendientes</p>
			</div>
		</div>

		<!-- Vencimientos -->
		<section class="card">
			<div class="flex items-center gap-2 mb-3">
				<span class="w-2 h-2 rounded-full bg-peligro animate-pulse"></span>
				<h3 class="text-sm font-bold uppercase tracking-wider text-text-primary">
					Vencimientos de vehículos ({vencimientosFiltrados.length})
				</h3>
			</div>
			{#if vencimientosFiltrados.length === 0}
				<p class="text-sm text-text-secondary py-4 text-center">Sin vencimientos próximos. <Icon name="check" class="w-4 h-4 inline-block align-[-2px] text-exito" /></p>
			{:else}
				<div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-3">
					{#each vencimientosFiltrados as v}
						<div class="rounded-xl border border-border bg-alt-row/40 p-3.5 flex items-start justify-between gap-3 hover:border-primary/40 transition-colors">
							<div class="min-w-0">
								<p class="font-semibold text-text-primary flex items-center gap-2">
									<span class="font-mono text-xs">{v.placa}</span>
									<span class="text-xs font-medium text-text-secondary truncate">{v.marca} {v.modelo}</span>
								</p>
								<p class="text-sm text-text-primary mt-1">{v.tipo}: <span class="font-bold">{formatDate(v.fecha)}</span></p>
								<p class="text-xs text-text-secondary mt-0.5 truncate">{v.detalle}</p>
							</div>
							<span class="inline-flex items-center rounded-full border px-2.5 py-0.5 text-[11px] font-bold whitespace-nowrap shrink-0 {badgeClases(v.critica)}">
								{diasLabel(v.diasRestantes)}
							</span>
						</div>
					{/each}
				</div>
			{/if}
		</section>

		<!-- Mantenimiento por km -->
		<section class="card">
			<div class="flex items-center gap-2 mb-3">
				<span class="w-2 h-2 rounded-full bg-alerta animate-pulse"></span>
				<h3 class="text-sm font-bold uppercase tracking-wider text-text-primary">
					Mantenimiento por kilometraje ({kmFiltrados.length})
				</h3>
			</div>
			{#if kmFiltrados.length === 0}
				<p class="text-sm text-text-secondary py-4 text-center">Sin mantenimientos próximos por km. <Icon name="check" class="w-4 h-4 inline-block align-[-2px] text-exito" /></p>
			{:else}
				<div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-3">
					{#each kmFiltrados as k}
						<div class="rounded-xl border border-border bg-alt-row/40 p-3.5 flex items-start justify-between gap-3 hover:border-primary/40 transition-colors">
							<div class="min-w-0">
								<p class="font-semibold text-text-primary flex items-center gap-2">
									<span class="font-mono text-xs">{k.placa}</span>
									<span class="text-xs font-medium text-text-secondary truncate">{k.marca} {k.modelo}</span>
								</p>
								<p class="text-sm text-text-primary mt-1">{k.tipo}</p>
								<p class="text-xs text-text-secondary mt-0.5 tabular-nums">
									Actual {k.kmActual.toLocaleString('es-CO')} km · próximo {k.kmProximo.toLocaleString('es-CO')} km
								</p>
							</div>
							<span class="inline-flex items-center rounded-full border px-2.5 py-0.5 text-[11px] font-bold whitespace-nowrap shrink-0 {badgeClases(k.critica)}">
								{k.kmRestante <= 0 ? `${Math.abs(k.kmRestante).toLocaleString('es-CO')} km vencido` : `${k.kmRestante.toLocaleString('es-CO')} km`}
							</span>
						</div>
					{/each}
				</div>
			{/if}
		</section>

		<!-- Rentas por vencer -->
		<section class="card">
			<div class="flex items-center gap-2 mb-3">
				<span class="w-2 h-2 rounded-full bg-primary animate-pulse"></span>
				<h3 class="text-sm font-bold uppercase tracking-wider text-text-primary">
					Rentas por vencer ({rentasFiltradas.length})
				</h3>
			</div>
			{#if rentasFiltradas.length === 0}
				<p class="text-sm text-text-secondary py-4 text-center">No hay rentas activas por vencer en los próximos 3 días. <Icon name="check" class="w-4 h-4 inline-block align-[-2px] text-exito" /></p>
			{:else}
				<div class="overflow-x-auto">
					<table class="w-full text-sm">
						<thead>
							<tr class="border-b border-border text-left">
								<th class="py-2 pr-3 font-semibold text-text-secondary">Renta</th>
								<th class="py-2 pr-3 font-semibold text-text-secondary">Cliente</th>
								<th class="py-2 pr-3 font-semibold text-text-secondary">Vehículo</th>
								<th class="py-2 pr-3 font-semibold text-text-secondary">Retorno</th>
								<th class="py-2 pr-3 font-semibold text-text-secondary text-right">Saldo</th>
							</tr>
						</thead>
						<tbody class="divide-y divide-border/60">
							{#each rentasFiltradas as r}
								<tr class="hover:bg-alt-row/50 transition-colors">
									<td class="py-2.5 pr-3 font-bold text-primary tabular-nums">#{String(r.id).padStart(4, '0')}</td>
									<td class="py-2.5 pr-3 text-text-primary">{r.nombreCliente}</td>
									<td class="py-2.5 pr-3 text-text-primary">
										<span class="font-mono text-xs">{r.placa ?? '—'}</span> <span class="text-text-secondary text-xs">{r.vehiculo}</span>
									</td>
									<td class="py-2.5 pr-3">
										<span class="{diasClases(diasVencimiento(r.fechaRetorno))} tabular-nums">{formatDate(r.fechaRetorno)}</span>
										<p class="text-xs text-text-secondary">{diasLabel(diasVencimiento(r.fechaRetorno))}</p>
									</td>
									<td class="py-2.5 text-right">
										{#if parseFloat(r.saldoPendiente) > 0}
											<span class="font-bold text-alerta tabular-nums">{formatCOP(r.saldoPendiente)}</span>
										{:else}
											<span class="text-exito text-xs font-semibold">Al día</span>
										{/if}
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{/if}
		</section>

		<!-- Comparendos pendientes -->
		<section class="card">
			<div class="flex items-center gap-2 mb-3">
				<span class="w-2 h-2 rounded-full bg-alerta animate-pulse"></span>
				<h3 class="text-sm font-bold uppercase tracking-wider text-text-primary">
					Comparendos pendientes ({comparendosFiltrados.length})
				</h3>
			</div>
			{#if comparendosFiltrados.length === 0}
				<p class="text-sm text-text-secondary py-4 text-center">Sin comparendos pendientes de pago. <Icon name="check" class="w-4 h-4 inline-block align-[-2px] text-exito" /></p>
			{:else}
				<div class="overflow-x-auto">
					<table class="w-full text-sm">
						<thead>
							<tr class="border-b border-border text-left">
								<th class="py-2 pr-3 font-semibold text-text-secondary">No.</th>
								<th class="py-2 pr-3 font-semibold text-text-secondary">Placa</th>
								<th class="py-2 pr-3 font-semibold text-text-secondary">Fecha</th>
								<th class="py-2 pr-3 font-semibold text-text-secondary">Observación</th>
								<th class="py-2 pr-3 font-semibold text-text-secondary text-right">Monto</th>
							</tr>
						</thead>
						<tbody class="divide-y divide-border/60">
							{#each comparendosFiltrados as c}
								<tr class="hover:bg-alt-row/50 transition-colors">
									<td class="py-2.5 pr-3 font-bold text-primary tabular-nums">#{String(c.id).padStart(4, '0')}</td>
									<td class="py-2.5 pr-3 font-mono text-xs">{c.placa}</td>
									<td class="py-2.5 pr-3 text-text-primary tabular-nums">{formatDate(c.fechaInfraccion)}</td>
									<td class="py-2.5 pr-3 text-text-secondary truncate max-w-[260px]">{c.observaciones ?? '—'}</td>
									<td class="py-2.5 text-right font-bold text-alerta tabular-nums">{formatCOP(c.monto)}</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{/if}
		</section>
	{/if}
</div>
