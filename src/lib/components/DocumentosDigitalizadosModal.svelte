<script lang="ts">
	import { onMount } from 'svelte';
	import Modal from './Modal.svelte';
	import Icon from './Icon.svelte';
	import { syncWebApi } from '$lib/api';
	import { empresa } from '$lib/stores/empresa.svelte';
	import { sid } from '$lib/stores/session.svelte';
	import { imprimirDocumento } from '$lib/utils/imprimir';

	interface Props {
		open: boolean;
		codigoReserva: string;
		nombreCliente?: string;
		onClose: () => void;
	}

	let { open, codigoReserva, nombreCliente = '', onClose }: Props = $props();

	let cargando = $state(false);
	let error = $state('');
	let docs = $state<{
		code?: string;
		docPhotoUrl?: string | null;
		licensePhotoUrl?: string | null;
		signatureDataUrl?: string | null;
		customer?: {
			fullName: string;
			docNumber: string;
			docType?: string;
			license?: string | null;
			phone: string;
			email: string;
		};
	} | null>(null);

	let fotoAmpliada = $state<{ url: string; titulo: string } | null>(null);

	const hoy = new Date().toLocaleDateString('es-CO', {
		year: 'numeric',
		month: 'long',
		day: 'numeric'
	});

	onMount(() => {
		void empresa.cargarCompleta(sid());
	});

	$effect(() => {
		if (open && codigoReserva) {
			void cargarDocumentos(codigoReserva);
		} else if (!open) {
			docs = null;
			error = '';
			fotoAmpliada = null;
		}
	});

	async function cargarDocumentos(code: string) {
		cargando = true;
		error = '';
		try {
			const res = await syncWebApi.obtenerDocumentosReserva(code);
			if (res.ok) {
				docs = res;
			} else {
				error = res.error || 'No se pudieron recuperar los documentos de esta reserva.';
			}
		} catch (e) {
			error = (e as Error).message;
		} finally {
			cargando = false;
		}
	}

	function descargarImagen(dataUrl: string | null | undefined, nombreArchivo: string) {
		if (!dataUrl) return;
		const a = document.createElement('a');
		a.href = dataUrl;
		a.download = nombreArchivo;
		document.body.appendChild(a);
		a.click();
		document.body.removeChild(a);
	}

	function imprimirAnexo() {
		imprimirDocumento();
	}
</script>

<Modal
	{open}
	title={`Documentos digitalizados — Reserva ${codigoReserva}`}
	subtitle={`Documento de identidad y licencia del titular: ${docs?.customer?.fullName || nombreCliente}`}
	{onClose}
	width="max-w-5xl"
	rawBody
