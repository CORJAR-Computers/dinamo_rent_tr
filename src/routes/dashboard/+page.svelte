<script lang="ts">
	import { onMount } from 'svelte';
	import { dashboardApi, ApiError, type DashboardData } from '$lib/api';
	import { session, sid } from '$lib/stores/session.svelte';
	import { toast } from '$lib/stores/toast.svelte';
	import { formatDate } from '$lib/utils/format';
	import { guardSesion, haySesion } from '$lib/utils/guards';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import PiiKeyDialog from '$lib/components/PiiKeyDialog.svelte';

	// sid() viene del store (reemplaza `const sid = () => session.token ?? ''`). Ver TAREA E3.

	let data = $state<DashboardData | null>(null);
	let piiDialogOpen = $state(false);
	let loading = $state(true);
	let error = $state('');

	const greeting = $derived.by(() => {
		const h = new Date().getHours();
		if (h < 12) return 'Buenos días';
		if (h < 19) return 'Buenas tardes';
		return 'Buenas noches';
	});

	const estadoColors: Record<string, string> = {
		'Disponible': 'bg-estado-disponible',
		'Rentado': 'bg-estado-rentado',
		'Mantenimiento': 'bg-estado-mantenimiento',
		'Vendido': 'bg-text-secondary',
		'Baja': 'bg-estado-inactivo'
	};

	async function cargar() {
		// Guard de sesión: cortar llamadas a la API durante una redirección
		if (!haySesion()) return;
		loading = true;
		error = '';
		try {
			data = await dashboardApi.getData(sid());
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'No se pudieron cargar los indicadores.';
			toast.error(error);
		} finally {
			loading = false;
		}
	}

	onMount(() => {
		if (!guardSesion()) return;
		cargar();
	});

	const maxEstado = $derived(
		data ? Math.max(1, ...data.autosPorEstado.map((e) => e.total)) : 1
	);

	const criticalas = $derived((data?.alertas ?? []).filter((a) => a.critica));
	const totalAlertas = $derived(data?.alertas.length ?? 0);

	const kpis = $derived([
		{
			label: 'Autos en flota',
			value: data?.totalAutos ?? '—',
			icon: 'car',
			hint: 'Total de vehículos registrados',
			tint: 'bg-primary/10 text-primary'
		},
		{
			label: 'Rentas activas',
			value: data?.rentasActivas ?? '—',
			icon: 'clipboard',
			hint: 'En curso en este momento',
			tint: 'bg-estado-rentado/10 text-estado-rentado'
		},
		{
			label: 'Clientes registrados',
			value: data?.totalClientes ?? '—',
			icon: 'users',
			hint: 'Total en base de datos',
			tint: 'bg-exito/10 text-exito'
		},
		{
			label: 'Vencimientos próximos',
			value: totalAlertas,
			icon: 'alert',
			hint: 'SOAT, técnico-mecánica, extintor y aceite',
			tint: criticalas.length > 0 ? 'bg-peligro/10 text-peligro' : 'bg-alerta/10 text-alerta'
		}
	]);
</script>

<svelte:head>
	<title>Dashboard — Dinamo Rent ERP</title>
</svelte:head>

