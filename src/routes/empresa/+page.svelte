<script lang="ts">
	import { onMount } from 'svelte';
	import { ApiError, empresaApi, type EmpresaConfig } from '$lib/api';
	import { toast } from '$lib/stores/toast.svelte';
	import { empresa } from '$lib/stores/empresa.svelte';
	import { sid } from '$lib/stores/session.svelte';
	import { guardRole, guardSesion, haySesion } from '$lib/utils/guards';
	import FormField from '$lib/components/FormField.svelte';

	let cargando = $state(true);
	let guardando = $state(false);
	let error = $state('');

	let form = $state({
		nombre: '',
		nit: '',
		direccion: '',
		telefono: '',
		email: '',
		web: ''
	});

	// Logo: data URL (persistida) mientras no se cambie; null = sin logo.
	let logoDataUrl = $state<string | null>(null);

	async function cargar() {
		if (!haySesion()) return;
		cargando = true;
		error = '';
		try {
			const cfg: EmpresaConfig = await empresaApi.obtener(sid());
			form.nombre = cfg.nombre ?? '';
			form.nit = cfg.nit ?? '';
			form.direccion = cfg.direccion ?? '';
			form.telefono = cfg.telefono ?? '';
			form.email = cfg.email ?? '';
			form.web = cfg.web ?? '';
			logoDataUrl = cfg.logo;
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'No se pudo cargar la configuración de la empresa.';
		} finally {
			cargando = false;
		}
	}

	onMount(async () => {
		// Solo administradores configuran la empresa (roles_con_usuarios).
		if (!guardSesion()) return;
		if (!guardRole(['Administrador'], '/dashboard')) return;
		await cargar();
	});

	function onLogoChange(e: Event) {
		const input = e.target as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) return;
		const permitidos = ['image/png', 'image/jpeg', 'image/webp', 'image/svg+xml'];
		if (!permitidos.includes(file.type)) {
			toast.error('Formato de logo no soportado. Usa PNG, JPG, WebP o SVG.');
			input.value = '';
			return;
		}
		if (file.size > 2 * 1024 * 1024) {
			toast.error('El logo supera el máximo de 2 MB.');
			input.value = '';
			return;
		}
		const reader = new FileReader();
		reader.onload = () => (logoDataUrl = String(reader.result));
		reader.readAsDataURL(file);
	}

	function quitarLogo() {
		logoDataUrl = null;
	}

	async function guardar() {
		error = '';
		guardando = true;
		try {
			const cfg = await empresaApi.guardar(sid(), {
				nombre: form.nombre.trim() || null,
				nit: form.nit.trim() || null,
				direccion: form.direccion.trim() || null,
				telefono: form.telefono.trim() || null,
				email: form.email.trim() || null,
				web: form.web.trim() || null,
				logo: logoDataUrl
			});
			// Refrescar branding en caliente (login / menú lateral / impresiones).
			empresa.actualizar(cfg);
			logoDataUrl = cfg.logo;
			toast.success('Configuración de la empresa guardada.');
		} catch (e) {
			error = e instanceof ApiError ? e.message : 'No se pudo guardar la configuración.';
		} finally {
			guardando = false;
		}
	}
</script>

<svelte:head>
	<title>Empresa — Dinamo Rent ERP</title>
</svelte:head>

