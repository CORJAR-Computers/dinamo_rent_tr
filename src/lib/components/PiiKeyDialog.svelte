<script lang="ts">
	// PiiKeyDialog.svelte — Configura db_encryption_key y verifica la
	// desencriptación de los datos legacy Fernet de la tabla clientes.
	import { piiApi, ApiError, type PiiAnalisis } from '$lib/api';
	import { session } from '$lib/stores/session.svelte';
	import { toast } from '$lib/stores/toast.svelte';
	import Modal from './Modal.svelte';
	import Icon from './Icon.svelte';

	let {
		open,
		onClose,
		onSaved
	}: {
		open: boolean;
		onClose: () => void;
		/** Se invoca tras guardar/eliminar para refrescar listas/dashboard */
		onSaved?: () => void;
	} = $props();

	const sid = () => session.token ?? '';

	let clave = $state('');
	let showClave = $state(false);
	let estado = $state<PiiAnalisis | null>(null);
	let probando = $state(false);
	let guardando = $state(false);
	let probandoError = $state('');
	let probandoInfo = $state('');

	// Cargar el estado actual al abrir
	$effect(() => {
		if (open) {
			clave = '';
			showClave = false;
			estado = null;
			probandoError = '';
			probandoInfo = '';
			cargarEstado();
		}
	});

	async function cargarEstado() {
		try {
			estado = await piiApi.status(sid());
		} catch {
			/* el diálogo sigue utilizable sin el estado previo */
		}
	}

	async function probar() {
		probandoError = '';
		probandoInfo = '';
		if (!clave.trim()) {
			probandoError = 'Escribe la clave para probarla.';
			return;
		}
		probando = true;
		try {
			const r = await piiApi.probar(sid(), clave.trim());
			estado = r;
			probandoInfo =
				r.clientesDescifrados > 0
					? `La clave descifra ${r.clientesDescifrados} de ${r.clientesLegacy} clientes legacy.`
					: `La clave no descifra ninguno de los ${r.clientesLegacy} clientes legacy. Revisa que sea la clave original.`;
		} catch (e) {
			probandoError = e instanceof ApiError ? e.message : 'No se pudo probar la clave.';
		} finally {
			probando = false;
		}
	}

	async function guardar() {
		probandoError = '';
		if (!clave.trim()) {
			probandoError = 'Escribe la clave para guardarla.';
			return;
		}
		guardando = true;
		try {
			const r = await piiApi.guardar(sid(), clave.trim());
			estado = r.analisis;
			toast.success(
				r.analisis.clientesDescifrados > 0
					? `Clave guardada: ${r.analisis.clientesDescifrados} clientes legacy descifrados.`
					: 'Clave guardada. Los datos legacy siguen ocultos (clave no coincide).'
			);
			onSaved?.();
			onClose();
		} catch (e) {
			probandoError = e instanceof ApiError ? e.message : 'No se pudo guardar la clave.';
		} finally {
			guardando = false;
		}
	}

	async function eliminar() {
		if (
			!confirm('¿Eliminar la clave PII configurada? Los datos Fernet legacy volverán a ocultarse.')
		)
			return;
		guardando = true;
		try {
			const r = await piiApi.eliminar(sid());
			estado = r.analisis;
			toast.success('Clave PII eliminada.');
			onSaved?.();
			onClose();
		} catch (e) {
			probandoError = e instanceof ApiError ? e.message : 'No se pudo eliminar la clave.';
		} finally {
			guardando = false;
		}
	}

	const tieneClave = $derived(estado?.claveConfigurada ?? false);
</script>

<Modal
	{open}
	title="Clave de cifrado de datos (PII)"
	subtitle="Configura db_encryption_key para descifrar los datos de clientes de versiones anteriores (Fernet)."
	onClose={() => (guardando ? null : onClose())}
	width="max-w-lg"
