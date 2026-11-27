// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { OSD_SCRIM } from '@/ui/Osd';
import { BufferedVideoNote } from '@/ui/VideoSurface';
import { BUFFERED_VIDEO_NOTE, BUFFERED_VIDEO_SHORT } from '@/ui/VideoView';

const shown = vi.hoisted(() => ({ view: '{}' }));
vi.mock('@/bridge/QgcState', async (importOriginal) => ({ ...(await importOriginal<typeof import('@/bridge/QgcState')>()), useQgcPath: () => parseJson(shown.view) }));

const channelOf = (hex: string, at: number): number => parseInt(hex.slice(1 + at * 2, 3 + at * 2), 16);
const luminance = (rgb: readonly number[]): number =>
  rgb.map((value) => value / 255).map((value) => (value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4)).reduce((sum, value, at) => sum + value * [0.2126, 0.7152, 0.0722][at]!, 0);
const contrast = (above: readonly number[], below: readonly number[]): number => (Math.max(luminance(above), luminance(below)) + 0.05) / (Math.min(luminance(above), luminance(below)) + 0.05);

describe('the buffered video notice', () => {
  beforeEach(() => {
    shown.view = '{"hlsEndpoints":[null,"http://falcon:8888/belly/index.m3u8"]}';
    vi.spyOn(Qgc, 'invoke').mockResolvedValue(true);
  });
  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it('a picture-in-picture camera that fell back says so and has its own way back', () => {
    render(<BufferedVideoNote channel={1} playing text={BUFFERED_VIDEO_SHORT} />);
    expect(screen.getByRole('status').textContent).toContain(BUFFERED_VIDEO_SHORT);
    fireEvent.click(screen.getByRole('button', { name: 'Try low latency' }));
    expect(Qgc.invoke).toHaveBeenCalledWith('video.tryLowLatency', 1);
  });

  it('says nothing for a camera on WebRTC, or one whose buffered twin is not playing yet, and keeps its live region mounted', () => {
    const { rerender } = render(<BufferedVideoNote channel={0} playing text={BUFFERED_VIDEO_NOTE} />);
    expect(screen.getByRole('status').textContent).toBe('');
    rerender(<BufferedVideoNote channel={1} playing={false} text={BUFFERED_VIDEO_NOTE} />);
    expect(screen.getByRole('status').textContent).toBe('');
  });

  it('the way back reads on the scrim in both themes: white, not the theme primary that is near-black in light', () => {
    render(<BufferedVideoNote channel={1} playing text={BUFFERED_VIDEO_NOTE} />);
    const button = screen.getByRole('button', { name: 'Try low latency' });
    expect(button.style.color).not.toContain('var(');
    const [, colour, percent] = /color-mix\(in srgb, (#[0-9a-f]{6}) (\d+)%, transparent\)/.exec(OSD_SCRIM)!;
    const alpha = Number(percent) / 100;
    const brightestPicture = [0, 1, 2].map((at) => Math.round(channelOf(colour!, at) * alpha + 255 * (1 - alpha)));
    const text = /rgb\((\d+), (\d+), (\d+)\)/.exec(button.style.color)!.slice(1).map(Number);
    expect(contrast(text, brightestPicture)).toBeGreaterThanOrEqual(4.5);
  });
});
