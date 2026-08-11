<script lang="ts">
	import { onMount } from 'svelte';
	import { autoApi, businessApi, ApiError, type Auto, type AutoDatos, type AlertaVencimiento, type BusinessLists } from '$lib/api';
	import { sid, session } from '$lib/stores/session.svelte';
	import { toast } from '$lib/stores/toast.svelte';
	import { formatDate } from '$lib/utils/format';
	import { guardSesion, haySesion } from '$lib/utils/guards';
	import DataTable from '$lib/components/DataTable.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import FormField from '$lib/components/FormField.svelte';
	import CopiarExistente from '$lib/components/CopiarExistente.svelte';

	// sid() viene del store (reemplaza `const sid = () => session.token ?? ''`). Ver TAREA E3.

	let autos = $state<Auto[]>([]);
	let alertas = $state<AlertaVencimiento[]>([]);
	let lists = $state<BusinessLists | null>(null);
	let loading = $state(true);
	let loadingAlertas = $state(true);

	// ¿El rol actual puede eliminar registros? (roles_con_eliminar de config.ini)
	const puedeEliminar = $derived(
		(lists?.rolesConEliminar ?? ['Administrador', 'Supervisor']).includes(session.user?.rol ?? '')
	);

	// Filtros
	let busqueda = $state('');
	let estadoFiltro = $state('');
	let searchTimer: ReturnType<typeof setTimeout> | undefined;

	// Modal
	let modalOpen = $state(false);
	let editando = $state(false);
	let guardando = $state(false);
	let form = $state<AutoDatos>(defaultForm());
	let formError = $state('');

	let placaInput: HTMLInputElement | undefined;

	// Eliminar
	let eliminarPlaca = $state<string | null>(null);
	let eliminando = $state(false);

	function today(): string {
		return new Date().toISOString().slice(0, 10);
	}

	function defaultForm(): AutoDatos {
		return {
			placa: '',
			marca: '',
			modelo: '',
			version: '',
			color: '',
			tipo: 'Automóvil',
			cilindraje: '',
			transmision: 'Automática',
			combustible: 'Gasolina',
			noMotor: '',
			noChasis: '',
			propietario: '',
			estado: 'Disponible',
			costoFijoMensual: '0',
			kilometraje: 0,
			ubicacion: '',
			tipoAdquisicion: 'Propio',
			proximoAceite: null,
			proximoFrenos: null,
			vencimientoSoat: '',
			vencimientoTecnico: '',
			vencimientoExtintor: '',
			vencimientoBateria: '',
			observaciones: '',
			fechaIngreso: today()
		};
	}

	async function cargar() {
		if (!haySesion()) return;
		loading = true;
		try {
			autos = await autoApi.listar(sid(), busqueda.trim() || undefined, estadoFiltro || undefined);
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'No se pudieron cargar los vehículos.');
		} finally {
			loading = false;
		}
	}

	async function cargarAlertas() {
		if (!haySesion()) return;
		loadingAlertas = true;
		try {
			alertas = await autoApi.alertas(sid());
		} catch {
			alertas = [];
		} finally {
			loadingAlertas = false;
		}
	}

	onMount(async () => {
		if (!guardSesion()) return;
		if (!lists) {
			try {
				lists = await businessApi.listas(sid());
			} catch {
				/* las listas son opcionales */
			}
		}
		// La carga inicial de autos la dispara el $effect de filtros (una sola vez)
		await cargarAlertas();
	});

	// Carga inicial + filtros (debounce solo al escribir)
	let primerCiclo = true;
	$effect(() => {
		const term = busqueda;
		const _est = estadoFiltro;
		if (primerCiclo) {
			primerCiclo = false;
			cargar();
			return;
		}
		clearTimeout(searchTimer);
		searchTimer = setTimeout(() => cargar(), term.trim() ? 350 : 0);
		return () => clearTimeout(searchTimer);
	});

	function desdeAuto(a: Auto): AutoDatos {
		return {
			placa: a.placa,
			marca: a.marca,
			modelo: a.modelo,
			version: a.version ?? '',
			color: a.color ?? '',
			tipo: a.tipo,
			cilindraje: a.cilindraje ?? '',
			transmision: a.transmision ?? 'Automática',
			combustible: a.combustible ?? 'Gasolina',
			noMotor: a.noMotor ?? '',
			noChasis: a.noChasis ?? '',
			propietario: a.propietario ?? '',
			estado: a.estado,
			costoFijoMensual: a.costoFijoMensual,
			kilometraje: a.kilometraje,
			ubicacion: a.ubicacion ?? '',
			tipoAdquisicion: a.tipoAdquisicion ?? '',
			proximoAceite: a.proximoAceite,
			proximoFrenos: a.proximoFrenos,
			vencimientoSoat: a.vencimientoSoat ?? '',
			vencimientoTecnico: a.vencimientoTecnico ?? '',
			vencimientoExtintor: a.vencimientoExtintor ?? '',
			vencimientoBateria: a.vencimientoBateria ?? '',
			observaciones: a.observaciones ?? '',
			fechaIngreso: a.fechaIngreso
		};
	}

	function abrirNuevo() {
		form = defaultForm();
		editando = false;
		formError = '';
		modalOpen = true;
	}

	function abrirEditar(auto: Auto) {
		form = desdeAuto(auto);
		editando = true;
		formError = '';
		modalOpen = true;
	}

	/** Rellena el formulario con los datos de un vehículo existente (duplicado). */
	function copiarDe(a: Auto) {
		form = desdeAuto(a);
		// La placa es única: se fuerza a escribir la nueva antes de guardar.
		form.placa = '';
		// La fecha de ingreso del vehículo origen casi nunca aplica a uno nuevo
		form.fechaIngreso = today();
		formError = '';
		requestAnimationFrame(() => placaInput?.focus());
	}

	async function guardar() {
		formError = '';
		if (!form.placa.trim() || !form.marca.trim() || !form.modelo.trim()) {
			formError = 'La placa, marca y modelo son obligatorios.';
			return;
		}
		guardando = true;
		try {
			if (editando) {
				await autoApi.actualizar(sid(), form.placa.trim().toUpperCase(), form);
				toast.success(`Vehículo ${form.placa.toUpperCase()} actualizado.`);
			} else {
				await autoApi.crear(sid(), form);
				toast.success(`Vehículo ${form.placa.toUpperCase()} creado.`);
			}
			modalOpen = false;
			await Promise.all([cargar(), cargarAlertas()]);
		} catch (e) {
			formError = e instanceof ApiError ? e.message : 'No se pudo guardar el vehículo.';
		} finally {
			guardando = false;
		}
	}

	async function confirmarEliminar() {
		if (!eliminarPlaca) return;
		eliminando = true;
		try {
			await autoApi.eliminar(sid(), eliminarPlaca);
			toast.success(`Vehículo ${eliminarPlaca} eliminado.`);
			eliminarPlaca = null;
			await Promise.all([cargar(), cargarAlertas()]);
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'No se pudo eliminar el vehículo.');
			eliminarPlaca = null;
		} finally {
			eliminando = false;
		}
	}

	// ── Vencimientos en la tabla ──
	function diasRestantes(fecha: string | null): number | null {
		if (!fecha) return null;
		const d = new Date(fecha + 'T00:00:00');
		const hoy = new Date();
		hoy.setHours(0, 0, 0, 0);
		return Math.round((d.getTime() - hoy.getTime()) / 86_400_000);
	}

	function venClases(dias: number): string {
		if (dias < 0) return 'text-peligro border-peligro/30 bg-peligro/10';
		if (dias <= 15) return 'text-alerta border-alerta/30 bg-alerta/10';
		return 'text-text-secondary border-border bg-alt-row/60';
	}

	// Filas para la tabla (cast a Record para DataTable genérico)
	const tablaAutos = $derived(autos as unknown as Record<string, unknown>[]);

	const columnas = [
		{ key: 'placa', header: 'Placa' },
		{ key: 'marca', header: 'Vehículo' },
		{ key: 'tipo', header: 'Tipo' },
		{ key: 'estado', header: 'Estado' },
		{ key: 'kilometraje', header: 'Km', align: 'right' as const },
		{ key: 'vencimientos', header: 'Vencimientos' },
		{ key: 'acciones', header: '', align: 'right' as const }
	];
