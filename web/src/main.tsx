import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { HostPlatform } from '@/app/HostPlatform';
import { Qgc } from '@/bridge/Qgc';
import { MainActivity } from '@/app/MainActivity';
import '@/theme/theme.css';

Qgc.start();
void HostPlatform.start();

createRoot(document.getElementById('root')!).render(<StrictMode><MainActivity /></StrictMode>);
