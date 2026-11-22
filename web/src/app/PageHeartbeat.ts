import { sendCommand } from '@/core/desktopCommands';

const BEAT_MS = 2_000;

export const drawsSomething = (root: Element): boolean => [...root.children].some((child) => child.getBoundingClientRect().height > 0);

export const PageHeartbeat = {
  start(root: Element): void {
    if (!__DESKTOP__) return;
    const beat = () => sendCommand('page_alive', { rendered: drawsSomething(root), hidden: document.hidden });
    beat();
    setInterval(beat, BEAT_MS);
  },
};
