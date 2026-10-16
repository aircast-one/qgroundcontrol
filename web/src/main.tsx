import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { startCore } from '@/core/client';
import { Qgc } from '@/bridge/Qgc';
import { AircastRoot } from '@/shell/AircastRoot';
import '@/theme/theme.css';

Qgc.start();
void startCore();

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <AircastRoot />
  </StrictMode>,
);
