import { createContext } from 'react';
import { mutableStateOf } from '@/kotlin/State';
import type { LoiterEdit } from './GotoMarker';
import type { OrbitCircle } from './OrbitMarker';
import type { TrackPoint } from './VehicleMap';

export class FlyMapEdits {
  readonly orbit = mutableStateOf<OrbitCircle | null>(null);
  readonly gotoLoiter = mutableStateOf<LoiterEdit | null>(null);
  readonly gotoPreview = mutableStateOf<TrackPoint | null>(null);
}

const unprovided = new Proxy({} as FlyMapEdits, {
  get: () => {
    throw new Error('FlyMapEdits is provided by the screen that hosts the map');
  },
});

export const LocalFlyMapEdits = createContext<FlyMapEdits>(unprovided);
