import { call } from '@/core/client';

const SOURCE_TOKEN = 'internalGps';
const UPDATE_INTERVAL_MS = 1000;
const ACCESS_ERROR = 0;
const CLOSED_ERROR = 1;

const state: { watch: number | null; listening: boolean } = { watch: null, listening: false };

const orNaN = (value: number | null | undefined): number => (typeof value === 'number' && Number.isFinite(value) ? value : NaN);

const report = (position: GeolocationPosition) => {
  if (!state.listening) {
    state.listening = true;
    void call('gcsPositionSource', SOURCE_TOKEN);
  }
  const { latitude, longitude, altitude, accuracy, altitudeAccuracy, heading, speed } = position.coords;
  void call('gcsPositionUpdate', latitude, longitude, orNaN(altitude), orNaN(accuracy), orNaN(altitudeAccuracy), orNaN(heading), NaN, orNaN(speed));
};

const failed = (error: GeolocationPositionError) => {
  void call('gcsPositionError', error.code === error.PERMISSION_DENIED ? ACCESS_ERROR : CLOSED_ERROR);
};

export const GcsLocation = {
  start(): void {
    if (state.watch !== null) return;
    if (!('geolocation' in navigator)) {
      void call('gcsPositionError', CLOSED_ERROR);
      return;
    }
    state.watch = navigator.geolocation.watchPosition(report, failed, { enableHighAccuracy: true, maximumAge: UPDATE_INTERVAL_MS });
  },

  stop(): void {
    if (state.watch !== null) navigator.geolocation.clearWatch(state.watch);
    state.watch = null;
    state.listening = false;
  },
};
