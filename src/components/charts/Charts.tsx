/**
 * Gráficos de Aureon.
 *
 * Se usa Chart.js con los registradores mínimos necesarios para mantener
 * pequeño el paquete final.
 */
import {
  ArcElement,
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
import { Doughnut, Line } from 'react-chartjs-2';

import { formatAmount, exponentOf } from '../../utils/format';

ChartJS.register(
  ArcElement,
  CategoryScale,
  Filler,
  Legend,
  LinearScale,
  LineController,
  LineElement,
  PointElement,
  Tooltip,
);

const INCOME = '#2f7a5a';
const EXPENSE = '#b0574b';
const ACCENT = '#b08b4a';
const GRID = 'rgba(140, 132, 118, 0.18)';
const TICK = '#948e83';

function cssVar(name: string, fallback: string): string {
  if (typeof window === 'undefined') return fallback;
  const value = getComputedStyle(document.documentElement).getPropertyValue(name);
  return value.trim() || fallback;
}

interface DualSeriesProps {
  labels: string[];
  income: number[];
  expenses: number[];
  currency: string;
  height?: number;
}

/** Ingresos frente a gastos. */
export function IncomeExpenseChart({
  labels,
  income,
  expenses,
  currency,
  height = 220,
}: DualSeriesProps) {
  const exponent = exponentOf(currency);
  return (
    <Line
      height={height}
      data={{
        labels,
        datasets: [
          {
            label: 'Ingresos',
            data: income,
            borderColor: INCOME,
            backgroundColor: 'rgba(47, 122, 90, 0.12)',
            tension: 0.32,
            borderWidth: 2,
            pointRadius: 0,
            pointHoverRadius: 4,
            fill: true,
          },
          {
            label: 'Gastos',
            data: expenses,
            borderColor: EXPENSE,
            backgroundColor: 'rgba(176, 87, 75, 0.1)',
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
              color: TICK,
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
            ticks: { color: TICK, font: { size: 10 }, maxRotation: 0, autoSkip: true },
          },
          y: {
            grid: { color: GRID },
            border: { display: false },
            ticks: {
              color: TICK,
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
  return (
    <Line
      height={height}
      data={{
        labels,
        datasets: [
          {
            label: 'Saldo',
            data: values,
            borderColor: ACCENT,
            backgroundColor: 'rgba(176, 139, 74, 0.12)',
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
            ticks: { color: TICK, font: { size: 10 }, maxRotation: 0, autoSkip: true },
          },
          y: {
            grid: { color: GRID },
            border: { display: false },
            ticks: {
              color: TICK,
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

const PALETTE = [
  '#b08b4a',
  '#4c7ba8',
  '#3f8a7a',
  '#b0574b',
  '#7a62a8',
  '#c2703d',
  '#6c6f86',
  '#8a6f9c',
  '#4f7d3f',
  '#b0768c',
];

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
  return (
    <Doughnut
      height={height}
      data={{
        labels: slices.map((slice) => slice.label),
        datasets: [
          {
            data: slices.map((slice) => slice.amount),
            backgroundColor: slices.map(
              (slice, index) => slice.color ?? PALETTE[index % PALETTE.length],
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
              color: TICK,
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
                background: item.color ?? PALETTE[index % PALETTE.length],
              }}
            />
          </div>
        </li>
      ))}
    </ul>
  );
}
