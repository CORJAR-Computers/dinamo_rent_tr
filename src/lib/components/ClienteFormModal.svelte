<script lang="ts">
	import {
		clienteApi,
		ApiError,
		type Cliente,
		type ClienteConPii,
		type ClienteDatos,
		type BusinessLists
	} from '$lib/api';
	import { session } from '$lib/stores/session.svelte';
	import Modal from './Modal.svelte';
	import FormField from './FormField.svelte';
	import SelectConNuevo from './SelectConNuevo.svelte';
	import CopiarExistente from './CopiarExistente.svelte';
	import { geografia } from '$lib/utils/geografia';

	interface Props {
		open: boolean;
		/** Cliente en edición (null = crear nuevo). */
		editando: Cliente | null;
		lists: BusinessLists | null;
		/** Clientes ya registrados (para derivar valores geográficos usados). */
		clientes: ClienteConPii[];
		onClose: () => void;
		/** Se llama con el cliente creado/actualizado para que el padre lo use. */
		onGuardado: (r: ClienteConPii) => void;
	}

	let { open, editando, lists, clientes, onClose, onGuardado }: Props = $props();

	const sid = () => session.token ?? '';

	let form = $state<ClienteDatos>(defaultForm());
	let guardando = $state(false);
	let formError = $state('');

	let noDocInput: HTMLInputElement | undefined;

	function defaultForm(): ClienteDatos {
		return {
			tipoDoc: 'Cédula',
			noDoc: '',
			nombres: '',
			apellidos: '',
			celular: '',
			celular2: '',
			email: '',
			ciudad: '',
			estadoRegion: '',
			pais: 'Colombia',
			nacionalidad: '',
			dirResidencia: '',
			dirTemporal: '',
			hotel: '',
			habitacion: '',
			noLicencia: '',
			tipoLicencia: '',
			vencimientoLicencia: '',
			estado: 'Activo'
		};
	}

	function desdeCliente(c: Cliente): ClienteDatos {
		return {
			tipoDoc: c.tipoDoc ?? 'Cédula',
			noDoc: c.noDoc ?? '',
			nombres: c.nombres,
			apellidos: c.apellidos ?? '',
			celular: c.celular ?? '',
			celular2: c.celular2 ?? '',
			email: c.email ?? '',
			ciudad: c.ciudad ?? '',
			estadoRegion: c.estadoRegion ?? '',
			pais: c.pais ?? 'Colombia',
			nacionalidad: c.nacionalidad ?? '',
			dirResidencia: c.dirResidencia ?? '',
			dirTemporal: c.dirTemporal ?? '',
			hotel: c.hotel ?? '',
			habitacion: c.habitacion ?? '',
			noLicencia: c.noLicencia ?? '',
			tipoLicencia: c.tipoLicencia ?? '',
			vencimientoLicencia: c.vencimientoLicencia ?? '',
			estado: c.estado
		};
	}

	// Inicializar el form cada vez que se abre el modal (nuevo o edición).
	// `guardando` también se reinicia: si el usuario cerró el modal a mitad de
	// un guardado, al reabrir no debe quedar atascado en «Guardando…».
	$effect(() => {
		if (open) {
			formError = '';
			guardando = false;
			form = editando ? desdeCliente(editando) : defaultForm();
		}
	});

	// Opciones geográficas: catálogo base + valores ya usados en la BD
	const paises = $derived(geografia.paises(clientes.map((x) => x.cliente.pais)));
	const departamentos = $derived(
		geografia.departamentos(clientes.map((x) => x.cliente.estadoRegion))
	);
	const ciudades = $derived(geografia.ciudades(clientes.map((x) => x.cliente.ciudad)));

	/** Rellena el formulario con los datos de un cliente existente (duplicado). */
	function copiarDe(c: Cliente) {
		form = desdeCliente(c);
		// El documento es único: se fuerza a escribir el nuevo antes de guardar.
		form.noDoc = '';
		formError = '';
		requestAnimationFrame(() => noDocInput?.focus());
	}

	async function guardar() {
		formError = '';
		if (!form.nombres.trim()) {
			formError = 'El nombre del cliente es obligatorio.';
			return;
		}
		guardando = true;
		try {
			const r = editando
				? await clienteApi.actualizar(sid(), editando.id, form)
				: await clienteApi.crear(sid(), form);
			onGuardado(r);
		} catch (e) {
			formError = e instanceof ApiError ? e.message : 'No se pudo guardar el cliente.';
		} finally {
			guardando = false;
		}
	}
