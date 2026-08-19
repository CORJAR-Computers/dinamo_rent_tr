<script lang="ts">
	// Página de error global (SvelteKit). Se renderiza cuando un `load` falla,
	// cuando una ruta no existe (404) o cuando el servidor devuelve un error (5xx).
	// Muestra el código de estado y el mensaje, con botones para reintentar o
	// volver al dashboard. Usa los tokens de tema de app.css (claro/oscuro).
	import { page } from '$app/state';
	import { goto } from '$app/navigation';

	function reintentar() {
		// Recarga la página actual: fuerza a SvelteKit a re-ejecutar los `load`.
		if (typeof window !== 'undefined') {
			window.location.reload();
		}
	}

	function irDashboard() {
		goto('/');
	}
</script>

<svelte:head>
	<title>Error {page.status} — Dinamo Rent ERP</title>
</svelte:head>

<div class="min-h-screen flex items-center justify-center bg-fondo p-6">
	<div class="card max-w-md w-full px-8 py-10 text-center">
		<!-- Icono de error -->
		<div class="w-16 h-16 rounded-2xl bg-peligro/10 text-peligro flex items-center justify-center mx-auto mb-5">
			<svg xmlns="http://www.w3.org/2000/svg" class="w-9 h-9" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8" aria-hidden="true">
				<path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z" />
			</svg>
		</div>

		<!-- Código de estado -->
		<p class="text-5xl font-bold text-peligro mb-2 tabular-nums" aria-live="polite">
			{page.status}
		</p>

		<!-- Título contextual según el código -->
		<h1 class="text-lg font-bold text-text-primary mb-1">
			{#if page.status === 404}
				Página no encontrada
			{:else if page.status >= 500}
				Error del servidor
			{:else}
				Algo salió mal
			{/if}
		</h1>

		<!-- Mensaje de error (si existe) -->
		{#if page.error?.message}
			<p class="text-sm text-text-secondary leading-relaxed mb-6 break-words">
				{page.error.message}
			</p>
		{:else}
			<p class="text-sm text-text-secondary leading-relaxed mb-6">
				Ocurrió un error inesperado. Intenta recargar la página o vuelve al dashboard.
			</p>
		{/if}

		<!-- Acciones -->
		<div class="flex flex-col sm:flex-row gap-2 justify-center">
			<button class="btn-primary" onclick={reintentar}>
				<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2" aria-hidden="true">
					<path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" />
				</svg>
				Reintentar
			</button>
			<button class="btn-ghost" onclick={irDashboard}>
				<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2" aria-hidden="true">
					<path stroke-linecap="round" stroke-linejoin="round" d="M2.25 12l8.954-8.955c.44-.439 1.152-.439 1.591 0L21.75 12M4.5 9.75v10.125c0 .621.504 1.125 1.125 1.125H9.75v-4.875c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125V21h4.125c.621 0 1.125-.504 1.125-1.125V9.75M8.25 21h8.25" />
				</svg>
				Ir al dashboard
			</button>
		</div>
	</div>
</div>
