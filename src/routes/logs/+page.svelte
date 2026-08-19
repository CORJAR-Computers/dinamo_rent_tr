<script lang="ts">
	import { onMount } from 'svelte';
	import { logApi, ApiError } from '$lib/api';
	import { sid } from '$lib/stores/session.svelte';
	import { toast } from '$lib/stores/toast.svelte';
	import { guardSesion, tieneRol } from '$lib/utils/guards';
	import { goto } from '$app/navigation';

	// Solo admin
	const esAdmin = $derived(tieneRol(['Administrador']));

	let logsBackend = $state('(cargando...)');
	let logsFrontend = $state('(cargando...)');
	let loading = $state(true);
	let tab = $state<'backend' | 'frontend'>('backend');
	let lineasBackend = $state(500);
	let lineasFrontend = $state(200);
	let exportando = $state(false);
	let limpiando = $state(false);

	onMount(async () => {
		if (!guardSesion()) return;
		if (!esAdmin) {
			goto('/');
			return;
		}
		await cargarLogs();
	});

	async function cargarLogs() {
		loading = true;
		try {
			const [backend, frontend] = await Promise.all([
				logApi.leer(sid(), lineasBackend),
				logApi.erroresFrontend(sid(), lineasFrontend)
			]);
			logsBackend = backend;
			logsFrontend = frontend;
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'Error cargando logs');
		} finally {
			loading = false;
		}
	}

	async function exportar() {
		exportando = true;
		try {
			const contenido = await logApi.exportar(sid());
			// Descargar como archivo
			const blob = new Blob([contenido], { type: 'text/plain;charset=utf-8' });
			const url = URL.createObjectURL(blob);
			const a = document.createElement('a');
			a.href = url;
			const fecha = new Date().toISOString().slice(0, 10);
			a.download = `dinamo_rent_logs_${fecha}.txt`;
			a.click();
			URL.revokeObjectURL(url);
			toast.success('Logs exportados correctamente');
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'Error exportando logs');
		} finally {
			exportando = false;
		}
	}

	async function copiarAlPortapapeles() {
		try {
			const contenido = tab === 'backend' ? logsBackend : logsFrontend;
			await navigator.clipboard.writeText(contenido);
			toast.success('Copiado al portapapeles');
		} catch {
			toast.error('No se pudo copiar al portapapeles');
		}
	}

	async function limpiarLogs() {
		if (!confirm('¿Truncar los archivos de log? Esta acción no se puede deshacer.')) return;
		limpiando = true;
		try {
			const eliminados = await logApi.limpiar(sid());
			toast.success(`Logs truncados: ${eliminados} archivo(s)`);
			await cargarLogs();
		} catch (e) {
			toast.error(e instanceof ApiError ? e.message : 'Error limpiando logs');
		} finally {
			limpiando = false;
		}
	}
</script>

<svelte:head>
	<title>Logs — Dinamo Rent ERP</title>
</svelte:head>

{#if esAdmin}
	<div class="max-w-6xl mx-auto">
		<!-- Header -->
		<div class="flex items-center justify-between mb-6">
			<div>
				<h1 class="text-xl font-bold text-text-primary">📋 Logs del Sistema</h1>
				<p class="text-sm text-text-secondary mt-0.5">
					Backend (app.log) y errores del frontend (JS)
				</p>
			</div>
			<div class="flex gap-2">
				<button class="btn-ghost text-sm" onclick={cargarLogs} disabled={loading}>
					🔄 Actualizar
				</button>
				<button class="btn-ghost text-sm" onclick={copiarAlPortapapeles}>
					📋 Copiar
				</button>
				<button class="btn-primary text-sm" onclick={exportar} disabled={exportando}>
					{exportando ? '⏳ Exportando...' : '💾 Exportar archivo'}
				</button>
				<button
					class="btn-ghost text-sm text-peligro"
					onclick={limpiarLogs}
					disabled={limpiando}
				>
					🗑️ Truncar
				</button>
			</div>
		</div>

		<!-- Tabs -->
		<div class="flex gap-1 mb-4 border-b border-border">
			<button
				class="px-4 py-2 text-sm font-medium transition-colors {tab === 'backend'
					? 'text-primary border-b-2 border-primary'
					: 'text-text-secondary hover:text-text-primary'}"
				onclick={() => (tab = 'backend')}
			>
				🔧 Backend ({logsBackend.split('\n').length} líneas)
			</button>
			<button
				class="px-4 py-2 text-sm font-medium transition-colors {tab === 'frontend'
					? 'text-primary border-b-2 border-primary'
					: 'text-text-secondary hover:text-text-primary'}"
				onclick={() => (tab = 'frontend')}
			>
				🌐 Frontend ({logsFrontend.split('\n').length} líneas)
			</button>
		</div>

		<!-- Log content -->
		<div class="card p-4">
			{#if loading}
				<div class="flex items-center justify-center py-12 text-text-secondary">
					⏳ Cargando logs...
				</div>
			{:else}
				<pre class="text-xs font-mono text-text-primary whitespace-pre-wrap break-words max-h-[70vh] overflow-auto bg-fondo rounded p-4">{tab === 'backend'
					? logsBackend
					: logsFrontend}</pre>
			{/if}
		</div>
	</div>
{:else}
	<div class="flex items-center justify-center min-h-[60vh]">
		<div class="card p-8 text-center">
			<p class="text-lg font-bold text-peligro mb-2">Acceso denegado</p>
			<p class="text-sm text-text-secondary">Solo los administradores pueden ver los logs.</p>
		</div>
	</div>
{/if}
