import { type ReactNode, createContext } from 'react';
import { ModalBottomSheet, type SheetAnchor } from '@/m3';

export const LocalImmersive = createContext(false);

export const AircastSheet = ({ onDismissRequest, label, className, anchor, children }: { onDismissRequest: () => void; label: string; className?: string; anchor?: SheetAnchor; children?: ReactNode }) => (
  <ModalBottomSheet onDismissRequest={onDismissRequest} label={label} anchor={anchor}>
    <div className={`flex flex-col ${className ?? ''}`}>{children}</div>
  </ModalBottomSheet>
);
