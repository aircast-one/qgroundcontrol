import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { geoTagOutputText, parsedOffset, publishedNote, shownOffset } from './GeoTagRun';
import { type GeoTagState, geoTagButton, geoTagCoordinate, geoTagError, geoTagImageText, geoTagState, geoTagStep, geoTagSummary, isGeoTagImage } from './GeoTagView';

const state = (body: string): GeoTagState => {
  const parsed = geoTagState(parseJson(`{"kind":"object","class":"GeoTagController",${body}}`));
  if (parsed === null) throw new Error('parsed');
  return parsed;
};

describe('GeoTagViewTest', () => {
  it("the button reads as GeoTagPage's does", () => {
    expect(geoTagButton(state('"inProgress":true,"previewMode":true'))).toBe('Cancel');
    expect(geoTagButton(state('"inProgress":false,"previewMode":true'))).toBe('Preview');
    expect(geoTagButton(state('"inProgress":false,"previewMode":false'))).toBe('Start tagging');
  });

  it('the summary lists only what went wrong', () => {
    expect(geoTagSummary(state('"taggedCount":5'))).toBe('Successfully tagged 5 images');
    expect(geoTagSummary(state('"taggedCount":5,"skippedCount":2,"failedCount":1'))).toBe('Successfully tagged 5 images (2 skipped, 1 failed)');
    expect(geoTagSummary(state('"taggedCount":5,"inProgress":true'))).toBeNull();
    expect(geoTagSummary(state('"taggedCount":0'))).toBeNull();
  });

  it("each image shows its outcome and a tagged one its coordinate, as GeoTagPage's list does", () => {
    const images = state(
      '"imageModel":[{"fileName":"a.jpg","status":2,"statusString":"Tagged","errorMessage":"","coordinate":{"latitude":-35.1234567,"longitude":149.0}},{"fileName":"b.jpg","status":3,"statusString":"Skipped","errorMessage":"No matching trigger","coordinate":null}]',
    ).images;
    expect(images.map((it) => it.fileName)).toEqual(['a.jpg', 'b.jpg']);
    const [first, second] = images;
    if (first === undefined || second === undefined) throw new Error('two images');
    expect(geoTagImageText(first)).toBe('Tagged');
    expect(geoTagImageText(second)).toBe('No matching trigger');
    expect(geoTagCoordinate(first)).toBe('-35.123457, 149.000000');
    expect(geoTagCoordinate(second)).toBeNull();
  });

  it('the error is hidden while tagging runs', () => {
    expect(geoTagError(state('"inProgress":true,"errorMessage":"Tagging cancelled"'))).toBeNull();
    expect(geoTagError(state('"inProgress":false,"errorMessage":"Tagging cancelled"'))).toBe('Tagging cancelled');
  });

  it("steps tick once filled and only the core's image types are staged", () => {
    expect(geoTagStep(true, 1)).toBe('✓');
    expect(geoTagStep(false, 2)).toBe('2');
    expect(isGeoTagImage('IMG_0001.JPG')).toBe(true);
    expect(isGeoTagImage('raw.dng')).toBe(true);
    expect(isGeoTagImage('notes.txt')).toBe(false);
    expect(geoTagState(parseJson('{"class":"Something"}'))).toBeNull();
  });
});

describe('GeoTagRunTest', () => {
  it("the count compared is the core's tagged count, which already leaves failures out", () => {
    expect(publishedNote(4, 4)).toBeNull();
    expect(publishedNote(3, 4)).toBe('Only 3 of the 4 tagged images could be written to the chosen folder.');
  });

  it('the offset reads with either decimal mark and is shown with a point', () => {
    expect(parsedOffset('2,5')).toBe(2.5);
    expect(parsedOffset(' -1.0 ')).toBe(-1.0);
    expect(parsedOffset('abc')).toBeNull();
    expect(shownOffset(-3.5)).toBe('-3.5');
    expect(parsedOffset('9000')).toBeNull();
    expect(parsedOffset('-3600')).toBe(-3600.0);
    expect(parsedOffset('1.26')).toBeNull();
    expect(parsedOffset(' ')).toBe(0.0);
  });

  it('the output folder reads like GeoTagPage - chosen, else the photos TAGGED subfolder, else the default', () => {
    expect(geoTagOutputText('Out', 'Photos')).toBe('Out');
    expect(geoTagOutputText(null, 'Photos')).toBe('Photos/TAGGED');
    expect(geoTagOutputText(null, null)).toBe('Default: /TAGGED subfolder');
  });
});
