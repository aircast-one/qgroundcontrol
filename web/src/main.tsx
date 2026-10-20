import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { DebugRelay } from '@/app/DebugRelay';
import { HostPlatform } from '@/app/HostPlatform';
import { Qgc } from '@/bridge/Qgc';
import { MainActivity } from '@/app/MainActivity';
import '@/theme/theme.css';

Qgc.start();
void HostPlatform.start();
if (import.meta.env.DEV) DebugRelay.start(window.location.search);

createRoot(document.getElementById('root')!).render(<StrictMode><MainActivity /></StrictMode>);
