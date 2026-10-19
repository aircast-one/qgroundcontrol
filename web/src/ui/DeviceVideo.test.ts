import { describe, expect, it } from 'vitest';
import { SharingStage, modelFields, modelPath, parseVideo, uploadMessage } from '@/ui/DeviceVideo';

const served = {
  radio: { status: { level: 0, text: 'on air' }, channel: '165', region: 'BO', txPower: '', details: 'UDP 5602' },
  sharing: { status: { level: 1, text: 'finish setup' }, stage: 'deploy', deployUrl: 'https://deploy.workers.cloudflare.com/?url=x', signKey: 'sk', deviceToken: 'dt' },
  detection: {
    status: { level: 3, text: 'visdrone · Aerial' },
    unavailable: true,
    models: [{ name: 'visdrone', path: '/m/visdrone.hef', about: 'Aerial', size: '12 MB', classes: { level: 3, text: '10 classes' }, active: true }],
  },
  engineering: { title: 'Engineering', href: '/settings/engineering', icon: 'server', summary: 'mediamtx' },
};

describe('DeviceVideoTest', () => {
  it('reads the video page the device serves', () => {
    const view = parseVideo(served);
    expect(view?.radio).toEqual({ status: { level: 0, text: 'on air' }, channel: '165', region: 'BO', txPower: '', details: 'UDP 5602' });
    expect(view?.sharing).toMatchObject({ stage: SharingStage.Deploy, signKey: 'sk', deviceToken: 'dt', baseUrl: '' });
    expect(view?.detection.unavailable).toBe(true);
    expect(view?.detection.models[0]).toMatchObject({ name: 'visdrone', active: true, classes: { level: 3, text: '10 classes' } });
    expect(view?.engineering).toEqual({ title: 'Engineering', href: '/settings/engineering', summary: 'mediamtx' });
  });

  it('a device with no radio has no radio card', () => {
    expect(parseVideo({ ...served, radio: undefined })?.radio).toBeNull();
    expect(parseVideo({ radio: served.radio })).toBeNull();
  });

  it('an upload sends the model and only the extras that were given', () => {
    const hef = new File(['x'], 'm.hef');
    const labels = new File(['a\nb'], 'm.labels');
    expect(modelFields({ hef, labels: null, about: '  ', use: false })).toEqual([{ name: 'hef', file: hef }]);
    expect(modelFields({ hef, labels, about: ' Aerial ', use: true })).toEqual([
      { name: 'hef', file: hef },
      { name: 'labels', file: labels },
      { name: 'about', value: 'Aerial' },
      { name: 'use', value: '1' },
    ]);
  });

  it('a switch that did not happen after upload is said, not swallowed', () => {
    expect(uploadMessage(true, true)).toEqual({ refusal: null, message: 'Model uploaded and switched — detection restarts on it.' });
    expect(uploadMessage(false, true).refusal?.message).toContain('switching to it failed');
    expect(uploadMessage(false, false)).toMatchObject({ refusal: null });
  });

  it('a model name is escaped into its path', () => {
    expect(modelPath('aerial v2')).toBe('/api/detect/models/aerial%20v2');
  });
});
