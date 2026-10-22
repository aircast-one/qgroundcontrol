import { type Json, isNull, jsonObjects, optBoolean, optDouble, optJSONArray, optText } from '@/bridge/Json';

export const DETECTIONS = 'view.detections';

export type DetectionBox = { x: number; y: number; w: number; h: number; caption: string; target: boolean };

export type Detections = { available: boolean; stale: boolean; boxes: readonly DetectionBox[]; error: string | null };

export const detections = (view: Json | null): Detections | null => {
  if (view === null || optText(view, 'class') !== 'Detections') return null;
  const error = isNull(view, 'error') ? '' : optText(view, 'error');
  return {
    available: optBoolean(view, 'available'),
    stale: optBoolean(view, 'stale'),
    boxes: jsonObjects(optJSONArray(view, 'boxes')).map((box) => ({
      x: optDouble(box, 'x', 0),
      y: optDouble(box, 'y', 0),
      w: optDouble(box, 'w', 0),
      h: optDouble(box, 'h', 0),
      caption: optText(box, 'caption'),
      target: optBoolean(box, 'target'),
    })),
    error: error.trim() === '' ? null : error,
  };
};

export const visibleBoxes = (reading: Detections | null): readonly DetectionBox[] => (reading !== null && reading.available && !reading.stale ? reading.boxes : []);

export const NO_FRAMES = 'no frames from the detector';

export const detectionTrouble = (reading: Detections | null): string | null =>
  reading === null || !reading.available ? null : (reading.error ?? (reading.stale ? NO_FRAMES : null));

