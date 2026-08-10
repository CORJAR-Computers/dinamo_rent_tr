<script lang="ts">
	import { onMount } from 'svelte';
	import {
		rentaApi,
		reservaApi,
		type Renta,
		type Reserva
	} from '$lib/api';
	import { sid } from '$lib/stores/session.svelte';
	import { toast } from '$lib/stores/toast.svelte';
	import { guardSesion } from '$lib/utils/guards';
	import {
		celdasDelMes,
		diasSemanaCorto,
		detectarSolapamientos,
		nombreMes,
		rangoCubreDia
	} from '$lib/utils/calendario';
	import Modal from '$lib/components/Modal.svelte';
	import Icon from '$lib/components/Icon.svelte';

	// sid() viene del store (reemplaza `const sid = () => session.token ?? ''`). Ver TAREA E3.

	let rentas = $state<Renta[]>([]);
	let reservas = $state<Reserva[]>([]);
	let loading = $state(true);
	let mesActual = $state(new Date());

	// Dia seleccionado (para el panel de detalle)
	let diaSeleccionado = $state<string | null>(null);

	// Items combinados con placa (para solapamientos)
	const itemsConPlaca = $derived([
		...rentas
			.filter((r) => r.placa && r.estado !== 'Cancelada')
			.map((r) => ({
				id: r.id,
				placa: r.placa as string,
				inicio: r.fechaRecogida,
				fin: r.fechaRetorno
			})),
		...reservas
			.filter((r) => r.placaAsignada && (r.estado === 'Confirmada' || r.estado === 'Pendiente'))
			.map((r) => ({
				id: r.id,
				placa: r.placaAsignada as string,
				inicio: r.fechaRecogida,
				fin: r.fechaRetorno
			}))
	]);

	const solapamientos = $derived(detectarSolapamientos(itemsConPlaca));

	// Un id de renta o reserva está en conflicto
	const idsEnConflicto = $derived(
		new Set(solapamientos.flatMap((s) => [s.a, s.b]))
	);

	// Rango visible actual (mes ± 6 meses). Se usa para recortar en cliente
	// las rentas/reservas demasiado antiguas o futuras, ya que la API todavía
	// no soporta filtro por fecha.
	// TODO: añadir parámetros `fecha_inicio`/`fecha_fin` al comando `listar_rentas`
	// y `listar_reservas` en Rust (src-tauri/src/commands/renta.rs y reserva.rs)
	// y propagarlos en `rentaApi.listar` / `reservaApi.listar` de src/lib/api.ts
	// para filtrar en backend y no cargar TODAS las rentas históricas en cada
	// navegación de mes.
	function limiteInferiorIso(): string {
		const base = mesActual;
		const d = new Date(base.getFullYear(), base.getMonth() - 6, 1);
		return d.toISOString().slice(0, 10);
	}

	async function cargar() {
		if (!guardSesion()) return;
		loading = true;
		try {
			// TODO: pasar `fecha_inicio`/`fecha_fin` del mes visible cuando el
			// backend los soporte. Por ahora la API carga todo y recortamos en
			// cliente al rango visible (mes actual ± 6 meses) para acotar memoria.
			const limiteInf = limiteInferiorIso();
			const [r, rs] = await Promise.all([
				rentaApi.listar(sid()),
				reservaApi.listar(sid())
			]);
			rentas = r.filter((x) => x.fechaRetorno >= limiteInf);
			reservas = rs.filter((x) => x.fechaRetorno >= limiteInf);
		} catch {
			toast.error('No se pudieron cargar los datos del calendario.');
		} finally {
			loading = false;
		}
	}

	onMount(cargar);

	// Refresca al navegar meses: la API no filtra por fecha todavía, así que
	// recargamos todo y recortamos en cliente al nuevo rango visible.
	let primerCiclo = true;
	$effect(() => {
		const _mes = mesActual; // suscripción
		if (primerCiclo) {
			primerCiclo = false;
			return; // onMount ya disparó la primera carga
		}
		cargar();
	});

	function mesAnterior() {
		mesActual = new Date(mesActual.getFullYear(), mesActual.getMonth() - 1, 1);
	}

	function mesSiguiente() {
		mesActual = new Date(mesActual.getFullYear(), mesActual.getMonth() + 1, 1);
	}

	function irAHoy() {
		mesActual = new Date();
	}

	const semanas = $derived(
		celdasDelMes(mesActual.getFullYear(), mesActual.getMonth())
	);

	const tituloMes = $derived(
		nombreMes(mesActual.getFullYear(), mesActual.getMonth())
	);

	// Items de un día: rentas (activas y cerradas en rango) + reservas (no canceladas)
	function itemsDelDia(dia: string): { rentas: Renta[]; reservas: Reserva[] } {
		const r = rentas.filter((x) => rangoCubreDia(x.fechaRecogida, x.fechaRetorno, dia) && x.estado !== 'Cancelada');
		const rs = reservas.filter((x) => rangoCubreDia(x.fechaRecogida, x.fechaRetorno, dia) && x.estado !== 'Cancelada');
		return { rentas: r, reservas: rs };
	}

	function totalDia(dia: string): number {
		const { rentas: r, reservas: rs } = itemsDelDia(dia);
		return r.length + rs.length;
	}

	// Chip de color según tipo/estado
	function chipClases(tipo: 'renta' | 'reserva', conflicto: boolean): string {
		if (conflicto) return 'bg-peligro/15 text-peligro border-peligro/40';
		return tipo === 'renta'
			? 'bg-primary/10 text-primary border-primary/25'
			: 'bg-alerta/10 text-alerta border-alerta/25';
	}

	// ── Panel de detalle del día ──
	const detalleDia = $derived.by(() => {
		if (!diaSeleccionado) return null;
		const { rentas: r, reservas: rs } = itemsDelDia(diaSeleccionado);
		const items = [
			...r.map((x) => ({
				tipo: 'renta' as const,
				id: x.id,
				nombre: x.nombreCliente,
				placa: x.placa ?? 'Sin placa',
				vehiculo: x.vehiculo,
				fechaRecogida: x.fechaRecogida,
				fechaRetorno: x.fechaRetorno,
				total: x.total,
				saldo: x.saldoPendiente,
				estado: x.estado,
				conflicto: idsEnConflicto.has(x.id)
			})),
			...rs.map((x) => ({
				tipo: 'reserva' as const,
				id: x.id,
				nombre: x.nombreCliente,
				placa: x.placaAsignada ?? 'Sin asignar',
				vehiculo: x.categoriaVehiculo ?? '—',
				fechaRecogida: x.fechaRecogida,
				fechaRetorno: x.fechaRetorno,
				total: x.total,
				saldo: x.abono,
				estado: x.estado,
				conflicto: idsEnConflicto.has(x.id)
			}))
		].sort((a, b) => a.fechaRecogida.localeCompare(b.fechaRecogida));
		return { dia: diaSeleccionado, items };
	});

	function fmtFecha(iso: string): string {
		const d = new Date(iso + 'T00:00:00');
		return d.toLocaleDateString('es-CO', { day: 'numeric', month: 'short' });
	}
