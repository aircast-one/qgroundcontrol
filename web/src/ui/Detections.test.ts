import { describe, expect, it } from 'vitest';
import type { Json } from '@/bridge/Json';
import { type Detections, NO_FRAMES, boxCaption, detectionTrouble, detections, visibleBoxes } from '@/ui/Detections';

const served = (): Json => ({
  kind: 'object',
  class: 'Detections',
  available: true,
  host: '10.0.0.4',
  camera: 'front',
  stale: false,
  ageMs: 40,
  boxes: [
    { x: 0.1, y: 0.2, w: 0.3, h: 0.4, label: 'car', confidence: 0.91, target: true },
    { x: 0.5, y: 0.5, w: 0.1, h: 0.1, label: 'person', confidence: 0.42, target: false },
  ],
  track: null,
  error: null,
});

describe('DetectionsTest', () => {
  it('a frame carries its boxes with the target marked', () => {
    const reading = detections(served())!;

    expect(reading.boxes.map((it) => it.label)).toEqual(['car', 'person']);
    expect(reading.boxes.map((it) => it.target)).toEqual([true, false]);
    expect(reading.boxes[0]!.x).toBeCloseTo(0.1, 9);
    expect(reading.boxes[0]!.h).toBeCloseTo(0.4, 9);
    expect(reading.error).toBeNull();
  });

  it('a reply that is not the detections view reads as nothing', () => {
    expect(detections(null)).toBeNull();
    expect(detections({ kind: 'null' })).toBeNull();
  });

  it('nothing is drawn when the feed is stale or unconfigured', () => {
    const stale = { ...served(), stale: true };
    const unconfigured = { ...served(), available: false };

    expect(visibleBoxes(detections(stale))).toEqual([]);
    expect(visibleBoxes(detections(unconfigured))).toEqual([]);
    expect(visibleBoxes(detections(served()))).toHaveLength(2);
  });

  it('a configured feed reports its trouble and an unconfigured one stays quiet', () => {
    const broken = { ...served(), error: 'connection refused' };
    const unconfigured = { ...served(), available: false, error: 'connection refused' };

    expect(detectionTrouble(detections(broken))).toBe('connection refused');
    expect(detectionTrouble(detections(unconfigured))).toBeNull();
    expect(detectionTrouble(detections(served()))).toBeNull();
  });

  it('a caption names what was seen and how sure the detector is', () => {
    const reading = detections(served())!;
    const [car, person] = reading.boxes;

    expect(boxCaption(car!)).toBe('car 91%');
    expect(boxCaption(person!)).toBe('person 42%');
    expect(boxCaption({ ...car!, label: '', confidence: 0.0 })).toBe('');
    expect(boxCaption({ ...car!, confidence: 0.0 })).toBe('car');
  });
});

describe('DetectionTroubleTest', () => {
  const reading = (available: boolean, stale: boolean, error: string | null): Detections => ({ available, stale, boxes: [], error });

  it('an unconfigured detector says nothing', () => {
    expect(detectionTrouble(reading(false, true, 'boom'))).toBeNull();
  });

  it('a detector that stopped sending frames says so', () => {
    expect(detectionTrouble(reading(true, true, null))).toBe(NO_FRAMES);
  });

  it('a working detector with nothing in view says nothing', () => {
    expect(detectionTrouble(reading(true, false, null))).toBeNull();
  });

  it('a reported error outranks staleness', () => {
    expect(detectionTrouble(reading(true, true, 'stream refused'))).toBe('stream refused');
  });
});
