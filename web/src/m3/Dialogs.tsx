import type { CSSProperties, ReactNode } from 'react';
import { Dialog } from '@base-ui/react';

type DialogProps = { onDismissRequest: () => void; dismissOnClickOutside?: boolean };

export type SheetAnchor = { readonly x: number; readonly y: number };

const ANCHORED_WIDTH = 320;
const ANCHOR_EDGE = 16;
const ANCHOR_FLIP_FRACTION = 0.6;

const SHEET = '[--layout-list:var(--layout-sheet)] fixed inset-x-0 bottom-0 z-50 mx-auto max-h-[90vh] w-full max-w-[640px] overflow-auto rounded-t-shape-xl bg-surface-container-low pb-[max(16px,env(safe-area-inset-bottom))] text-on-surface shadow-2xl outline-none';
const SHEET_DESKTOP = 'desktop:inset-x-auto desktop:bottom-auto desktop:left-1/2 desktop:top-1/2 desktop:max-h-[calc(100vh-48px)] desktop:w-[min(480px,calc(100vw-48px))] desktop:-translate-x-1/2 desktop:-translate-y-1/2 desktop:rounded-shape-xl desktop:pt-dialog desktop:pb-0';
const SHEET_ANCHORED = 'desktop:inset-x-auto desktop:mx-0 desktop:left-[var(--anchor-left)] desktop:top-[var(--anchor-top,auto)] desktop:bottom-[var(--anchor-bottom,auto)] desktop:max-h-[60vh] desktop:w-[320px] desktop:rounded-shape-md desktop:py-1 desktop:bg-surface-container desktop:[--layout-sheet:var(--layout-menu)]';
const DIALOG = 'fixed left-1/2 top-1/2 z-50 max-h-[calc(100vh-48px)] w-[min(560px,calc(100vw-48px))] min-w-[280px] -translate-x-1/2 -translate-y-1/2 overflow-auto rounded-shape-xl bg-surface-container-high text-on-surface shadow-2xl outline-none';

export const anchorPlacement = (anchor: SheetAnchor, viewport: { width: number; height: number }): Record<string, string> => ({
  '--anchor-left': `${Math.max(ANCHOR_EDGE, Math.min(anchor.x, viewport.width - ANCHORED_WIDTH - ANCHOR_EDGE))}px`,
  ...(anchor.y < viewport.height * ANCHOR_FLIP_FRACTION ? { '--anchor-top': `${anchor.y}px` } : { '--anchor-bottom': `${viewport.height - anchor.y}px` }),
});

const Shell = ({ onDismissRequest, dismissOnClickOutside = true, children, sheet = false, label, anchor }: DialogProps & { children: ReactNode; sheet?: boolean; label?: string; anchor?: SheetAnchor }) => (
  <Dialog.Root open onOpenChange={(open, details) => !open && (dismissOnClickOutside || details.reason !== 'outside-press') && onDismissRequest()}>
    <Dialog.Portal>
      <Dialog.Backdrop className={`fixed inset-0 z-50 bg-black/40 ${anchor === undefined ? '' : 'desktop:bg-transparent'}`} />
      <Dialog.Popup
        aria-label={label}
        className={sheet ? `${SHEET} ${anchor === undefined ? SHEET_DESKTOP : SHEET_ANCHORED}` : DIALOG}
        style={anchor === undefined ? undefined : (anchorPlacement(anchor, { width: window.innerWidth, height: window.innerHeight }) as CSSProperties)}
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
    <div className="grid gap-4 p-dialog">
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

export const ModalBottomSheet = ({ onDismissRequest, children, label, anchor }: DialogProps & { children: ReactNode; label?: string; anchor?: SheetAnchor }) => (
  <Shell onDismissRequest={onDismissRequest} sheet label={label} anchor={anchor}>
    <div className="mx-auto mt-3 mb-2 h-1 w-8 rounded-full bg-on-surface-variant/40 desktop:hidden" aria-hidden />
    {children}
  </Shell>
);