<div class="space-y-5 max-w-3xl">
	<!-- Encabezado -->
	<div>
		<h2 class="text-2xl font-bold text-text-primary">Configuración de la empresa</h2>
		<p class="text-sm text-text-secondary mt-0.5">
			Nombre, datos de contacto y logo. Se usan en el login, el menú lateral, los contratos y las órdenes de renta/reserva.
		</p>
	</div>

	{#if cargando}
		<div class="card flex items-center justify-center py-16">
			<div class="flex flex-col items-center gap-3">
				<svg class="animate-spin h-8 w-8 text-primary" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
				<p class="text-sm text-text-secondary">Cargando configuración...</p>
			</div>
		</div>
	{:else}
		{#if error}
			<div class="rounded-lg bg-peligro/10 border border-peligro/30 px-3 py-2.5 text-sm text-peligro" role="alert">{error}</div>
		{/if}

		<div class="card p-6">
			<!-- Logo -->
			<h3 class="text-xs font-bold uppercase tracking-wider text-primary mb-4">Logo</h3>
			<div class="flex items-center gap-5 mb-8">
				<div class="w-24 h-24 rounded-2xl bg-white shadow-lg ring-1 ring-border overflow-hidden flex items-center justify-center shrink-0">
					{#if logoDataUrl}
						<img src={logoDataUrl} alt="Logo de la empresa" class="w-full h-full object-contain" />
					{:else}
						<div class="text-text-secondary/50 flex flex-col items-center gap-1 text-[10px]">
							<svg xmlns="http://www.w3.org/2000/svg" class="w-7 h-7" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.5"><path stroke-linecap="round" stroke-linejoin="round" d="M2.25 15.75l5.159-5.159a2.25 2.25 0 013.182 0l5.159 5.159m-1.5-1.5l1.409-1.409a2.25 2.25 0 013.182 0l2.909 2.909M3.75 21h16.5A1.5 1.5 0 0021.75 19.5V4.5A1.5 1.5 0 0020.25 3H3.75A1.5 1.5 0 002.25 4.5v15A1.5 1.5 0 003.75 21z" /></svg>
							<span>Sin logo</span>
						</div>
					{/if}
				</div>
				<div class="flex flex-col gap-2">
					<label class="btn-outline cursor-pointer inline-flex items-center gap-2 w-fit">
						<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5m-13.5-9L12 3m0 0l4.5 4.5M12 3v13.5" /></svg>
						Subir logo
						<input type="file" accept="image/png,image/jpeg,image/webp,image/svg+xml" class="hidden" onchange={onLogoChange} />
					</label>
					{#if logoDataUrl}
						<button class="text-sm text-peligro hover:text-peligro/80 transition-colors inline-flex items-center gap-1.5 w-fit" onclick={quitarLogo}>
							<svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2"><path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" /></svg>
							Quitar logo
						</button>
					{/if}
					<p class="text-xs text-text-secondary max-w-sm">PNG, JPG, WebP o SVG · máximo 2 MB. Se muestra en el login, el menú lateral y las impresiones.</p>
				</div>
			</div>

			<!-- Datos -->
			<h3 class="text-xs font-bold uppercase tracking-wider text-primary mb-4">Datos de la empresa</h3>
			<div class="grid grid-cols-1 sm:grid-cols-2 gap-x-4 gap-y-4">
				<div class="sm:col-span-2">
					<FormField label="Nombre de la empresa" required hint="Aparece en el login, los contratos y las órdenes.">
						<input class="input" placeholder="Ej: DynaRent S.A.S." bind:value={form.nombre} maxlength="120" />
					</FormField>
				</div>
				<FormField label="NIT">
					<input class="input" placeholder="Ej: 900.123.456-7" bind:value={form.nit} maxlength="40" />
				</FormField>
				<FormField label="Teléfono">
					<input class="input" placeholder="Ej: (601) 234 5678" bind:value={form.telefono} maxlength="40" />
				</FormField>
				<div class="sm:col-span-2">
					<FormField label="Dirección">
						<input class="input" placeholder="Ej: Cra 12 # 34-56, Bogotá" bind:value={form.direccion} maxlength="200" />
					</FormField>
				</div>
				<FormField label="Email">
					<input class="input" type="email" placeholder="contacto@empresa.com" bind:value={form.email} maxlength="120" />
				</FormField>
				<FormField label="Sitio web">
					<input class="input" placeholder="www.empresa.com" bind:value={form.web} maxlength="120" />
				</FormField>
			</div>

			<div class="flex items-center justify-end gap-3 mt-8 pt-5 border-t border-border">
				<button class="btn-primary" onclick={guardar} disabled={guardando}>
					{#if guardando}
						<svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24"><circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle><path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path></svg>
						Guardando...
					{:else}
						Guardar cambios
					{/if}
				</button>
			</div>
		</div>
	{/if}
</div>
