import { describe, expect, it } from 'vitest';
import { detecting, modelFields, modelPath, parseDetection, uploadMessage } from '@/ui/DeviceDetection';

const served = {
  status: { level: 3, text: 'sim · Sim targets · on the CPU' },
  unavailable: false,
  cameras: [
    { name: 'front', on: true, status: { level: 0, text: 'live' } },
    { name: 'belly', on: false, status: { level: 3, text: 'off' } },
  ],
  models: [{ name: 'sim', path: '/m/sim.onnx', about: 'Sim targets', size: '12 MB', classes: { level: 3, text: '3 classes' }, engine: 'CPU', active: true }],
};

describe('DeviceDetectionTest', () => {
  it('reads the detection page the device serves', () => {
    const view = parseDetection(served);
    expect(view?.status.text).toBe('sim · Sim targets · on the CPU');
    expect(view?.cameras).toEqual([
      { name: 'front', on: true, status: { level: 0, text: 'live' } },
      { name: 'belly', on: false, status: { level: 3, text: 'off' } },
    ]);
    expect(view?.models[0]).toMatchObject({ name: 'sim', engine: 'CPU', active: true, classes: { level: 3, text: '3 classes' } });
    expect(parseDetection('nope')).toBeNull();
  });

  it('knows whether any camera is detecting', () => {
    const view = parseDetection(served);
    expect(view !== null && detecting(view)).toBe(true);
    expect(detecting({ ...served, cameras: [{ name: 'belly', on: false, status: { level: 3, text: 'off' } }] })).toBe(false);
  });

  it('an upload sends the model and only the extras that were given', () => {
    const hef = new File(['x'], 'm.onnx');
    const labels = new File(['a\nb'], 'm.labels');
    expect(modelFields({ hef, labels: null, about: '  ', use: false })).toEqual([{ name: 'hef', file: hef }]);
    expect(modelFields({ hef, labels, about: ' Aerial ', use: true })).toEqual([
      { name: 'hef', file: hef },
      { name: 'labels', file: labels },
      { name: 'about', value: 'Aerial' },
      { name: 'use', value: '1' },
    ]);
  });

  it('an upload never claims detection runs when no camera is detecting', () => {
    expect(uploadMessage(true, true, true)).toEqual({ refusal: null, message: 'Model uploaded and switched — detection restarts on it.' });
    expect(uploadMessage(true, true, false)).toEqual({ refusal: null, message: 'Model uploaded and switched — no camera is detecting yet. Turn it on for a camera on this page.' });
    expect(uploadMessage(false, true, true).refusal?.message).toContain('switching to it failed');
    expect(uploadMessage(false, false, false)).toEqual({ refusal: null, message: 'Model uploaded — use it from the list when you want it.' });
  });

  it('a model name is escaped into its path', () => {
    expect(modelPath('aerial v2')).toBe('/api/detect/models/aerial%20v2');
  });
});