>
	{#snippet children()}
		<!-- Estado actual -->
		{#if estado}
			<div
				class="mb-4 rounded-xl border px-4 py-3 flex items-start gap-3 {tieneClave
					? 'border-exito/25 bg-exito/5'
					: 'border-alerta/25 bg-alerta/5'}"
			>
				<span class="shrink-0 {tieneClave ? 'text-exito' : 'text-alerta'}">
					<Icon name={tieneClave ? 'lock' : 'alert'} class="w-5 h-5" />
				</span>
				<div class="text-sm">
					<p class="font-semibold text-text-primary">
						{tieneClave ? 'Clave configurada' : 'Sin clave configurada'}
					</p>
					<p class="text-xs text-text-secondary mt-0.5">
						{estado.totalClientes} clientes · {estado.clientesLegacy} con datos legacy Fernet ·
						{estado.clientesDescifrados} descifrados · {estado.clientesOcultos} ocultos
					</p>
					{#if estado.muestra}
						<p class="text-xs text-text-secondary/80 mt-1 font-mono">
							Muestra: {estado.muestra.cliente} · {estado.muestra.campo}: {estado.muestra.valor}
						</p>
					{/if}
				</div>
			</div>
		{:else}
			<div
				class="mb-4 rounded-xl border border-border bg-alt-row/60 px-4 py-3 text-xs text-text-secondary animate-pulse"
			>
				Consultando estado del descifrado…
			</div>
		{/if}

		<!-- Entrada de clave -->
		<div class="mb-2">
			<label class="label" for="pii-key-input"
				>Clave original de la app anterior (db_encryption_key)</label
			>
			<div class="relative">
				<input
					id="pii-key-input"
					class="input pr-10 font-mono"
					type={showClave ? 'text' : 'password'}
					placeholder="Pega la clave de [security] db_encryption_key"
					bind:value={clave}
					autocomplete="off"
					spellcheck="false"
					disabled={guardando}
				/>
				<button
					type="button"
					class="absolute inset-y-0 right-0 pr-3 flex items-center text-text-secondary hover:text-text-primary transition-colors"
					onclick={() => (showClave = !showClave)}
					aria-label={showClave ? 'Ocultar clave' : 'Mostrar clave'}
				>
					{#if showClave}
						<svg
							xmlns="http://www.w3.org/2000/svg"
							class="w-5 h-5"
							fill="none"
							viewBox="0 0 24 24"
							stroke="currentColor"
							stroke-width="1.8"
							><path
								stroke-linecap="round"
								stroke-linejoin="round"
								d="M3.98 8.223A10.477 10.477 0 001.934 12C3.226 16.338 7.244 19.5 12 19.5c.993 0 1.953-.138 2.863-.395M6.228 6.228A10.45 10.45 0 0112 4.5c4.756 0 8.773 3.162 10.065 7.498a10.523 10.523 0 01-4.293 5.774M6.228 6.228L3 3m3.228 3.228l3.65 3.65m7.894 7.894L21 21m-3.228-3.228l-3.65-3.65m0 0a3 3 0 10-4.243-4.243m4.242 4.242L9.88 9.88"
							/></svg
						>
					{:else}
						<svg
							xmlns="http://www.w3.org/2000/svg"
							class="w-5 h-5"
							fill="none"
							viewBox="0 0 24 24"
							stroke="currentColor"
							stroke-width="1.8"
							><path
								stroke-linecap="round"
								stroke-linejoin="round"
								d="M2.036 12.322a1.012 1.012 0 010-.639C3.423 7.51 7.36 4.5 12 4.5c4.638 0 8.573 3.007 9.963 7.178.07.207.07.431 0 .639C20.577 16.49 16.64 19.5 12 19.5c-4.638 0-8.573-3.007-9.963-7.178z"
							/><path
								stroke-linecap="round"
								stroke-linejoin="round"
								d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"
							/></svg
						>
					{/if}
				</button>
			</div>
			<p class="mt-1 text-xs text-text-secondary/70">
				Se guarda en <code class="font-mono bg-alt-row px-1 py-0.5 rounded">config.ini</code> [security]
				y se aplica sin reiniciar la app.
			</p>
		</div>

		{#if probandoError}
			<div
				class="mb-3 rounded-lg bg-peligro/10 border border-peligro/30 px-3 py-2 text-sm text-peligro"
				role="alert"
			>
				{probandoError}
			</div>
		{/if}
		{#if probandoInfo}
			<div class="mb-3 rounded-lg bg-exito/10 border border-exito/30 px-3 py-2 text-sm text-exito">
				{probandoInfo}
			</div>
		{/if}

		<div class="flex flex-wrap items-center gap-2 mt-2">
			<button
				class="btn-outline !px-3 !py-1.5 text-xs"
				onclick={probar}
				disabled={probando || guardando}
			>
				{#if probando}
					<svg
						class="animate-spin h-3.5 w-3.5"
						xmlns="http://www.w3.org/2000/svg"
						fill="none"
						viewBox="0 0 24 24"
						><circle
							class="opacity-25"
							cx="12"
							cy="12"
							r="10"
							stroke="currentColor"
							stroke-width="4"
						></circle><path
							class="opacity-75"
							fill="currentColor"
							d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
						></path></svg
					>
					Probando…
				{:else}
					Probar clave
				{/if}
			</button>
			{#if tieneClave}
				<button
					class="btn-ghost !px-3 !py-1.5 text-xs text-peligro hover:text-peligro"
					onclick={eliminar}
					disabled={guardando}
				>
					Eliminar clave
				</button>
			{/if}
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
				Guardando…
			{:else}
				Guardar clave
			{/if}
		</button>
	{/snippet}
</Modal>
