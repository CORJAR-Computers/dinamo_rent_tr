<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { session } from '$lib/stores/session.svelte';
	import { empresa } from '$lib/stores/empresa.svelte';
	import { authApi } from '$lib/api';
	import { validarSesion } from '$lib/utils/guards';
	import Toast from '$lib/components/Toast.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import AtajosModal, { esAtajoAyuda } from '$lib/components/AtajosModal.svelte';
	import PaletaComandos, { esAtajoPaleta } from '$lib/components/PaletaComandos.svelte';
	import ConfirmarCierre from '$lib/components/ConfirmarCierre.svelte';

	// Props de SvelteKit (snippet de la página hija)
	let { children } = $props();

	type Tema = 'light' | 'dark' | 'auto';

	let ready = $state(false);
	let checking = $state(true);
	let sidebarOpen = $state(true);
	let ayudaOpen = $state(false);
	let paletaOpen = $state(false);

	// Tema persistido (claro | oscuro | auto). Se lee en la inicialización del
	// estado (no en onMount) para que el primer $effect aplique el tema correcto
	// desde el primer frame y no dependa del orden onMount→$effect.
	function temaInicial(): Tema {
		if (typeof window === 'undefined') return 'light';
		const t = localStorage.getItem('dinamo.theme');
		return t === 'dark' || t === 'auto' ? t : 'light';
	}
	let tema = $state<Tema>(temaInicial());
	// Estado real del sistema operativo (solo relevante en modo 'auto')
	let sistemaOscuro = $state(
		typeof window !== 'undefined' && typeof matchMedia !== 'undefined'
			? window.matchMedia('(prefers-color-scheme: dark)').matches
			: false
	);

	const darkMode = $derived(tema === 'dark' || (tema === 'auto' && sistemaOscuro));

	// ── Guard de sesión (validación centralizada en guards.ts) ──
	onMount(async () => {
		await validarSesion();
		checking = false;
		ready = true;
		// Branding de la empresa (nombre + logo) para el menú lateral;
		// best-effort: ante error se conserva el fallback estático.
		void empresa.cargarPublica();
	});

	// ── Tema por usuario (persistido en BD, tabla usuarios) ──
	// La BD es la fuente de verdad: al iniciar sesión (o al validar una sesión
	// persistida) se sobreescribe el valor local con el del usuario. El flag
	// evita recargar en cada re-render; se resetea al cerrar sesión para que el
	// próximo login vuelva a leer la preferencia.
	let temaCargadoDeBD = false;
	$effect(() => {
		if (!session.isAuthenticated || !session.token) {
			temaCargadoDeBD = false;
			ayudaOpen = false; // no abrir la ayuda «de memoria» en el próximo login
			paletaOpen = false; // idem para la paleta de comandos
			return;
		}
		if (temaCargadoDeBD) return;
		temaCargadoDeBD = true;
		// Snapshot para ignorar respuestas obsoletas: si el usuario cambia el tema
		// o la sesión cambia mientras la promesa vuela, no se sobreescribe.
		const tokenAlCargar = session.token;
		const temaAlCargar = tema;
		authApi
			.obtenerTema(tokenAlCargar)
			.then((t) => {
				if (session.token !== tokenAlCargar || tema !== temaAlCargar) return;
				if (t === 'light' || t === 'dark' || t === 'auto') tema = t;
			})
			.catch((e) => {
				// Sin conexión/BD o sesión recién invalidada: conservar el valor local
				temaCargadoDeBD = false;
				console.warn('No se pudo cargar el tema del usuario:', e);
			});
	});

	// En modo automático, seguir en vivo los cambios del sistema operativo
	$effect(() => {
		if (typeof window === 'undefined' || tema !== 'auto') return;
		const mql = window.matchMedia('(prefers-color-scheme: dark)');
		sistemaOscuro = mql.matches;
		const onChange = (e: MediaQueryListEvent) => (sistemaOscuro = e.matches);
		mql.addEventListener('change', onChange);
		return () => mql.removeEventListener('change', onChange);
	});

	$effect(() => {
		if (typeof document !== 'undefined') {
			document.documentElement.classList.toggle('dark', darkMode);
			localStorage.setItem('dinamo.theme', tema);
		}
	});

	// Ciclo: claro → oscuro → auto → claro
	const ordenTemas: Tema[] = ['light', 'dark', 'auto'];
	function cambiarTema() {
		const i = ordenTemas.indexOf(tema);
		tema = ordenTemas[(i + 1) % ordenTemas.length];
		// Persistir por usuario en BD (best-effort; el localStorage queda como
		// caché local para el primer frame de la próxima sesión)
		if (session.token) {
			authApi.guardarTema(session.token, tema).catch((e) => {
				// Best-effort: ante error de red/BD, la preferencia local (localStorage)
				// sigue funcionando esta sesión; solo se pierde la sincronización.
				console.warn('No se pudo guardar el tema del usuario:', e);
			});
		}
	}

	const etiquetaTema = $derived.by(() => {
		if (tema === 'dark') return 'Tema oscuro';
		if (tema === 'auto') return 'Tema automático (sigue al sistema)';
		return 'Tema claro';
	});

	// Indicador visual del modo activo: etiqueta corta + color del badge
	const etiquetaCorta = $derived(tema === 'dark' ? 'Oscuro' : tema === 'auto' ? 'Auto' : 'Claro');
	const colorIndicador = $derived(
		tema === 'dark' ? 'bg-violet-400' : tema === 'auto' ? 'bg-emerald-400' : 'bg-amber-400'
	);

	// ── Estructura de menú (paridad con MainWindow._MENU_STRUCTURE) ──
	// Tipado explícito: los ítems pueden restringirse por rol (adminOnly para
	// administración, roles para informes) y el acceso por URL se valida en
	// la página y en el comando del backend.
	interface ItemMenu {
		label: string;
		href: string;
		icon: string;
		adminOnly?: boolean;
		roles?: string[];
	}
	const menu: { section: string; items: ItemMenu[] }[] = [
		{ section: 'PRINCIPAL', items: [{ label: 'Dashboard', href: '/dashboard', icon: 'dashboard' }] },
		{
			section: 'OPERACIÓN',
			items: [
				{ label: 'Calendario', href: '/calendario', icon: 'calendar' },
				{ label: 'Rentas', href: '/rentas', icon: 'rentas' },
				{ label: 'Reservas', href: '/reservas', icon: 'reservas' },
				{ label: 'Clientes', href: '/clientes', icon: 'clientes' },
				{ label: 'Autos', href: '/autos', icon: 'autos' },
				{ label: 'Mantenimiento', href: '/mantenimiento', icon: 'mantenimiento' }
			]
		},
		{
			section: 'ADMINISTRACIÓN',
			items: [
				{ label: 'Usuarios', href: '/usuarios', icon: 'usuarios', adminOnly: true },
				{ label: 'Auditoría', href: '/auditoria', icon: 'auditoria', adminOnly: true },
				{ label: 'Empresa', href: '/empresa', icon: 'empresa', adminOnly: true },
				{ label: 'Comparendos', href: '/comparendos', icon: 'comparendos' },
				{ label: 'Alertas', href: '/alertas', icon: 'alertas' }
			]
		},
		{
			section: 'FINANZAS',
			items: [
				{
					label: 'Informes',
					href: '/informes',
					icon: 'informes',
					// Espejo del default de config.ini (business.roles_con_informes).
					// El guard real de la página y del comando lee la config.
					roles: ['Administrador', 'Supervisor']
				},
				{ label: 'Gastos', href: '/gastos', icon: 'gastos' }
			]
		}
	];

	const current = $derived(page.url.pathname);
