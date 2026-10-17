import type { FenceCircle, FencePolygon } from './FenceBridge';
import type { MapHit } from './MissionEditing';
import { LANDING_PLACE_APPROACH } from './MissionLayers';
import type { MissionItem } from './PlanBridge';
import { altitudeWithFrame } from './PlanItems';

export const circleText = (circle: FenceCircle): string => circle.detailText;

export const selectionText = (selected: MapHit | null, items: readonly MissionItem[], circles: readonly FenceCircle[], polygons: readonly FencePolygon[]): string | null => {
  switch (selected?.kind) {
    case 'Waypoint': {
      const item = items.find((it) => it.index === selected.index);
      const altitude = item === undefined ? null : altitudeWithFrame(item);
      return item === undefined || altitude === null ? null : `#${item.sequence} at ${altitude}`;
    }
    case 'Circle':
    case 'CircleCentre':
    case 'CircleRadius': {
      const circle = circles.find((it) => it.index === selected.index);
      return circle === undefined ? null : circleText(circle);
    }
    case 'FenceVertex': {
      const polygon = polygons.find((it) => it.index === selected.polygon);
      return polygon === undefined ? null : `corner ${selected.vertex + 1} of ${polygon.vertices.length}`;
    }
    case 'LandingPlace':
      return selected.place === LANDING_PLACE_APPROACH ? 'final approach' : 'touchdown';
    default:
      return null;
  }
};
