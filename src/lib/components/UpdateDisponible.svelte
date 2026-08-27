<script lang="ts">
	import { onMount } from 'svelte';
	import { check, type Update } from '@tauri-apps/plugin-updater';
	import { relaunch } from '@tauri-apps/plugin-process';
	import { toast } from '$lib/stores/toast.svelte';
	import Modal from './Modal.svelte';

	// Diálogo de actualización: comprueba en GitHub Releases (endpoints de
	// tauri.conf.json → plugins.updater) si hay una versión más nueva y, si
	// existe, pide permiso para descargarla e instalarla.
	//
	// Dos disparadores:
	//   1. Automático al arrancar la app (best-effort: sin red o sin runtime
	//      Tauri el chequeo se omite en silencio — la app sigue funcionando).
	//   2. Manual con el botón «Buscar actualización» de la barra superior:
	//      el layout pasa la prop onReady y el componente le entrega la función
	//      buscar() en el mount (alternativa tipada a $expose, que no está
	//      declarado en los tipos de este Svelte).
	//
	// Se monta en +layout.svelte a nivel raíz para cubrir también login.

	let { onReady }: { onReady?: (buscar: () => Promise<void>) => void } = $props();

	let update = $state<Update | null>(null);
	let descargando = $state(false);
	let progreso = $state<number | null>(null);
	let error = $state<string | null>(null);
	let comprobando = $state(false);

	const abrir = $derived(update !== null);

	async function ejecutarCheck(conFeedback: boolean): Promise<void> {
		if (comprobando) return;
		comprobando = true;
		try {
			const disponible = await check();
			if (disponible) {
				update = disponible;
			} else if (conFeedback) {
				toast.success('Ya tienes la versión más reciente.');
			}
		} catch (e) {
			console.warn('No se pudo comprobar actualizaciones:', e);
			if (conFeedback) {
				toast.error('No se pudo comprobar actualizaciones: ' + String(e));
			}
		} finally {
			comprobando = false;
		}
	}

	onMount(() => {
		// Entrega la búsqueda manual al layout (botón «Buscar actualización»).
		onReady?.(() => ejecutarCheck(true));
		// Pequeño retraso: no competir con la validación de sesión del arranque.
		const timer = setTimeout(() => void ejecutarCheck(false), 3000);
		return () => clearTimeout(timer);
	});

	function masTarde() {
		update = null;
	}

	async function instalar() {
		if (!update || descargando) return;
		descargando = true;
		error = null;
		progreso = null;
		let descargado = 0;
		let total: number | null = null;
		try {
			await update.downloadAndInstall((event) => {
				if (event.event === 'Started') {
					total = event.data.contentLength ?? null;
				} else if (event.event === 'Progress') {
					descargado += event.data.chunkLength;
					if (total && total > 0) {
						progreso = Math.round((descargado / total) * 100);
					}
				}
			});
			// En Windows la app se cierra sola al instalar; el relaunch deja la
			// nueva versión arrancada.
			await relaunch();
		} catch (e) {
			error = String(e);
			descargando = false;
		}
	}
</script>

<Modal
	open={abrir}
	title="Actualización disponible"
	onClose={masTarde}
	width="max-w-md"
	dismissible={!descargando}
>
	{#snippet children()}
		<div class="flex items-start gap-4">
			<div
				class="w-11 h-11 rounded-xl bg-primary/10 text-primary flex items-center justify-center shrink-0"
			>
				<svg
					xmlns="http://www.w3.org/2000/svg"
					class="w-6 h-6"
					fill="none"
					viewBox="0 0 24 24"
					stroke="currentColor"
					stroke-width="1.8"
					><path
						stroke-linecap="round"
						stroke-linejoin="round"
						d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5M16.5 12L12 16.5m0 0L7.5 12m4.5 4.5V3"
					/></svg
				>
			</div>
			<div class="text-sm text-text-primary leading-relaxed pt-1 space-y-2">
				<p>
					Hay una nueva versión disponible:
					<strong class="font-semibold">{update?.version}</strong>
				</p>
				{#if update?.body}
					<p class="text-text-secondary text-xs whitespace-pre-line">{update.body}</p>
				{/if}
				{#if descargando}
					<p class="text-text-secondary text-xs">
						Descargando e instalando…
						{#if progreso !== null}{progreso}%{/if}
					</p>
				{/if}
				{#if error}
					<p class="text-alerta text-xs">No se pudo instalar: {error}</p>
				{/if}
			</div>
		</div>
	{/snippet}

	{#snippet footer()}
		{#if descargando}
			<button class="btn-primary" disabled>Instalando…</button>
		{:else}
			<button class="btn-ghost" onclick={masTarde}>Más tarde</button>
			<button class="btn-primary" onclick={instalar}>Instalar ahora</button>
		{/if}
	{/snippet}
</Modal>
