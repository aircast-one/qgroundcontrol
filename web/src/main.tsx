import { CSPProvider } from '@base-ui/react';
import { StrictMode } from 'react';
import { DebugRelay } from '@/app/DebugRelay';
import { DebugUi } from '@/app/DebugUi';
import { DeviceLink } from '@/app/DeviceLink';
import { HostPlatform } from '@/app/HostPlatform';
import { Qgc } from '@/bridge/Qgc';
import { MainActivity } from '@/app/MainActivity';
import { type PageFacts, recordPageErrors, recoveringRoot } from '@/app/RecoveringRoot';
import { AppCommands } from '@/bridge/BridgeCommands';
import { coreLoaded, coreReady } from '@/core/client';
import { useValue } from '@/kotlin/State';
import { CircularProgressIndicator, Text } from '@/m3';
import '@/theme/theme.css';

const LOADING = 'Loading Aircast';
const PAGE_LOG = 'page';
const PAGE_FACTS: PageFacts = { screen: () => DebugUi.state, desktop: __DESKTOP__, version: import.meta.env.VITE_APP_VERSION ?? 'dev', sha: __BUILD_SHA__ };

const logPageError = (line: string): void => void AppCommands.recordLog(PAGE_LOG, line, 'error').catch(() => false);
recordPageErrors(logPageError, PAGE_FACTS);

const keepsBrowserMenu = (target: EventTarget | null): boolean => target instanceof Element && target.closest('input, textarea, [contenteditable]') !== null;
window.addEventListener('contextmenu', (event) => {
  if (!keepsBrowserMenu(event.target)) event.preventDefault();
});

const CoreLoading = () => {
  const percent = useValue(coreLoaded);
  return (
    <div className="grid h-full place-content-center justify-items-center gap-4">
      <CircularProgressIndicator progress={percent > 0 ? percent / 100 : undefined} label={LOADING} />
      <Text typography="bodyMedium">{percent > 0 ? `${LOADING}… ${percent}%` : `${LOADING}…`}</Text>
    </div>
  );
};

const root = document.getElementById('root')!;
const show = recoveringRoot(root, logPageError, PAGE_FACTS);
show(<StrictMode><CoreLoading /></StrictMode>);
const handoff = await DeviceLink.handoff();
if ('to' in handoff) {
  location.replace(handoff.to);
} else {
  Qgc.start();
  void HostPlatform.start();
  if (import.meta.env.DEV) DebugRelay.start(window.location.search);
  await coreReady();
  DeviceLink.recordStay(handoff);
  show(
    <StrictMode>
      <CSPProvider disableStyleElements>
        <MainActivity />
      </CSPProvider>
    </StrictMode>,
  );
}
