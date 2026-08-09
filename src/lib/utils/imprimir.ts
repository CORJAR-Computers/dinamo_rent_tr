// imprimir.ts — Impresión de documentos (órdenes y contratos).
//
// El área imprimible (.print-area) vive dentro de un Modal con overflow
// propio. Para imprimir hay que sacarla de esa cadena de contenedores con
// scroll: se clona el documento a un contenedor raíz de <body> (fuera del
// layout de la app), se activa la clase `printing` y se llama a window.print().
// De este modo el navegador pagina el contenido de forma natural, incluso
// documentos largos de varias páginas (contrato).
//
// NOTA (WebView2/Chromium): el diálogo de impresión del navegador tiene la
// opción «Encabezados y pies de página» activada por defecto, que superpone
// su propio encabezado (título + fecha) y pie (URL + «1/3»). Si el usuario
// la deja marcada, ese pie adicional aparece junto al pie propio del
// contrato («Página X de Y»). No es controlable desde código; se recomienda
// desmarcarla en el diálogo. Para que el encabezado (si se imprime) diga
// algo útil, se renombra temporalmente document.title con el nombre del
// documento y se restaura al terminar.
//
// Las reglas @media print de app.css muestran únicamente:
//   - el clon raíz (`.print-clone`) cuando `body.printing-clone` está activo
//   - el `.print-area` original en el caso de una sola área (backup)

/** Imprime el documento `.print-area` visible en la página. */
export function imprimirDocumento(): void {
	if (typeof document === 'undefined') return;
	const area = document.querySelector<HTMLElement>('.print-area');
	if (!area) return;
	if (document.querySelector('#print-clone')) return; // impresión en curso

	const clon = area.cloneNode(true) as HTMLElement;
	clon.classList.add('print-clone');
	clon.id = 'print-clone';
	// Insertar como PRIMER hijo del body: si va al final, el layout de la app
	// (sidebar + main, oculto con visibility:hidden pero que SÍ ocupa espacio)
	// consumiría la primera página impresa y el documento empezaría en la
	// página 2 con la hoja 1 en blanco.
	document.body.prepend(clon);
	document.body.classList.add('printing', 'printing-clone');

	// Si el diálogo de impresión imprime «Encabezados y pies de página», el
	// encabezado muestra document.title: lo cambiamos al nombre del documento
	// durante la impresión y lo restauramos al terminar. Las clases marcadoras
	// están en la RAÍZ del área imprimible (el propio clon), por eso se usa
	// classList.contains y no querySelector.
	const tituloOriginal = document.title;
	if (clon.classList.contains('contrato-carta')) document.title = 'Contrato de renta';
	else if (clon.classList.contains('orden-carta')) document.title = 'Orden de renta';

	function limpiar() {
		document.body.classList.remove('printing', 'printing-clone');
		clon.remove();
		document.title = tituloOriginal;
		window.removeEventListener('afterprint', limpiar);
	}

	// Espera a que fuentes e imágenes del clon estén listas antes de imprimir
	// (el logo del encabezado puede tardar); con tope por si algo no carga.
	const esperarCarga = Promise.allSettled([
		document.fonts ? document.fonts.ready : Promise.resolve(),
		...Array.from(clon.querySelectorAll('img')).map(
			(img) =>
				new Promise((res) => {
					if (img.complete) return res(null);
					img.onload = () => res(null);
					img.onerror = () => res(null);
				})
		)
	]);

	const conTope = Promise.race([esperarCarga, new Promise((res) => setTimeout(res, 1500))]);

	conTope.then(() => {
		window.addEventListener('afterprint', limpiar);
		window.print();
		// Respaldo por si el evento afterprint no llega (p. ej. cancelación).
		setTimeout(limpiar, 1000);
	});
}
