// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { playHls } from './hls';

const hlsjs = vi.hoisted(() => ({ supported: true, loaded: [] as string[], destroyed: 0 }));

vi.mock('hls.js/light', () => ({
  default: class {
    static isSupported = () => hlsjs.supported;
    static Events = { ERROR: 'hlsError' };
    on = () => undefined;
    loadSource = (url: string) => void hlsjs.loaded.push(url);
    attachMedia = (video: HTMLVideoElement) => {
      video.removeAttribute('src');
      video.appendChild(Object.assign(document.createElement('source'), { src: 'blob:managed-media-source' }));
    };
    destroy = () => {
      hlsjs.destroyed += 1;
    };
  },
}));

const TWIN = 'http://falcon:8888/front/index.m3u8';

describe('the buffered HLS player', () => {
  const element = () => {
    const video = document.createElement('video');
    video.play = () => Promise.reject(new Error('NotAllowedError'));
    video.load = () => undefined;
    video.getVideoPlaybackQuality = () => ({ totalVideoFrames: 0 }) as VideoPlaybackQuality;
    return video;
  };

  beforeEach(() => {
    vi.useFakeTimers();
    hlsjs.supported = true;
    hlsjs.loaded = [];
    hlsjs.destroyed = 0;
  });
  afterEach(() => vi.useRealTimers());

  it('leaves the source hls.js attached alone: on Safari it is a <source> child, and writing src back over it sets src="" and plays nothing', async () => {
    const video = element();
    const stop = playHls(video, TWIN, () => undefined);
    await vi.dynamicImportSettled();
    expect(hlsjs.loaded).toEqual([TWIN]);
    expect(video.hasAttribute('src')).toBe(false);
    expect(video.querySelector('source')?.getAttribute('src')).toBe('blob:managed-media-source');
    stop();
    expect(hlsjs.destroyed).toBe(1);
  });

  it('plays the playlist itself where the browser has HLS built in and no Media Source', async () => {
    hlsjs.supported = false;
    const video = element();
    playHls(video, TWIN, () => undefined);
    await vi.dynamicImportSettled();
    expect(video.getAttribute('src')).toBe(TWIN);
  });

  it('says why when the browser refuses to start playing, instead of a black picture with no reason', async () => {
    const video = element();
    const reports: string[] = [];
    playHls(video, TWIN, (reading) => void reports.push(reading.error));
    await vi.dynamicImportSettled();
    await vi.advanceTimersByTimeAsync(600);
    expect(reports.at(-1)).toContain('NotAllowedError');
  });

  it('a player stopped before hls.js arrived never attaches', async () => {
    const video = element();
    playHls(video, TWIN, () => undefined)();
    await vi.dynamicImportSettled();
    expect(hlsjs.loaded).toEqual([]);
    expect(video.querySelector('source')).toBeNull();
  });
});