const isFullscreen = $derived(['/login', '/cambiar-password'].includes(page.url.pathname));

	async function handleLogout() {
		await session.logout();
		goto('/login');
	}

	// ── Atajos globales: F1/Ctrl+/ (ayuda) y Ctrl+K (paleta) ──
	// Solo con sesión activa; abrir uno cierra el otro para evitar modales anidados.
	function onGlobalKeydown(e: KeyboardEvent) {
		if (!session.isAuthenticated) return;
		if (esAtajoAyuda(e)) {
			e.preventDefault();
			ayudaOpen = !ayudaOpen;
			if (ayudaOpen) paletaOpen = false;
		} else if (esAtajoPaleta(e)) {
			e.preventDefault();
			paletaOpen = !paletaOpen;
			if (paletaOpen) ayudaOpen = false;
		}
	}

	function abrirAyuda() {
		ayudaOpen = true;
	}

	// ── Iconos del sidebar ──
	// Migrado a componente `<Icon>` (TAREA E5 del Grupo E): evita {@html} con
	// strings SVG hardcodeados y centraliza todos los iconos en
	// src/lib/components/Icon.svelte. Los 13 nombres del array `menu` arriba
	// (dashboard, calendar, rentas, reservas, clientes, autos, mantenimiento,
	// usuarios, auditoria, comparendos, alertas, informes, gastos) están
	// registrados como entradas en el mapa `paths` de Icon.svelte.
	//
	// TODO: revisar si otros componentes (PaletaComandos, AtajosModal) también
	// construyen SVG inline con `class="w-5 h-5"` y migrarlos a `<Icon>`.

	function pageTitle(path: string): string {
		const map: Record<string, string> = {
			'/dashboard': 'Dashboard',
			'/calendario': 'Calendario',
			'/rentas': 'Rentas',
			'/reservas': 'Reservas',
			'/clientes': 'Clientes',
			'/autos': 'Flota de Autos',
			'/mantenimiento': 'Mantenimiento',
			'/usuarios': 'Usuarios',
			'/auditoria': 'Auditoría',
			'/empresa': 'Empresa',
			'/comparendos': 'Comparendos',
			'/alertas': 'Alertas',
			'/informes': 'Informes',
			'/gastos': 'Caja Menor (Gastos)'
		};
		for (const [k, v] of Object.entries(map)) {
			if (path.startsWith(k)) return v;
		}
		return 'Dinamo Rent ERP';
	}
