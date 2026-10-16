import type { ReactNode } from 'react';
import { Dialog } from '@base-ui/react';

type DialogProps = { onDismissRequest: () => void; dismissOnClickOutside?: boolean };

const Shell = ({ onDismissRequest, dismissOnClickOutside = true, children, sheet = false, label }: DialogProps & { children: ReactNode; sheet?: boolean; label?: string }) => (
  <Dialog.Root open onOpenChange={(open, details) => !open && (dismissOnClickOutside || details.reason !== 'outside-press') && onDismissRequest()}>
    <Dialog.Portal>
      <Dialog.Backdrop className="fixed inset-0 z-50 bg-black/40" />
      <Dialog.Popup
        aria-label={label}
        className={sheet
          ? 'fixed inset-x-0 bottom-0 z-50 mx-auto max-h-[90vh] w-full max-w-[640px] overflow-auto rounded-t-shape-xl bg-surface-container-low pb-[max(16px,env(safe-area-inset-bottom))] text-on-surface shadow-2xl outline-none'
          : 'fixed left-1/2 top-1/2 z-50 max-h-[calc(100vh-48px)] w-[min(560px,calc(100vw-48px))] min-w-[280px] -translate-x-1/2 -translate-y-1/2 overflow-auto rounded-shape-xl bg-surface-container-high text-on-surface shadow-2xl outline-none'}
      >
        {children}
      </Dialog.Popup>
    </Dialog.Portal>
  </Dialog.Root>
);

export type AlertDialogProps = DialogProps & {
  title?: ReactNode;
  text?: ReactNode;
  icon?: ReactNode;
  confirmButton?: ReactNode;
  dismissButton?: ReactNode;
};

export const AlertDialog = ({ onDismissRequest, dismissOnClickOutside, title, text, icon, confirmButton, dismissButton }: AlertDialogProps) => (
  <Shell onDismissRequest={onDismissRequest} dismissOnClickOutside={dismissOnClickOutside}>
    <div className="grid gap-4 p-6">
      {icon ? <div className="flex justify-center text-secondary">{icon}</div> : null}
      {title ? <Dialog.Title className={`text-headline-small text-on-surface ${icon ? 'text-center' : ''}`}>{title}</Dialog.Title> : null}
      {text ? <Dialog.Description render={<div />} className="text-body-medium text-on-surface-variant">{text}</Dialog.Description> : null}
      {confirmButton || dismissButton ? (
        <div className="flex flex-wrap justify-end gap-2 pt-2">
          {dismissButton}
          {confirmButton}
        </div>
      ) : null}
    </div>
  </Shell>
);

export const BasicAlertDialog = ({ onDismissRequest, dismissOnClickOutside, children, label }: DialogProps & { children: ReactNode; label?: string }) => (
  <Shell onDismissRequest={onDismissRequest} dismissOnClickOutside={dismissOnClickOutside} label={label}>
    {children}
  </Shell>
);

export const ModalBottomSheet = ({ onDismissRequest, children, label }: DialogProps & { children: ReactNode; label?: string }) => (
  <Shell onDismissRequest={onDismissRequest} sheet label={label}>
    <div className="mx-auto mt-3 mb-2 h-1 w-8 rounded-full bg-on-surface-variant/40" aria-hidden />
    {children}
  </Shell>
);
