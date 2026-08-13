<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { authApi, ApiError, type LoginStatus } from '$lib/api';
	import { session } from '$lib/stores/session.svelte';
	import { empresa } from '$lib/stores/empresa.svelte';

	let username = $state('');
	let password = $state('');
	let loading = $state(false);
	let error = $state('');
	let showPassword = $state(false);
	let loginStatus = $state<LoginStatus | null>(null);

	// Branding dinámico: nombre y logo configurados por la empresa (setup inicial),
	// con fallback estático mientras no haya configuración.
	onMount(() => {
		void empresa.cargarPublica();
	});

	// Debounce: consultar estado de login mientras se escribe el usuario
	let statusTimer: ReturnType<typeof setTimeout> | undefined;
	$effect(() => {
		const u = username.trim();
		clearTimeout(statusTimer);
		if (u.length >= 2) {
			statusTimer = setTimeout(async () => {
				try {
					loginStatus = await authApi.getLoginStatus(u);
				} catch {
					loginStatus = null;
				}
			}, 400);
		} else {
			loginStatus = null;
		}
		return () => clearTimeout(statusTimer);
	});

	async function handleSubmit() {
		error = '';
		const u = username.trim();
		if (!u || !password) {
			error = 'Ingresa tu usuario y contraseña.';
			return;
		}
		loading = true;
		try {
			const result = await authApi.login(u, password);
			session.setSession(result);
			if (result.debeCambiarPassword) {
				goto('/cambiar-password');
			} else {
				goto('/dashboard');
			}
		} catch (err) {
			if (err instanceof ApiError) {
				error = err.message;
			} else {
				error = 'No se pudo conectar con la aplicación.';
			}
			// Refrescar estado (bloqueos, intentos restantes)
			try {
				loginStatus = await authApi.getLoginStatus(u);
			} catch {
				/* noop */
			}
		} finally {
			loading = false;
			password = '';
		}
	}

	const locked = $derived(loginStatus?.isLocked ?? false);
	const remaining = $derived(loginStatus?.remainingAttempts ?? null);
</script>

<svelte:head>
	<title>Iniciar sesión — {empresa.nombreMostrar}</title>
</svelte:head>

<div class="min-h-screen flex items-center justify-center bg-fondo relative overflow-hidden">
	<!-- Fondo decorativo -->
	<div class="absolute inset-0 pointer-events-none" aria-hidden="true">
		<div class="absolute -top-40 -right-40 w-[500px] h-[500px] rounded-full bg-primary/10 blur-3xl"></div>
		<div class="absolute -bottom-40 -left-40 w-[400px] h-[400px] rounded-full bg-primary-focus/10 blur-3xl"></div>
	</div>

	<div class="relative w-full max-w-md px-4">
		<!-- Logo -->
		<div class="flex flex-col items-center mb-8">
			<div class="w-28 h-28 mb-4 rounded-3xl bg-white shadow-lg shadow-primary/20 ring-1 ring-border overflow-hidden">
				<img src={empresa.logoSrc} alt={empresa.nombreMostrar} class="w-full h-full object-contain" />
			</div>
			<h1 class="text-2xl font-bold text-text-primary tracking-tight">{empresa.nombreMostrar}{#if !empresa.nombre} ERP{/if}</h1>
			<p class="text-sm text-text-secondary mt-1">Sistema de gestión de flota y renta de vehículos</p>
		</div>

		<!-- Tarjeta de login -->
		<div class="card p-8 shadow-xl">
			<h2 class="text-lg font-semibold text-text-primary mb-6">Iniciar sesión</h2>

			<form onsubmit={(e) => { e.preventDefault(); handleSubmit(); }}>
				<div class="mb-4">
					<label class="label" for="username">Usuario</label>
					<input
						id="username"
						class="input"
						type="text"
						placeholder="Ej: admin"
						autocomplete="username"
						bind:value={username}
						disabled={loading}
					/>
				</div>

				<div class="mb-2">
					<label class="label" for="password">Contraseña</label>
					<div class="relative">
						<input
							id="password"
							class="input pr-10"
							type={showPassword ? 'text' : 'password'}
							placeholder="••••••••"
							autocomplete="current-password"
							bind:value={password}
							disabled={loading || locked}
						/>
						<button
							type="button"
							class="absolute inset-y-0 right-0 pr-3 flex items-center text-text-secondary hover:text-text-primary transition-colors"
							onclick={() => (showPassword = !showPassword)}
							aria-label={showPassword ? 'Ocultar contraseña' : 'Mostrar contraseña'}
						>
							{#if showPassword}
								<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" /><path stroke-linecap="round" stroke-linejoin="round" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" /></svg>
							{:else}
								<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M3.98 8.223A10.477 10.477 0 001.934 12C3.226 16.338 7.244 19.5 12 19.5c.993 0 1.953-.138 2.863-.395M6.228 6.228A10.45 10.45 0 0112 4.5c4.756 0 8.773 3.162 10.065 7.498a10.523 10.523 0 01-4.293 5.774M6.228 6.228L3 3m3.228 3.228l3.65 3.65m7.894 7.894L21 21m-3.228-3.228l-3.65-3.65m0 0a3 3 0 10-4.243-4.243m4.242 4.242L9.88 9.88" /></svg>
							{/if}
						</button>
					</div>
				</div>

				<!-- Errores y estados -->
				{#if error}
					<div
						class="mt-3 mb-1 rounded-lg bg-peligro/10 border border-peligro/30 px-3 py-2.5 text-sm text-peligro flex items-start gap-2"
						role="alert"
					>
						<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4 mt-0.5 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z" /></svg>
						<span>{error}</span>
					</div>
				{/if}

				{#if locked && loginStatus}
					<div class="mt-3 mb-1 rounded-lg bg-alerta/10 border border-alerta/30 px-3 py-2.5 text-sm text-alerta flex items-center gap-2" role="alert">
						<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" /></svg>
						<span>
							Cuenta bloqueada. Intenta nuevamente en {Math.ceil(loginStatus.lockoutRemainingSeconds / 60)} min.
						</span>
					</div>
				{/if}

				{#if !locked && remaining !== null && remaining >= 0 && remaining < 5 && !error}
					<p class="mt-3 text-xs text-text-secondary flex items-center gap-1.5">
						<svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 text-alerta" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M12 9v3.75m9-.75a9 9 0 11-18 0 9 9 0 0118 0zm-9 3.75h.008v.008H12v-.008z" /></svg>
						Intentos restantes antes del bloqueo: <strong>{remaining}</strong>
					</p>
				{/if}

				<button
					type="submit"
					class="btn-primary w-full mt-5 py-2.5"
					disabled={loading || locked}
				>
					{#if loading}
						<svg class="animate-spin -ml-1 mr-2 h-4 w-4 text-white" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
							<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
							<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
						</svg>
						Verificando...
					{:else}
						Ingresar
					{/if}
				</button>
			</form>

			<footer class="mt-6 pt-5 border-t border-border text-center">
				<p class="text-xs text-text-secondary">© {new Date().getFullYear()} {empresa.nombreMostrar} · v3.2.0</p>
			</footer>
		</div>
	</div>
</div>
