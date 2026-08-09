// toast.svelte.ts — Store de notificaciones toast (Svelte 5 runes)

export type ToastType = 'success' | 'error' | 'info' | 'warning';

export interface Toast {
	id: number;
	type: ToastType;
	message: string;
}

let nextId = 1;

export const toasts = $state<Toast[]>([]);

function push(type: ToastType, message: string): void {
	const id = nextId++;
	toasts.push({ id, type, message });
	setTimeout(() => dismiss(id), 4000);
}

export function dismiss(id: number): void {
	const i = toasts.findIndex((t) => t.id === id);
	if (i >= 0) toasts.splice(i, 1);
}

export const toast = {
	success: (m: string) => push('success', m),
	error: (m: string) => push('error', m),
	info: (m: string) => push('info', m),
	warning: (m: string) => push('warning', m)
};
