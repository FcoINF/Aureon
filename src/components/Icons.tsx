/**
 * Iconos de Aureon.
 *
 * Un único trazo, 24×24, sin relleno. Mantiene la coherencia visual de toda
 * la aplicación y evita cargar varias librerías de iconos.
 */
import type { SVGProps } from 'react';

export type IconName =
  | 'home'
  | 'arrows'
  | 'wallet'
  | 'target'
  | 'bag'
  | 'repeat'
  | 'incoming'
  | 'outgoing'
  | 'send'
  | 'heart'
  | 'chart'
  | 'calendar'
  | 'settings'
  | 'plus'
  | 'edit'
  | 'trash'
  | 'check'
  | 'close'
  | 'chevron-down'
  | 'chevron-right'
  | 'search'
  | 'download'
  | 'upload'
  | 'external'
  | 'filter'
  | 'menu'
  | 'alert'
  | 'info'
  | 'card'
  | 'coins'
  | 'receipt'
  | 'unlock'
  | 'pause'
  | 'play'
  | 'link'
  | 'sun'
  | 'moon';

const PATHS: Record<IconName, string> = {
  home: 'M3 10.2 12 3l9 7.2M5.5 8.7V20h13V8.7M9.5 20v-6h5v6',
  arrows: 'M7 4v13m0 0-3-3m3 3 3-3M17 20V7m0 0-3 3m3-3 3 3',
  wallet: 'M3 8.5A2.5 2.5 0 0 1 5.5 6H18a2 2 0 0 1 2 2v1M3 8.5V17a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-2M21 10h-4a2 2 0 0 0 0 4h4a1 1 0 0 0 1-1v-2a1 1 0 0 0-1-1Z',
  target: 'M12 3a9 9 0 1 0 9 9M12 7.5a4.5 4.5 0 1 0 4.5 4.5M12 11.2a.8.8 0 1 0 .8.8',
  bag: 'M5 8h14l-1 12H6L5 8Zm3.5 0V6a3.5 3.5 0 0 1 7 0v2',
  repeat: 'M4 9a5 5 0 0 1 5-5h9m0 0-3-3m3 3-3 3M20 15a5 5 0 0 1-5 5H6m0 0 3 3m-3-3 3-3',
  incoming: 'M12 4v11m0 0 4-4m-4 4-4-4M4 19h16',
  outgoing: 'M12 20V9m0 0 4 4m-4-4-4 4M4 5h16',
  send: 'M21 3 10.5 13.5M21 3l-6.5 18-4-8-8-4L21 3Z',
  heart: 'M12 20s-7-4.4-7-9a4 4 0 0 1 7-2.6A4 4 0 0 1 19 11c0 4.6-7 9-7 9Z',
  chart: 'M4 20V10m5 10V4m5 16v-7m5 7V8',
  calendar: 'M4 6.5A1.5 1.5 0 0 1 5.5 5h13A1.5 1.5 0 0 1 20 6.5v12a1.5 1.5 0 0 1-1.5 1.5h-13A1.5 1.5 0 0 1 4 18.5v-12ZM8 3v4m8-4v4M4 10h16',
  settings:
    'M12 15.2a3.2 3.2 0 1 0 0-6.4 3.2 3.2 0 0 0 0 6.4Zm8-3.2c0-.5-.05-1-.15-1.5l1.7-1.3-1.7-3-2 .8a7.6 7.6 0 0 0-2.6-1.5L15 3.2h-3.4l-.3 2.3a7.6 7.6 0 0 0-2.6 1.5l-2-.8-1.7 3 1.7 1.3a8.4 8.4 0 0 0 0 3l-1.7 1.3 1.7 3 2-.8a7.6 7.6 0 0 0 2.6 1.5l.3 2.3H15l.3-2.3a7.6 7.6 0 0 0 2.6-1.5l2 .8 1.7-3-1.7-1.3c.1-.5.15-1 .15-1.5Z',
  plus: 'M12 5v14M5 12h14',
  edit: 'M4 20h4L19 9a2.1 2.1 0 0 0-3-3L5 17v3ZM14.5 6.5l3 3',
  trash: 'M4 7h16M9 7V5h6v2m-8 0 1 13h8l1-13M10 11v6m4-6v6',
  check: 'm5 13 4.5 4.5L19 7',
  close: 'M6 6l12 12M18 6 6 18',
  'chevron-down': 'm6 9 6 6 6-6',
  'chevron-right': 'm9 6 6 6-6 6',
  search: 'M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14Zm5.2-1.8L21 21',
  download: 'M12 3v12m0 0 4-4m-4 4-4-4M4 19h16',
  upload: 'M12 21V9m0 0 4 4M12 9l-4 4M4 5h16',
  external: 'M14 4h6v6M20 4l-8.5 8.5M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5',
  filter: 'M4 5h16l-6 7v6l-4 2v-8L4 5Z',
  menu: 'M4 7h16M4 12h16M4 17h16',
  alert: 'M12 4 2.5 20h19L12 4Zm0 6v5m0 3v.5',
  info: 'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18Zm0-13v.5m0 3.5v5',
  card: 'M3 6.5A1.5 1.5 0 0 1 4.5 5h15A1.5 1.5 0 0 1 21 6.5v11a1.5 1.5 0 0 1-1.5 1.5h-15A1.5 1.5 0 0 1 3 17.5v-11ZM3 10h18M7 15h4',
  coins: 'M4 8c0-1.7 3.6-3 8-3s8 1.3 8 3-3.6 3-8 3-8-1.3-8-3Zm0 0v8c0 1.7 3.6 3 8 3s8-1.3 8-3V8M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3',
  receipt:
    'M6 3h12v18l-3-2-3 2-3-2-3 2V3Zm3 5h6M9 12h6M9 16h3',
  unlock: 'M7 11V8a5 5 0 0 1 9.6-2M6 11h12v9H6v-9Zm6 4v2',
  pause: 'M9 5v14M15 5v14',
  play: 'M8 5l11 7-11 7V5Z',
  link: 'M10 13a4 4 0 0 0 5.7 0l2.6-2.6a4 4 0 0 0-5.7-5.7L11 6.3M14 11a4 4 0 0 0-5.7 0l-2.6 2.6a4 4 0 0 0 5.7 5.7L13 17.7',
  sun: 'M12 17a5 5 0 1 0 0-10 5 5 0 0 0 0 10Zm0-14v2m0 18v-2M3 12h2m14 0h2M5.6 5.6 7 7m10 10 1.4 1.4M18.4 5.6 17 7M7 17l-1.4 1.4',
  moon: 'M20 14.5A8.5 8.5 0 0 1 9.5 4a8.5 8.5 0 1 0 10.5 10.5Z',
};

interface IconProps extends Omit<SVGProps<SVGSVGElement>, 'name'> {
  name: IconName;
  size?: number;
}

export function Icon({ name, size = 16, ...rest }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.6}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      focusable="false"
      {...rest}
    >
      <path d={PATHS[name]} />
    </svg>
  );
}
