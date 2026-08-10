<script lang="ts">
	import { onMount } from 'svelte';
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
	import { appApi, invokeCmd } from '$lib/api';
	import Modal from './Modal.svelte';

	// Diálogo de confirmación al pulsar la X de la ventana. El backend
	// (on_window_event en lib.rs) intercepta CloseRequested y emite
	// 'app-close-requested'; aquí se muestra el modal y, si el usuario pulsa
	// «Sí», se invoca el comando `confirmar_cierre`, que destruye la ventana
	// (destroy() no vuelve a disparar CloseRequested → sin bucles).
	//
	// Se monta en +layout.svelte a nivel raíz (fuera del if/else de sesión)
	// para que funcione también en login / cambiar-password.
	let abrir = $state(false);
	let unlisten: UnlistenFn | null = null;
	// Referencia al botón «No» para darle el foco al abrir (acción segura).
	let btnNo = $state<HTMLButtonElement | null>(null);

	// Al abrir, enfocar «No»: Enter/Esc equivale a cancelar, nunca cierra por
	// accidente. (El Modal no hace autofocus por sí solo.)
	$effect(() => {
		if (abrir) btnNo?.focus();
	});

	onMount(() => {
		// Confirmar al backend que ya escuchamos el evento (si no está listo,
		// la X cierra directamente para no bloquear la ventana).
		appApi.frontendLista().catch(() => {});

		let activo = true;
		listen('app-close-requested', () => {
			if (activo) abrir = true;
		})
			.then((u) => {
				if (activo) unlisten = u;
				else u();
			})
			.catch(() => {
				// Sin runtime Tauri (tests / vite standalone): no hay evento de cierre.
			});
		return () => {
			activo = false;
			unlisten?.();
		};
	});

	function cancelar() {
		abrir = false;
	}

	async function confirmar() {
		abrir = false;
		try {
			await invokeCmd<void>('confirmar_cierre');
		} catch {
			// Backend no disponible (p.ej. vite standalone): el cierre real no aplica.
		}
	}
</script>

<Modal open={abrir} title="Cerrar aplicación" onClose={cancelar} width="max-w-md">
	{#snippet children()}
		<div class="flex items-start gap-4">
			<div
				class="w-11 h-11 rounded-xl bg-alerta/10 text-alerta flex items-center justify-center shrink-0"
			>
				<svg xmlns="http://www.w3.org/2000/svg" class="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z" /></svg>
			</div>
			<p class="text-sm text-text-primary leading-relaxed pt-1">
				¿Está seguro de cerrar la aplicación?
			</p>
		</div>
	{/snippet}

	{#snippet footer()}
		<button class="btn-ghost" bind:this={btnNo} onclick={cancelar}>No</button>
		<button class="btn-primary" onclick={confirmar}>Sí</button>
	{/snippet}
</Modal>
