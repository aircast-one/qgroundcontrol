import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { HostPlatform } from '@/app/HostPlatform';
import { Qgc } from '@/bridge/Qgc';
import { MainActivity } from '@/app/MainActivity';
import { OtherTabScreen } from '@/app/OtherTabScreen';
import { claimTab } from '@/app/SingleTab';
import '@/theme/theme.css';

const owner = await claimTab();
if (owner) {
  Qgc.start();
  void HostPlatform.start();
}

createRoot(document.getElementById('root')!).render(<StrictMode>{owner ? <MainActivity /> : <OtherTabScreen />}</StrictMode>);