</script>

<Modal
	{open}
	title={editando ? `Editar cliente #${editando.id}` : 'Nuevo cliente'}
	subtitle="Los datos de contacto y dirección se cifran al guardarse."
	{onClose}
	width="max-w-2xl"
	dismissible={!guardando}
>
	{#snippet children()}
		{#if formError}
			<div
				class="mb-4 rounded-lg bg-peligro/10 border border-peligro/30 px-3 py-2.5 text-sm text-peligro"
				role="alert"
			>
				{formError}
			</div>
		{/if}
		{#if !editando}
			<CopiarExistente
				activo={open}
				titulo="Copiar datos de un cliente existente"
				placeholder="Buscar por nombre, documento o celular…"
				notaPaso="Escribe el documento nuevo antes de guardar."
				buscar={async (termino) =>
					(await clienteApi.listar(sid(), termino)).map((r) => ({
						id: String(r.cliente.id),
						titulo: r.cliente.nombreCompleto,
						subtitulo: `${r.cliente.noDoc ? `${r.cliente.tipoDoc}: ${r.cliente.noDoc}` : (r.cliente.tipoDoc ?? 'Cliente')}${r.cliente.ciudad ? ` · ${r.cliente.ciudad}` : ''}`,
						bloqueado: r.piiOculto,
						razonBloqueo: 'Tiene datos cifrados con clave antigua: no se pueden copiar.',
						datos: r.cliente
					}))}
				onSeleccionar={(datos) => copiarDe(datos as Cliente)}
			/>
		{/if}

		<div class="grid grid-cols-1 sm:grid-cols-2 gap-x-4">
			<div class="col-span-full mb-1">
				<h3
					class="text-xs font-bold uppercase tracking-wider text-primary mb-3 flex items-center gap-2"
				>
					<span
						class="w-4 h-4 rounded-md bg-primary/10 flex items-center justify-center text-[10px]"
						>1</span
					>
					Identificación
				</h3>
			</div>
			<FormField label="Tipo de documento">
				<select class="input" bind:value={form.tipoDoc}>
					<option value="">—</option>
					{#each lists?.tiposDoc ?? ['Cédula', 'Pasaporte', 'Cédula Extranjería', 'NIT', 'Licencia USA'] as t}
						<option value={t}>{t}</option>
					{/each}
				</select>
			</FormField>
			<FormField label="Número de documento" hint="Debe ser único.">
				<input
					class="input"
					placeholder="Ej: 1036672369"
					bind:this={noDocInput}
					bind:value={form.noDoc}
					maxlength="30"
				/>
			</FormField>
			<FormField label="Nombres" required>
				<input
					class="input"
					placeholder="Nombres del cliente"
					bind:value={form.nombres}
					maxlength="100"
				/>
			</FormField>
			<FormField label="Apellidos">
				<input
					class="input"
					placeholder="Apellidos del cliente"
					bind:value={form.apellidos}
					maxlength="100"
				/>
			</FormField>

			<div class="col-span-full mt-4 mb-1">
				<h3
					class="text-xs font-bold uppercase tracking-wider text-primary mb-3 flex items-center gap-2"
				>
					<span
						class="w-4 h-4 rounded-md bg-primary/10 flex items-center justify-center text-[10px]"
						>2</span
					>
					Contacto <span class="normal-case font-medium text-text-secondary/70">(se cifra)</span>
				</h3>
			</div>
			<FormField label="Celular">
				<input
					class="input"
					inputmode="tel"
					placeholder="Ej: 3101234567"
					bind:value={form.celular}
					maxlength="30"
				/>
			</FormField>
			<FormField label="Celular 2">
				<input
					class="input"
					inputmode="tel"
					placeholder="Opcional"
					bind:value={form.celular2}
					maxlength="30"
				/>
			</FormField>
			<FormField label="Correo electrónico">
				<input
					class="input"
					type="email"
					placeholder="cliente@correo.com"
					bind:value={form.email}
					maxlength="100"
				/>
			</FormField>

			<div class="col-span-full mt-4 mb-1">
				<h3
					class="text-xs font-bold uppercase tracking-wider text-primary mb-3 flex items-center gap-2"
				>
					<span
						class="w-4 h-4 rounded-md bg-primary/10 flex items-center justify-center text-[10px]"
						>3</span
					>
					Dirección y geografía
				</h3>
			</div>
			<FormField label="Dirección de residencia">
				<input class="input" bind:value={form.dirResidencia} maxlength="255" />
			</FormField>
			<FormField label="Dirección temporal / hotel">
				<input class="input" bind:value={form.dirTemporal} maxlength="255" />
			</FormField>
			<SelectConNuevo
				label="Ciudad"
				value={form.ciudad ?? ''}
				opciones={ciudades}
				placeholder="— Seleccionar ciudad —"
				onchange={(v) => (form.ciudad = v)}
			/>
			<SelectConNuevo
				label="Departamento / Región"
				value={form.estadoRegion ?? ''}
				opciones={departamentos}
				placeholder="— Seleccionar departamento —"
				onchange={(v) => (form.estadoRegion = v)}
			/>
			<SelectConNuevo
				label="País"
				value={form.pais ?? ''}
				opciones={paises}
				placeholder="— Seleccionar país —"
				onchange={(v) => (form.pais = v)}
			/>
			<FormField label="Nacionalidad">
				<input class="input" bind:value={form.nacionalidad} maxlength="80" />
			</FormField>
			<FormField label="Hotel">
				<input class="input" bind:value={form.hotel} maxlength="150" />
			</FormField>
			<FormField label="Habitación">
				<input class="input" bind:value={form.habitacion} maxlength="30" />
			</FormField>

			<div class="col-span-full mt-4 mb-1">
				<h3
					class="text-xs font-bold uppercase tracking-wider text-primary mb-3 flex items-center gap-2"
				>
					<span
						class="w-4 h-4 rounded-md bg-primary/10 flex items-center justify-center text-[10px]"
						>4</span
					>
					Licencia y estado
				</h3>
			</div>
			<FormField label="No. licencia" hint="Se cifra.">
				<input class="input" bind:value={form.noLicencia} maxlength="30" />
			</FormField>
			<FormField label="Tipo de licencia">
				<input class="input" placeholder="Ej: B1" bind:value={form.tipoLicencia} maxlength="50" />
			</FormField>
			<FormField label="Vencimiento de licencia">
				<input class="input" type="date" bind:value={form.vencimientoLicencia} />
			</FormField>
			<FormField label="Estado">
				<select class="input" bind:value={form.estado}>
					{#each lists?.estadosCliente ?? ['Activo', 'Inactivo', 'Lista Negra', 'VIP'] as e}
						<option value={e}>{e}</option>
					{/each}
				</select>
			</FormField>
		</div>
	{/snippet}

	{#snippet footer()}
		<button class="btn-ghost" onclick={onClose} disabled={guardando}>Cancelar</button>
		<button class="btn-primary" onclick={guardar} disabled={guardando}>
			{#if guardando}
				<svg
					class="animate-spin h-4 w-4"
					xmlns="http://www.w3.org/2000/svg"
					fill="none"
					viewBox="0 0 24 24"
					><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"
					></circle><path
						class="opacity-75"
						fill="currentColor"
						d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
					></path></svg
				>
				Guardando...
			{:else}
				{editando ? 'Guardar cambios' : 'Crear cliente'}
			{/if}
		</button>
	{/snippet}
</Modal>
