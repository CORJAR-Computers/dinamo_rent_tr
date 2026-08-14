<script lang="ts">
	// ContratoRenta.svelte — ANEXO DE CONTRATO DE ALQUILER DE VEHICULOS Y ACTA DE ENTREGA
	// Texto legal tomado de Contrato_Dinamo.docx (fuente de verdad). Se imprime en
	// papel Carta / Letter (ver @page y .contrato-carta en app.css).
	import { onMount } from 'svelte';
	import type { Renta, Cliente, Auto } from '$lib/api';
	import { formatCOP, formatContrato, formatDate } from '$lib/utils/format';
	import { empresa } from '$lib/stores/empresa.svelte';
	import { sid } from '$lib/stores/session.svelte';

	// Datos + logo de la empresa (setup inicial); fallback estático si no hay config.
	onMount(() => {
		void empresa.cargarCompleta(sid());
	});

	// ── Encabezado de la empresa con renderizado limpio si faltan datos ──
	const razonSocial = $derived(empresa.nombreMostrar.toUpperCase());
	// Ciudad para la cláusula compromisoria (la configurada por la empresa; si no
	// hay ninguna, se conserva la del contrato original: Cartagena).
	const ciudadClausula = $derived(empresa.ciudadMostrar || 'Cartagena');
	// "DYNARENT, RUT 900694866-3, www.dynarent.com, domiciliado en ..." (omite vacíos)
	const datosArrendador = $derived(
		[
			razonSocial,
			empresa.nitMostrar ? `RUT ${empresa.nitMostrar}` : '',
			empresa.webMostrar,
			empresa.direccionMostrar ? `domiciliado en ${empresa.direccionMostrar}` : ''
		]
			.filter(Boolean)
			.join(', ')
	);
	const pieContacto = $derived(
		[
			empresa.direccionMostrar,
			empresa.telefonoMostrar ? `Tel: ${empresa.telefonoMostrar}` : '',
			empresa.emailMostrar
		]
			.filter(Boolean)
			.join(' • ')
	);

	let {
		renta,
		cliente,
		auto
	}: { renta: Renta; cliente?: Cliente; auto?: Auto } = $props();

	const hoy = new Date().toLocaleDateString('es-CO', {
		year: 'numeric',
		month: 'long',
		day: 'numeric'
	});

	// ── Datos dinámicos del contrato (fallback a la renta cuando falta el cliente/auto) ──
	const tipoDoc = $derived(cliente?.tipoDoc || 'CC');
	const noDoc = $derived(cliente?.noDoc || renta.noLicencia || '—');
	const nacionalidad = $derived(cliente?.nacionalidad || renta.nacionalidad || '—');
	const tipoLicencia = $derived(cliente?.tipoLicencia || 'Particular');
	const noLicencia = $derived(cliente?.noLicencia || renta.noLicencia || '—');
	const telefono = $derived(cliente?.celular || '—');
	const direccion = $derived(cliente?.dirResidencia || '—');
	const email = $derived(cliente?.email || '—');

	const tipoAuto = $derived(auto?.tipo || '—');
	const marcaAuto = $derived(auto?.marca || '—');
	const modeloAuto = $derived(auto?.modelo || '—');
	const cilindraje = $derived(auto?.cilindraje || '—');
	const versionAuto = $derived(auto?.version || '—');
	const combustible = $derived(auto?.combustible || '—');
	const placaAuto = $derived(auto?.placa || renta.placa || '—');

	function hora(h: string | null): string {
		if (!h) return '—';
		const [hh, mm] = h.split(':');
		const hora12 = ((Number(hh) + 11) % 12) + 1;
		return `${hora12}:${mm} ${Number(hh) >= 12 ? 'PM' : 'AM'}`;
	}
</script>