</script>

<svelte:head>
	<title>Calendario — Dinamo Rent ERP</title>
</svelte:head>

<div class="space-y-5">
	<!-- Encabezado -->
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div>
			<h2 class="text-2xl font-bold text-text-primary">Calendario de Rentas</h2>
			<p class="text-sm text-text-secondary mt-0.5">
				Ocupación mensual por vehículo · {solapamientos.length} conflicto{solapamientos.length === 1 ? '' : 's'} de fechas detectado{solapamientos.length === 1 ? '' : 's'}
			</p>
		</div>
		<div class="flex items-center gap-2">
			<button class="btn-ghost" onclick={mesAnterior} aria-label="Mes anterior">
				<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M15.75 19.5L8.25 12l7.5-7.5" /></svg>
			</button>
			<button class="btn-ghost" onclick={irAHoy}>Hoy</button>
			<button class="btn-ghost" onclick={mesSiguiente} aria-label="Mes siguiente">
				<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M8.25 4.5l7.5 7.5-7.5 7.5" /></svg>
			</button>
			<span class="text-lg font-black text-text-primary capitalize min-w-[160px] text-center">{tituloMes}</span>
		</div>
	</div>

	<!-- Leyenda -->
	<div class="flex flex-wrap items-center gap-4 text-xs text-text-secondary">
		<span class="inline-flex items-center gap-1.5">
			<span class="w-3 h-3 rounded bg-primary/70"></span> Renta
		</span>
		<span class="inline-flex items-center gap-1.5">
			<span class="w-3 h-3 rounded bg-alerta/70"></span> Reserva
		</span>
		<span class="inline-flex items-center gap-1.5">
			<span class="w-3 h-3 rounded bg-peligro/70"></span> Solapamiento
		</span>
	</div>

	{#if loading}
		<div class="card flex items-center justify-center py-16">
			<div class="flex flex-col items-center gap-3">
				<svg class="animate-spin h-8 w-8 text-primary" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				<p class="text-sm text-text-secondary">Cargando calendario...</p>
			</div>
		</div>
	{:else}
		<!-- Grid -->
		<div class="card overflow-hidden">
			<div class="grid grid-cols-7 border-b border-border">
				{#each diasSemanaCorto() as d, i}
					<div class="px-2 py-2 text-center text-[11px] font-bold uppercase tracking-wider text-text-secondary {i >= 5 ? 'text-alerta/80' : ''}">
						{d}
					</div>
				{/each}
			</div>
			{#each semanas as semana}
				<div class="grid grid-cols-7">
					{#each semana as celda}
						{@const nDia = totalDia(celda.dia)}
						{@const esDiaActual = celda.dia === diaSeleccionado}
						<button
							class="min-h-[96px] p-1.5 border-b border-r border-border/60 text-left align-top transition-colors
								{celda.enMes ? 'bg-surface' : 'bg-alt-row/30'}
								hover:bg-primary/5
								{esDiaActual ? 'ring-2 ring-primary ring-inset' : ''}"
							onclick={() => (diaSeleccionado = celda.dia)}
							aria-label={`Día ${celda.dia}${nDia > 0 ? `, ${nDia} evento${nDia === 1 ? '' : 's'}` : ''}`}
						>
							<div class="flex items-center justify-between">
								<span class="text-xs font-semibold tabular-nums {celda.enMes ? (celda.hoy ? 'text-primary' : 'text-text-primary') : 'text-text-secondary/50'}">
									{Number(celda.dia.slice(8, 10))}
								</span>
								{#if nDia > 0}
									<span class="inline-flex items-center justify-center rounded-full bg-primary/10 text-primary text-[10px] font-bold tabular-nums px-1.5 py-0.5">
										{nDia}
									</span>
								{/if}
							</div>
							{#if celda.enMes}
								{@const { rentas: r, reservas: rs } = itemsDelDia(celda.dia)}
								<div class="mt-1 space-y-1">
									{#each r.slice(0, 3) as x}
										<span class="block truncate rounded border px-1 py-0.5 text-[10px] font-semibold leading-tight {chipClases('renta', idsEnConflicto.has(x.id))}" title={`Renta #${x.id} · ${x.nombreCliente}`}>
											R{x.id} · {x.nombreCliente.split(' ')[0]}
										</span>
									{/each}
									{#each rs.slice(0, 3) as x}
										<span class="block truncate rounded border px-1 py-0.5 text-[10px] font-semibold leading-tight {chipClases('reserva', idsEnConflicto.has(x.id))}" title={`Reserva #${x.id} · ${x.nombreCliente}`}>
											Rv{x.id} · {x.nombreCliente.split(' ')[0]}
										</span>
									{/each}
									{#if r.length + rs.length > 3}
										<span class="block text-[10px] text-text-secondary font-medium">+{r.length + rs.length - 3} más</span>
									{/if}
								</div>
							{/if}
						</button>
					{/each}
				</div>
			{/each}
		</div>
	{/if}
</div>

<!-- Panel de detalle del día -->
<Modal
	open={detalleDia !== null}
	title={detalleDia ? `Ocupación del ${new Date(detalleDia.dia + 'T00:00:00').toLocaleDateString('es-CO', { weekday: 'long', day: 'numeric', month: 'long' })}` : ''}
	subtitle="Rentas y reservas que cubren este día. En rojo: vehículos con fechas solapadas."
	onClose={() => (diaSeleccionado = null)}
	width="max-w-2xl"
>
	{#snippet children()}
		{#if detalleDia}
			{#if detalleDia.items.length === 0}
				<p class="text-sm text-text-secondary py-6 text-center">Sin rentas ni reservas este día. <Icon name="sparkles" class="w-4 h-4 inline-block align-[-2px] text-primary" /></p>
			{:else}
				<div class="space-y-2">
					{#each detalleDia.items as it}
						<div class="rounded-xl border p-3.5 flex items-start justify-between gap-3 transition-colors
							{it.conflicto ? 'border-peligro/40 bg-peligro/5' : 'border-border bg-alt-row/40'}">
							<div class="min-w-0">
								<p class="font-semibold text-text-primary flex items-center gap-2">
									<span class="inline-flex items-center rounded px-1.5 py-0.5 text-[10px] font-bold uppercase tracking-wide {it.tipo === 'renta' ? 'bg-primary/10 text-primary' : 'bg-alerta/10 text-alerta'}">
										{it.tipo === 'renta' ? `Renta #${it.id}` : `Reserva #${it.id}`}
									</span>
									<span class="truncate">{it.nombre}</span>
								</p>
								<p class="text-xs text-text-secondary mt-1 truncate">
									<span class="font-mono">{it.placa}</span> · {it.vehiculo} · {fmtFecha(it.fechaRecogida)} → {fmtFecha(it.fechaRetorno)}
								</p>
								{#if it.conflicto}
									<p class="text-xs font-semibold text-peligro mt-1"><Icon name="alert" class="w-3.5 h-3.5 inline-block align-[-2px] mr-1" />Vehículo con fechas solapadas con otra renta/reserva</p>
								{/if}
							</div>
							<div class="text-right shrink-0">
								<p class="font-bold text-text-primary tabular-nums">{Number(it.total) > 0 ? new Intl.NumberFormat('es-CO', { style: 'currency', currency: 'COP', maximumFractionDigits: 0 }).format(Number(it.total)) : '—'}</p>
								<span class="inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[10px] font-semibold mt-1
									{it.estado === 'Cancelada' ? 'border-peligro/30 text-peligro' :
									it.estado === 'Cerrada' ? 'border-exito/30 text-exito' :
									'border-primary/30 text-primary'}">
									{it.estado}
								</span>
							</div>
						</div>
					{/each}
				</div>
			{/if}
		{/if}
	{/snippet}
</Modal>
