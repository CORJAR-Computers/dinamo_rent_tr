<script lang="ts">
	import '../app.css';
	import { onMount } from 'svelte';
	import { page } from '$app/state';
	import { goto } from '$app/navigation';
	import { session } from '$lib/stores/session.svelte';
	import { authApi } from '$lib/api';
	import { validarSesion } from '$lib/utils/guards';
	import Toast from '$lib/components/Toast.svelte';
	import Icon from '$lib/components/Icon.svelte';
	import AtajosModal, { esAtajoAyuda } from '$lib/components/AtajosModal.svelte';
	import PaletaComandos, { esAtajoPaleta } from '$lib/components/PaletaComandos.svelte';

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
	const menu = [
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
				{ label: 'Comparendos', href: '/comparendos', icon: 'comparendos' },
				{ label: 'Alertas', href: '/alertas', icon: 'alertas' }
			]
		},
		{
			section: 'FINANZAS',
			items: [
				{ label: 'Informes', href: '/informes', icon: 'informes' },
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

	// ── Iconos inline (SVG) ──
	function iconFor(name: string): string {
		const icons: Record<string, string> = {
			dashboard: '<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M3.75 6A2.25 2.25 0 016 3.75h2.25A2.25 2.25 0 0110.5 6v2.25a2.25 2.25 0 01-2.25 2.25H6a2.25 2.25 0 01-2.25-2.25V6zM3.75 15.75A2.25 2.25 0 016 13.5h2.25a2.25 2.25 0 012.25 2.25V18a2.25 2.25 0 01-2.25 2.25H6A2.25 2.25 0 013.75 18v-2.25zM13.5 6a2.25 2.25 0 012.25-2.25H18A2.25 2.25 0 0120.25 6v2.25A2.25 2.25 0 0118 10.5h-2.25a2.25 2.25 0 01-2.25-2.25V6zM13.5 15.75a2.25 2.25 0 012.25-2.25H18a2.25 2.25 0 012.25 2.25V18A2.25 2.25 0 0118 20.25h-2.25A2.25 2.25 0 0113.5 18v-2.25z" /></svg>',
			calendar: '<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M6.75 3v2.25M17.25 3v2.25M3 18.75V7.5a2.25 2.25 0 012.25-2.25h13.5A2.25 2.25 0 0121 7.5v11.25m-18 0A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75m-18 0v-7.5A2.25 2.25 0 015.25 9h13.5A2.25 2.25 0 0121 11.25v7.5" /></svg>',
			rentas: '<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M9 12h3.75M9 15h3.75M9 18h3.75m3 .75H18a2.25 2.25 0 002.25-2.25V6.108c0-1.135-.845-2.098-1.976-2.192a48.424 48.424 0 00-1.123-.08m-5.801 0c-.065.21-.1.433-.1.664 0 .414.336.75.75.75h4.5a.75.75 0 00.75-.75 2.25 2.25 0 00-.1-.664m-5.8 0A2.251 2.251 0 0113.5 2.25H15c1.012 0 1.867.668 2.15 1.586m-5.8 0c-.376.023-.75.05-1.124.08C9.095 4.01 8.25 4.973 8.25 6.108V8.25m0 0H4.875c-.621 0-1.125.504-1.125 1.125v11.25c0 .621.504 1.125 1.125 1.125h9.75c.621 0 1.125-.504 1.125-1.125V9.375c0-.621-.504-1.125-1.125-1.125H8.25z" /></svg>',
			reservas: '<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M8.25 6.75h12M8.25 12h12m-12 5.25h12M3.75 6.75h.007v.008H3.75V6.75zm.375 0a.375.375 0 11-.75 0 .375.375 0 01.75 0zM3.75 12h.007v.008H3.75V12zm.375 0a.375.375 0 11-.75 0 .375.375 0 01.75 0zm-.375 5.25h.007v.008H3.75v-.008zm.375 0a.375.375 0 11-.75 0 .375.375 0 01.75 0z" /></svg>',
			clientes: '<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M15 19.128a9.38 9.38 0 002.625.372 9.337 9.337 0 004.121-.952 4.125 4.125 0 00-7.533-2.493M15 19.128v-.003c0-1.113-.285-2.16-.786-3.07M15 19.128v.106A12.318 12.318 0 018.624 21c-2.331 0-4.512-.645-6.374-1.766l-.001-.109a6.375 6.375 0 0111.964-3.07M12 6.375a3.375 3.375 0 11-6.75 0 3.375 3.375 0 016.75 0zm8.25 2.25a2.625 2.625 0 11-5.25 0 2.625 2.625 0 015.25 0z" /></svg>',
			autos: '<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M8.25 18.75a1.5 1.5 0 01-3 0m3 0a1.5 1.5 0 00-3 0m3 0h6m-9 0H3.375a1.125 1.125 0 01-1.125-1.125V14.25m17.25 4.5a1.5 1.5 0 01-3 0m3 0a1.5 1.5 0 00-3 0m3 0h1.125c.621 0 1.129-.504 1.09-1.124a17.902 17.902 0 00-3.213-9.193 2.056 2.056 0 00-1.58-.86H14.25M16.5 18.75h-2.25m0-11.177v-.958c0-.568-.422-1.048-.987-1.106a48.554 48.554 0 00-10.026 0 1.106 1.106 0 00-.987 1.106v7.635m12-6.677v6.677m0 4.5v-4.5m0 0h-12" /></svg>',
			mantenimiento: '<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M11.42 15.17L17.25 21A2.652 2.652 0 0021 17.25l-5.877-5.877M11.42 15.17l2.496-3.03c.317-.384.74-.626 1.208-.766M11.42 15.17l-4.655 5.653a2.548 2.548 0 11-3.586-3.586l6.837-5.63m5.108-.233c.55-.164 1.163-.188 1.743-.14a4.5 4.5 0 004.486-6.336l-3.276 3.277a3.004 3.004 0 01-2.25-2.25l3.276-3.276a4.5 4.5 0 00-6.336 4.486c.091 1.076-.071 2.264-.904 2.95l-.102.085m-1.745 1.437L5.909 7.5H4.5L2.25 3.75l1.5-1.5L7.5 4.5v1.409l4.26 4.26m-1.745 1.437l1.745-1.437m6.615 8.206L15.75 15.75M4.867 19.125h.008v.008h-.008v-.008z" /></svg>',
			usuarios: '<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M17.982 18.725A7.488 7.488 0 0012 15.75a7.488 7.488 0 00-5.982 2.975m11.963 0a9 9 0 10-11.963 0m11.963 0A8.966 8.966 0 0112 21a8.966 8.966 0 01-5.982-2.275M15 9.75a3 3 0 11-6 0 3 3 0 016 0z" /></svg>',
			auditoria: '<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" /></svg>',
			comparendos: '<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75L11.25 15 15 9.75M21 12a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>',
			alertas: '<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M14.857 17.082a23.848 23.848 0 005.454-1.31A8.967 8.967 0 0118 9.75v-.7V9A6 6 0 006 9v.75a8.967 8.967 0 01-2.312 6.022c1.733.64 3.56 1.085 5.455 1.31m5.714 0a24.255 24.255 0 01-5.714 0m5.714 0a3 3 0 11-5.714 0M3.124 7.5A8.969 8.969 0 015.292 3m13.416 0a8.969 8.969 0 012.168 4.5" /></svg>',
			informes: '<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M3 13.125C3 12.504 3.504 12 4.125 12h2.25c.621 0 1.125.504 1.125 1.125v6.75C7.5 20.496 6.996 21 6.375 21h-2.25A1.125 1.125 0 013 19.875v-6.75zM9.75 8.625c0-.621.504-1.125 1.125-1.125h2.25c.621 0 1.125.504 1.125 1.125v11.25c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 01-1.125-1.125V8.625zM16.5 4.125c0-.621.504-1.125 1.125-1.125h2.25C20.496 3 21 3.504 21 4.125v15.75c0 .621-.504 1.125-1.125 1.125h-2.25a1.125 1.125 0 01-1.125-1.125V4.125z" /></svg>',
			gastos: '<svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M2.25 18.75a60.07 60.07 0 0115.797 2.101c.727.198 1.453-.342 1.453-1.096V18.75M3.75 4.5v.75A.75.75 0 013 6h-.75m0 0v-.375c0-.621.504-1.125 1.125-1.125H20.25M2.25 6v9m18-10.5v.75c0 .414.336.75.75.75h.75m-1.5-1.5h.375c.621 0 1.125.504 1.125 1.125v9.75c0 .621-.504 1.125-1.125 1.125h-.375m1.5-1.5H21a.75.75 0 00-.75.75v.75m0 0H3.75m0 0h-.375a1.125 1.125 0 01-1.125-1.125V15m1.5 1.5v-.75A.75.75 0 003 15h-.75M15 10.5a3 3 0 11-6 0 3 3 0 016 0zm3 0h.008v.008H18V10.5zm-12 0h.008v.008H6V10.5z" /></svg>'
		};
		return icons[name] || icons.dashboard;
	}

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

<Toast />	<svelte:window onkeydown={onGlobalKeydown} />

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
					<img src="/LogoDinamo.png" alt="Logo" class="w-full h-full object-contain" />
				</div>
				{#if sidebarOpen}
					<div class="overflow-hidden">
						<p class="font-bold text-sm leading-tight">Dinamo Rent</p>
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
							{#if !item.adminOnly || session.user?.rol === 'Administrador'}
								<a
									href={item.href}
									title={item.label}
									class={'flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-all duration-150 hover:bg-white/10 ' +
										(current === item.href ? 'bg-white/20 text-white ' : 'text-white/70 ') +
										(!sidebarOpen ? 'justify-center' : '')}
								>
									<span class="shrink-0">{@html iconFor(item.icon)}</span>
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

