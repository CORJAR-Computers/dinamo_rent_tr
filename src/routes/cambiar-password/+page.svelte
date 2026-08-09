<script lang="ts">
	import { goto } from '$app/navigation';
	import { authApi, ApiError } from '$lib/api';
	import { session } from '$lib/stores/session.svelte';

	let currentPassword = $state('');
	let newPassword = $state('');
	let confirmPassword = $state('');
	let loading = $state(false);
	let error = $state('');
	let showPassword = $state(false);

	const username = $derived(session.user?.username ?? '');

	// El guard principal vive en +page.ts (load): sin sesión o sin exigencia
	// de cambio → redirect antes de renderizar. El load lee el store (localStorage)
	// de forma síncrona; tras el mount, el layout valida contra el backend y
	// puede corregir el flag (p. ej. F5 con localStorage desactualizado). Este
	// $effect es el respaldo reactivo para ese caso y para el post-submit.
	$effect(() => {
		if (session.debeCambiarPassword !== true) {
			goto('/dashboard', { replaceState: true });
		}
	});

	async function handleSubmit() {
		error = '';
		if (!currentPassword || !newPassword || !confirmPassword) {
			error = 'Todos los campos son obligatorios.';
			return;
		}
		if (newPassword !== confirmPassword) {
			error = 'La confirmación no coincide con la nueva contraseña.';
			return;
		}
		if (newPassword === currentPassword) {
			error = 'La nueva contraseña debe ser diferente a la actual.';
			return;
		}
		loading = true;
		try {
			await authApi.changePassword(username, currentPassword, newPassword);
			session.debeCambiarPassword = false;
			goto('/dashboard');
		} catch (err) {
			if (err instanceof ApiError) {
				error = err.message;
			} else {
				error = 'No se pudo cambiar la contraseña.';
			}
		} finally {
			loading = false;
			currentPassword = '';
			newPassword = '';
			confirmPassword = '';
		}
	}

	const passwordHints = [
		{ label: 'Mínimo 8 caracteres', check: () => newPassword.length >= 8 },
		{ label: 'Una mayúscula', check: () => /[A-Z]/.test(newPassword) },
		{ label: 'Una minúscula', check: () => /[a-z]/.test(newPassword) },
		{ label: 'Un número', check: () => /\d/.test(newPassword) },
		{ label: 'Un símbolo', check: () => /[!@#$%^&*(),.?":{}|<>]/.test(newPassword) }
	];
</script>

<svelte:head>
	<title>Cambiar contraseña — Dinamo Rent ERP</title>
</svelte:head>

<div class="min-h-screen flex items-center justify-center bg-fondo p-4 relative overflow-hidden">
	<div class="absolute -top-40 -right-40 w-[500px] h-[500px] rounded-full bg-primary/10 blur-3xl pointer-events-none" aria-hidden="true"></div>

	<div class="relative w-full max-w-md">
		<div class="card p-8 shadow-xl">
			<div class="flex items-center gap-3 mb-6">
				<div class="w-11 h-11 rounded-xl bg-alerta/10 text-alerta flex items-center justify-center">
					<svg xmlns="http://www.w3.org/2000/svg" class="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" /></svg>
				</div>
				<div>
					<h2 class="text-xl font-bold text-text-primary">Cambio de contraseña</h2>
					<p class="text-sm text-text-secondary">Por seguridad, debes establecer una nueva contraseña.</p>
				</div>
			</div>

			<form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }}>
				<div class="mb-4">
					<label class="label" for="current">Contraseña actual</label>
					<input id="current" class="input" type="password" bind:value={currentPassword} autocomplete="current-password" disabled={loading} />
				</div>
				<div class="mb-4">
					<label class="label" for="new">Nueva contraseña</label>
					<input id="new" class="input" type={showPassword ? 'text' : 'password'} bind:value={newPassword} autocomplete="new-password" disabled={loading} />
				</div>
				<div class="mb-2">
					<label class="label" for="confirm">Confirmar nueva contraseña</label>
					<input id="confirm" class="input" type={showPassword ? 'text' : 'password'} bind:value={confirmPassword} autocomplete="new-password" disabled={loading} />
				</div>

				<label class="flex items-center gap-2 mt-2 mb-4 text-sm text-text-secondary cursor-pointer select-none">
					<input type="checkbox" bind:checked={showPassword} class="accent-primary" />
					Mostrar contraseñas
				</label>

				{#if error}
					<div class="mb-4 rounded-lg bg-peligro/10 border border-peligro/30 px-3 py-2.5 text-sm text-peligro" role="alert">
						{error}
					</div>
				{/if}

				<!-- Requisitos -->
				<div class="mb-5 grid grid-cols-1 gap-1.5">
					{#each passwordHints as hint}
						<div class="flex items-center gap-2 text-xs" class:text-exito={hint.check()} class:text-text-secondary={!hint.check()}>
							{#if hint.check()}
								<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2.5"><path stroke-linecap="round" stroke-linejoin="round" d="M4.5 12.75l6 6 9-13.5" /></svg>
							{:else}
								<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M9.879 7.519c1.171-1.025 3.071-1.025 4.242 0 1.172 1.025 1.172 2.687 0 3.712-.203.179-.43.326-.67.442-.745.361-1.45.999-1.45 1.827v.75M21 12a9 9 0 11-18 0 9 9 0 0118 0zm-9 5.25h.008v.008H12v-.008z" /></svg>
							{/if}
							<span>{hint.label}</span>
						</div>
					{/each}
				</div>

				<button type="submit" class="btn-primary w-full py-2.5" disabled={loading}>
					{#if loading}Guardando...{:else}Guardar contraseña{/if}
				</button>
			</form>
		</div>
	</div>
</div>
