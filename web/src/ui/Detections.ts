import { type Json, isNull, jsonObjects, optBoolean, optDouble, optJSONArray, optText } from '@/bridge/Json';

export const DETECTIONS = 'view.detections';

export type DetectionBox = { x: number; y: number; w: number; h: number; label: string; confidence: number; target: boolean };

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
      label: optText(box, 'label'),
      confidence: optDouble(box, 'confidence', 0),
      target: optBoolean(box, 'target'),
    })),
    error: error.trim() === '' ? null : error,
  };
};

export const visibleBoxes = (reading: Detections | null): readonly DetectionBox[] => (reading !== null && reading.available && !reading.stale ? reading.boxes : []);

export const NO_FRAMES = 'no frames from the detector';

export const detectionTrouble = (reading: Detections | null): string | null =>
  reading === null || !reading.available ? null : (reading.error ?? (reading.stale ? NO_FRAMES : null));

export const boxCaption = (box: DetectionBox): string => {
  const percent = Math.trunc(box.confidence * 100);
  if (box.label.trim() === '' && percent <= 0) return '';
  if (box.label.trim() === '') return `${percent}%`;
  if (percent <= 0) return box.label;
  return `${box.label} ${percent}%`;
};