>
	{#snippet children()}
		<div class="p-6 overflow-y-auto max-h-[80vh] din-scroll">
			{#if cargando}
				<div class="py-16 flex flex-col items-center justify-center gap-3 text-text-secondary">
					<div
						class="w-8 h-8 rounded-full border-2 border-primary border-t-transparent animate-spin"
					></div>
					<p class="text-sm font-medium">Cargando documentos digitalizados desde la nube…</p>
				</div>
			{:else if error}
				<div
					class="p-6 rounded-2xl bg-peligro/10 border border-peligro/20 text-center text-peligro space-y-3"
				>
					<Icon name="alert" class="w-8 h-8 mx-auto text-peligro" />
					<p class="text-sm font-semibold">{error}</p>
					<button
						type="button"
						class="px-4 py-2 rounded-xl text-xs font-bold bg-peligro text-white hover:bg-peligro/90 transition-all cursor-pointer"
						onclick={() => cargarDocumentos(codigoReserva)}
					>
						Reintentar carga
					</button>
				</div>
			{:else if docs}
				<!-- Ficha resumen del cliente -->
				<div
					class="mb-6 p-4 rounded-2xl bg-surface-hover/50 border border-border flex flex-wrap items-center justify-between gap-4 text-xs"
				>
					<div>
						<p class="text-[10px] font-bold uppercase tracking-wider text-text-tertiary">
							Titular de la reserva
						</p>
						<p class="text-base font-bold text-text-primary">
							{docs.customer?.fullName || nombreCliente}
						</p>
					</div>
					<div>
						<p class="text-[10px] font-bold uppercase tracking-wider text-text-tertiary">
							Documento
						</p>
						<p class="text-sm font-semibold text-text-primary">
							{docs.customer?.docType || 'CC'}: {docs.customer?.docNumber || '—'}
						</p>
					</div>
					{#if docs.customer?.license}
						<div>
							<p class="text-[10px] font-bold uppercase tracking-wider text-text-tertiary">
								No. Licencia
							</p>
							<p class="text-sm font-semibold text-text-primary">{docs.customer.license}</p>
						</div>
					{/if}
					<div>
						<p class="text-[10px] font-bold uppercase tracking-wider text-text-tertiary">
							Teléfono
						</p>
						<p class="text-sm font-semibold text-text-primary">{docs.customer?.phone || '—'}</p>
					</div>
					<div class="ml-auto flex items-center gap-2">
						<button
							type="button"
							class="inline-flex items-center gap-2 px-4 py-2 rounded-xl text-xs font-bold bg-primary text-white shadow-sm hover:bg-primary-hover transition-all cursor-pointer"
							onclick={imprimirAnexo}
						>
							<Icon name="printer" class="w-4 h-4" />
							<span>Imprimir Anexo Carta</span>
						</button>
					</div>
				</div>

				<!-- Galería de Documentos -->
				<div class="grid grid-cols-1 md:grid-cols-2 gap-6 mb-6">
					<!-- 1. Documento de Identidad -->
					<div
						class="rounded-2xl border border-border bg-surface p-4 flex flex-col justify-between shadow-sm"
					>
						<div>
							<div class="flex items-center justify-between mb-3">
								<div class="flex items-center gap-2">
									<div
										class="w-7 h-7 rounded-lg bg-primary/10 text-primary flex items-center justify-center font-bold text-xs"
									>
										1
									</div>
									<h4 class="font-bold text-sm text-text-primary">Documento de Identidad</h4>
								</div>
								<span
									class="px-2 py-0.5 rounded-full text-[10px] font-bold uppercase tracking-wider {docs.docPhotoUrl
										? 'bg-exito/10 text-exito'
										: 'bg-peligro/10 text-peligro'}"
								>
									{docs.docPhotoUrl ? 'Digitalizado' : 'Pendiente'}
								</span>
							</div>

							{#if docs.docPhotoUrl}
								<div
									class="relative rounded-xl overflow-hidden bg-slate-900/5 dark:bg-slate-900/50 border border-border/80 aspect-[4/3] flex items-center justify-center group"
								>
									<button
										type="button"
										class="w-full h-full flex items-center justify-center cursor-pointer p-0 border-0 bg-transparent"
										onclick={() =>
											(fotoAmpliada = {
												url: docs?.docPhotoUrl || '',
												titulo: 'Documento de Identidad'
											})}
										title="Ampliar documento"
									>
										<img
											src={docs.docPhotoUrl}
											alt="Documento de Identidad"
											class="w-full h-full object-contain transition-transform group-hover:scale-105"
										/>
									</button>
									<button
										type="button"
										class="absolute bottom-2 right-2 px-2.5 py-1.5 rounded-lg bg-black/75 hover:bg-black text-white text-[11px] font-semibold backdrop-blur-sm transition-all flex items-center gap-1.5 opacity-90 hover:opacity-100"
										onclick={() =>
											(fotoAmpliada = {
												url: docs?.docPhotoUrl || '',
												titulo: 'Documento de Identidad'
											})}
									>
										<Icon name="eye" class="w-3.5 h-3.5" />
										<span>Ampliar</span>
									</button>
								</div>
							{:else}
								<div
									class="rounded-xl border border-dashed border-border/80 aspect-[4/3] flex flex-col items-center justify-center text-text-tertiary gap-2 p-6 text-center"
								>
									<Icon name="document" class="w-8 h-8 opacity-40" />
									<p class="text-xs">No se adjuntó fotografía del documento en la web.</p>
								</div>
							{/if}
						</div>

						{#if docs.docPhotoUrl}
							<div class="mt-4 pt-3 border-t border-border flex justify-end">
								<button
									type="button"
									class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold text-text-secondary hover:text-text-primary hover:bg-surface-hover border border-border transition-all cursor-pointer"
									onclick={() =>
										descargarImagen(
											docs?.docPhotoUrl,
											`documento_${docs?.customer?.docNumber || codigoReserva}.webp`
										)}
								>
									<Icon name="download" class="w-3.5 h-3.5" />
									<span>Descargar archivo</span>
								</button>
							</div>
						{/if}
					</div>

					<!-- 2. Licencia de Conducción -->
					<div
						class="rounded-2xl border border-border bg-surface p-4 flex flex-col justify-between shadow-sm"
					>
						<div>
							<div class="flex items-center justify-between mb-3">
								<div class="flex items-center gap-2">
									<div
										class="w-7 h-7 rounded-lg bg-primary/10 text-primary flex items-center justify-center font-bold text-xs"
									>
										2
									</div>
									<h4 class="font-bold text-sm text-text-primary">Licencia de Conducción</h4>
								</div>
								<span
									class="px-2 py-0.5 rounded-full text-[10px] font-bold uppercase tracking-wider {docs.licensePhotoUrl
										? 'bg-exito/10 text-exito'
										: 'bg-peligro/10 text-peligro'}"
								>
									{docs.licensePhotoUrl ? 'Digitalizada' : 'Pendiente'}
								</span>
							</div>

							{#if docs.licensePhotoUrl}
								<div
									class="relative rounded-xl overflow-hidden bg-slate-900/5 dark:bg-slate-900/50 border border-border/80 aspect-[4/3] flex items-center justify-center group"
								>
									<button
										type="button"
										class="w-full h-full flex items-center justify-center cursor-pointer p-0 border-0 bg-transparent"
										onclick={() =>
											(fotoAmpliada = {
												url: docs?.licensePhotoUrl || '',
												titulo: 'Licencia de Conducción'
											})}
										title="Ampliar licencia"
									>
										<img
											src={docs.licensePhotoUrl}
											alt="Licencia de Conducción"
											class="w-full h-full object-contain transition-transform group-hover:scale-105"
										/>
									</button>
									<button
										type="button"
										class="absolute bottom-2 right-2 px-2.5 py-1.5 rounded-lg bg-black/75 hover:bg-black text-white text-[11px] font-semibold backdrop-blur-sm transition-all flex items-center gap-1.5 opacity-90 hover:opacity-100"
										onclick={() =>
											(fotoAmpliada = {
												url: docs?.licensePhotoUrl || '',
												titulo: 'Licencia de Conducción'
											})}
									>
										<Icon name="eye" class="w-3.5 h-3.5" />
										<span>Ampliar</span>
									</button>
								</div>
							{:else}
								<div
									class="rounded-xl border border-dashed border-border/80 aspect-[4/3] flex flex-col items-center justify-center text-text-tertiary gap-2 p-6 text-center"
								>
									<Icon name="car" class="w-8 h-8 opacity-40" />
									<p class="text-xs">No se adjuntó fotografía de la licencia en la web.</p>
								</div>
							{/if}
						</div>

						{#if docs.licensePhotoUrl}
							<div class="mt-4 pt-3 border-t border-border flex justify-end">
								<button
									type="button"
									class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-semibold text-text-secondary hover:text-text-primary hover:bg-surface-hover border border-border transition-all cursor-pointer"
									onclick={() =>
										descargarImagen(
											docs?.licensePhotoUrl,
											`licencia_${docs?.customer?.docNumber || codigoReserva}.webp`
										)}
								>
									<Icon name="download" class="w-3.5 h-3.5" />
									<span>Descargar archivo</span>
								</button>
							</div>
						{/if}
					</div>
				</div>

				<!-- Firma digital (si existe) -->
				{#if docs.signatureDataUrl}
					<div class="rounded-2xl border border-border bg-surface p-4 mb-6 shadow-sm">
						<div class="flex items-center justify-between mb-3">
							<h4 class="font-bold text-sm text-text-primary">Firma Digital del Cliente</h4>
							<span
								class="px-2 py-0.5 rounded-full text-[10px] font-bold uppercase tracking-wider bg-primary/10 text-primary"
							>
								Verificada en Web
							</span>
						</div>
						<div
							class="rounded-xl bg-white border border-slate-200 p-4 max-w-md mx-auto flex items-center justify-center h-28"
						>
							<img
								src={docs.signatureDataUrl}
								alt="Firma del Cliente"
								class="max-h-full object-contain"
							/>
						</div>
						<p class="text-[11px] text-center text-text-tertiary mt-2">
							Firma estampada digitalmente por {docs.customer?.fullName} en aceptación de términos y condiciones.
						</p>
					</div>
				{/if}

				<!-- ── ÁREA IMPRIMIBLE OCULTA EN PANTALLA, ACTIVA AL IMPRIMIR (.print-area.anexo-carta) ── -->
				<div class="print-area anexo-carta hidden bg-white text-slate-900 p-8">
					<!-- Encabezado corporativo -->
					<div class="border-b-2 border-slate-800 pb-4 mb-6 flex items-start justify-between gap-5">
						<div class="flex items-center gap-3">
							<div
								class="w-16 h-16 rounded-lg bg-blue-900 text-white flex items-center justify-center shrink-0 p-2"
							>
								<img
									src={empresa.logoSrc}
									alt={empresa.nombreMostrar}
									class="w-12 h-12 object-contain rounded-md"
								/>
							</div>
							<div>
								<p class="text-lg font-black tracking-tight">
									{empresa.nombreMostrar.toUpperCase()}
								</p>
								<p class="text-xs text-slate-600">
									NIT: {empresa.nit || '901.812.836-1'} · Cartagena de Indias
								</p>
								<p class="text-xs text-slate-600">Renta de vehículos · Anexo Documental</p>
							</div>
						</div>
						<div class="text-right">
							<p class="text-2xl font-black tracking-wide text-blue-900">ANEXO DE DOCUMENTACIÓN</p>
							<p class="text-sm font-bold mt-1">
								Reserva No. <span class="tabular-nums">{codigoReserva}</span>
							</p>
							<p class="text-xs text-slate-600 mt-0.5">Fecha: {hoy}</p>
						</div>
					</div>

					<!-- Datos del Cliente -->
					<div class="rounded-lg border border-slate-300 p-3.5 mb-6 text-xs grid grid-cols-3 gap-3">
						<div>
							<span class="text-slate-500 font-bold block uppercase text-[10px]">Titular:</span>
							<span class="text-sm font-bold text-slate-900"
								>{docs.customer?.fullName || nombreCliente}</span
							>
						</div>
						<div>
							<span class="text-slate-500 font-bold block uppercase text-[10px]"
								>Identificación:</span
							>
							<span class="text-sm font-bold text-slate-900"
								>{docs.customer?.docType || 'CC'} {docs.customer?.docNumber || '—'}</span
							>
						</div>
						<div>
							<span class="text-slate-500 font-bold block uppercase text-[10px]"
								>No. Licencia Conducción:</span
							>
							<span class="text-sm font-bold text-slate-900">{docs.customer?.license || '—'}</span>
						</div>
						<div>
							<span class="text-slate-500 font-bold block uppercase text-[10px]">Teléfono:</span>
							<span class="font-semibold text-slate-900">{docs.customer?.phone || '—'}</span>
						</div>
						<div class="col-span-2">
							<span class="text-slate-500 font-bold block uppercase text-[10px]"
								>Correo Electrónico:</span
							>
							<span class="font-semibold text-slate-900">{docs.customer?.email || '—'}</span>
						</div>
					</div>

					<!-- Fotos impresas en calidad nítida -->
					<div class="grid grid-cols-2 gap-6 mb-6">
						<!-- Documento -->
						<div class="border border-slate-400 rounded-lg p-2.5">
							<p
								class="text-xs font-black uppercase text-blue-900 mb-2 border-b border-slate-200 pb-1"
							>
								1. Documento de Identidad (Cédula / Pasaporte)
							</p>
							{#if docs.docPhotoUrl}
								<div class="h-64 flex items-center justify-center overflow-hidden">
									<img
										src={docs.docPhotoUrl}
										alt="Documento"
										class="max-h-full max-w-full object-contain"
									/>
								</div>
							{:else}
								<div
									class="h-64 flex items-center justify-center text-slate-400 italic text-xs border border-dashed border-slate-300"
								>
									Sin imagen adjunta
								</div>
							{/if}
						</div>

						<!-- Licencia -->
						<div class="border border-slate-400 rounded-lg p-2.5">
							<p
								class="text-xs font-black uppercase text-blue-900 mb-2 border-b border-slate-200 pb-1"
							>
								2. Licencia de Conducción
							</p>
							{#if docs.licensePhotoUrl}
								<div class="h-64 flex items-center justify-center overflow-hidden">
									<img
										src={docs.licensePhotoUrl}
										alt="Licencia"
										class="max-h-full max-w-full object-contain"
									/>
								</div>
							{:else}
								<div
									class="h-64 flex items-center justify-center text-slate-400 italic text-xs border border-dashed border-slate-300"
								>
									Sin imagen adjunta
								</div>
							{/if}
						</div>
					</div>

					<!-- Sección de verificación y firma -->
					<div class="mt-8 pt-4 border-t border-slate-300 grid grid-cols-2 gap-8 text-xs">
						<div>
							<p class="font-bold text-slate-700 mb-1">Declaración del Cliente:</p>
							<p class="text-[11px] text-slate-600 leading-relaxed">
								Certifico que los documentos aportados corresponden fielmente a mi identidad y
								licencia de conducción vigente para la conducción del vehículo arrendado.
							</p>
							{#if docs.signatureDataUrl}
								<div class="mt-2 h-16 flex items-center">
									<img
										src={docs.signatureDataUrl}
										alt="Firma del cliente"
										class="max-h-full object-contain"
									/>
								</div>
							{:else}
								<div class="mt-8 border-b border-slate-400 w-48"></div>
							{/if}
							<p class="text-[10px] text-slate-500 mt-1">Firma del Titular</p>
						</div>

						<div>
							<p class="font-bold text-slate-700 mb-1">Verificación en Mostrador Dinamo:</p>
							<p class="text-[11px] text-slate-600 leading-relaxed">
								Documentos físicos contrastados con los digitales y validados por el asesor de
								servicio al momento de la entrega del vehículo.
							</p>
							<div class="mt-8 border-b border-slate-400 w-48"></div>
							<p class="text-[10px] text-slate-500 mt-1">Firma y Sello Asesor de Mostrador</p>
						</div>
					</div>

					<p class="text-center text-[10px] text-slate-400 mt-8 border-t border-slate-200 pt-2">
						{empresa.nombreMostrar} · Reserva {codigoReserva} · Anexo Documental impreso el {hoy}
					</p>
				</div>
			{/if}
		</div>
	{/snippet}
</Modal>

<!-- Modal emergente para Zoom / Pantalla completa de una foto -->
{#if fotoAmpliada}
	<div
		class="fixed inset-0 z-[9999] bg-black/90 flex flex-col items-center justify-center p-4 backdrop-blur-sm animate-in fade-in"
		role="dialog"
		tabindex="-1"
		aria-modal="true"
		aria-label={fotoAmpliada.titulo}
	>
		<div class="w-full max-w-4xl flex items-center justify-between text-white mb-3 px-2">
			<h3 class="text-sm font-bold">{fotoAmpliada.titulo} — {codigoReserva}</h3>
			<div class="flex items-center gap-2">
				<button
					type="button"
					class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-white/10 hover:bg-white/20 text-xs font-semibold transition-all cursor-pointer"
					onclick={() =>
						descargarImagen(fotoAmpliada?.url, `${fotoAmpliada?.titulo}_${codigoReserva}.webp`)}
				>
					<Icon name="download" class="w-4 h-4" />
					<span>Descargar</span>
				</button>
				<button
					type="button"
					class="p-2 rounded-lg bg-white/10 hover:bg-white/20 text-white transition-all cursor-pointer"
					onclick={() => (fotoAmpliada = null)}
					aria-label="Cerrar vista ampliada"
				>
					<Icon name="x" class="w-5 h-5" />
				</button>
			</div>
		</div>
		<div
			class="max-w-4xl max-h-[85vh] overflow-hidden rounded-2xl bg-black border border-white/10 flex items-center justify-center p-2"
		>
			<img
				src={fotoAmpliada.url}
				alt={fotoAmpliada.titulo}
				class="max-h-[80vh] max-w-full object-contain rounded-xl"
			/>
		</div>
	</div>
{/if}
