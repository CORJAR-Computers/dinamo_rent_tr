<script lang="ts">
	// OrdenRenta.svelte — Orden de renta imprimible en papel Carta (Letter).
	// Diseño compacto: debe caber en UNA sola hoja carta. La compresión de
	// impresión vive en app.css (@media print) vía .orden-carta; el estilo de
	// aquí es el que se ve en pantalla (vista previa) y en papel.
	import type { Renta } from '$lib/api';
	import { formatCOP, formatContrato, formatDate } from '$lib/utils/format';

	// El cliente y el vehículo se muestran desde `renta` (nombreCliente, vehiculo,
	// placa — ya denormalizados por el backend); no se necesitan props aparte.
	let { renta }: { renta: Renta } = $props();

	const hoy = new Date().toLocaleDateString('es-CO', {
		year: 'numeric',
		month: 'long',
		day: 'numeric'
	});

	function hora(h: string | null): string {
		if (!h) return '—';
		const [hh, mm] = h.split(':');
		const hora12 = ((Number(hh) + 11) % 12) + 1;
		return `${hora12}:${mm} ${Number(hh) >= 12 ? 'PM' : 'AM'}`;
	}

	function extra(campo: string, valor: string): { nombre: string; monto: string } | null {
		const num = parseFloat(valor) || 0;
		if (num <= 0) return null;
		return { nombre: campo, monto: valor };
	}

	const extras = $derived(
		[
			extra('Lavado', renta.costoLavado),
			extra('Silla de bebé', renta.costoSilla),
			extra('Recogida/retorno', renta.costoRetorno),
			extra('Domicilio', renta.costoDomicilio),
			extra('Cables', renta.costoCables),
			extra('Inversor', renta.costoInversor),
			extra('Día extra', renta.valorDiaExtra)
		].filter((e): e is { nombre: string; monto: string } => e !== null)
	);

	const estadoBadge = $derived(
		renta.estado === 'Cancelada'
			? 'estado-cancelada'
			: renta.estado === 'Cerrada'
				? 'estado-cerrada'
				: renta.estado === 'Activa' || renta.estado === 'Activo'
					? 'estado-activa'
					: 'estado-otro'
	);

	const insSalida = $derived(renta.inspecciones.find((i) => i.tipo === 'Salida'));
	const insEntrada = $derived(renta.inspecciones.find((i) => i.tipo === 'Entrada'));

	function si(v: boolean): string {
		return v ? 'Sí' : 'No';
	}
</script>

