import { Qgc } from '@/bridge/Qgc';
import { LINKS_VIEW } from '@/ui/LinksScreen';
import { linkAwaitingVehicle, vehicleConnected } from '@/ui/ConnectionLocks';
import { VEHICLES_VIEW } from '@/map/VehiclePicker';

const LOG_TAG = 'VehicleLink';

const refusal = (error: unknown): string => (error instanceof Error ? `${error.name}: ${error.message}` : String(error));

export const keepScreenOn = (onRefused: (error: unknown) => void = () => undefined): (() => void) => {
  const held: { on: boolean; sentinel: WakeLockSentinel | null } = { on: true, sentinel: null };
  const acquire = () => {
    if (!held.on || document.visibilityState !== 'visible') return;
    if (!('wakeLock' in navigator)) return onRefused(new Error('this browser cannot keep the screen on'));
    navigator.wakeLock.request('screen').then(
      (sentinel) => {
        void held.sentinel?.release();
        if (held.on) held.sentinel = sentinel;
        else void sentinel.release();
      },
      onRefused,
    );
  };
  document.addEventListener('visibilitychange', acquire);
  acquire();
  return () => {
    held.on = false;
    document.removeEventListener('visibilitychange', acquire);
    void held.sentinel?.release();
  };
};

const service: { release: (() => void) | null } = { release: null };

export const VehicleLinkService = {
  get running(): boolean {
    return service.release !== null;
  },

  start(): void {
    if (service.release !== null) return;
    service.release = keepScreenOn((error) => console.warn(LOG_TAG, `event=link_service_refused reason="${refusal(error)}" retry=on_foreground`));
  },

  stop(): void {
    service.release?.();
    service.release = null;
  },
};

const keeper: { started: boolean; linked: boolean | null } = { started: false, linked: null };

export const VehicleLinkKeeper = {
  start(): void {
    if (keeper.started) return;
    keeper.started = true;
    Qgc.watch([VEHICLES_VIEW, LINKS_VIEW]);
    const follow = () => {
      const now = vehicleConnected(Qgc.path(VEHICLES_VIEW)) || linkAwaitingVehicle(Qgc.path(LINKS_VIEW));
      if (now === keeper.linked) return;
      keeper.linked = now;
      if (now) VehicleLinkService.start();
      else VehicleLinkService.stop();
    };
    Qgc.subscribe(VEHICLES_VIEW, follow);
    Qgc.subscribe(LINKS_VIEW, follow);
    follow();
  },
};