</script>

<Toast />
<!-- Confirmación al pulsar la X de la ventana (Sí/No). Se monta a nivel raíz,
     fuera del if/else de sesión, para cubrir también login / cambiar-password. -->
<ConfirmarCierre />
<svelte:window onkeydown={onGlobalKeydown} />

<svelte:head>
	<meta name="theme-color" content={darkMode ? '#0b1220' : '#f5f7fb'} />
</svelte:head>

{#if isFullscreen}
	{@render children()}
{:else if checking}
	<div class="min-h-screen flex items-center justify-center bg-fondo">
		<div class="flex flex-col items-center gap-3">
			<svg class="animate-spin h-8 w-8 text-primary" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
				<circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
				<path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
			</svg>
			<p class="text-sm text-text-secondary">Verificando sesión...</p>
		</div>
	</div>
{:else if ready && session.isAuthenticated}
	<div class="flex h-screen overflow-hidden">
		<!-- Sidebar -->
		<aside
			class="bg-primary text-white flex flex-col transition-all duration-300 shrink-0"
			class:w-64!={sidebarOpen}
			class:w-16!={!sidebarOpen}
		>
			<!-- Logo -->
			<div class="flex items-center gap-3 px-4 py-5 h-16 border-b border-white/10">
				<div class="w-9 h-9 rounded-lg bg-white/15 flex items-center justify-center shrink-0 overflow-hidden p-1">
					<img src={empresa.logoSrc} alt="Logo" class="w-full h-full object-contain" />
				</div>
				{#if sidebarOpen}
					<div class="overflow-hidden">
						<p class="font-bold text-sm leading-tight">{empresa.nombreMostrar}</p>
						<p class="text-[11px] text-white/60">ERP v3.2.0</p>
					</div>
				{/if}
			</div>

			<!-- Navegación -->
			<nav class="flex-1 overflow-y-auto py-4 px-2 space-y-5">
				{#each menu as group}
					{#if sidebarOpen}
						<p class="px-3 text-[10px] font-bold tracking-widest text-white/40 uppercase">{group.section}</p>
					{/if}
					<div class="space-y-1">
						{#each group.items as item}
							{#if (!item.roles || item.roles.includes(session.user?.rol ?? '')) && (!item.adminOnly || session.user?.rol === 'Administrador')}
								<a
									href={item.href}
									title={item.label}
									class={'flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-all duration-150 hover:bg-white/10 ' +
										(current === item.href ? 'bg-white/20 text-white ' : 'text-white/70 ') +
										(!sidebarOpen ? 'justify-center' : '')}
								>
									<span class="shrink-0"><Icon name={item.icon} class="w-5 h-5" /></span>
									{#if sidebarOpen}<span class="truncate">{item.label}</span>{/if}
								</a>
							{/if}
						{/each}
					</div>
				{/each}
			</nav>

			<!-- Footer usuario -->
			<div class="border-t border-white/10 p-3">
				<div class="flex items-center gap-3" class:justify-center={!sidebarOpen}>
					<button
						onclick={cambiarTema}
						class="p-2 rounded-lg text-white/60 hover:text-white hover:bg-white/10 transition-colors shrink-0 inline-flex items-center gap-1.5"
						title={`${etiquetaTema} · clic para cambiar`}
						aria-label={`${etiquetaTema} · clic para cambiar`}
						data-tema={tema}
					>
						<span class="relative shrink-0">
							{#if tema === 'dark'}
								<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M21.752 15.002A9.718 9.718 0 0118 15.75c-5.385 0-9.75-4.365-9.75-9.75 0-1.33.266-2.597.748-3.752A9.753 9.753 0 003 11.25C3 16.635 7.365 21 12.75 21a9.753 9.753 0 009.002-5.998z" /></svg>
							{:else if tema === 'auto'}
								<Icon name="computer" class="w-5 h-5" />
							{:else}
								<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M12 3v2.25m6.364.386l-1.591 1.591M21 12h-2.25m-.386 6.364l-1.591-1.591M12 18.75V21m-4.773-4.227l-1.591 1.591M5.25 12H3m4.227-4.773L5.636 5.636M15.75 12a3.75 3.75 0 11-7.5 0 3.75 3.75 0 017.5 0z" /></svg>
							{/if}
							<span
								class="absolute -top-1 -right-1 w-2.5 h-2.5 rounded-full ring-2 ring-primary {colorIndicador} transition-colors"
								aria-hidden="true"
							></span>
						</span>
						{#if sidebarOpen}
							<span class="text-[10px] font-bold uppercase tracking-wider">{etiquetaCorta}</span>
						{/if}
					</button>
					<div class="w-9 h-9 rounded-full bg-primary-focus flex items-center justify-center text-sm font-bold shrink-0 uppercase">
						{(session.user?.nombre || session.user?.username || '?').slice(0, 2)}
					</div>
					{#if sidebarOpen}
						<div class="flex-1 overflow-hidden">
							<p class="text-sm font-semibold truncate">{session.user?.nombre || session.user?.username}</p>
							<p class="text-[11px] text-white/60 truncate">{session.user?.rol}</p>
						</div>
						<button
							onclick={handleLogout}
							class="text-white/60 hover:text-white transition-colors p-1 rounded"
							title="Cerrar sesión"
						>
							<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M15.75 9V5.25A2.25 2.25 0 0013.5 3h-6a2.25 2.25 0 00-2.25 2.25v13.5A2.25 2.25 0 007.5 21h6a2.25 2.25 0 002.25-2.25V15m3 0l3-3m0 0l-3-3m3 3H9" /></svg>
						</button>
					{/if}
				</div>
			</div>
		</aside>

		<!-- Contenido -->
		<div class="flex-1 flex flex-col overflow-hidden bg-fondo">
			<!-- Header -->
			<header class="h-16 bg-surface border-b border-border flex items-center gap-4 px-4 shrink-0">
				<button
					onclick={() => (sidebarOpen = !sidebarOpen)}
					class="p-2 rounded-lg text-text-secondary hover:bg-alt-row hover:text-text-primary transition-colors"
					title={sidebarOpen ? 'Contraer menú' : 'Expandir menú'}
				>
					<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6.75h16.5M3.75 12h16.5m-16.5 5.25h16.5" /></svg>
				</button>
				<h1 class="text-lg font-semibold text-text-primary">
					{pageTitle(current)}
				</h1>
				<div class="flex-1"></div>
				<button
					onclick={() => (paletaOpen = true)}
					class="hidden sm:flex items-center gap-2 px-2.5 py-1.5 rounded-lg text-text-secondary hover:bg-alt-row hover:text-text-primary transition-colors border border-border/60"
					title="Buscar página (Ctrl+K)"
					aria-label="Buscar página (Ctrl+K)"
				>
					<Icon name="search" class="w-4 h-4" />
					<span class="hidden md:inline text-xs">Buscar</span>
					<kbd class="hidden md:inline-flex items-center rounded border border-border bg-alt-row/60 px-1.5 py-0.5 text-[10px] font-mono leading-none">Ctrl K</kbd>
				</button>
				<button
					onclick={abrirAyuda}
					class="p-2 rounded-lg text-text-secondary hover:bg-alt-row hover:text-text-primary transition-colors"
					title="Atajos de teclado (F1)"
					aria-label="Atajos de teclado (F1)"
				>
					<Icon name="lightbulb" class="w-5 h-5" />
				</button>
				<span class="text-xs px-2.5 py-1 rounded-full bg-primary/10 text-primary font-medium">{session.user?.rol}</span>
			</header>

			<!-- Vista actual -->
			<main class="flex-1 overflow-y-auto p-6">
				{@render children()}
			</main>
		</div>
	</div>

	<!-- Ayuda de atajos de teclado (F1 / Ctrl+/) -->
	<AtajosModal open={ayudaOpen} onClose={() => (ayudaOpen = false)} />

	<!-- Paleta de comandos (Ctrl+K) -->
	<PaletaComandos
		open={paletaOpen}
		onClose={() => (paletaOpen = false)}
		menu={menu}
		rol={session.user?.rol}
		actualPath={current}
	/>
{:else}
	{@render children()}
{/if}

