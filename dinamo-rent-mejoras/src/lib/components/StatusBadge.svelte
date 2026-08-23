<script lang="ts">
	interface Props {
		estado: string;
		/** Muestra el estado con la primera letra en mayúscula */
		capitalize?: boolean;
	}

	let { estado, capitalize = true }: Props = $props();

	const MAP: Record<string, string> = {
		// Autos
		'Disponible': 'bg-estado-disponible/10 text-estado-disponible border-estado-disponible/25',
		'Rentado': 'bg-estado-rentado/10 text-estado-rentado border-estado-rentado/25',
		'Mantenimiento': 'bg-estado-mantenimiento/10 text-estado-mantenimiento border-estado-mantenimiento/25',
		'Vendido': 'bg-text-secondary/10 text-text-secondary border-text-secondary/25',
		'Baja': 'bg-estado-inactivo/10 text-estado-inactivo border-estado-inactivo/25',
		// Clientes
		'Activo': 'bg-estado-activo/10 text-estado-activo border-estado-activo/25',
		'Inactivo': 'bg-text-secondary/10 text-text-secondary border-text-secondary/25',
		'Lista Negra': 'bg-estado-lista-negra/10 text-estado-lista-negra border-estado-lista-negra/25',
		'VIP': 'bg-estado-vip/10 text-estado-vip border-estado-vip/25'
	};

	const text = $derived(capitalize ? estado.charAt(0).toUpperCase() + estado.slice(1) : estado);
	const classes = $derived(MAP[estado] ?? 'bg-primary/10 text-primary border-primary/25');
</script>

<span
	class="inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-[11px] font-semibold whitespace-nowrap {classes}"
>
	<span class="w-1.5 h-1.5 rounded-full bg-current opacity-70"></span>
	{text}
</span>