<style>
	/* Contrato de renta — impresión en papel Carta / Letter (ver @page y
	   .contrato-carta en app.css). Cuerpo 6.2pt, interlineado 0.98, logo 70px
	   y encabezado compacto: el texto legal cabe en 2 hojas Carta (verificado
	   con el PDF real: printToPDF + verificar-paginacion = 2 páginas con pie
	   «Página X de Y»). */
	/* Paleta por rol en variables: el modo oscuro (pantalla) y la impresión
	   (siempre en claro, en app.css @media print) solo redefinen estas
	   variables. */
	.contrato-body {
		--ct-fondo: #ffffff;
		--ct-texto: #000000;
		--ct-acento: #1a237e;
		--ct-secundario: #666666;
		--ct-contacto: #444444;
		--ct-fondo-suave: #f5f5f5;
		--ct-fondo-caja: #f9f9f9;
		--ct-fondo-poliza: #f8f8f8;
		--ct-fondo-resaltado: #e8eaf6;
		--ct-borde: #cccccc;
		--ct-borde-tabla: #dddddd;
		--ct-borde-firma: #333333;
		--ct-footer-borde: #eeeeee;
		font-family: 'Arial', sans-serif;
		font-size: 6.2pt;
		line-height: 0.98;
		color: var(--ct-texto);
		text-align: justify;
		hyphens: auto;
	}

	.encabezado-principal {
		display: flex;
		align-items: flex-start;
		margin-bottom: 5px;
		padding-bottom: 3px;
		border-bottom: 1px solid var(--ct-acento);
	}

	.logo-container {
		flex: 0 0 12%;
		text-align: left;
		padding-top: 2px;
	}

	.titulo-container {
		flex: 1;
		text-align: center;
		padding: 0 8px;
	}

	.titulo-principal {
		color: var(--ct-acento);
		font-size: 8.2pt;
		font-weight: bold;
		margin: 0 0 2px 0;
		text-transform: uppercase;
	}

	.subtitulo {
		font-size: 5.8pt;
		color: var(--ct-secundario);
		margin: 0;
	}

	.info-contacto {
		flex: 0 0 25%;
		text-align: right;
		font-size: 6.3pt;
		color: var(--ct-contacto);
		padding-top: 2px;
	}

	.numero-contrato {
		text-align: center;
		font-size: 8.4pt;
		font-weight: bold;
		margin: 2px 0 3px 0;
		color: var(--ct-acento);
		background-color: var(--ct-fondo-suave);
		padding: 2px;
		border-radius: 3px;
	}

	.seccion-partes {
		margin-bottom: 4px;
		padding: 4px;
		background-color: var(--ct-fondo-caja);
		border-left: 3px solid var(--ct-acento);
		font-size: 6.2pt;
		line-height: 0.98;
	}

	.clausula-titulo {
		font-weight: bold;
		margin: 3px 0 1px 0;
		color: var(--ct-texto);
		font-size: 7.2pt;
		page-break-after: avoid;
	}

	.clausula-contenido {
		margin: 0 0 1px 0;
		text-align: justify;
		line-height: 0.98;
		text-indent: 8px;
	}

	.lista-numerada {
		margin: 3px 0;
		padding-left: 18px;
	}

	.lista-numerada li {
		margin-bottom: 2px;
		line-height: 0.98;
	}

	.campo-resaltado {
		background-color: var(--ct-fondo-resaltado);
		padding: 1px 3px;
		border-radius: 2px;
		font-weight: bold;
		font-size: 6.2pt;
		border-bottom: 1px dotted var(--ct-borde);
	}

	.contrato-table {
		width: 100%;
		border-collapse: collapse;
		font-size: 6.2pt;
		margin: 3px 0;
	}

	.contrato-table td {
		padding: 2px;
		border: 1px solid var(--ct-borde-tabla);
	}

	.poliza-container {
		margin: 3px 0;
		padding: 4px;
		border: 1px dashed var(--ct-borde);
		background-color: var(--ct-fondo-poliza);
		font-size: 6.2pt;
		page-break-inside: avoid;
	}

	.linea-pago {
		margin: 3px 0;
	}

	.linea-pago .casilla {
		display: inline-block;
		width: 10px;
		height: 10px;
		border: 1px solid var(--ct-texto);
		margin-right: 6px;
		vertical-align: middle;
	}

	.linea-pago .firma {
		border-bottom: 1px dotted var(--ct-borde);
		padding: 0 14px;
	}

	.firmas-container {
		margin-top: 12px;
		page-break-inside: avoid;
		padding-top: 4px;
		border-top: 1px solid var(--ct-borde);
	}

	.firma-linea {
		margin: 10px 0 3px;
		border-top: 1px solid var(--ct-borde-firma);
		width: 55%;
	}

	.negrita {
		font-weight: bold;
	}

	.footer {
		font-size: 6pt;
		color: var(--ct-secundario);
		text-align: center;
		margin-top: 10px;
		padding-top: 4px;
		border-top: 1px solid var(--ct-footer-borde);
	}
</style>

