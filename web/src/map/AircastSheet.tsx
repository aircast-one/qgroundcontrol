import { type ReactNode, createContext } from 'react';
import { ModalBottomSheet } from '@/m3';

export const LocalImmersive = createContext(false);

export const AircastSheet = ({ onDismissRequest, className, children }: { onDismissRequest: () => void; className?: string; children?: ReactNode }) => (
  <ModalBottomSheet onDismissRequest={onDismissRequest}>
    <div className={`flex flex-col ${className ?? ''}`}>{children}</div>
  </ModalBottomSheet>
);
