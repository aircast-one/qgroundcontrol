import { type Json, jsonObjects, optBoolean, optDouble, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { format } from '@/kotlin/Std';

export const GEOTAG_ROOT = 'geoTag';

export type GeoTagState = {
  logFile: string;
  imageDirectory: string;
  saveDirectory: string;
  errorMessage: string;
  progress: number;
  inProgress: boolean;
  tagged: number;
  skipped: number;
  failed: number;
  timeOffsetSecs: number;
  previewMode: boolean;
  images: readonly GeoTagImage[];
};

export type GeoTagImage = {
  fileName: string;
  status: number;
  statusString: string;
  errorMessage: string;
  coordinate: readonly [number, number] | null;
};

export const GEOTAG_TAGGED = 2;

export const geoTagImages = (json: Json): readonly GeoTagImage[] =>
  jsonObjects(optJSONArray(json, 'imageModel')).map((row) => {
    const coordinate = optJSONObject(row, 'coordinate');
    return {
      fileName: optText(row, 'fileName'),
      status: optInt(row, 'status'),
      statusString: optText(row, 'statusString'),
      errorMessage: optText(row, 'errorMessage'),
      coordinate: coordinate === null ? null : [optDouble(coordinate, 'latitude'), optDouble(coordinate, 'longitude')],
    };
  });

export const geoTagImageText = (image: GeoTagImage): string => (image.errorMessage.trim() === '' ? image.statusString : image.errorMessage);

export const geoTagCoordinate = (image: GeoTagImage): string | null =>
  image.coordinate !== null && image.status === GEOTAG_TAGGED ? format('%.6f, %.6f', image.coordinate[0], image.coordinate[1]) : null;

export const geoTagError = (state: GeoTagState): string | null => (state.errorMessage.trim() !== '' && !state.inProgress ? state.errorMessage : null);

export const geoTagState = (json: Json | null): GeoTagState | null =>
  json !== null && optText(json, 'class') === 'GeoTagController'
    ? {
        logFile: optText(json, 'logFile'),
        imageDirectory: optText(json, 'imageDirectory'),
        saveDirectory: optText(json, 'saveDirectory'),
        errorMessage: optText(json, 'errorMessage'),
        progress: optDouble(json, 'progress', 0),
        inProgress: optBoolean(json, 'inProgress'),
        tagged: optInt(json, 'taggedCount'),
        skipped: optInt(json, 'skippedCount'),
        failed: optInt(json, 'failedCount'),
        timeOffsetSecs: optDouble(json, 'timeOffsetSecs', 0),
        previewMode: optBoolean(json, 'previewMode'),
        images: geoTagImages(json),
      }
    : null;

export const geoTagButton = (state: GeoTagState): string => (state.inProgress ? 'Cancel' : state.previewMode ? 'Preview' : 'Start tagging');

export const geoTagSummary = (state: GeoTagState): string | null => {
  if (state.inProgress || state.tagged <= 0) return null;
  const details = [state.skipped > 0 ? `${state.skipped} skipped` : null, state.failed > 0 ? `${state.failed} failed` : null].filter((it) => it !== null);
  return `Successfully tagged ${state.tagged} images` + (details.length === 0 ? '' : ` (${details.join(', ')})`);
};

export const geoTagStep = (done: boolean, number: number): string => (done ? '✓' : `${number}`);

export const GEOTAG_IMAGE_EXTENSIONS = 'jpg,jpeg,tiff,tif,dng';

export const isGeoTagImage = (name: string): boolean => {
  const dot = name.lastIndexOf('.');
  return GEOTAG_IMAGE_EXTENSIONS.split(',').includes(dot < 0 ? '' : name.slice(dot + 1).toLowerCase());
};