</script>

<svelte:head>
	<title>Flota de Autos — Dinamo Rent ERP</title>
</svelte:head>

<div class="space-y-5">
	<!-- Encabezado -->
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div>
			<h2 class="text-2xl font-bold text-text-primary">Flota de Autos</h2>
			<p class="text-sm text-text-secondary mt-0.5">
				{autos.length} vehículo{autos.length === 1 ? '' : 's'} · alertas de vencimientos en tiempo real
			</p>
		</div>
		<button class="btn-primary" onclick={abrirNuevo}>
			<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" /></svg>
			Nuevo Auto
		</button>
	</div>

	<!-- Filtros -->
	<div class="flex flex-wrap items-center gap-3">
		<div class="relative grow max-w-sm">
			<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-text-secondary/60 pointer-events-none" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" /></svg>
			<input
				class="input pl-9"
				type="search"
				placeholder="Buscar por placa, marca o modelo..."
				bind:value={busqueda}
			/>
		</div>
		<select class="input w-auto" bind:value={estadoFiltro} aria-label="Filtrar por estado">
			<option value="">Todos los estados</option>
			{#each (lists?.estadosAuto ?? ['Disponible', 'Rentado', 'Mantenimiento', 'Vendido', 'Baja']) as est}
				<option value={est}>{est}</option>
			{/each}
		</select>
	</div>

	<!-- Alertas de vencimientos -->
	{#if !loadingAlertas && alertas.length > 0}
		<div class="rounded-xl border border-alerta/25 bg-alerta/5 px-4 py-3">
			<div class="flex items-center gap-2 mb-2">
				<span class="w-2 h-2 rounded-full bg-alerta animate-pulse"></span>
				<h3 class="text-xs font-bold uppercase tracking-wider text-alerta">
					{alertas.length} vencimiento{alertas.length === 1 ? '' : 's'} próximo{alertas.length === 1 ? '' : 's'}
				</h3>
			</div>
			<div class="flex flex-wrap gap-2">
				{#each alertas.slice(0, 8) as a}
					<span
						class="inline-flex items-center gap-1.5 rounded-lg border px-2.5 py-1.5 text-[11px] font-semibold cursor-default transition-transform hover:scale-[1.03] {a.critica ? 'border-peligro/30 bg-peligro/10 text-peligro' : 'border-alerta/30 bg-surface text-alerta'}"
						title={a.detalle}
					>
						<span class="w-1.5 h-1.5 rounded-full bg-current"></span>
						{a.placa} · {a.tipo} · {a.detalle}
					</span>
				{/each}
			</div>
		</div>
	{/if}

	<!-- Tabla -->
	{#if loading}
		<div class="card flex items-center justify-center py-16">
			<div class="flex flex-col items-center gap-3">
				<svg class="animate-spin h-8 w-8 text-primary" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				<p class="text-sm text-text-secondary">Cargando flota...</p>
			</div>
		</div>
	{:else}
		<DataTable
			columns={columnas}
			items={tablaAutos}
			emptyTitle="No hay vehículos"
			emptyDescription="Crea el primer vehículo de la flota con el botón «Nuevo Auto»."
			emptyIcon="car"
		>
			{#snippet children(col, item)}
				{@const a = item as unknown as Auto}
				{#if col.key === 'placa'}
					<span class="font-bold text-primary">{a.placa}</span>
				{:else if col.key === 'marca'}
					<div>
						<p class="font-semibold text-text-primary">{a.marca}</p>
						<p class="text-xs text-text-secondary">{a.modelo}{a.version ? ` · ${a.version}` : ''}{a.color ? ` · ${a.color}` : ''}</p>
					</div>
				{:else if col.key === 'estado'}
					<StatusBadge estado={a.estado} />
				{:else if col.key === 'kilometraje'}
					<span class="tabular-nums text-text-secondary">{Math.round(a.kilometraje).toLocaleString('es-CO')} km</span>
				{:else if col.key === 'vencimientos'}
					<div class="flex flex-wrap gap-1.5">
						{#each [
							{ tipo: 'SOAT', fecha: a.vencimientoSoat },
							{ tipo: 'Téc.', fecha: a.vencimientoTecnico },
							{ tipo: 'Ext.', fecha: a.vencimientoExtintor }
						] as v}
							{@const d = diasRestantes(v.fecha)}
							{#if d !== null}
								<span
									class="inline-flex items-center gap-1 rounded-md border px-1.5 py-0.5 text-[10px] font-semibold {venClases(d)}"
									title={`${v.tipo}: ${formatDate(v.fecha)}`}
								>{v.tipo} · {d < 0 ? `${-d}d venc` : `${d}d`}</span>
							{/if}
						{/each}
					</div>
				{:else if col.key === 'acciones'}
					<div class="flex items-center justify-end gap-1">
						<a
							href={`/autos/${a.placa}`}
							class="p-2 rounded-lg text-text-secondary hover:text-primary hover:bg-primary/10 transition-colors inline-flex"
							title="Historial: rentas y multas del vehículo"
							aria-label={`Historial de ${a.placa}`}
						>
							<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M12 6v6h4.5m4.5 0a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>
						</a>
						<button
							class="p-2 rounded-lg text-text-secondary hover:text-primary hover:bg-primary/10 transition-colors"
							title="Editar"
							onclick={() => abrirEditar(a)}
						>
							<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M16.862 4.487l1.687-1.688a1.875 1.875 0 112.652 2.652L6.832 19.82a4.5 4.5 0 01-1.897 1.13l-2.685.8.8-2.685a4.5 4.5 0 011.13-1.897L16.862 4.487zm0 0L19.5 7.125" /></svg>
						</button>
						{#if puedeEliminar}
							<button
								class="p-2 rounded-lg text-text-secondary hover:text-peligro hover:bg-peligro/10 transition-colors"
								title="Eliminar"
								onclick={() => (eliminarPlaca = a.placa)}
							>
								<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M14.74 9l-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 01-2.244 2.077H8.084a2.25 2.25 0 01-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 00-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 013.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 00-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 00-7.5 0" /></svg>
							</button>
						{/if}
					</div>
				{:else}
					<span>{String(item[col.key] ?? '—')}</span>
				{/if}
			{/snippet}
		</DataTable>
	{/if}
</div>

<!-- Modal crear/editar -->
<Modal
	open={modalOpen}
	title={editando ? `Editar vehículo ${form.placa.toUpperCase()}` : 'Nuevo vehículo'}
	subtitle={editando ? 'Modifica los datos y guarda los cambios.' : 'Registra un vehículo en la flota.'}
	onClose={() => (modalOpen = false)}
	width="max-w-2xl"
	dismissible={!guardando}
>
	{#snippet children()}
		{#if formError}
			<div class="mb-4 rounded-lg bg-peligro/10 border border-peligro/30 px-3 py-2.5 text-sm text-peligro" role="alert">{formError}</div>
		{/if}

		{#if !editando}
			<CopiarExistente
				activo={modalOpen}
				titulo="Copiar datos de un vehículo existente"
				placeholder="Buscar por placa, marca o modelo…"
				notaPaso="Escribe la placa nueva antes de guardar."
				buscar={async (termino) =>
					(await autoApi.listar(sid(), termino)).map((a) => ({
						id: a.placa,
						titulo: a.placa,
						subtitulo: `${a.marca}${a.modelo ? ` · ${a.modelo}` : ''}${a.version ? ` · ${a.version}` : ''}`,
						datos: a
					}))}
				onSeleccionar={(datos) => copiarDe(datos as Auto)}
			/>
		{/if}

		<div class="grid grid-cols-1 sm:grid-cols-2 gap-x-4">
			<!-- Identificación -->
			<div class="col-span-full mb-1">
				<h3 class="text-xs font-bold uppercase tracking-wider text-primary mb-3 flex items-center gap-2">
					<span class="w-4 h-4 rounded-md bg-primary/10 flex items-center justify-center text-[10px]">1</span>
					Identificación
				</h3>
			</div>
			<FormField label="Placa" required hint="Sin espacios. Se guarda en mayúsculas." error={formError && !form.placa ? 'Obligatoria' : ''}>
				<input class="input uppercase" placeholder="ABC123" bind:this={placaInput} bind:value={form.placa} maxlength="20" disabled={editando} />
			</FormField>
			<FormField label="Fecha de ingreso" required>
				<input class="input" type="date" bind:value={form.fechaIngreso} />
			</FormField>
			<FormField label="Marca" required>
				<input class="input" placeholder="Ej: Toyota" bind:value={form.marca} maxlength="80" />
			</FormField>
			<FormField label="Modelo" required>
				<input class="input" placeholder="Ej: Corolla" bind:value={form.modelo} maxlength="80" />
			</FormField>
			<FormField label="Versión">
				<input class="input" placeholder="Ej: XEI 1.8" bind:value={form.version} maxlength="80" />
			</FormField>
			<FormField label="Color">
				<input class="input" placeholder="Ej: Blanco" bind:value={form.color} maxlength="50" />
			</FormField>
			<FormField label="Tipo">
				<select class="input" bind:value={form.tipo}>
					{#each (lists?.tiposAuto ?? ['Automóvil', 'Camioneta', 'Van', 'Lujo', 'Moto']) as t}
						<option value={t}>{t}</option>
					{/each}
				</select>
			</FormField>
			<FormField label="Cilindraje">
				<input class="input" placeholder="Ej: 1800 cc" bind:value={form.cilindraje} maxlength="30" />
			</FormField>
			<FormField label="Transmisión">
				<select class="input" bind:value={form.transmision}>
					{#each (lists?.tiposTransmision ?? ['Automática', 'Mecánica']) as t}
						<option value={t}>{t}</option>
					{/each}
				</select>
			</FormField>
			<FormField label="Combustible">
				<select class="input" bind:value={form.combustible}>
					{#each (lists?.tiposCombustible ?? ['Gasolina', 'Diesel', 'Híbrido', 'Eléctrico', 'Gas']) as t}
						<option value={t}>{t}</option>
					{/each}
				</select>
			</FormField>

			<!-- Adquisición y estado -->
			<div class="col-span-full mt-4 mb-1">
				<h3 class="text-xs font-bold uppercase tracking-wider text-primary mb-3 flex items-center gap-2">
					<span class="w-4 h-4 rounded-md bg-primary/10 flex items-center justify-center text-[10px]">2</span>
					Estado y adquisición
				</h3>
			</div>
			<FormField label="Estado">
				<select class="input" bind:value={form.estado}>
					{#each (lists?.estadosAuto ?? ['Disponible', 'Rentado', 'Mantenimiento', 'Vendido', 'Baja']) as e}
						<option value={e}>{e}</option>
					{/each}
				</select>
			</FormField>
			<FormField label="Tipo de adquisición">
				<select class="input" bind:value={form.tipoAdquisicion}>
					<option value="">—</option>
					{#each (lists?.tiposAdquisicion ?? ['Propio', 'Leasing', 'Subarrendado']) as t}
						<option value={t}>{t}</option>
					{/each}
				</select>
			</FormField>
			<FormField label="Costo fijo mensual (COP)">
				<input class="input" inputmode="decimal" placeholder="0" bind:value={form.costoFijoMensual} />
			</FormField>
			<FormField label="Kilometraje actual (km)">
				<input class="input" type="number" min="0" step="1" bind:value={form.kilometraje} />
			</FormField>
			<FormField label="Ubicación">
				<input class="input" placeholder="Ej: Parqueadero principal" bind:value={form.ubicacion} maxlength="150" />
			</FormField>
			<FormField label="Propietario">
				<input class="input" placeholder="Nombre del propietario" bind:value={form.propietario} maxlength="150" />
			</FormField>
			<FormField label="No. motor">
				<input class="input" bind:value={form.noMotor} maxlength="80" />
			</FormField>
			<FormField label="No. chasis">
				<input class="input" bind:value={form.noChasis} maxlength="80" />
			</FormField>

			<!-- Vencimientos y mantenimiento -->
			<div class="col-span-full mt-4 mb-1">
				<h3 class="text-xs font-bold uppercase tracking-wider text-primary mb-3 flex items-center gap-2">
					<span class="w-4 h-4 rounded-md bg-primary/10 flex items-center justify-center text-[10px]">3</span>
					Vencimientos y mantenimiento
				</h3>
			</div>
			<FormField label="Vencimiento SOAT">
				<input class="input" type="date" bind:value={form.vencimientoSoat} />
			</FormField>
			<FormField label="Vencimiento tecno-mecánica">
				<input class="input" type="date" bind:value={form.vencimientoTecnico} />
			</FormField>
			<FormField label="Vencimiento extintor">
				<input class="input" type="date" bind:value={form.vencimientoExtintor} />
			</FormField>
			<FormField label="Vencimiento batería">
				<input class="input" type="date" bind:value={form.vencimientoBateria} />
			</FormField>
			<FormField label="Próximo cambio de aceite (km)">
				<input class="input" type="number" min="0" placeholder="Ej: 20000" bind:value={form.proximoAceite} />
			</FormField>
			<FormField label="Próximo cambio de frenos (km)">
				<input class="input" type="number" min="0" placeholder="Ej: 30000" bind:value={form.proximoFrenos} />
			</FormField>
			<FormField label="Observaciones" hint="Máx. 2000 caracteres.">
				<textarea class="input min-h-[80px] resize-y" bind:value={form.observaciones} maxlength="2000"></textarea>
			</FormField>
		</div>
	{/snippet}

	{#snippet footer()}
		<button class="btn-ghost" onclick={() => (modalOpen = false)} disabled={guardando}>Cancelar</button>
		<button class="btn-primary" onclick={guardar} disabled={guardando}>
			{#if guardando}
				<svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				Guardando...
			{:else}
				{editando ? 'Guardar cambios' : 'Crear vehículo'}
			{/if}
		</button>
	{/snippet}
</Modal>

<!-- Confirmación de eliminación -->
<ConfirmDialog
	open={eliminarPlaca !== null}
	title="Eliminar vehículo"
	message={`¿Seguro que deseas eliminar el vehículo ${eliminarPlaca}? Esta acción no se puede deshacer.`}
	confirmLabel="Eliminar"
	loading={eliminando}
	onConfirm={confirmarEliminar}
	onCancel={() => (eliminarPlaca = null)}
/>