<div class="print-area contrato-carta bg-white text-black contrato-body px-6 py-6">
	<!-- ENCABEZADO -->
	<div class="encabezado-principal">
		<div class="logo-container">
			<img src={empresa.logoSrc} alt={empresa.nombreMostrar} class="w-[70px] h-[70px] object-contain rounded-md" />
		</div>

		<div class="titulo-container">
			<h1 class="titulo-principal">ANEXO DE CONTRATO DE ALQUILER DE VEHICULOS Y ACTA DE ENTREGA</h1>
			<p class="subtitulo">Documento Legal - Vigencia durante el período de alquiler</p>
		</div>

		<div class="info-contacto">
			<p class="negrita" style="margin-bottom: 3px;">ASISTENCIA A CLIENTES</p>
			{#if empresa.telefonoMostrar}<p style="margin: 0;">{empresa.telefonoMostrar}</p>{/if}
			{#if empresa.webMostrar}<p style="margin: 2px 0 0 0; font-size: 6.5pt;">{empresa.webMostrar}</p>{/if}
		</div>
	</div>

	<!-- NÚMERO DE CONTRATO (secuencia independiente del id de la renta) -->
	<div class="numero-contrato">
		CONTRATO Nº: {formatContrato(renta.anioContrato, renta.noContrato)}
	</div>

	<!-- PARTES CONTRATANTES -->
	<div class="seccion-partes">
		<p class="negrita" style="text-indent: 0;">ENTRE LOS SUSCRITOS:</p>
		<p>
			{datosArrendador} por una parte, como <span class="negrita">arrendador</span>, y
			{renta.nombreCliente}, identificado con <span class="campo-resaltado">{tipoDoc} No: {noDoc}</span>,
			nacionalidad <span class="campo-resaltado">{nacionalidad}</span>, licencia tipo
			<span class="campo-resaltado">{tipoLicencia}</span>, número teléfono
			<span class="campo-resaltado">{telefono}</span> y domiciliado en
			<span class="campo-resaltado">{direccion}</span>, correo electrónico
			<span class="campo-resaltado">{email}</span>, que para efectos del presente contrato se
			denominará el <span class="negrita">ARRENDATARIO</span>, convenido en celebrar el presente
			contrato de servicio de transporte terrestre, que se rige por las siguientes cláusulas y lo no
			previsto en ellas por lo dispuesto en la ley.
		</p>
	</div>

	<!-- CLÁUSULA PRIMERA -->
	<div class="clausula-titulo">CLÁUSULA PRIMERA: OBJETO DEL CONTRATO</div>
	<div class="clausula-contenido">
		El ARRENDADOR, da en contrato de servicio de transporte terrestre el siguiente vehículo en
		buenas condiciones como se describe a continuación y en el inventario:
	</div>

	<table class="contrato-table">
		<tbody>
			<tr>
				<td><span class="negrita">Tipo:</span> {tipoAuto}</td>
				<td><span class="negrita">Marca:</span> {marcaAuto}</td>
				<td><span class="negrita">Modelo:</span> {modeloAuto}</td>
			</tr>
			<tr>
				<td><span class="negrita">Cilindraje:</span> {cilindraje}</td>
				<td><span class="negrita">Versión:</span> {versionAuto}</td>
				<td><span class="negrita">Combustible:</span> {combustible}</td>
			</tr>
			<tr>
				<td colspan="3"><span class="negrita">Placa:</span> {placaAuto}</td>
			</tr>
		</tbody>
	</table>

	<div class="clausula-contenido">
		<span class="negrita">PRIMERO:</span> {razonSocial} es único y exclusivo poseedor del vehículo
		anteriormente descrito.
	</div>
	<div class="clausula-contenido">
		<span class="negrita">SEGUNDO:</span> Por este acto {razonSocial} viene en dar en arrendamiento
		el vehículo antes singularizado, a quien lo acepta y recibe para sí y para destinarlo a su uso.
	</div>

	<!-- CLÁUSULA SEGUNDA -->
	<div class="clausula-titulo">CLÁUSULA SEGUNDA: ESTADO DEL VEHÍCULO</div>
	<div class="clausula-contenido">
		El ARRENDATARIO acepta que el vehículo se encuentra en perfecto estado de funcionamiento con sus
		equipos y accesorios según detalle que se suscribe en anexo a este contrato y con la carrocería,
		tapizado, neumáticos y demás accesorios tales como
		<span class="campo-resaltado">________________________________________</span>
	</div>
	<div class="clausula-contenido">
		Dejan las partes constancia que se ha verificado por el arrendatario el buen funcionamiento
		especialmente del sistema de frenos, luces de estacionamiento, de freno, de tránsito y de viraje,
		así como el cinturón de seguridad en perfectas condiciones de uso, por lo cual el mismo		hará entrega a {razonSocial} el valor de los bienes por la pérdida o daño de estos por cualquier
		naturaleza.
	</div>

	<!-- CLÁUSULA TERCERA -->
	<div class="clausula-titulo">CLÁUSULA TERCERA: PAGOS Y GARANTÍAS</div>
	<div class="clausula-contenido">
		EL ARRENDATARIO depositará al arrendador el valor del alquiler del vehículo y un depósito de
		seguridad del automóvil, en garantía de su cumplimiento facultando AL ARRENDADOR para disponer
		total o parcialmente del depósito a fin de cubrir cualquiera de las obligaciones emergentes del
		arriendo y de más costos que se deriven del mismo sin necesidad de aviso previo, siempre que medie
		incumplimiento por parte del ARRENDATARIO, por valor de <span class="campo-resaltado">$2.500.000 M/L</span>,
		el cual según el término del contrato deberá ser renovada cada 3 días, si el cupo no se puede
		renovar por la entidad financiera del tarjeta habiente el arrendatario debe presentar
		inmediatamente otra tarjeta que cumpla los requisitos del contrato, de no ser así estaría en
		incumplimiento el mismo y por lo tanto el ARRENDATARIO deberá devolver el vehículo inmediatamente y
		no se hará ningún tipo de reembolso por parte de la ARRENDADORA. Según las bases y tarifas que
		considere la empresa por el término pactado. El método de pago del presente contrato de
		arrendamiento se establece de la siguiente forma en <span class="campo-resaltado">{formatCOP(renta.total)}</span>
		pesos y de manera anticipada, y se establece como costo de Arrendamiento los siguientes valores.
	</div>

	<!-- CLÁUSULA CUARTA (PÁGINA 1) -->
	<div class="clausula-titulo">CLÁUSULA CUARTA: PLAZO DEL ARRENDAMIENTO</div>
	<div class="clausula-contenido">
		El plazo del arrendamiento se establece a partir de la siguiente fecha de inicio el
		<span class="campo-resaltado">{formatDate(renta.fechaRecogida)}</span>, y por fecha de término el
		<span class="campo-resaltado">{formatDate(renta.fechaRetorno)}</span>. Al término del arrendamiento
		el arrendatario deberá devolver el vehículo en el estado en que lo recibió, a las
		<span class="campo-resaltado">{hora(renta.horaRetorno)}</span> horas, vencido el cual podrá
		solicitarse el secuestro de este, por parte de la ARRENDADORA. La duración del arrendamiento surge
		del plazo anteriormente establecido, no pudiendo EL ARRENDATARIO alegar la tácita renovación
		automática del arrendamiento por ningún motivo; salvo estipulación en contrato celebrada entre las
		partes, pues de pretenderse la renovación o extensión del presente contrato, esta deberá ser
		notificada a la ARRENDADORA en un término mínimo de 48 horas, de forma que la ARRENDADORA pueda
		verificar la disponibilidad y viabilidad en la extensión de la entrega, quien podrá recuperarlo en
		cualquier momento y en forma que estime más conveniente a sus intereses, sin que EL ARRENDATARIO
		pueda oponer excepción alguna. El plazo en exceso del término convenido será pagado por EL
		ARRENDATARIO de conformidad con las tarifas plenas en vigencia para el alquiler del vehículo tomado
		en renta. De igual manera en los casos donde se presenta la entrega anticipada del vehículo a LA
		ARRENDADORA, se entenderá como incumplimiento parcial de la convención por lo cual será
		responsabilidad del ARRENDATARIO el pago de los días restantes para la finalización del contrato,
		de conformidad con las tarifas plenas en vigencia para alquiler de vehículo tomado en renta. El
		atraso en la devolución del vehículo hará incurrir al arrendatario en una multa de
		<span class="campo-resaltado">______________________ POR HORA</span> de retardo. Sin
		perjuicio de las demás acciones civiles o penales que se pudieren intentar en su contra.
	</div>

	<div class="clausula-contenido">
		Vencido el plazo EL ARRENDATARIO debe devolver el vehículo alquilado de forma inmediata en el mismo
		estado en que lo recibió y en domicilio del ARRENDADOR salvo estipulación en contrario celebrada
		entre las partes que acuerden como lugar de entrega del vehículo, alguno diferente para lo cual
		será el ARRENDATARIO quien deberá asumir los costos adicionales que de este contrato se deriven;
		tales como peajes, gasolina, parqueadero y gastos de movilización del conductor. Esta estipulación
		deberá ser anotada en la parte inferior del presente contrato. De presentarse cualquier plazo de
		exceso trascurrido hasta el momento de la devolución al ARRENDADOR del vehículo y su recepción a
		entera satisfacción, deberá pagar el ARRENDATARIO el valor de
		<span class="campo-resaltado">______________________ POR HORA</span>. Lo anterior será
		pagado por el ARRENDATARIO cuando se presenten retardos hasta por tres horas pues en adelante
		deberá cancelar la tarifa plena del vehículo rentado, estos valores serán descontados de los
		depósitos anticipados efectuados a favor de la ARRENDADORA; y si el monto resultante de la
		liquidación practicada en este instrumento no alcanzare a ser cubierto en su totalidad el depósito,
		el restante adecuado por EL ARRENDATARIO deberá ser pagado íntegramente en el momento de la
		devolución del vehículo arrendado.
	</div>

	<!-- CLÁUSULA QUINTA -->
	<div class="clausula-titulo">CLÁUSULA QUINTA: OBLIGACIONES Y PROHIBICIONES DEL ARRENDATARIO</div>
	<div class="clausula-contenido">
		EL ARRENDATARIO no podrá destinar el vehículo a un uso diferente para el que ha sido diseñado de
		fábrica (o al uso remunerado o como transporte de servicio público), o en labores peligrosas o
		ilícitas. Tampoco podrá ser conducido bajo la influencia del alcohol o en estado de ebriedad o bajo
		la influencia de sedantes, somníferos o drogas. EL ARRENDATARIO deberá portar al momento de la
		conducción todos sus documentos de identificación personal y de conductor. EL ARRENDADOR hace
		entrega al arrendatario de la siguiente documentación del vehículo: Permiso de Circulación,
		Certificado de Revisión Técnica, comprobante de seguro de accidentes personales. EL ARRENDATARIO
		deberá conducir en forma personal el vehículo, sin poder ceder su conducción a un tercero. Para
		este efecto se deja constancia que el arrendamiento se hace en base a la declaración del
		arrendatario mediante la cual declara encontrarse física y legalmente habilitado para conducir
		vehículos automotores, y que posee licencia vigente Nº
		<span class="campo-resaltado">{noLicencia}</span> tipo
		<span class="campo-resaltado">{tipoLicencia}</span>. Además se obliga a no utilizar el vehículo
		bajo las siguientes estipulaciones y le está totalmente prohibido conducir el automóvil en las
		siguientes condiciones:
	</div>

	<ol class="lista-numerada">
		<li>Deberá manejarlo personalmente (o lo manejará su conductor, esta persona deberá estar registrada ante la ARRENDADORA).</li>
		<li>No podrá subarrendarlo, cederlo o disponer de él en ninguna forma sin el consentimiento expreso manifestado por escrito por el ARRENDADOR, pues de lo contrario el ARRENDADOR podrá celebrar un nuevo contrato con los usuarios reales, sin necesidad de requerimientos judiciales o privados, a los cuales renuncia el ARRENDADOR.</li>
		<li>Conducir el vehículo en zonas fronterizas y FUERA DEL TERRITORIO DE LA REPUBLICA DE COLOMBIA, o fuera de CARTAGENA sin conocimiento y autorización expresa DEL ARRENDADOR; la sanción estipulada para lo anterior es $1000 M/T por KM recorrido por auto económico, compacto y sedan sencillo y $1500 M/T por KM por un automóvil Sedan de lujo y cualquier tipo de Camioneta, la distancia se calcula desde CARTAGENA como punto de origen hasta el lugar que el auto salió sin autorización sumando el recorrido de ida y regreso.</li>
		<li>Bajo las influencias de sustancias psicoactivas o alcohol, en estado de ebriedad cuando este bajo los efectos de calmantes, tranquilizantes o narcóticos. ESTA TOTALMENTE PROHIBIDO FUMAR, REGAR BEBIDAS, MOJAR LOS ASIENTOS CON AGUA DE MAR ETC, EL INCUMPLIMIENTO TENDRA UN CARGO DE $150.000.</li>
		<li>Conducir sin licencia valida, expedida por las autoridades competentes y aceptadas por las autoridades colombianas.</li>
		<li>Permitir conducir personas menores de edad y por ningún motivo y en ninguna circunstancia conducir un vehículo rentado.</li>
		<li>El transporte de personas en violación a la ley o bajo falsa identidad.</li>
		<li>En pruebas de competencias y velocidad o participar con el vehículo en carreras automovilísticas, por regularidad, etc., o certámenes de cualquier otro tipo, bajo ningún pretexto.</li>
		<li>En violación de la regla de tránsito o transporte de sobrecupo de pasajeros.</li>
		<li>Para el transporte, movilización o almacenamiento de sustancias de estupefacientes, armas de fuego o municiones, materiales inflamables, explosivos, combustibles o carburantes, en el momento de transgredir esta obligación o misiva, será el ARRENDATARIO responsable de los hechos que se deriven.</li>
		<li>Efectuar cualquier tipo de negociación particular sobre los vehículos rentados tales como ventas, depósito en prendas de cambio y en general cualquier tipo de convención que afecte o altere de algún modo la propiedad privada del vehículo, mediante la falsificación o copia no autorizada de los documentos del vehículo.</li>
	</ol>

	<!-- CLÁUSULA SEXTA -->
	<div class="clausula-titulo">CLÁUSULA SEXTA: REPRESENTACIÓN</div>
	<div class="clausula-contenido">
		En ningún motivo ni en ninguna circunstancia podrá EL ARRENDATARIO arrogarse el carácter de
		representante, agente, empleado o mandatario de esta compañía o como persona que pueda actuar en
		representación de esta.
	</div>

	<!-- CLÁUSULA SEPTIMA -->
	<div class="clausula-titulo">CLÁUSULA SEPTIMA: CLÁUSULA COMPROMISORIA</div>
	<div class="clausula-contenido">
		Toda controversia o diferencia relativa a este contrato, su ejecución y liquidación se resolverá
		por un tribunal de arbitramiento designado por la junta directiva de la cámara de comercio de
		{ciudadClausula}, mediante un sorteo efectuado entre los árbitros inscritos en las listas que lleva el
		centro de arbitraje y conciliación mercantiles de dicha cámara. EL tribunal se sujetará a lo
		dispuesto en el decreto 1818 de 1998 o estatuto orgánico de los sistemas alternos de solución de
		conflictos y demás normas concordantes de acuerdo con las siguientes reglas: a) el tribunal estará
		integrado por 5 árbitros; b) la organización interna del tribunal se sujetará a las reglas previas
		en el centro de arbitraje y conciliación mercantiles; c) el tribunal decidirá en derecho, en
		conciencia o en principios técnicos; d) el tribunal funcionará en el centro de arbitraje y
		conciliación mercantiles. A tal efecto constituye domicilio en {ciudadClausula}.
	</div>
	<div class="clausula-contenido">
		Debido a lo anterior se deja especial constancia que la anterior declaración es determinante para
		celebrar el presente contrato. Si la declaración antes mencionada resultase ser falsa, el
		arrendatario asumirá las responsabilidades del caso, cesando este arriendo de inmediato.
	</div>

	<!-- CLÁUSULA OCTAVA -->
	<div class="clausula-titulo">CLÁUSULA OCTAVA: CLÁUSULA PENAL</div>
	<div class="clausula-contenido">
		El incumplimiento por cualquiera de las partes de las obligaciones derivadas de este contrato la
		constituirá en deudora de la otra parte por el valor correspondiente al 50% de la suma total del
		valor del contrato a título de pena, o en caso de depósito de seguridad necesario para la renta del
		vehículo la pérdida de este en su totalidad, además de responder económicamente por los perjuicios
		que pudieren ocasionarse como consecuencia del incumplimiento.
	</div>

	<!-- CLÁUSULA NOVENA -->
	<div class="clausula-titulo">CLÁUSULA NOVENA: TERMINACIÓN UNILATERAL DEL CONTRATO</div>
	<div class="clausula-contenido">
		Son causales para la terminación del contrato las siguientes:
	</div>
	<ol class="lista-numerada">
		<li>La inmovilización del vehículo rentado por un término superior a 24 horas, cualquiera sea la causa de esta, y siempre que esta afecte de forma directa a los intereses de la empresa.</li>
		<li>La constante imposibilidad de comunicación entre los representantes de la ARRENDADORA y el ARRENDATARIO de forma que se bloquee el conocimiento pleno del estado del vehículo.</li>
		<li>La entrega tardía o anticipada del vehículo rentado.</li>
		<li>El no tener cupo de seguridad disponible en la tarjeta de crédito durante la vigencia de la renta del vehículo y no ser otorgada la renovación para la autorización de este por el banco emisor de la tarjeta.</li>
	</ol>
	<div class="clausula-contenido">
		EL ARRENDATARIO se compromete a dar cumplimiento a todas y cada una de estas cláusulas y acuerda
		que el incumplimiento de cualquiera de ellas dará lugar al ARRENDADOR a declarar rescindido el
		presente contrato sin necesidad de requerimiento previo por parte de este, quedando a cargo del
		primero todas las obligaciones que ha asumido con la firma del presente contrato, quedando la
		ARRENDADORA autorizada para retirar el vehículo del lugar donde se encuentre por medio de cualquiera
		de sus empleados y mediante los duplicados que tienen en su poder. EL ARRENDATARIO autoriza y acepta
		que el incumplimiento en el pago de las obligaciones económicas adquiridas en el presente contrato
		dará lugar a un reporte negativo a su historia crediticia con {razonSocial} a todas las centrales
		de riesgo como pro crédito y data crédito. Las controversias a las distintas cláusulas de
		este contrato configurarán para el ARRENDATARIO la consumación de los delitos penales que las
		circunstancias indiquen.
	</div>

	<!-- CLÁUSULA DÉCIMA -->
	<div class="clausula-titulo">CLÁUSULA DÉCIMA: RESPONSABILIDAD</div>
	<div class="clausula-contenido">
		EL ARRENDATARIO es enteramente responsable mientras permanezca en la tenencia del automóvil de la
		guardia material y jurídica del mismo asumiendo todas las responsabilidades civiles y penales que
		tal condición implica, hasta tanto proceda a su devolución AL ARRENDADOR y respondiendo
		económicamente por toda situación que sea responsabilidad suya y producto de su negligencia.
		Además, se hará responsable de todos los pagos que se deriven de sus conductas omisivas o por
		acción que resulte perjudicial para los intereses de la empresa cualquiera sea su naturaleza. En
		caso de siniestro el ARRENDATARIO deberá resarcir al ARRENDADOR a su compañía de seguros mediante el
		pago de los valores deducibles de la póliza de seguros los perjuicios que puedan ocasionarse. Si el
		pago se diera por medio distinto al efectivo aceptará pagar los recargos correspondientes a
		impuestos, comisiones financieras, retenciones, cuatro por mil entre otros pagos necesarios que se
		puedan derivar de la siguiente convención. En caso de lo contrario se procederá a ejecutar el
		correspondiente cobro jurídico para lo cual este contrato presta mérito ejecutivo sin necesidad de
		requerimiento privado ni judicial a las cuales renuncian expresamente el ARRENDATARIO. Por lo
		anterior el ARRENDADOR acepta las condiciones de la limitación de la póliza para vehículos en
		accidentes, muertes o lesiones y la cobertura por responsabilidad civil y terceras personas, la
		cual queda incorporada a este contrato.
	</div>

	<!-- CLÁUSULA DÉCIMA PRIMERA -->
	<div class="clausula-titulo">CLÁUSULA DÉCIMA PRIMERA: SEGURO DE LUCRO CESANTE</div>
	<div class="clausula-contenido">
		EL ARRENDATARIO podrá tomar a su cargo el costo adicional por el seguro de lucro cesante, a raíz de
		daños causados al vehículo objeto del ARRENDAMIENTO, cualquiera fuere la índole del hecho que dé
		como resultado imposibilidad física y/o jurídica de afectación del vehículo al servicio de
		alquiler. El valor de dicha póliza se encuentra expresamente contemplado dentro del presente
		contrato y de ser tomado por un día corriente de alquiler del vehículo mientras persista el término
		de ejecución del mismo. Este pago no exime al ARRENDATARIO de hacer frente a los gastos de
		recuperación o reparación del vehículo mediante el pago del valor del deducible de la póliza de
		seguro exigido por la aseguradora; y en caso de presentarse estos hechos o en caso de cualquier
		accidente, EL ARRENDATARIO deberá dar aviso inmediato al ARRENDADOR y a las autoridades no pudiendo
		exceder el lapso para hacerlo de 24 horas de ocurrido el mismo. De no dar previo aviso EL
		ARRENDADOR será responsable del pago total e íntegro del valor del lucro cesante que resultare del
		siniestro, o inmovilización del vehículo en la tarifa plena la cual fue rentada el vehículo.
	</div>

	<!-- PÓLIZA -->
	<div class="poliza-container">
		<p class="negrita" style="text-indent: 0; font-size: 8pt;">
			PÓLIZA DE SEGURO POR LUCRO CESANTE — SI DESEA TOMAR LA PÓLIZA MARQUE CON UNA X Y FIRME
		</p>
		<div class="linea-pago">
			<span class="casilla"></span>VEHICULO COMPACTO: cuarenta mil pesos diarios ($40.000)
			<span class="firma"></span>
		</div>
		<div class="linea-pago">
			<span class="casilla"></span>VEHICULO SEDAN: cincuenta mil pesos diarios ($50.000)
			<span class="firma"></span>
		</div>
		<div class="linea-pago">
			<span class="casilla"></span>VEHICULO CAMIONETA: setenta mil pesos diarios ($70.000)
			<span class="firma"></span>
		</div>
		<p style="margin: 6px 0 0 0; text-indent: 0;">
			De ocurrir algún siniestro o accidente de cualquier naturaleza, el arrendatario deberá dar aviso
			de inmediato al arrendador y concurrir hasta la unidad policial más cercana para dejar constancia
			del hecho.
		</p>
	</div>

	<!-- CLÁUSULA DÉCIMA SEGUNDA -->
	<div class="clausula-titulo">CLÁUSULA DÉCIMA SEGUNDA: FACTURA</div>
	<div class="clausula-contenido">
		El arrendador extenderá factura por el arriendo conforme lo establece la legislación vigente.
	</div>

	<!-- CLÁUSULA DÉCIMA TERCERA -->
	<div class="clausula-titulo">CLÁUSULA DÉCIMA TERCERA: DAÑOS A TERCEROS</div>
	<div class="clausula-contenido">
		El arrendatario se hace responsable de todo daño, perjuicio, lesión o muerte causada a terceros,
		sus bienes o enseres con ocasión o por causa de la conducción descuidada, culpable o dolosa. El
		arrendatario se obliga y compromete al fiel cumplimiento de las normas sobre regulación de tránsito
		debiendo responder personalmente por el pago de las multas o indemnizaciones a que resultare
		obligado o condenado.
	</div>

	<!-- CLÁUSULA DÉCIMA CUARTA -->
	<div class="clausula-titulo">CLÁUSULA DÉCIMA CUARTA: COBERTURA DE PÓLIZA DE SEGURO</div>
	<ol class="lista-numerada">
		<li>Protección por daños ocasionados al vehículo y/o a la propiedad privada de terceros.</li>
		<li>Protección por daños ocasionados a terceras personas que no constituyan pasajero del vehículo.</li>
		<li>Protección por daños a dos o más personas que no se constituyan en pasajero del vehículo.</li>
		<li>El vehículo rentado tendrá una cobertura de aseguramiento correspondiente al 90% quedando en cabeza del arrendatario la obligación de asumir el pago del valor deducible de la póliza de seguridad exigido por la aseguradora correspondiente al 10% que deberá ser pagado íntegramente y de forma inmediata por el arrendatario.</li>
		<li>La empresa aseguradora no se hará responsable del reconocimiento de pago en casos de siniestros o daños ocasionados al vehículo por valores inferiores a $2.500.000 (DOS MILLONES QUINIENTOS MIL PESOS M/L). Este valor de $2.500.000, y daños superiores a este correspondería al 10% del valor del siniestro en general. Daños a terceros tiene una cobertura del 100%.</li>
		<li>Asistencia médica prioritaria en accidente de tránsito, incluida la movilización de heridos a centros hospitalarios.</li>
		<li>Asistencia legal.</li>
		<li>Asistencia en caso de fallas técnicas a nivel nacional, que imposibiliten el uso del vehículo tomado en arriendo.</li>
	</ol>
	<div class="clausula-contenido">
		El arrendatario se obliga a tomar un seguro por el vehículo arrendado, contra todo riesgo, a favor
		del arrendador por la cantidad de $<span class="campo-resaltado">____________________</span>. Son de
		cargo del arrendatario: a) los daños ocasionados al vehículo o a sus accesorios, por sí o por un
		tercero, aun por caso fortuito; b) los daños que se causen a terceros; c) los perjuicios que pueda
		experimentar por causa de robo.
	</div>

	<!-- CLÁUSULA FOTOMULTAS (así figura en el documento original) -->
	<div class="clausula-titulo">CLÁUSULA DÉCIMA TERCERA: FOTO MULTA O COMPARENDOS DE TRÁNSITO</div>
	<div class="clausula-contenido">
		El ARRENDATARIO es responsable por todas las fotomultas o comparendos emitidos por la entidad de
		tránsito encargada, como consecuencia de las infracciones de tránsito practicadas durante el
		período de vigencia de este contrato,		AUTORIZANDO a {razonSocial} (EL ARRENDADOR) para
		efectuar la cobranza de las mismas, acrecidas en 15% (QUINCE POR CIENTO) como cargo de gestión
		administrativa, automáticamente de la tarjeta de crédito presentada como garantía en este contrato
		de arrendamiento de vehículo. La empresa no es responsable de asumir y asistir a cursos de tránsito
		para generar descuentos en las fotomultas, en caso de presentarse algún problema con el cobro
		correspondiente a lo acordado en la siguiente cláusula en relación a la tarjeta de crédito el
		ARRENDADOR en los términos anteriormente descritos. Si pasados los 5 días hábiles contados a partir
		de enviado el correo electrónico el ARRENDATARIO no se ha comunicado con el ARRENDADOR para acordar
		el pago de lo debido según la presente cláusula se iniciará en contra del ARRENDATARIO el respectivo
		cobro pre jurídico y jurídico con reporte a las centrales de riesgo. El ARRENDATARIO asumirá todos
		los gastos correspondientes al respectivo proceso.
	</div>

	<!-- CLÁUSULA ADICIONAL -->
	<div class="clausula-titulo">CLÁUSULA ADICIONAL</div>
	<div class="clausula-contenido" style="height: 40px;"></div>

	<!-- FIRMAS -->
	<div class="firmas-container">
		<p style="text-indent: 0;">
			En comprobante de previa lectura, ratifican y firman:
		</p>

		<div class="firma-linea"></div>
		<p style="margin: 2px 0; text-indent: 0; font-weight: bold;">Arrendador</p>
		<p style="margin: 2px 0; text-indent: 0;">{razonSocial}{#if empresa.nitMostrar} · RUT {empresa.nitMostrar}{/if}</p>

		<div class="firma-linea"></div>
		<p style="margin: 2px 0; text-indent: 0; font-weight: bold; text-transform: uppercase;">
			{renta.nombreCliente}
		</p>
		<p style="margin: 2px 0; text-indent: 0;">{tipoDoc} No. {noDoc}</p>

		<p style="margin-top: 18px; text-indent: 0;">
			Firmaron ante mí en las calidades que comparecen. {empresa.ciudadMostrar}{#if empresa.ciudadMostrar}, {/if}{hoy}.
		</p>
	</div>

	<!-- PIE DE PÁGINA -->
	<div class="footer">
		<p>{pieContacto}</p>
		<p>Documento legal conforme a la legislación colombiana • Impreso el {hoy}</p>
	</div>
</div>
