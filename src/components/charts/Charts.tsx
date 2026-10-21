/**
 * Gráficos de Aureon.
 *
 * Se usa Chart.js con los registradores mínimos necesarios para mantener
 * pequeño el paquete final.
 */
import {
  ArcElement,
  BarController,
  BarElement,
  CategoryScale,
  Chart as ChartJS,
  Filler,
  Legend,
  LinearScale,
  LineController,
  LineElement,
  PointElement,
  Tooltip,
} from 'chart.js';
import { Bar, Doughnut, Line } from 'react-chartjs-2';

import { formatAmount, exponentOf } from '../../utils/format';

ChartJS.register(
  ArcElement,
  BarController,
  BarElement,
  CategoryScale,
  Filler,
  Legend,
  LinearScale,
  LineController,
  LineElement,
  PointElement,
  Tooltip,
);

/**
 * Paleta monocromática de los gráficos. Los colores se leen de los tokens CSS
 * en cada render para que acompañen al tema claro u oscuro.
 */
function cssVar(name: string, fallback: string): string {
  if (typeof window === 'undefined') return fallback;
  const value = getComputedStyle(document.documentElement).getPropertyValue(name);
  return value.trim() || fallback;
}

/** Devuelve el mismo color con transparencia para rellenos suaves. */
function withAlpha(color: string, alpha: number): string {
  const hex = color.trim().replace(/^#/, '');
  if (/^[0-9a-f]{6}$/i.test(hex)) {
    const r = parseInt(hex.slice(0, 2), 16);
    const g = parseInt(hex.slice(2, 4), 16);
    const b = parseInt(hex.slice(4, 6), 16);
    return `rgba(${r}, ${g}, ${b}, ${alpha})`;
  }
  return `rgba(128, 128, 128, ${alpha})`;
}

/** Rampa de grises para categorías que no tienen un color propio. */
function monoPalette(): string[] {
  const dark =
    typeof document !== 'undefined' &&
    document.documentElement.getAttribute('data-theme') === 'dark';
  return dark
    ? [
        '#f2f2f2',
        '#d9d9d9',
        '#c2c2c2',
        '#a8a8a8',
        '#8f8f8f',
        '#737373',
        '#5c5c5c',
        '#e6e6e6',
        '#b8b8b8',
        '#9c9c9c',
      ]
    : [
        '#1f1f1f',
        '#3d3d3d',
        '#5c5c5c',
        '#7a7a7a',
        '#999999',
        '#b8b8b8',
        '#2b2b2b',
        '#4d4d4d',
        '#6b6b6b',
        '#8a8a8a',
      ];
}

interface DualSeriesProps {
  labels: string[];
  income: number[];
  expenses: number[];
  currency: string;
  height?: number;
  /** «line» para tendencias continuas; «bar» para totales por período. */
  variant?: 'line' | 'bar';
}

/** Ingresos frente a gastos. */
export function IncomeExpenseChart({
  labels,
  income,
  expenses,
  currency,
  height = 220,
  variant = 'line',
}: DualSeriesProps) {
  const exponent = exponentOf(currency);
  const incomeColor = cssVar('--income', '#1f1f1f');
  const expenseColor = cssVar('--expense', '#666666');
  const tickColor = cssVar('--text-soft', '#6f6f6f');
  const gridColor = cssVar('--chart-grid', 'rgba(0, 0, 0, 0.08)');

  if (variant === 'bar') {
    return (
      <Bar
        height={height}
        data={{
          labels,
          datasets: [
            {
              label: 'Ingresos',
              data: income,
              backgroundColor: incomeColor,
              borderRadius: 4,
              borderSkipped: false,
              maxBarThickness: 40,
              categoryPercentage: 0.7,
              barPercentage: 0.9,
            },
            {
              label: 'Gastos',
              data: expenses,
              backgroundColor: expenseColor,
              borderRadius: 4,
              borderSkipped: false,
              maxBarThickness: 40,
              categoryPercentage: 0.7,
              barPercentage: 0.9,
            },
          ],
        }}
        options={{
          responsive: true,
          maintainAspectRatio: false,
          interaction: { mode: 'index', intersect: false },
          plugins: {
            legend: {
              position: 'bottom',
              labels: {
                color: tickColor,
                boxWidth: 10,
                boxHeight: 10,
                usePointStyle: true,
                pointStyle: 'rectRounded',
                font: { size: 11 },
              },
            },
            tooltip: {
              callbacks: {
                label: (context) =>
                  `${context.dataset.label}: ${formatAmount(
                    Number(context.parsed.y ?? 0),
                    exponent,
                  )}`,
              },
            },
          },
          scales: {
            x: {
              grid: { display: false },
              ticks: {
                color: tickColor,
                font: { size: 10 },
                maxRotation: 0,
                autoSkip: true,
              },
            },
            y: {
              beginAtZero: true,
              grid: { color: gridColor },
              border: { display: false },
              ticks: {
                color: tickColor,
                font: { size: 10 },
                callback: (value) => formatAmount(Number(value), exponent),
              },
            },
          },
        }}
      />
    );
  }

  return (
    <Line
      height={height}
      data={{
        labels,
        datasets: [
          {
            label: 'Ingresos',
            data: income,
            borderColor: incomeColor,
            backgroundColor: withAlpha(incomeColor, 0.12),
            tension: 0.32,
            borderWidth: 2,
            pointRadius: 0,
            pointHoverRadius: 4,
            fill: true,
          },
          {
            label: 'Gastos',
            data: expenses,
            borderColor: expenseColor,
            backgroundColor: withAlpha(expenseColor, 0.1),
            tension: 0.32,
            borderWidth: 2,
            pointRadius: 0,
            pointHoverRadius: 4,
            fill: true,
          },
        ],
      }}
      options={{
        responsive: true,
        maintainAspectRatio: false,
        interaction: { mode: 'index', intersect: false },
        plugins: {
          legend: {
            position: 'bottom',
            labels: {
              color: tickColor,
              boxWidth: 10,
              boxHeight: 10,
              usePointStyle: true,
              pointStyle: 'circle',
              font: { size: 11 },
            },
          },
          tooltip: {
            callbacks: {
              label: (context) =>
                `${context.dataset.label}: ${formatAmount(
                  Number(context.parsed.y ?? 0),
                  exponent,
                )}`,
            },
          },
        },
        scales: {
          x: {
            grid: { display: false },
            ticks: { color: tickColor, font: { size: 10 }, maxRotation: 0, autoSkip: true },
          },
          y: {
            grid: { color: gridColor },
            border: { display: false },
            ticks: {
              color: tickColor,
              font: { size: 10 },
              callback: (value) => formatAmount(Number(value), exponent),
            },
          },
        },
      }}
    />
  );
}

interface BalanceLineProps {
  labels: string[];
  values: number[];
  currency: string;
  height?: number;
}

/** Evolución del saldo. */
export function BalanceChart({ labels, values, currency, height = 220 }: BalanceLineProps) {
  const exponent = exponentOf(currency);
  const accentColor = cssVar('--accent', '#1a1a1a');
  const tickColor = cssVar('--text-soft', '#6f6f6f');
  const gridColor = cssVar('--chart-grid', 'rgba(0, 0, 0, 0.08)');
  return (
    <Line
      height={height}
      data={{
        labels,
        datasets: [
          {
            label: 'Saldo',
            data: values,
            borderColor: accentColor,
            backgroundColor: withAlpha(accentColor, 0.12),
            tension: 0.3,
            borderWidth: 2,
            pointRadius: 0,
            pointHoverRadius: 4,
            fill: true,
          },
        ],
      }}
      options={{
        responsive: true,
        maintainAspectRatio: false,
        interaction: { mode: 'index', intersect: false },
        plugins: {
          legend: { display: false },
          tooltip: {
            callbacks: {
              label: (context) =>
                `Saldo: ${formatAmount(Number(context.parsed.y ?? 0), exponent)}`,
            },
          },
        },
        scales: {
          x: {
            grid: { display: false },
            ticks: { color: tickColor, font: { size: 10 }, maxRotation: 0, autoSkip: true },
          },
          y: {
            grid: { color: gridColor },
            border: { display: false },
            ticks: {
              color: tickColor,
              font: { size: 10 },
              callback: (value) => formatAmount(Number(value), exponent),
            },
          },
        },
      }}
    />
  );
}

interface CategorySlice {
  label: string;
  amount: number;
  color: string | null;
}

/** Distribución de gastos por categoría. */
export function CategoryDonut({
  slices,
  currency,
  height = 220,
}: {
  slices: CategorySlice[];
  currency: string;
  height?: number;
}) {
  const exponent = exponentOf(currency);
  const total = slices.reduce((sum, slice) => sum + slice.amount, 0);
  const palette = monoPalette();
  const tickColor = cssVar('--text-soft', '#6f6f6f');
  return (
    <Doughnut
      height={height}
      data={{
        labels: slices.map((slice) => slice.label),
        datasets: [
          {
            data: slices.map((slice) => slice.amount),
            backgroundColor: slices.map(
              (slice, index) => slice.color ?? palette[index % palette.length],
            ),
            borderColor: cssVar('--surface', '#ffffff'),
            borderWidth: 2,
            hoverOffset: 4,
          },
        ],
      }}
      options={{
        responsive: true,
        maintainAspectRatio: false,
        cutout: '62%',
        plugins: {
          legend: {
            position: 'bottom',
            labels: {
              color: tickColor,
              boxWidth: 9,
              boxHeight: 9,
              usePointStyle: true,
              pointStyle: 'circle',
              font: { size: 11 },
              padding: 12,
            },
          },
          tooltip: {
            callbacks: {
              label: (context) => {
                const value = Number(context.parsed ?? 0);
                const percent = total > 0 ? Math.round((value / total) * 100) : 0;
                return `${context.label}: ${formatAmount(value, exponent)} (${percent}%)`;
              },
            },
          },
        },
      }}
    />
  );
}

interface BreakdownItem {
  label: string;
  amount: number;
  color: string | null;
}

/** Barras horizontales para gastar por comercio o plataforma. */
export function BreakdownBars({
  items,
  currency,
}: {
  items: BreakdownItem[];
  currency: string;
}) {
  const exponent = exponentOf(currency);
  const palette = monoPalette();
  const max = items.reduce((sum, item) => Math.max(sum, item.amount), 0) || 1;
  return (
    <ul style={{ listStyle: 'none', margin: 0, padding: 0, display: 'grid', gap: 14 }}>
      {items.map((item, index) => (
        <li key={`${item.label}-${index}`}>
          <div className="row-between" style={{ marginBottom: 6 }}>
            <span className="truncate" style={{ fontSize: 'var(--text-base)' }}>
              {item.label}
            </span>
            <span className="amount text-sm">
              {formatAmount(item.amount, exponent)}
            </span>
          </div>
          <div className="progress">
            <div
              className="progress-bar"
              style={{
                width: `${Math.max(2, (item.amount / max) * 100)}%`,
                background: item.color ?? palette[index % palette.length],
              }}
            />
          </div>
        </li>
      ))}
    </ul>
  );
}
