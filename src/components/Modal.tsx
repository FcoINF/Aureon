import { useEffect, useRef, type ReactNode } from 'react';

import { Icon } from './Icons';

/**
 * Pila de diálogos abiertos. Sólo el diálogo superior reacciona a la tecla
 * Escape: así, un diálogo de confirmación sobre un detalle no cierra ambos.
 */
const pilaDialogos: symbol[] = [];

interface ModalProps {
  open: boolean;
  title: string;
  subtitle?: string;
  wide?: boolean;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
}

/** Diálogo modal con cierre por Escape y foco contenido dentro. */
export function Modal({
  open,
  title,
  subtitle,
  wide,
  onClose,
  children,
  footer,
}: ModalProps) {
  const dialogRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return undefined;
    const token = Symbol('dialogo');
    pilaDialogos.push(token);
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      // Sólo responde el diálogo que está encima.
      if (pilaDialogos[pilaDialogos.length - 1] !== token) return;
      event.stopPropagation();
      onClose();
    };
    document.addEventListener('keydown', onKeyDown);
    const timer = window.setTimeout(() => {
      const focusable = dialogRef.current?.querySelector<HTMLElement>(
        'input, select, textarea, button:not([disabled])',
      );
      focusable?.focus();
    }, 40);
    return () => {
      document.removeEventListener('keydown', onKeyDown);
      window.clearTimeout(timer);
      const posicion = pilaDialogos.indexOf(token);
      if (posicion >= 0) pilaDialogos.splice(posicion, 1);
    };
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      className="overlay"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        className={`dialog${wide ? ' dialog-wide' : ''}`}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        ref={dialogRef}
      >
        <header className="dialog-header">
          <div>
            <h2 className="dialog-title">{title}</h2>
            {subtitle ? <p className="dialog-subtitle">{subtitle}</p> : null}
          </div>
          <button
            type="button"
            className="btn btn-icon"
            onClick={onClose}
            aria-label="Cerrar"
          >
            <Icon name="close" size={15} />
          </button>
        </header>
        <div className="dialog-body">{children}</div>
        {footer ? <footer className="dialog-footer">{footer}</footer> : null}
      </div>
    </div>
  );
}

interface ConfirmDialogProps {
  open: boolean;
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
  busy?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmDialog({
  open,
  title,
  message,
  confirmLabel = 'Confirmar',
  cancelLabel = 'Cancelar',
  danger,
  busy,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  return (
    <Modal
      open={open}
      title={title}
      onClose={onCancel}
      footer={
        <>
          <button type="button" className="btn" onClick={onCancel} disabled={busy}>
            {cancelLabel}
          </button>
          <button
            type="button"
            className={danger ? 'btn btn-danger' : 'btn btn-primary'}
            onClick={onConfirm}
            disabled={busy}
          >
            {busy ? 'Procesando…' : confirmLabel}
          </button>
        </>
      }
    >
      <p>{message}</p>
    </Modal>
  );
}