<style>
	/* Documento compacto en papel Carta: 8.5x11in ≈ 816px x 1056px a 96dpi.
	   La tipografía está en px (no en rem) para que el zoom/DPI no la
	   desborde; el interlineado es 1.3 y las filas de tablas son mínimas. */
	/* Paleta por rol en variables: el modo oscuro (pantalla) y la impresión
	   (siempre en claro, en app.css @media print) solo redefinen estas
	   variables. */
	.orden-carta {
		--ord-fondo: #ffffff;
		--ord-texto: #0f172a;
		--ord-acento: #1e40af;
		--ord-secundario: #64748b;
		--ord-borde: #cbd5e1;
		--ord-borde-fila: #e2e8f0;
		--ord-fondo-tabla: #f1f5f9;
		--ord-fondo-subtotal: #f8fafc;
		--ord-texto-medio: #334155;
		--ord-texto-danios: #475569;
		--ord-pie: #94a3b8;
		--ord-peligro: #b91c1c;
		--ord-exito: #15803d;
		background: var(--ord-fondo);
		color: var(--ord-texto);
		font-family: 'Segoe UI', Arial, sans-serif;
		font-size: 10px;
		line-height: 1.3;
		width: 100%;
		page-break-inside: avoid;
	}

	.encabezado {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 10px;
		border-bottom: 2px solid var(--ord-acento);
		padding-bottom: 6px;
		margin-bottom: 6px;
	}

	.logo-bloque {
		display: flex;
		align-items: center;
		gap: 7px;
	}

	.logo-caja {
		width: 52px;
		height: 52px;
		border-radius: 8px;
		background: var(--ord-acento);
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
		padding: 4px;
	}

	.logo-caja img {
		width: 44px;
		height: 44px;
		object-fit: contain;
		border-radius: 5px;
	}

	.nombre-empresa {
		font-size: 13px;
		font-weight: 800;
		letter-spacing: -0.01em;
		line-height: 1.1;
	}

	.subtitulo-empresa {
		font-size: 8px;
		color: var(--ord-secundario);
		margin-top: 1px;
	}

	.titulo-derecha {
		text-align: right;
	}

	.titulo-orden {
		font-size: 15px;
		font-weight: 800;
		letter-spacing: 0.04em;
		color: var(--ord-acento);
		line-height: 1.1;
	}

	.ref-contrato {
		font-size: 10px;
		font-weight: 600;
		margin-top: 1px;
	}

	.ref-renta {
		font-size: 8px;
		color: var(--ord-secundario);
		margin-top: 1px;
	}

	.fila-estado {
		display: flex;
		align-items: center;
		gap: 6px;
		margin-bottom: 6px;
	}

	.etiqueta-estado {
		font-size: 8px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--ord-secundario);
	}

	.badge-estado {
		display: inline-flex;
		align-items: center;
		border-radius: 999px;
		border: 1px solid;
		padding: 1px 8px;
		font-size: 9px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}

	.estado-cancelada {
		border-color: var(--ord-peligro);
		color: var(--ord-peligro);
	}

	.estado-cerrada {
		border-color: var(--ord-exito);
		color: var(--ord-exito);
	}

	.estado-activa {
		border-color: var(--ord-acento);
		color: var(--ord-acento);
	}

	.estado-otro {
		border-color: var(--ord-secundario);
		color: var(--ord-texto-danios);
	}

	.malla {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 7px;
		margin-bottom: 6px;
	}

	.caja {
		border: 1px solid var(--ord-borde);
		border-radius: 6px;
		padding: 5px 8px;
		page-break-inside: avoid;
	}

	.caja-titulo {
		font-size: 8px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--ord-secundario);
		margin-bottom: 3px;
	}

	.linea {
		display: flex;
		justify-content: space-between;
		align-items: baseline;
		gap: 8px;
		font-size: 10px;
		line-height: 1.45;
	}

	.linea .l {
		color: var(--ord-secundario);
		flex-shrink: 0;
	}

	.linea .v {
		font-weight: 600;
		text-align: right;
	}

	.valor-fuerte {
		font-weight: 800;
	}

	.nombre-cliente {
		font-weight: 800;
		font-size: 10.5px;
	}

	.tabla {
		width: 100%;
		border-collapse: collapse;
		border: 1px solid var(--ord-borde);
		border-radius: 6px;
		overflow: hidden;
		margin-bottom: 7px;
	}

	.tabla th {
		background: var(--ord-fondo-tabla);
		text-align: left;
		font-size: 8px;
		font-weight: 700;
		text-transform: uppercase;
		letter-spacing: 0.08em;
		color: var(--ord-secundario);
		padding: 3px 8px;
	}

	.tabla th.derecha {
		text-align: right;
	}

	.tabla td {
		padding: 2px 8px;
		font-size: 10px;
		border-top: 1px solid var(--ord-borde-fila);
	}

	.tabla td.derecha {
		text-align: right;
		font-weight: 600;
	}

	.fila-subtotal {
		background: var(--ord-fondo-subtotal);
	}

	.fila-subtotal td {
		font-weight: 800;
	}

	.fila-total {
		background: var(--ord-fondo-tabla);
	}

	.fila-total td {
		font-weight: 800;
		font-size: 10.5px;
	}

	.pagos td {
		padding: 2px 8px;
		font-size: 9.5px;
	}

	.pago-concepto {
		font-weight: 600;
	}

	.pago-detalle {
		color: var(--ord-secundario);
	}

	.inspeccion-lista {
		font-size: 9.5px;
		line-height: 1.5;
		color: var(--ord-texto-medio);
	}

	.inspeccion-lista .danos {
		margin-top: 2px;
		color: var(--ord-texto-danios);
	}

	.devolucion-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 4px 12px;
	}

	.observaciones {
		font-size: 9.5px;
		white-space: pre-wrap;
		color: var(--ord-texto-medio);
		line-height: 1.4;
	}

	.condiciones {
		font-size: 9px;
		color: var(--ord-secundario);
		line-height: 1.45;
		margin-top: 2px;
		margin-bottom: 8px;
	}

	.condiciones p {
		margin: 0;
	}

	.firmas {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 12px;
		margin-top: 10px;
		page-break-inside: avoid;
	}

	.firma {
		text-align: center;
	}

	.firma-linea {
		border-top: 1px solid var(--ord-pie);
		padding-top: 3px;
		font-size: 9px;
		font-weight: 600;
	}

	.firma-nombre {
		font-size: 8px;
		color: var(--ord-secundario);
		margin-top: 2px;
	}

	.pie {
		text-align: center;
		font-size: 8px;
		color: var(--ord-pie);
		margin-top: 6px;
	}
</style>

