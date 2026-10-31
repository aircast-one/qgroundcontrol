import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { DebugRelay } from '@/app/DebugRelay';
import { HostPlatform } from '@/app/HostPlatform';
import { Qgc } from '@/bridge/Qgc';
import { MainActivity } from '@/app/MainActivity';
import { coreLoaded, coreReady } from '@/core/client';
import { useValue } from '@/kotlin/State';
import { CircularProgressIndicator, Text } from '@/m3';
import '@/theme/theme.css';

const LOADING = 'Loading Aircast';

const keepsBrowserMenu = (target: EventTarget | null): boolean => target instanceof Element && target.closest('input, textarea, [contenteditable]') !== null;
window.addEventListener('contextmenu', (event) => {
  if (!keepsBrowserMenu(event.target)) event.preventDefault();
});

Qgc.start();
void HostPlatform.start();
if (import.meta.env.DEV) DebugRelay.start(window.location.search);

const CoreLoading = () => {
  const percent = useValue(coreLoaded);
  return (
    <div className="grid h-full place-content-center justify-items-center gap-4">
      <CircularProgressIndicator progress={percent > 0 ? percent / 100 : undefined} label={LOADING} />
      <Text typography="bodyMedium">{percent > 0 ? `${LOADING}… ${percent}%` : `${LOADING}…`}</Text>
    </div>
  );
};

const root = createRoot(document.getElementById('root')!);
root.render(<StrictMode><CoreLoading /></StrictMode>);
await coreReady();
root.render(<StrictMode><MainActivity /></StrictMode>);
