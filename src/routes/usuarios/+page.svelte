<script lang="ts">
	import { onMount } from 'svelte';
	import { usuarioApi, businessApi, ApiError, type Usuario, type UsuarioDatos, type UsuarioDatosActualizar, type BusinessLists } from '$lib/api';
	import { session } from '$lib/stores/session.svelte';
	import { toast } from '$lib/stores/toast.svelte';
	import { formatDateTime } from '$lib/utils/format';
	import { guardRole, guardSesion, haySesion, tieneRol } from '$lib/utils/guards';
	import DataTable from '$lib/components/DataTable.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
	import FormField from '$lib/components/FormField.svelte';

	const sid = () => session.token ?? '';
	/** Máximo de intentos fallidos que bloquea una cuenta (default core/security.rs, se puede ajustar con max_login_attempts en config.ini) */
	let maxIntentos = $state(5);

	let usuarios = $state<Usuario[]>([]);
	let lists = $state<BusinessLists | null>(null);
	let loading = $state(true);

	// Búsqueda
	let busqueda = $state('');
	let searchTimer: ReturnType<typeof setTimeout> | undefined;

	// Modal crear/editar
	let modalOpen = $state(false);
	let editando = $state(false);
	let editandoId = $state<number | null>(null);
	let guardando = $state(false);
	let form = $state<UsuarioDatos>(defaultForm());
	let confirmacion = $state('');
	let formError = $state('');

	// Modal forzar cambio de contraseña
	let forzarUsuario = $state<Usuario | null>(null);
	let nuevaPassword = $state('');
	let confirmPassword = $state('');
	let forcando = $state(false);
	let forzarError = $state('');

	// Eliminar
	let eliminarId = $state<number | null>(null);
	let eliminando = $state(false);

	function defaultForm(): UsuarioDatos {
		return {
			username: '',
			password: '',
			nombre: '',
			rol: 'Operador',
			email: '',
			activo: true,
			debeCambiarPassword: true
		};
	}

	async function cargar() {
		// Guard de sesión + rol: nunca consultar sin sesión ni si el usuario
		// no es administrador (cubre también el debounce durante una redirección).
		if (!haySesion()) return;
		if (!tieneRol(['Administrador'])) return;
		loading = true;
		try {
			usuarios = await usuarioApi.listar(sid(), busqueda.trim() || undefined);
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'No se pudieron cargar los usuarios.');
		} finally {
			loading = false;
		}
	}

	onMount(async () => {
		// Guard de sesión + rol: solo administradores gestionan usuarios.
		// El menú ya oculta la ruta, pero esto protege el acceso directo por URL.
		if (!guardSesion()) return;
		if (!guardRole(['Administrador'], '/dashboard')) return;
		if (!lists) {
			try {
				lists = await businessApi.listas(sid());
			} catch {
				/* las listas son opcionales */
			}
		}
		// La carga inicial la dispara el $effect de filtros (una sola vez)
	});

	// Carga inicial + búsqueda con debounce
	let primerCiclo = true;
	$effect(() => {
		const term = busqueda;
		if (primerCiclo) {
			primerCiclo = false;
			cargar();
			return;
		}
		clearTimeout(searchTimer);
		searchTimer = setTimeout(() => cargar(), term.trim() ? 350 : 0);
		return () => clearTimeout(searchTimer);
	});

	function abrirNuevo() {
		form = defaultForm();
		editando = false;
		editandoId = null;
		confirmacion = '';
		formError = '';
		modalOpen = true;
	}

	function abrirEditar(u: Usuario) {
		form = {
			username: u.username,
			password: '',
			nombre: u.nombre ?? '',
			rol: u.rol ?? 'Operador',
			email: u.email ?? '',
			activo: u.activo,
			debeCambiarPassword: u.debeCambiarPassword
		};
		editando = true;
		editandoId = u.id;
		formError = '';
		modalOpen = true;
	}

	async function guardar() {
		formError = '';
		if (!form.username.trim() || !form.nombre.trim() || !form.rol) {
			formError = 'El nombre de usuario, nombre y rol son obligatorios.';
			return;
		}
		if (!editando) {
			if (form.password.length < 8) {
				formError = 'La contraseña debe tener al menos 8 caracteres.';
				return;
			}
			if (form.password !== confirmacion) {
				formError = 'Las contraseñas no coinciden.';
				return;
			}
		}
		guardando = true;
		try {
			if (editando && editandoId !== null) {
				const datos: UsuarioDatosActualizar = {
					nombre: form.nombre.trim(),
					rol: form.rol,
					email: (form.email ?? '').trim() || undefined,
					activo: form.activo
				};
				await usuarioApi.actualizar(sid(), editandoId, datos);
				toast.success(`Usuario ${form.username} actualizado.`);
			} else {
				await usuarioApi.crear(sid(), form);
				toast.success(`Usuario ${form.username} creado.`);
			}
			modalOpen = false;
			await cargar();
		} catch (e) {
			formError = e instanceof ApiError ? e.message : 'No se pudo guardar el usuario.';
		} finally {
			guardando = false;
		}
	}

	function abrirForzar(u: Usuario) {
		forzarUsuario = u;
		nuevaPassword = '';
		confirmPassword = '';
		forzarError = '';
	}

	async function confirmarForzar() {
		if (!forzarUsuario) return;
		forzarError = '';
		if (nuevaPassword.length < 8) {
			forzarError = 'La contraseña debe tener al menos 8 caracteres.';
			return;
		}
		if (nuevaPassword !== confirmPassword) {
			forzarError = 'Las contraseñas no coinciden.';
			return;
		}
		forcando = true;
		try {
			await usuarioApi.forzarCambioPassword(sid(), forzarUsuario.id, nuevaPassword);
			toast.success(`Se reinició la contraseña de ${forzarUsuario.username}. Deberá cambiarla en el próximo ingreso.`);
			forzarUsuario = null;
			await cargar();
		} catch (e) {
			forzarError = e instanceof ApiError ? e.message : 'No se pudo reiniciar la contraseña.';
		} finally {
			forcando = false;
		}
	}

	async function confirmarEliminar() {
		if (eliminarId === null) return;
		eliminando = true;
		try {
			await usuarioApi.eliminar(sid(), eliminarId);
			toast.success('Usuario eliminado.');
			eliminarId = null;
			await cargar();
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'No se pudo eliminar el usuario.');
		} finally {
			eliminando = false;
		}
	}

	async function desbloquear(u: Usuario) {
		try {
			const estabaBloqueada = await usuarioApi.desbloquear(sid(), u.username);
			toast.success(
				estabaBloqueada
					? `Cuenta ${u.username} desbloqueada.`
					: `La cuenta ${u.username} no estaba bloqueada.`
			);
			await cargar();
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'No se pudo desbloquear la cuenta.');
		}
	}

	// ── Helpers de presentación ──
	function iniciales(u: Usuario): string {
		const n = (u.nombre || u.username || '?').trim();
		const parts = n.split(/\s+/).filter(Boolean);
		return (parts[0]?.[0] ?? '') + (parts[1]?.[0] ?? '');
	}

	function fmtDbTs(v: string | null): string {
		if (!v) return '—';
		const iso = v.includes('T') ? v : v.replace(' ', 'T');
		return formatDateTime(iso);
	}

	function rolClases(rol: string | null): string {
		if (rol === 'Administrador') return 'bg-primary/10 text-primary border-primary/25';
		if (rol === 'Supervisor') return 'bg-violet-500/10 text-violet-600 border-violet-500/25 dark:text-violet-400';
		return 'bg-text-secondary/10 text-text-secondary border-text-secondary/25';
	}

	const esCuentaPropia = (u: Usuario) => u.username === session.user?.username;

	const tablaUsuarios = $derived(usuarios as unknown as Record<string, unknown>[]);

	const columnas = [
		{ key: 'usuario', header: 'Usuario' },
		{ key: 'rol', header: 'Rol' },
		{ key: 'email', header: 'Email' },
		{ key: 'estado', header: 'Estado' },
		{ key: 'cuenta', header: 'Cuenta' },
		{ key: 'acceso', header: 'Último acceso' },
		{ key: 'acciones', header: '', align: 'right' as const }
	];
