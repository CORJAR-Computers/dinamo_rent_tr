// src/routes/cambiar-password/+page.ts — Guard de la ruta en el load.
//
// La ruta solo tiene sentido cuando la sesión exige el cambio de contraseña
// (el login redirige aquí con debeCambiarPassword=true). El guard corre antes
// de renderizar, así que el usuario no ve un frame intermedio de spinner:
//   - sin sesión          → /login
//   - sesión sin exigir   → /dashboard (acceso directo por URL o flag resuelto)
// El redirect usa replaceState por defecto en el navegador, evitando que
// «atrás» regrese a esta página.

import { redirect } from '@sveltejs/kit';
import { session } from '$lib/stores/session.svelte';

export function load() {
	// Sin sesión: el layout ya redirige a /login, pero por defensa en
	// profundidad (y porque esta ruta es fullscreen) se cubre aquí también.
	if (!session.isAuthenticated) {
		redirect(302, '/login');
	}
	// Sesión activa pero sin exigencia de cambio → al dashboard.
	if (session.debeCambiarPassword !== true) {
		redirect(302, '/dashboard');
	}
	return {};
}
