/**
 * Generación de reportes en PDF.
 *
 * El documento se arma en la interfaz con jsPDF y se guarda en el equipo
 * mediante un comando nativo, sin enviar nada por internet.
 */
import type { HistorySummary, Transaction } from '../types';
import {
  TRANSACTION_TYPE_LABELS,
  exponentOf,
  formatAmount,
  formatDate,
  symbolOf,
} from './format';

const ANCHO = 210;
const MARGEN = 16;
const ANCHO_UTIL = ANCHO - MARGEN * 2;

export interface PdfReportInput {
  periodLabel: string;
  currency: string;
  summary: HistorySummary | null;
  transactions: Transaction[];
  generatedAt: string;
}

function money(value: number, currency: string): string {
  return `${symbolOf(currency)}${formatAmount(value, exponentOf(currency))}`;
}

/** Genera el reporte en PDF y devuelve su contenido en base64. */
export async function buildHistoryPdf(input: PdfReportInput): Promise<string> {
  // La librería de PDF se carga sólo cuando el usuario la necesita.
  const { jsPDF } = await import('jspdf');
  const doc = new jsPDF({ unit: 'mm', format: 'a4', orientation: 'portrait' });
  const { periodLabel, currency, summary, transactions, generatedAt } = input;

  let y = MARGEN;

  // Encabezado
  doc.setFont('helvetica', 'bold');
  doc.setFontSize(18);
  doc.setTextColor(35, 33, 29);
  doc.text('Aureon', MARGEN, y);
  doc.setFontSize(11);
  doc.setFont('helvetica', 'normal');
  doc.setTextColor(110, 102, 93);
  doc.text('Reporte financiero', MARGEN, y + 6);

  doc.setFontSize(9);
  doc.text(`Período: ${periodLabel}`, ANCHO - MARGEN, y, { align: 'right' });
  doc.text(`Generado el ${formatDate(generatedAt)}`, ANCHO - MARGEN, y + 5, {
    align: 'right',
  });
  y += 14;

  doc.setDrawColor(176, 139, 74);
  doc.setLineWidth(0.4);
  doc.line(MARGEN, y, ANCHO - MARGEN, y);
  y += 8;

  // Indicadores
  if (summary) {
    doc.setFontSize(12);
    doc.setFont('helvetica', 'bold');
    doc.setTextColor(35, 33, 29);
    doc.text('Resumen', MARGEN, y);
    y += 6;

    const filas: [string, string][] = [
      ['Ingresos', money(summary.income, currency)],
      ['Gastos', money(summary.expenses, currency)],
      ['Balance neto', money(summary.net, currency)],
      ['Devoluciones', money(summary.refunds, currency)],
      ['Transferencias entre cuentas', money(summary.transfers, currency)],
      ['Compras realizadas', money(summary.purchasesTotal, currency)],
      ['Pagos de cuotas', money(summary.installmentsPaid, currency)],
      ['Pagos de suscripciones', money(summary.subscriptionsPaid, currency)],
      ['Dinero cobrado', money(summary.moneyReceived, currency)],
      ['Dinero pagado', money(summary.moneyPaid, currency)],
      ['Movimientos registrados', String(summary.transactionCount)],
    ];

    doc.setFontSize(9);
    const anchoColumna = ANCHO_UTIL / 2;
    filas.forEach(([etiqueta, valor], indice) => {
      const columna = indice % 2;
      const fila = Math.floor(indice / 2);
      const x = MARGEN + columna * anchoColumna;
      const yFila = y + fila * 6;
      doc.setFont('helvetica', 'normal');
      doc.setTextColor(110, 102, 93);
      doc.text(etiqueta, x, yFila);
      doc.setFont('helvetica', 'bold');
      doc.setTextColor(35, 33, 29);
      doc.text(valor, x + anchoColumna - 4, yFila, { align: 'right' });
    });
    y += Math.ceil(filas.length / 2) * 6 + 6;
  }

  // Movimientos
  doc.setFontSize(12);
  doc.setFont('helvetica', 'bold');
  doc.setTextColor(35, 33, 29);
  doc.text('Movimientos', MARGEN, y);
  y += 6;

  const columnas = [
    { titulo: 'Fecha', x: MARGEN, ancho: 22, align: 'left' as const },
    { titulo: 'Concepto', x: MARGEN + 22, ancho: 68, align: 'left' as const },
    { titulo: 'Categoría', x: MARGEN + 90, ancho: 40, align: 'left' as const },
    { titulo: 'Cuenta', x: MARGEN + 130, ancho: 30, align: 'left' as const },
    { titulo: 'Monto', x: ANCHO - MARGEN, ancho: 28, align: 'right' as const },
  ];

  doc.setFillColor(246, 245, 242);
  doc.rect(MARGEN, y - 4, ANCHO_UTIL, 6, 'F');
  doc.setFontSize(8);
  doc.setFont('helvetica', 'bold');
  doc.setTextColor(140, 132, 118);
  columnas.forEach((columna) => {
    doc.text(columna.titulo, columna.x, y, { align: columna.align });
  });
  y += 6;

  const limite = Math.min(transactions.length, 120);
  doc.setFont('helvetica', 'normal');
  doc.setFontSize(8);
  doc.setTextColor(35, 33, 29);

  for (let indice = 0; indice < limite; indice += 1) {
    if (y > 280) {
      doc.addPage();
      y = MARGEN;
    }
    const item = transactions[indice];
    const ingreso = item.transactionType === 'income' || item.transactionType === 'refund';
    const monto = `${ingreso ? '+' : '-'}${formatAmount(item.amount, exponentOf(item.currencyCode))}`;
    const valores = [
      formatDate(item.date),
      item.description.slice(0, 44),
      (item.categoryName ?? '—').slice(0, 20),
      (item.accountName ?? '—').slice(0, 15),
      monto,
    ];
    columnas.forEach((columna, indiceColumna) => {
      if (indiceColumna === 4) {
        doc.setFont('helvetica', 'bold');
        doc.setTextColor(ingreso ? 47 : 176, ingreso ? 122 : 87, ingreso ? 90 : 75);
      } else {
        doc.setFont('helvetica', 'normal');
        doc.setTextColor(80, 76, 70);
      }
      doc.text(valores[indiceColumna], columna.x, y, { align: columna.align });
    });
    y += 5;
    doc.setDrawColor(231, 227, 218);
    doc.setLineWidth(0.1);
    doc.line(MARGEN, y - 3.5, ANCHO - MARGEN, y - 3.5);
  }

  if (transactions.length > limite) {
    y += 2;
    doc.setFontSize(8);
    doc.setFont('helvetica', 'italic');
    doc.setTextColor(140, 132, 118);
    doc.text(
      `Se muestran los primeros ${limite} movimientos de ${transactions.length}. Exporta el CSV para ver el detalle completo.`,
      MARGEN,
      y,
    );
  }

  y += 10;
  doc.setFont('helvetica', 'normal');
  doc.setFontSize(8);
  doc.setTextColor(140, 132, 118);
  doc.text(
    'Documento generado localmente por Aureon. Los datos no se enviaron a ningún servidor.',
    MARGEN,
    y,
  );

  const dataUrl = doc.output('datauristring');
  return dataUrl.split(',')[1] ?? '';
}

/** Etiqueta legible de un tipo de movimiento para el PDF. */
export function pdfTypeLabel(transaction: Transaction): string {
  return TRANSACTION_TYPE_LABELS[transaction.transactionType] ?? transaction.transactionType;
}
