import { type FenceCircle, type FencePolygon, type RallyPoint, FenceBridge, inclusionText } from './FenceBridge';
import type { LandingPattern } from './LandingPattern';
import type { MapHit } from './MissionEditing';
import type { MissionItem } from './PlanBridge';
import type { Survey } from './SurveyBridge';

export const selectionSurvives = (
  selected: MapHit | null,
  items: readonly MissionItem[],
  polygons: readonly FencePolygon[],
  circles: readonly FenceCircle[],
  rally: readonly RallyPoint[],
  surveys: readonly Survey[],
  landings: readonly LandingPattern[] = [],
  breach = false,
): boolean => {
  if (selected === null) return true;
  switch (selected.kind) {
    case 'Waypoint':
      return items.some((it) => it.index === selected.index);
    case 'FenceVertex':
      return polygons.some((it) => it.index === selected.polygon && selected.vertex >= 0 && selected.vertex < it.vertices.length);
    case 'SurveyVertex':
      return surveys.some((it) => it.index === selected.item && selected.vertex >= 0 && selected.vertex < it.area.length);
    case 'Rally':
      return rally.some((it) => it.index === selected.index);
    case 'BreachReturn':
      return breach;
    case 'Circle':
    case 'CircleCentre':
    case 'CircleRadius':
      return circles.some((it) => it.index === selected.index);
    case 'Midpoint':
      return false;
    case 'LandingPlace':
      return landings.some((it) => it.index === selected.index);
    case 'ShapeCentre':
    case 'ShapeRadius':
      return selected.fence ? polygons.some((it) => it.index === selected.owner) : surveys.some((it) => it.index === selected.owner);
    case 'LoiterRadius':
    case 'LoiterRotation':
      return false;
  }
};

export const selectedItem = (selected: MapHit | null): number | null => {
  switch (selected?.kind) {
    case 'Waypoint':
    case 'LandingPlace':
    case 'LoiterRadius':
    case 'LoiterRotation':
      return selected.index;
    case 'SurveyVertex':
      return selected.item;
    case 'ShapeCentre':
    case 'ShapeRadius':
      return selected.fence ? null : selected.owner;
    default:
      return null;
  }
};

export type SelectedFence = {
  keepsIn: boolean;
  kindText: string;
  detailText: string;
  flip: (() => Promise<boolean>) | null;
};

const polygonFence = (polygon: FencePolygon): SelectedFence => ({
  keepsIn: polygon.inclusion,
  kindText: polygon.kindText,
  detailText: polygon.detailText,
  flip: () => FenceBridge.setPolygonInclusion(polygon.index, !polygon.inclusion),
});

const circleFence = (circle: FenceCircle): SelectedFence => ({
  keepsIn: circle.inclusion,
  kindText: circle.kindText,
  detailText: circle.detailText,
  flip: null,
});

export const selectedFence = (selected: MapHit | null, polygons: readonly FencePolygon[], circles: readonly FenceCircle[]): SelectedFence | null => {
  const circleAt = (index: number) => {
    const circle = circles.find((it) => it.index === index);
    return circle === undefined ? null : circleFence(circle);
  };
  switch (selected?.kind) {
    case 'FenceVertex': {
      const polygon = polygons.find((it) => it.index === selected.polygon);
      return polygon === undefined ? null : polygonFence(polygon);
    }
    case 'Circle':
    case 'CircleCentre':
      return circleAt(selected.index);
    default:
      return null;
  }
};

export const fenceDetail = (selected: MapHit | null, polygons: readonly FencePolygon[], circles: readonly FenceCircle[]): string | null => {
  const fence = selectedFence(selected, polygons, circles);
  const parts = fence === null ? [] : [fence.kindText, inclusionText(fence.keepsIn), fence.detailText].filter((it) => it.trim() !== '');
  return parts.length === 0 ? null : parts.join(' · ');
};

export const selectedLanding = (selected: MapHit | null, landings: readonly LandingPattern[]): LandingPattern | null => {
  const index = selectedItem(selected);
  return index === null ? null : (landings.find((it) => it.index === index) ?? null);
};

export const selectedSurvey = (selected: MapHit | null, surveys: readonly Survey[]): Survey | null => {
  const index = selectedItem(selected);
  return index === null ? null : (surveys.find((it) => it.index === index) ?? null);
};