</script>

<svelte:head>
	<title>Usuarios — Dinamo Rent ERP</title>
</svelte:head>

<div class="space-y-5">
	<!-- Encabezado -->
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div>
			<h2 class="text-2xl font-bold text-text-primary">Usuarios</h2>
			<p class="text-sm text-text-secondary mt-0.5">
				{usuarios.length} usuario{usuarios.length === 1 ? '' : 's'} · gestión de cuentas, roles y accesos
			</p>
		</div>
		<button class="btn-primary" onclick={abrirNuevo}>
			<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 4.5v15m7.5-7.5h-15" /></svg>
			Nuevo Usuario
		</button>
	</div>

	<!-- Búsqueda -->
	<div class="relative grow max-w-sm">
		<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-text-secondary/60 pointer-events-none" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" /></svg>
		<input
			class="input pl-9"
			type="search"
			placeholder="Buscar por usuario, nombre o rol..."
			bind:value={busqueda}
		/>
	</div>

	<!-- Tabla -->
	{#if loading}
		<div class="card flex items-center justify-center py-16">
			<div class="flex flex-col items-center gap-3">
				<svg class="animate-spin h-8 w-8 text-primary" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				<p class="text-sm text-text-secondary">Cargando usuarios...</p>
			</div>
		</div>
	{:else}
		<DataTable
			columns={columnas}
			items={tablaUsuarios}
			emptyTitle="No hay usuarios"
			emptyDescription="Crea el primer usuario con el botón «Nuevo Usuario»."
			emptyIcon="user"
		>
			{#snippet children(col, item)}
				{@const u = item as unknown as Usuario}
				{#if col.key === 'usuario'}
					<div class="flex items-center gap-3">
						<div class="w-9 h-9 rounded-full bg-primary/10 text-primary flex items-center justify-center text-xs font-bold uppercase shrink-0">
							{iniciales(u)}
						</div>
						<div>
							<p class="font-semibold text-text-primary flex items-center gap-2">
								<span class="font-mono">{u.username}</span>
								{#if esCuentaPropia(u)}
									<span class="text-[10px] font-bold uppercase tracking-wide px-1.5 py-0.5 rounded-md bg-primary/10 text-primary">Tú</span>
								{/if}
							</p>
							<p class="text-xs text-text-secondary">{u.nombre || '—'}</p>
						</div>
					</div>
				{:else if col.key === 'rol'}
					<span class="inline-flex items-center rounded-full border px-2.5 py-0.5 text-[11px] font-semibold whitespace-nowrap {rolClases(u.rol)}">
						{u.rol ?? '—'}
					</span>
				{:else if col.key === 'estado'}
					{#if u.activo}
						<span class="inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-[11px] font-semibold bg-estado-activo/10 text-estado-activo border-estado-activo/25">
							<span class="w-1.5 h-1.5 rounded-full bg-current opacity-70"></span>Activo
						</span>
					{:else}
						<span class="inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-[11px] font-semibold bg-text-secondary/10 text-text-secondary border-text-secondary/25">
							<span class="w-1.5 h-1.5 rounded-full bg-current opacity-70"></span>Inactivo
						</span>
					{/if}
				{:else if col.key === 'cuenta'}
					<div class="flex flex-wrap gap-1.5">
						{#if u.intentosFallidos >= maxIntentos}
							<span class="inline-flex items-center gap-1 rounded-md border border-peligro/30 bg-peligro/10 text-peligro px-1.5 py-0.5 text-[10px] font-bold" title={`${u.intentosFallidos} intentos fallidos`}>
								<svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" /></svg>
								Bloqueada
							</span>
						{:else if u.intentosFallidos > 0}
							<span class="inline-flex items-center rounded-md border border-alerta/30 bg-alerta/10 text-alerta px-1.5 py-0.5 text-[10px] font-semibold" title={`${u.intentosFallidos} de ${maxIntentos} intentos fallidos`}>
								{u.intentosFallidos}/{maxIntentos} intentos
							</span>
						{/if}
						{#if u.debeCambiarPassword}
							<span class="inline-flex items-center rounded-md border border-alerta/30 bg-alerta/10 text-alerta px-1.5 py-0.5 text-[10px] font-semibold" title="Deberá cambiar la contraseña en el próximo ingreso">
								Cambio obligatorio
							</span>
						{/if}
					</div>
				{:else if col.key === 'acceso'}
					<span class="text-xs text-text-secondary tabular-nums">{fmtDbTs(u.ultimoAcceso)}</span>
				{:else if col.key === 'acciones'}
					<div class="flex items-center justify-end gap-1">
						{#if u.intentosFallidos >= maxIntentos}
							<button
								class="p-2 rounded-lg text-estado-activo hover:bg-estado-activo/10 transition-colors"
								title="Desbloquear cuenta"
								onclick={() => desbloquear(u)}
							>
								<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M13.5 10.5V6.75a4.5 4.5 0 119 0v3.75M3.75 21.75h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H3.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" /></svg>
							</button>
						{/if}
						<button
							class="p-2 rounded-lg text-text-secondary hover:text-alerta hover:bg-alerta/10 transition-colors"
							title="Forzar cambio de contraseña"
							onclick={() => abrirForzar(u)}
						>
							<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" /></svg>
						</button>
						<button
							class="p-2 rounded-lg text-text-secondary hover:text-primary hover:bg-primary/10 transition-colors"
							title="Editar"
							onclick={() => abrirEditar(u)}
						>
							<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M16.862 4.487l1.687-1.688a1.875 1.875 0 112.652 2.652L6.832 19.82a4.5 4.5 0 01-1.897 1.13l-2.685.8.8-2.685a4.5 4.5 0 011.13-1.897L16.862 4.487zm0 0L19.5 7.125" /></svg>
						</button>
						{#if !esCuentaPropia(u)}
							<button
								class="p-2 rounded-lg text-text-secondary hover:text-peligro hover:bg-peligro/10 transition-colors"
								title="Eliminar"
								onclick={() => (eliminarId = u.id)}
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
	title={editando ? `Editar usuario ${form.username}` : 'Nuevo usuario'}
	subtitle={editando ? 'Modifica los datos de gestión y guarda los cambios.' : 'Crea una cuenta con contraseña inicial y rol.'}
	onClose={() => (modalOpen = false)}
	width="max-w-xl"
>
	{#snippet children()}
		{#if formError}
			<div class="mb-4 rounded-lg bg-peligro/10 border border-peligro/30 px-3 py-2.5 text-sm text-peligro" role="alert">{formError}</div>
		{/if}

		<div class="grid grid-cols-1 sm:grid-cols-2 gap-x-4">
			<FormField label="Nombre de usuario" required hint="Solo letras, números, puntos, guiones. Sin espacios.">
				<input
					class="input font-mono"
					placeholder="jperez"
					bind:value={form.username}
					maxlength="50"
					disabled={editando}
				/>
			</FormField>
			<FormField label="Nombre completo" required>
				<input class="input" placeholder="Ej: Juan Pérez" bind:value={form.nombre} maxlength="100" />
			</FormField>
			<FormField label="Rol" required>
				<select class="input" bind:value={form.rol}>
					{#each (lists?.rolesDisponibles ?? ['Administrador', 'Supervisor', 'Operador']) as r}
						<option value={r}>{r}</option>
					{/each}
				</select>
			</FormField>
			<FormField label="Email">
				<input class="input" type="email" placeholder="usuario@correo.com" bind:value={form.email} maxlength="100" />
			</FormField>

			{#if !editando}
				<div class="col-span-full mt-1 mb-1">
					<h3 class="text-xs font-bold uppercase tracking-wider text-primary mb-3 flex items-center gap-2">
						<span class="w-4 h-4 rounded-md bg-primary/10 flex items-center justify-center text-[10px]">1</span>
						Contraseña inicial
					</h3>
				</div>
				<FormField label="Contraseña" required hint="Mínimo 8 caracteres: mayúscula, minúscula, número y símbolo.">
					<input class="input" type="password" autocomplete="new-password" bind:value={form.password} maxlength="128" />
				</FormField>
				<FormField label="Confirmar contraseña" required>
					<input class="input" type="password" autocomplete="new-password" bind:value={confirmacion} maxlength="128" />
				</FormField>
			{/if}
		</div>

		<!-- Opciones -->
		<div class="mt-2 space-y-2.5">
			<button
				type="button"
				class="w-full flex items-center justify-between gap-3 rounded-xl border border-border bg-alt-row/50 px-4 py-3 text-left transition-colors hover:border-primary/40"
				onclick={() => (form.activo = !form.activo)}
			>
				<div>
					<p class="text-sm font-semibold text-text-primary">Cuenta activa</p>
					<p class="text-xs text-text-secondary mt-0.5">Puede iniciar sesión con esta cuenta.</p>
				</div>
				<span class={'relative inline-flex h-6 w-11 shrink-0 items-center rounded-full transition-colors ' + (form.activo ? 'bg-estado-activo' : 'bg-text-secondary/40')} aria-hidden="true">
					<span class={'inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform ' + (form.activo ? 'translate-x-6' : 'translate-x-1')}></span>
				</span>
			</button>
			{#if !editando}
			<button
				type="button"
				class="w-full flex items-center justify-between gap-3 rounded-xl border border-border bg-alt-row/50 px-4 py-3 text-left transition-colors hover:border-primary/40"
				onclick={() => (form.debeCambiarPassword = !form.debeCambiarPassword)}
			>
				<div>
					<p class="text-sm font-semibold text-text-primary">Obligar cambio de contraseña en el próximo ingreso</p>
					<p class="text-xs text-text-secondary mt-0.5">Recomendado para cuentas nuevas. Se pide cambiarla al iniciar sesión.</p>
				</div>
				<span class={'relative inline-flex h-6 w-11 shrink-0 items-center rounded-full transition-colors ' + (form.debeCambiarPassword ? 'bg-primary' : 'bg-text-secondary/40')} aria-hidden="true">
					<span class={'inline-block h-4 w-4 transform rounded-full bg-white shadow transition-transform ' + (form.debeCambiarPassword ? 'translate-x-6' : 'translate-x-1')}></span>
				</span>
			</button>
			{/if}
		</div>
	{/snippet}

	{#snippet footer()}
		<button class="btn-ghost" onclick={() => (modalOpen = false)} disabled={guardando}>Cancelar</button>
		<button class="btn-primary" onclick={guardar} disabled={guardando}>
			{#if guardando}
				<svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				Guardando...
			{:else}
				{editando ? 'Guardar cambios' : 'Crear usuario'}
			{/if}
		</button>
	{/snippet}
</Modal>

<!-- Modal forzar cambio de contraseña -->
<Modal
	open={forzarUsuario !== null}
	title={forzarUsuario ? `Reiniciar contraseña de ${forzarUsuario.username}` : ''}
	subtitle="El usuario deberá cambiar esta contraseña en su próximo ingreso."
	onClose={() => (forzarUsuario = null)}
	width="max-w-md"
>
	{#snippet children()}
		{#if forzarError}
			<div class="mb-4 rounded-lg bg-peligro/10 border border-peligro/30 px-3 py-2.5 text-sm text-peligro" role="alert">{forzarError}</div>
		{/if}
		<FormField label="Nueva contraseña" required hint="Mínimo 8 caracteres: mayúscula, minúscula, número y símbolo.">
			<input class="input" type="password" autocomplete="new-password" bind:value={nuevaPassword} maxlength="128" />
		</FormField>
		<FormField label="Confirmar nueva contraseña" required>
			<input class="input" type="password" autocomplete="new-password" bind:value={confirmPassword} maxlength="128" />
		</FormField>
	{/snippet}

	{#snippet footer()}
		<button class="btn-ghost" onclick={() => (forzarUsuario = null)} disabled={forcando}>Cancelar</button>
		<button class="btn-primary" onclick={confirmarForzar} disabled={forcando}>
			{#if forcando}
				<svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				Reiniciando...
			{:else}
				Reiniciar contraseña
			{/if}
		</button>
	{/snippet}
</Modal>

<!-- Confirmación de eliminación -->
<ConfirmDialog
	open={eliminarId !== null}
	title="Eliminar usuario"
	message="¿Seguro que deseas eliminar este usuario? Perderá el acceso al sistema y no se puede deshacer."
	confirmLabel="Eliminar"
	loading={eliminando}
	onConfirm={confirmarEliminar}
	onCancel={() => (eliminarId = null)}
/>
