import { describe, expect, it } from 'vitest';
import { SharingStage, parseVideo } from '@/ui/DeviceVideo';

const served = {
  radio: { status: { level: 0, text: 'on air' }, channel: '165', region: 'BO', txPower: '', details: 'UDP 5602' },
  sharing: { status: { level: 1, text: 'finish setup' }, stage: 'deploy', deployUrl: 'https://deploy.workers.cloudflare.com/?url=x', signKey: 'sk', deviceToken: 'dt' },
  engineering: { title: 'Engineering', href: '/settings/engineering', icon: 'server', summary: 'mediamtx' },
};

describe('DeviceVideoTest', () => {
  it('reads the video page the device serves', () => {
    const view = parseVideo(served);
    expect(view?.radio).toEqual({ status: { level: 0, text: 'on air' }, channel: '165', region: 'BO', txPower: '', details: 'UDP 5602' });
    expect(view?.sharing).toMatchObject({ stage: SharingStage.Deploy, signKey: 'sk', deviceToken: 'dt', baseUrl: '' });
    expect(view?.engineering).toEqual({ title: 'Engineering', href: '/settings/engineering', summary: 'mediamtx' });
  });

  it('a device with no radio has no radio card', () => {
    expect(parseVideo({ ...served, radio: undefined })?.radio).toBeNull();
    expect(parseVideo({ radio: served.radio })).toBeNull();
  });

});
