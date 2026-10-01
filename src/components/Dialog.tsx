import { ReactNode, useEffect } from "react";
import { useI18n } from "../i18n";

interface DialogProps {
  title: string;
  onClose: () => void;
  children: ReactNode;
  width?: number;
}

export function Dialog({ title, onClose, children, width = 420 }: DialogProps) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && onClose();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div className="backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="dialog" role="dialog" aria-modal="true" aria-label={title} style={{ width }}>
        <h2 className="dialog-title">{title}</h2>
        {children}
      </div>
    </div>
  );
}

export interface ConfirmRequest {
  title: string;
  message: string;
  confirmLabel: string;
  danger?: boolean;
  onConfirm: () => void | Promise<void>;
  /** Optional extra action shown on the left. */
  secondary?: { label: string; onClick: () => void };
}

export function ConfirmDialog({ req, onClose }: { req: ConfirmRequest; onClose: () => void }) {
  const { t } = useI18n();
  return (
    <Dialog title={req.title} onClose={onClose} width={380}>
      <p className="dialog-text">{req.message}</p>
      <div className="dialog-actions">
        {req.secondary && (
          <button
            className="link-btn link-btn-danger dialog-actions-left"
            onClick={() => {
              onClose();
              req.secondary!.onClick();
            }}
          >
            {req.secondary.label}
          </button>
        )}
        <button className="btn" onClick={onClose}>
          {t.cancel}
        </button>
        <button
          className={req.danger ? "btn btn-danger" : "btn btn-primary"}
          autoFocus
          onClick={async () => {
            onClose();
            await req.onConfirm();
          }}
        >
          {req.confirmLabel}
        </button>
      </div>
    </Dialog>
  );
}