<div class="space-y-6">
	<!-- Encabezado -->
	<div class="flex flex-wrap items-start justify-between gap-3">
		<div>
			<h2 class="text-2xl font-bold text-text-primary">
				{greeting}, {session.user?.nombre || session.user?.username}
				<span class="inline-flex align-middle ml-1.5"><Icon name="hand" class="w-6 h-6" /></span>
			</h2>
			<p class="text-text-secondary mt-1">Resumen general del sistema de gestión de flota.</p>
		</div>
		<div class="flex items-center gap-2">
			<button class="btn-ghost !px-3 !py-1.5 text-xs inline-flex items-center gap-1.5" onclick={cargar} title="Actualizar indicadores" aria-label="Actualizar indicadores">
				<Icon name="actualizar" class="w-4 h-4" />
				Actualizar
			</button>
			<span class="text-xs px-3 py-1.5 rounded-full bg-exito/10 text-exito font-semibold inline-flex items-center gap-1.5">
				<span class="w-1.5 h-1.5 rounded-full bg-exito animate-pulse"></span>
				Sistema operativo
			</span>
		</div>
	</div>

	{#if loading}
		<div class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4">
			{#each [1, 2, 3, 4] as _}
				<div class="card p-5 animate-pulse">
					<div class="h-8 w-8 rounded-xl bg-alt-row mb-4"></div>
					<div class="h-7 w-20 bg-alt-row rounded mb-2"></div>
					<div class="h-4 w-32 bg-alt-row rounded"></div>
				</div>
			{/each}
		</div>
	{:else if !data}
		<div class="card">
			<EmptyState title="No se pudieron cargar los indicadores" description={error} icon="chart" />
		</div>
	{:else}
		<!-- KPIs -->
		<div class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4">
			{#each kpis as kpi}
				<div
					class="card p-5 hover:shadow-md hover:-translate-y-0.5 transition-[transform,box-shadow] duration-150 group"
				>
					<div class="flex items-center justify-between mb-4">
						<span class="w-10 h-10 rounded-xl flex items-center justify-center {kpi.tint}">
							<Icon name={kpi.icon} class="w-5 h-5" />
						</span>
						<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4 text-text-secondary/40 group-hover:text-primary transition-colors" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M3.75 13.5l10.5-11.25L12 10.5h8.25L9.75 21.75 12 13.5H3.75z" /></svg>
					</div>
					<p class="text-3xl font-bold text-text-primary tabular-nums tracking-tight">{kpi.value}</p>
					<p class="text-sm font-medium text-text-secondary mt-1">{kpi.label}</p>
					<p class="text-[11px] text-text-secondary/60 mt-0.5">{kpi.hint}</p>
				</div>
			{/each}
		</div>

		<div class="grid grid-cols-1 lg:grid-cols-3 gap-4">
			<!-- Autos por estado -->
			<div class="card p-6 lg:col-span-2">
				<h3 class="font-semibold text-text-primary mb-5 flex items-center gap-2">
					<span class="w-2 h-2 rounded-full bg-primary"></span>
					Flota por estado
				</h3>
				{#if data.autosPorEstado.length === 0}
					<p class="text-sm text-text-secondary">No hay vehículos registrados.</p>
				{:else}
					<div class="space-y-3.5">
						{#each data.autosPorEstado as e}
							<div class="flex items-center gap-3">
								<span class="w-28 shrink-0 text-sm text-text-secondary truncate">{e.estado}</span>
								<div class="flex-1 h-3 rounded-full bg-alt-row overflow-hidden">
									<div
										class="h-full rounded-full {estadoColors[e.estado] ?? 'bg-primary'} transition-all duration-500"
										style="width: {Math.max(4, (e.total / maxEstado) * 100)}%"
									></div>
								</div>
								<span class="w-10 text-right text-sm font-bold text-text-primary tabular-nums">{e.total}</span>
							</div>
						{/each}
					</div>
				{/if}

				<div class="mt-6 border-t border-border pt-4">
					<h4 class="text-xs font-bold uppercase tracking-wider text-text-secondary mb-3">Últimos clientes registrados</h4>
					{#if data.clientesRecientes.length === 0}
						<p class="text-sm text-text-secondary">Aún no hay clientes.</p>
					{:else}
						<div class="divide-y divide-border/60">
							{#each data.clientesRecientes as c}
								<div class="flex items-center justify-between gap-3 py-2.5">
									<div class="min-w-0">
										<p class="text-sm font-medium text-text-primary truncate">{c.nombreCompleto}</p>
										<p class="text-xs text-text-secondary truncate">{c.noDoc ? `${c.tipoDoc ?? ''} ${c.noDoc}` : 'Sin documento'}</p>
									</div>
									<div class="flex items-center gap-2 shrink-0">
										{#if c.ciudad}<span class="text-xs text-text-secondary">{c.ciudad}</span>{/if}
										<StatusBadge estado={c.estado} />
									</div>
								</div>
							{/each}
						</div>
					{/if}
				</div>
			</div>

			<!-- Alertas -->
			<div class="card p-6">
				<div class="flex items-center justify-between mb-5">
					<h3 class="font-semibold text-text-primary flex items-center gap-2">
						<span class="w-2 h-2 rounded-full {criticalas.length > 0 ? 'bg-peligro' : 'bg-alerta'}"></span>
						Alertas de flota
					</h3>
					{#if totalAlertas > 0}
						<span class="text-[11px] font-bold px-2 py-1 rounded-full {criticalas.length > 0 ? 'bg-peligro/10 text-peligro' : 'bg-alerta/10 text-alerta'}">{totalAlertas}</span>
					{/if}
				</div>

				{#if data.alertas.length === 0}
					<EmptyState title="Sin alertas" description="No hay vencimientos próximos de SOAT, técnico-mecánica, extintor o aceite." icon="check" />
				{:else}
					<div class="space-y-2 max-h-[420px] overflow-y-auto pr-1">
						{#each data.alertas as a}
							<div
								class="rounded-lg border px-3 py-2.5 text-sm flex items-start gap-2.5 transition-transform hover:scale-[1.01] {a.critica ? 'border-peligro/30 bg-peligro/5' : 'border-alerta/25 bg-alerta/5'}"
							>
								<span class="w-2 h-2 rounded-full mt-1.5 shrink-0 {a.critica ? 'bg-peligro' : 'bg-alerta'}"></span>
								<div class="min-w-0">
									<p class="font-semibold text-text-primary">
										<span class="font-bold">{a.placa}</span>
										<span class="text-text-secondary font-normal"> · {a.tipo}</span>
									</p>
									<p class="text-xs {a.critica ? 'text-peligro' : 'text-alerta'}">{a.detalle}{a.fecha ? ` · ${formatDate(a.fecha)}` : ''}</p>
								</div>
							</div>
						{/each}
					</div>
				{/if}

				{#if !data.piiKeyConfigurada}
					<div class="mt-5 rounded-lg bg-alerta/5 border border-alerta/20 px-3 py-2.5 text-[11px] text-alerta leading-relaxed flex items-center justify-between gap-2">
						<span class="inline-flex items-center gap-1.5"><Icon name="lightbulb" class="w-3.5 h-3.5 shrink-0" />Hay datos de clientes de versiones anteriores cifrados (Fernet) que no se muestran.</span>
						<button class="btn-outline !px-2.5 !py-1 text-[11px] shrink-0" onclick={() => (piiDialogOpen = true)}>
							<span class="inline-flex items-center gap-1.5"><Icon name="lock" class="w-3.5 h-3.5" />Configurar clave</span>
						</button>
					</div>
				{:else}
					<div class="mt-5 flex items-center justify-between gap-2 rounded-lg bg-exito/5 border border-exito/20 px-3 py-2 text-[11px] text-exito">
						<span class="inline-flex items-center gap-1.5"><Icon name="lock" class="w-3.5 h-3.5" />Clave PII configurada.</span>
						<button class="btn-ghost !px-2.5 !py-1 text-[11px] shrink-0" onclick={() => (piiDialogOpen = true)}>
							Gestionar clave
						</button>
					</div>
				{/if}
			</div>
		</div>
	{/if}
</div>

<!-- Diálogo de clave PII -->
<PiiKeyDialog
	open={piiDialogOpen}
	onClose={() => (piiDialogOpen = false)}
	onSaved={cargar}
/>
