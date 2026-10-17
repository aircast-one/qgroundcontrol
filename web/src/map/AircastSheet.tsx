import { type ReactNode, createContext } from 'react';
import { ModalBottomSheet, type SheetAnchor } from '@/m3';

export const LocalImmersive = createContext(false);

export const AircastSheet = ({ onDismissRequest, className, anchor, children }: { onDismissRequest: () => void; className?: string; anchor?: SheetAnchor; children?: ReactNode }) => (
  <ModalBottomSheet onDismissRequest={onDismissRequest} anchor={anchor}>
    <div className={`flex flex-col ${className ?? ''}`}>{children}</div>
  </ModalBottomSheet>
);
