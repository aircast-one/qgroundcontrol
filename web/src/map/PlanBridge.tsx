import { type Json, optBoolean } from '@/bridge/Json';
import type { TrackPoint } from './VehicleMap';

export const PLAN_ROOT = 'plan';
export const PLAN_ITEMS = `${PLAN_ROOT}.missionController.visualItems`;

export type MissionItem = {
  index: number;
  sequence: number;
  latitude: number;
  longitude: number;
  command: string;
  selected: boolean;
  altitude: number;
  exit: TrackPoint | null;
  routed: boolean;
  kind: string;
  commandId: number;
  placed: boolean;
  afterRouteEnds: boolean;
  altitudeText: string;
  specifiesCoordinate: boolean;
  distance: number;
  distanceText: string;
  azimuthText: string;
  headingText: string;
  altitudeChange: number;
  altitudeChangeText: string;
  gradientText: string;
  altitudeBandText: string;
  blockedReason: string;
  readyForSave: boolean;
  cameraShots: number;
  extraSeconds: number;
  speedChangeText: string;
  foldedCommands: number;
  altitudeFrameText: string;
  altitudeMode: number;
  altitudeEditUnits: string;
  complexPattern: boolean;
  loiterRadius: number;
  terrainCollision: boolean;
  heading: number;
  gimbalYaw: number;
  closesRoute: boolean;
  legBroken: boolean;
  abbreviation: string;
};

export const missionItems = (_json: Json | null): readonly MissionItem[] => [];

export const linksStartToHome = (json: Json | null): boolean => optBoolean(json, 'linksStartToHome');