<div class="print-area orden-carta">
	<!-- Encabezado -->
	<div class="encabezado">
		<div class="logo-bloque">
			<div class="logo-caja">
				<img src="/LogoDinamo.png" alt="Logo" />
			</div>
			<div>
				<p class="nombre-empresa">DINAMO RENT A CAR</p>
				<p class="subtitulo-empresa">Renta de vehículos · Contrato de renta</p>
			</div>
		</div>
		<div class="titulo-derecha">
			<p class="titulo-orden">ORDEN DE RENTA</p>
			<p class="ref-contrato">Contrato <span class="tabular-nums">{formatContrato(renta.anioContrato, renta.noContrato)}</span></p>
			<p class="ref-renta">Renta No. <span class="tabular-nums">{String(renta.id).padStart(4, '0')}</span> · Emitida: {hoy}</p>
		</div>
	</div>

	<!-- Estado -->
	<div class="fila-estado">
		<span class="etiqueta-estado">Estado:</span>
		<span class="badge-estado {estadoBadge}">{renta.estado}</span>
	</div>

	<!-- Cliente y vehículo -->
	<div class="malla">
		<div class="caja">
			<p class="caja-titulo">Cliente</p>
			<p class="nombre-cliente">{renta.nombreCliente}</p>
			{#if renta.noLicencia || renta.nacionalidad}
				<div class="linea" style="margin-top: 2px;">
					<span class="l">
						{#if renta.noLicencia}Lic. {renta.noLicencia}{#if renta.nacionalidad} · {renta.nacionalidad}{/if}{:else}{renta.nacionalidad}{/if}
					</span>
				</div>
			{/if}
		</div>
		<div class="caja">
			<p class="caja-titulo">Vehículo</p>
			<p class="nombre-cliente">{renta.vehiculo || 'Por definir'}</p>
			<div class="linea" style="margin-top: 2px;">
				<span class="l">Placa: <span class="valor-fuerte">{renta.placa || '—'}</span></span>
				<span class="l">Km: <span class="valor-fuerte">{renta.kmSalida}</span> · Tanque: {renta.tanqueSalida || '—'}</span>
			</div>
		</div>
	</div>

	<!-- Itinerario -->
	<div class="malla">
		<div class="caja">
			<p class="caja-titulo">Recogida</p>
			<div class="linea"><span class="l">Fecha</span><span class="v tabular-nums">{formatDate(renta.fechaRecogida)}</span></div>
			<div class="linea"><span class="l">Hora</span><span class="v tabular-nums">{hora(renta.horaRecogida)}</span></div>
			{#if renta.ubicacionRecogida}
				<div class="linea"><span class="l">Lugar</span><span class="v">{renta.ubicacionRecogida}</span></div>
			{/if}
		</div>
		<div class="caja">
			<p class="caja-titulo">Retorno</p>
			<div class="linea"><span class="l">Fecha</span><span class="v tabular-nums">{formatDate(renta.fechaRetorno)}</span></div>
			<div class="linea"><span class="l">Hora</span><span class="v tabular-nums">{hora(renta.horaRetorno)}</span></div>
			{#if renta.ubicacionRetorno}
				<div class="linea"><span class="l">Lugar</span><span class="v">{renta.ubicacionRetorno}</span></div>
			{/if}
		</div>
	</div>

	<!-- Tarifas -->
	<table class="tabla">
		<thead>
			<tr>
				<th>Concepto</th>
				<th class="derecha">Valor</th>
			</tr>
		</thead>
		<tbody>
			<tr>
				<td>Valor del día × {renta.diasCalculados} día{renta.diasCalculados === 1 ? '' : 's'} ({formatCOP(renta.valorDia)})</td>
				<td class="derecha">{formatCOP((parseFloat(renta.valorDia) || 0) * renta.diasCalculados)}</td>
			</tr>
			{#if (parseFloat(renta.valorHoraExtra) || 0) * renta.horasExtras > 0}
				<tr>
					<td>Horas extras ({renta.horasExtras} × {formatCOP(renta.valorHoraExtra)})</td>
					<td class="derecha">{formatCOP((parseFloat(renta.valorHoraExtra) || 0) * renta.horasExtras)}</td>
				</tr>
			{/if}
			{#each extras as e}
				<tr>
					<td>{e.nombre}</td>
					<td class="derecha">{formatCOP(e.monto)}</td>
				</tr>
			{/each}
			{#if (parseFloat(renta.descuento) || 0) > 0}
				<tr>
					<td>Descuento</td>
					<td class="derecha">- {formatCOP(renta.descuento)}</td>
				</tr>
			{/if}
			<tr class="fila-subtotal">
				<td>Subtotal</td>
				<td class="derecha">{formatCOP(renta.subtotal, true)}</td>
			</tr>
			<tr>
				<td>Impuestos (IVA)</td>
				<td class="derecha">{formatCOP(renta.impuestos, true)}</td>
			</tr>
			<tr class="fila-total">
				<td>TOTAL</td>
				<td class="derecha">{formatCOP(renta.total, true)}</td>
			</tr>
			<tr>
				<td>Abono recibido</td>
				<td class="derecha">- {formatCOP(renta.abono, true)}</td>
			</tr>
			<tr class="fila-subtotal">
				<td>Saldo pendiente</td>
				<td class="derecha">{formatCOP(renta.saldoPendiente, true)}</td>
			</tr>
		</tbody>
	</table>

	<!-- Pagos -->
	{#if renta.pagos.length > 0}
		<table class="tabla pagos">
			<thead>
				<tr>
					<th>Pagos recibidos</th>
					<th class="derecha">Monto</th>
				</tr>
			</thead>
			<tbody>
				{#each renta.pagos as p}
					<tr>
						<td>
							<span class="pago-concepto">{p.concepto}</span>
							<span class="pago-detalle"> · {p.metodoPago} · {formatDate(p.fecha)}</span>
						</td>
						<td class="derecha">{formatCOP(p.monto, true)}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	{/if}

	<!-- Inspecciones -->
	{#if insSalida || insEntrada}
		<div class="malla">
			{#if insSalida}
				<div class="caja">
					<p class="caja-titulo">Inspección de salida</p>
					<div class="inspeccion-lista">
						<p>Km: <span class="valor-fuerte">{insSalida.kilometraje}</span> · Gasolina: {insSalida.nivelGasolina}</p>
						<p>Repuesto: {si(insSalida.tieneRepuesto)} · Gato/cruceta: {si(insSalida.tieneGatoCruceta)} · Kit: {si(insSalida.tieneKitCarretera)} · Docs: {si(insSalida.tieneDocumentos)}</p>
						{#if insSalida.danosCarroceria}<p class="danos">Daños: {insSalida.danosCarroceria}</p>{/if}
					</div>
				</div>
			{/if}
			{#if insEntrada}
				<div class="caja">
					<p class="caja-titulo">Inspección de entrada</p>
					<div class="inspeccion-lista">
						<p>Km: <span class="valor-fuerte">{insEntrada.kilometraje}</span> · Gasolina: {insEntrada.nivelGasolina}</p>
						{#if insEntrada.danosCarroceria}<p class="danos">Daños: {insEntrada.danosCarroceria}</p>{/if}
					</div>
				</div>
			{/if}
		</div>
	{/if}

	<!-- Devolución real -->
	{#if renta.estado === 'Cerrada'}
		<div class="caja" style="margin-bottom: 7px;">
			<p class="caja-titulo">Devolución real</p>
			<div class="devolucion-grid">
				<div class="linea"><span class="l">Fecha</span><span class="v">{formatDate(renta.fechaDevolucionReal)}</span></div>
				<div class="linea"><span class="l">Hora</span><span class="v">{hora(renta.horaDevolucionReal)}</span></div>
				<div class="linea"><span class="l">Km final</span><span class="v">{renta.kmFinal || '—'}</span></div>
				<div class="linea"><span class="l">Tanque final</span><span class="v">{renta.tanqueFinal || '—'}</span></div>
			</div>
		</div>
	{/if}

	<!-- Observaciones -->
	{#if renta.observaciones}
		<div class="caja" style="margin-bottom: 7px;">
			<p class="caja-titulo">Observaciones</p>
			<p class="observaciones">{renta.observaciones}</p>
		</div>
	{/if}

	<!-- Condiciones -->
	<div class="condiciones">
		<p>• El vehículo debe devolverse en las mismas condiciones en que fue entregado y con el tanque en el nivel pactado; de lo contrario se aplicarán los costos de lavado y combustible correspondientes.</p>
		<p>• El cliente es responsable por daños, multas y comparendos ocurridos durante el periodo de la renta. El saldo pendiente debe cancelarse al momento de la devolución del vehículo.</p>
	</div>

	<!-- Firmas -->
	<div class="firmas">
		<div class="firma">
			<div class="firma-linea">Firma del cliente</div>
			<p class="firma-nombre">{renta.nombreCliente}</p>
		</div>
		<div class="firma">
			<div class="firma-linea">Firma del representante</div>
			<p class="firma-nombre">Dinamo Rent a Car</p>
		</div>
	</div>

	<!-- Pie -->
	<p class="pie">
		Dinamo Rent a Car · Contrato {formatContrato(renta.anioContrato, renta.noContrato)} · Renta No. {String(renta.id).padStart(4, '0')} · Impresa el {hoy}
	</p>
</div>
