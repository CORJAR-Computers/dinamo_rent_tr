<script lang="ts">
	import { onMount } from 'svelte';
	import { dashboardApi, ApiError, type DashboardData } from '$lib/api';
	import { session } from '$lib/stores/session.svelte';
	import { toast } from '$lib/stores/toast.svelte';
	import { formatDate } from '$lib/utils/format';
	import { guardSesion, haySesion } from '$lib/utils/guards';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import PiiKeyDialog from '$lib/components/PiiKeyDialog.svelte';

	const sid = () => session.token ?? '';

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

	const iconos: Record<string, string> = {
		car: 'M8.25 18.75a1.5 1.5 0 01-3 0m3 0a1.5 1.5 0 00-3 0m3 0h6m-9 0H3.375a1.125 1.125 0 01-1.125-1.125V14.25m17.25 4.5a1.5 1.5 0 01-3 0m3 0a1.5 1.5 0 00-3 0m3 0h1.125c.621 0 1.129-.504 1.09-1.124a17.902 17.902 0 00-3.213-9.193 2.056 2.056 0 00-1.58-.86H14.25M16.5 18.75h-2.25m0-11.177v-.958c0-.568-.422-1.048-.987-1.106a48.554 48.554 0 00-10.026 0 1.106 1.106 0 00-.987 1.106v7.635m12-6.677v6.677m0 4.5v-4.5m0 0h-12',
		clipboard:
			'M9 12h3.75M9 15h3.75M9 18h3.75m3 .75H18a2.25 2.25 0 002.25-2.25V6.108c0-1.135-.845-2.098-1.976-2.192a48.424 48.424 0 00-1.123-.08m-5.801 0c-.065.21-.1.433-.1.664 0 .414.336.75.75.75h4.5a.75.75 0 00.75-.75 2.25 2.25 0 00-.1-.664m-5.8 0A2.251 2.251 0 0113.5 2.25H15c1.012 0 1.867.668 2.15 1.586m-5.8 0c-.376.023-.75.05-1.124.08C9.095 4.01 8.25 4.973 8.25 6.108V8.25m0 0H4.875c-.621 0-1.125.504-1.125 1.125v11.25c0 .621.504 1.125 1.125 1.125h9.75c.621 0 1.125-.504 1.125-1.125V9.375c0-.621-.504-1.125-1.125-1.125H8.25z',
		users:
			'M15 19.128a9.38 9.38 0 002.625.372 9.337 9.337 0 004.121-.952 4.125 4.125 0 00-7.533-2.493M15 19.128v-.003c0-1.113-.285-2.16-.786-3.07M15 19.128v.106A12.318 12.318 0 018.624 21c-2.331 0-4.512-.645-6.374-1.766l-.001-.109a6.375 6.375 0 0111.964-3.07M12 6.375a3.375 3.375 0 11-6.75 0 3.375 3.375 0 016.75 0zm8.25 2.25a2.625 2.625 0 11-5.25 0 2.625 2.625 0 015.25 0z',
		alert:
			'M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z'
	};

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
			<button class="btn-ghost !px-3 !py-1.5 text-xs" onclick={cargar} title="Actualizar indicadores" aria-label="Actualizar indicadores">
				<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" /></svg>
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
							<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8" aria-hidden="true"><path stroke-linecap="round" stroke-linejoin="round" d={iconos[kpi.icon]} /></svg>
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
