<script lang="ts">
	import { onMount } from 'svelte';
	import { backupApi, ApiError, type InfoBackup } from '$lib/api';
	import { session } from '$lib/stores/session.svelte';
	import { toast } from '$lib/stores/toast.svelte';
	import { formatDateTime } from '$lib/utils/format';
	import { guardRole, guardSesion } from '$lib/utils/guards';
	import Modal from '$lib/components/Modal.svelte';

	const sid = () => session.token ?? '';

	let estado = $state<InfoBackup | null>(null);
	let loading = $state(true);
	let creando = $state(false);
	// Restauración: modal de confirmación (+ contraseña si el backup está
	// cifrado) y banner de «reiniciando» (la app se cierra sola tras ~1.5 s).
	let restaurarNombre = $state<string | null>(null);
	let restaurarCifrado = $state(false);
	let password = $state('');
	let restaurando = $state(false);
	let reiniciando = $state(false);

	function tamano(bytes: number): string {
		if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
		if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${bytes} B`;
	}

	async function cargar() {
		// Guard de sesión + rol: la página y los comandos son solo Administrador.
		if (!guardSesion()) return;
		if (!guardRole(['Administrador'], '/dashboard')) return;
		loading = true;
		try {
			estado = await backupApi.estado(sid());
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'No se pudo cargar el estado de los backups.');
		} finally {
			loading = false;
		}
	}

	async function crearAhora() {
		if (creando || estado?.ejecutando || restaurando || reiniciando) return;
		creando = true;
		try {
			estado = await backupApi.ahora(sid());
			toast.success('Backup creado correctamente.');
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'No se pudo crear el backup.');
			// El error queda registrado en el backend; refrescar para mostrarlo
			await cargar();
		} finally {
			creando = false;
		}
	}

	function pedirRestaurar(nombre: string, cifrado: boolean) {
		if (creando || estado?.ejecutando || restaurando || reiniciando) return;
		restaurarNombre = nombre;
		restaurarCifrado = cifrado;
		password = '';
	}

	function cerrarModal() {
		if (restaurando) return;
		restaurarNombre = null;
	}

	async function confirmarRestaurar() {
		const nombre = restaurarNombre;
		if (!nombre || restaurando || reiniciando) return;
		restaurando = true;
		try {
			// El backend valida, prepara el staging (descifra si aplica), relanza
			// la app con --restaurar-backup y la cierra en ~1.5 s.
			estado = await backupApi.restaurar(sid(), nombre, password || null);
			restaurarNombre = null;
			reiniciando = true;
		} catch (e) {
			toast.error(
				e instanceof ApiError ? e.message : 'No se pudo iniciar la restauración.'
			);
			// El error queda registrado en el backend; refrescar para mostrarlo
			await cargar();
			restaurarNombre = null;
		} finally {
			restaurando = false;
		}
	}

	onMount(cargar);
</script>

<svelte:head>
	<title>Backups — Dinamo Rent ERP</title>
</svelte:head>

<div class="space-y-4">
	<div class="flex flex-wrap items-center justify-between gap-3">
		<div>
			<h2 class="text-2xl font-bold text-text-primary">Backups</h2>
			<p class="text-sm text-text-secondary mt-1">
				Copias de seguridad de la base de datos · solo Administrador
			</p>
		</div>
		<button class="btn-primary" onclick={crearAhora} disabled={creando || estado?.ejecutando}>
			{#if creando || estado?.ejecutando}
				<svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				Creando backup...
			{:else}
				<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5M16.5 12L12 16.5m0 0L7.5 12m4.5 4.5V3" /></svg>
				Crear backup ahora
			{/if}
		</button>
	</div>

	{#if estado}
		<!-- Estado de los backups automáticos -->
		<div class="card p-4 space-y-3">
			<div class="flex flex-wrap items-center gap-x-6 gap-y-2 text-sm">
				<span class="text-text-secondary">
					Horarios automáticos:
					<strong class="text-text-primary">
						{estado.horarios.length > 0 ? estado.horarios.join(' · ') : 'desactivados'}
					</strong>
				</span>
				<span class="text-text-secondary">
					Próxima corrida:
					<strong class="text-text-primary">{estado.proximaCorrida ? formatDateTime(estado.proximaCorrida) : '—'}</strong>
				</span>
				<span class="text-text-secondary">
					Última corrida:
					<strong class="text-text-primary">
						{estado.ultimoBackup ? formatDateTime(estado.ultimoBackup) : 'aún sin backups'}
					</strong>
				</span>
				{#if estado.cifrado}
					<span class="inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-[11px] font-semibold text-exito border-border">
						<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" /></svg>
						Cifrado AES-256-GCM
					</span>
				{:else}
					<span class="inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-[11px] font-semibold text-text-secondary border-border">Sin cifrado</span>
				{/if}
			</div>
			<p class="text-[11px] text-text-secondary font-mono truncate" title={estado.directorio}>
				Directorio: {estado.directorio}
			</p>
			{#if estado.ultimoError}
				<div class="rounded-lg border border-peligro/40 bg-peligro/10 p-3 text-xs text-peligro">
					<strong>Último backup falló:</strong> {estado.ultimoError}
				</div>
			{/if}
			{#if estado.ultimaRestauracion}
				<div class="rounded-lg border border-exito/40 bg-exito/10 p-3 text-xs text-exito">
					<strong>Última restauración OK:</strong> {estado.ultimaRestauracion}
				</div>
			{/if}
			{#if estado.ultimaRestauracionError}
				<div class="rounded-lg border border-peligro/40 bg-peligro/10 p-3 text-xs text-peligro">
					<strong>Última restauración falló:</strong> {estado.ultimaRestauracionError}
				</div>
			{/if}
		</div>

		{#if reiniciando}
			<div class="card p-6 border-peligro/40">
				<div class="flex items-start gap-4">
					<svg class="animate-spin h-6 w-6 text-primary shrink-0 mt-0.5" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
					<div>
						<p class="font-semibold text-text-primary">Restauración iniciada</p>
						<p class="text-sm text-text-secondary mt-1">
							La aplicación se cerrará y se reiniciará con los datos del backup
							seleccionado. No cierres ni apagues el equipo durante el proceso.
						</p>
					</div>
				</div>
			</div>
		{/if}

		<!-- Copias guardadas -->
		<div class="card overflow-hidden">
			<div class="flex flex-wrap items-center justify-between gap-2 px-4 py-3 border-b border-border">
				<p class="font-semibold text-text-primary">Copias guardadas</p>
				<span class="text-xs text-text-secondary">
					{estado.copias.length} / {estado.maxCopies === 0 ? '∞ (sin rotación)' : estado.maxCopies}
				</span>
			</div>
			{#if estado.copias.length === 0}
				<p class="text-sm text-text-secondary py-8 px-4 text-center">
					Aún no hay copias. La primera llegará con el backup automático
					{estado.proximaCorrida ? `del ${formatDateTime(estado.proximaCorrida)}` : ''}, o crea una ahora con el botón superior.
				</p>
			{:else}
				<div class="overflow-x-auto">
					<table class="w-full text-sm">
						<thead>
						<tr class="text-left text-xs uppercase tracking-wide text-text-secondary border-b border-border">
							<th class="px-4 py-2 font-semibold">Archivo</th>
							<th class="px-4 py-2 font-semibold text-right">Tamaño</th>
							<th class="px-4 py-2 font-semibold">Creado</th>
							<th class="px-4 py-2 font-semibold">Cifrado</th>
							<th class="px-4 py-2 font-semibold text-right">Acciones</th>
						</tr>
						</thead>
						<tbody>
							{#each estado.copias as c}
								<tr class="border-b border-border/60 last:border-0 hover:bg-primary/5">
									<td class="px-4 py-2 font-mono text-[13px] text-text-primary" title={c.nombre}>{c.nombre}</td>
									<td class="px-4 py-2 text-right tabular-nums text-text-secondary">{tamano(c.tamanoBytes)}</td>
									<td class="px-4 py-2 text-text-secondary">{formatDateTime(c.modificado)}</td>
									<td class="px-4 py-2">
										{#if c.cifrado}
											<span class="inline-flex items-center gap-1 text-[11px] font-semibold text-exito">
												<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" /></svg>
												Cifrado
											</span>
										{:else}												<span class="text-[11px] font-semibold text-text-secondary">Plano (.fbk)</span>
											{/if}
										</td>
										<td class="px-4 py-2 text-right">
											<button
												class="btn-ghost text-[12px] px-2.5 py-1"
												onclick={() => pedirRestaurar(c.nombre, c.cifrado)}
												disabled={creando || restaurando || reiniciando || estado?.ejecutando}
												title="Restaura la BD desde esta copia (la app se reinicia)"
											>
												<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 mr-1 inline-block" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" /></svg>
												Restaurar
											</button>
										</td>
									</tr>
								{/each}
						</tbody>
					</table>
				</div>
			{/if}
		</div>
	{:else if loading}
		<div class="card p-8 flex items-center justify-center">
			<svg class="animate-spin h-6 w-6 text-primary" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
		</div>
	{:else}
		<div class="card p-8 text-center">
			<p class="text-sm text-text-secondary mb-3">No se pudo cargar el estado de los backups.</p>
			<button class="btn-ghost" onclick={cargar}>Reintentar</button>
		</div>
	{/if}
</div>

<!-- Confirmación de restauración (+ contraseña si la copia está cifrada) -->
<Modal
	open={restaurarNombre !== null}
	title="Restaurar base de datos"
	subtitle={restaurarNombre ?? ''}
	onClose={cerrarModal}
	width="max-w-md"
	dismissible={!restaurando}
>
	{#snippet children()}
		<div class="space-y-3">
			<div class="flex items-start gap-3">
				<div class="w-10 h-10 rounded-xl bg-peligro/10 text-peligro flex items-center justify-center shrink-0">
					<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z" /></svg>
				</div>
			<p class="text-sm text-text-primary leading-relaxed pt-1">
				La base de datos actual será <strong>reemplazada</strong> por la de este
				backup y la aplicación se reiniciará. ¿Continuar?
			</p>
		</div>
		{#if restaurarCifrado}
			<label class="block">
				<span class="text-xs font-medium text-text-secondary">
					Contraseña de cifrado (obligatoria: la copia está cifrada)
				</span>
				<input
					type="password"
					class="input w-full mt-1"
					placeholder="Contraseña del backup"
					bind:value={password}
					disabled={restaurando}
					onkeydown={(e) => e.key === 'Enter' && confirmarRestaurar()}
				/>
			</label>
		{/if}
	</div>
	{/snippet}

	{#snippet footer()}
		<button class="btn-ghost" onclick={cerrarModal} disabled={restaurando}>Cancelar</button>
		<button
			class="btn-danger"
			onclick={confirmarRestaurar}
			disabled={restaurando || (restaurarCifrado && password.trim() === '')}
		>
			{#if restaurando}
				<svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				Restaurando...
			{:else}
				Restaurar y reiniciar
			{/if}
		</button>
	{/snippet}
</Modal>
