<script lang="ts">
	interface Props {
		open: boolean;
		title: string;
		subtitle?: string;
		onClose: () => void;
		width?: string;
		/** Evita cerrar con click fuera / Esc */
		dismissible?: boolean;
		/** Altura fija al viewport (h-[calc(100vh-2rem)]) en vez de max-h. Útil para
		 *  modales con panel lateral sticky que NO deben crecer con el contenido. */
		fullHeight?: boolean;
		/** Quita el padding y overflow del body; el contenido gestiona su propio
		 *  scroll (útil para layouts de 2 paneles). */
		rawBody?: boolean;
		/** Oculta el footer (cuando las acciones viven dentro del body, p.ej. en
		 *  un panel lateral). */
		noFooter?: boolean;
		children?: import('svelte').Snippet;
		footer?: import('svelte').Snippet;
	}

	let {
		open,
		title,
		subtitle,
		onClose,
		width = 'max-w-xl',
		dismissible = true,
		fullHeight = false,
		rawBody = false,
		noFooter = false,
		children,
		footer
	}: Props = $props();

	// ── Accesibilidad: focus trap + autofocus + restore (WCAG 2.1 SC 2.4.3) ──
	// Al abrir el modal: guardamos el elemento que tenía el foco (para restaurarlo
	// al cerrar) y movemos el foco al primer elemento enfocable del modal.
	// Mientras el modal está abierto: interceptamos Tab para que el foco no escape
	// (cicla entre los elementos enfocables dentro del modal).
	// Al cerrar: devolvemos el foco al elemento guardado.
	let dialogEl: HTMLDivElement | undefined = $state();
	let titleId = `modal-title-${Math.random().toString(36).slice(2, 10)}`;
	let previouslyFocused: HTMLElement | null = null;

	const FOCUSABLE =
		'button:not([disabled]):not([tabindex="-1"]), [href], input:not([disabled]):not([tabindex="-1"]), select:not([disabled]):not([tabindex="-1"]), textarea:not([disabled]):not([tabindex="-1"]), [tabindex]:not([tabindex="-1"])';

	function getFocusable(): HTMLElement[] {
		if (!dialogEl) return [];
		return Array.from(dialogEl.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
			(el) => el.offsetParent !== null || el === document.activeElement
		);
	}

	function handleKey(e: KeyboardEvent) {
		if (e.key === 'Escape' && dismissible) {
			e.preventDefault();
			onClose();
			return;
		}
		if (e.key === 'Tab' && dialogEl) {
			const focusable = getFocusable();
			if (focusable.length === 0) {
				// Sin elementos enfocables: mantener el foco en el diálogo mismo.
				e.preventDefault();
				dialogEl.focus();
				return;
			}
			const first = focusable[0];
			const last = focusable[focusable.length - 1];
			const active = document.activeElement as HTMLElement;
			if (e.shiftKey) {
				// Shift+Tab: si estamos en el primero, saltar al último.
				if (active === first || active === dialogEl) {
					e.preventDefault();
					last.focus();
				}
			} else {
				// Tab: si estamos en el último, saltar al primero.
				if (active === last) {
					e.preventDefault();
					first.focus();
				}
			}
		}
	}

	$effect(() => {
		if (open && typeof document !== 'undefined') {
			document.addEventListener('keydown', handleKey);
			const prev = document.body.style.overflow;
			document.body.style.overflow = 'hidden';
			// Guardar el elemento que tenía el foco ANTES de moverlo al modal.
			previouslyFocused = document.activeElement as HTMLElement;
			// Mover el foco al primer elemento enfocable del modal (autofocus).
			// requestAnimationFrame asegura que el DOM del modal ya está pintado.
			const raf = requestAnimationFrame(() => {
				// Si un consumidor ya movió el foco a un elemento dentro del modal
				// (p.ej. ConfirmarCierre enfoca el botón «No»), respetarlo.
				if (
					dialogEl &&
					document.activeElement &&
					dialogEl.contains(document.activeElement) &&
					document.activeElement !== document.body
				) {
					return;
				}
				const focusable = getFocusable();
				if (focusable.length > 0) {
					focusable[0].focus();
				} else if (dialogEl) {
					// Sin elementos enfocables: poner el foco en el contenedor del diálogo.
					dialogEl.focus();
				}
			});
			return () => {
				document.removeEventListener('keydown', handleKey);
				document.body.style.overflow = prev;
				cancelAnimationFrame(raf);
				// Restaurar el foco al elemento que lo tenía antes de abrir el modal.
				if (previouslyFocused && typeof previouslyFocused.focus === 'function') {
					previouslyFocused.focus();
				}
				previouslyFocused = null;
			};
		}
	});
</script>

{#if open}
	<div
		class="fixed inset-0 z-50 flex items-start justify-center overflow-y-auto overscroll-contain p-3 sm:p-4"
		role="dialog"
		aria-modal="true"
		aria-labelledby={titleId}
	>
		<button
			class="fixed inset-0 bg-slate-900/50 backdrop-blur-[2px] cursor-default animate-[modal-fade-in_150ms_ease-out]"
			onclick={() => (dismissible ? onClose() : null)}
			aria-label="Cerrar diálogo"
			tabindex="-1"
		></button>

		<div
			bind:this={dialogEl}
			class="relative w-full {width} mt-2 sm:mt-3 card shadow-2xl flex flex-col {fullHeight
				? 'h-[calc(100vh-1.5rem)]'
				: 'max-h-[calc(100vh-2rem)]'} animate-[modal-pop-in_180ms_ease-out] outline-none"
			tabindex="-1"
		>
			<!-- Header -->
			<div class="flex items-start justify-between gap-4 px-5 py-3 border-b border-border shrink-0">
				<div>
					<h2 id={titleId} class="text-base font-bold text-text-primary leading-tight">{title}</h2>
					{#if subtitle}
						<p class="text-[11px] text-text-secondary mt-0.5">{subtitle}</p>
					{/if}
				</div>
				<button
					onclick={onClose}
					class="p-1.5 rounded-lg text-text-secondary hover:text-text-primary hover:bg-alt-row transition-colors shrink-0"
					aria-label="Cerrar"
				>
					<svg
						xmlns="http://www.w3.org/2000/svg"
						class="w-5 h-5"
						fill="none"
						viewBox="0 0 24 24"
						stroke="currentColor"
						stroke-width="2"
						><path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" /></svg
					>
				</button>
			</div>

			<!-- Cuerpo -->
			<div class={rawBody ? 'grow min-h-0' : 'px-5 py-4 overflow-y-auto grow'}>
				{@render children?.()}
			</div>

			<!-- Footer -->
			{#if footer && !noFooter}
				<div
					class="px-5 py-3 border-t border-border flex items-center justify-end gap-2 shrink-0 bg-surface/50 rounded-b-xl"
				>
					{@render footer()}
				</div>
			{/if}
		</div>
	</div>
{/if}
