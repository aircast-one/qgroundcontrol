import { describe, expect, it } from 'vitest';
import { PipGeometry, PlaceholderUnits, pipRoom, placeholderUnits, portraitShowsCamera, portraitVideoThumbnail } from './FlyPortrait';
import { Offset, Rect, Size, rectCenter } from './FlyShell';
import { PipCorner, VideoSwipe, cameraStep, growingSwipe, hidingSwipe, videoSwipe } from '@/ui/VideoGestures';
import { awaitingAircraftText } from '@/ui/VideoSurface';
import type { VideoReading } from '@/ui/VideoView';

const reading = (available: boolean, streamEnabled: boolean, sourceChosen: boolean): VideoReading => ({
  available,
  decoding: false,
  sourceSize: null,
  summary: '',
  activeSource: 0,
  cameras: [],
  streaming: false,
  streamEnabled,
  noVideoText: '',
  noVideoReason: '',
  sourceChosen,
  standIn: null,
});

describe('FlyPortraitTest', () => {
  it('the camera pane stays in portrait while video is switched off so it can be turned back on', () => {
    expect(portraitShowsCamera(reading(true, true, true))).toBe(true);
    expect(portraitShowsCamera(reading(false, false, true))).toBe(true);
    expect(portraitShowsCamera(reading(false, true, false))).toBe(false);
    expect(portraitShowsCamera(reading(false, false, false))).toBe(false);
    expect(portraitShowsCamera(null)).toBe(false);
  });

  it('the map buttons drop below the video thumbnail only when the thumbnail is drawn', () => {
    expect(portraitVideoThumbnail(false, reading(true, true, true))).toBe(true);
    expect(portraitVideoThumbnail(false, reading(false, false, true)), 'video off draws no thumbnail in the map view').toBe(false);
    expect(portraitVideoThumbnail(true, reading(true, true, true)), 'the split view has the picture above, not a thumbnail').toBe(false);
    expect(portraitVideoThumbnail(false, null)).toBe(false);
  });

  it('a swipe past the threshold reads as its main direction, and a short one is ignored', () => {
    expect(videoSwipe(Offset(5, -80), 40)).toBe(VideoSwipe.Up);
    expect(videoSwipe(Offset(-10, 60), 40)).toBe(VideoSwipe.Down);
    expect(videoSwipe(Offset(0, -30), 40), 'too short').toBeNull();
    expect([VideoSwipe.Left, VideoSwipe.Right, VideoSwipe.Up].map(cameraStep)).toEqual([1, -1, null]);
    expect(videoSwipe(Offset(120, -60), 40), 'sideways switches the camera rather than hiding').toBe(VideoSwipe.Right);
    expect(videoSwipe(Offset(-90, 10), 40)).toBe(VideoSwipe.Left);
  });

  it('a dragged picture-in-picture settles on the corner nearest where it was dropped', () => {
    const geometry = PipGeometry({
      width: 1080,
      pip: Size(440, 248),
      inset: 33,
      pipTop: 300,
      bottomStartTop: 1600,
      bottomEndTop: 1550,
      split: Rect(0, 200, 1080, 808),
      full: Rect(0, 0, 1080, 2200),
    });
    const at = (x: number, y: number) => geometry.nearest(Offset(x, y));
    expect([at(300, 400), at(800, 400), at(300, 1700), at(800, 1650)]).toEqual([PipCorner.TopStart, PipCorner.TopEnd, PipCorner.BottomStart, PipCorner.BottomEnd]);
    expect(geometry.anchor(PipCorner.TopEnd)).toEqual(Offset(607, 300));
    expect(geometry.anchor(PipCorner.BottomStart), 'a bottom corner sits above the deck, clear of the compass or the scale bar').toEqual(Offset(33, 1600));
    const finger = Offset(540, 500);
    expect(rectCenter(geometry.pipAt(PipCorner.TopEnd, geometry.dragToCentre(PipCorner.TopEnd, finger))), 'pulled out of the split view, the picture is centred under the finger').toEqual(finger);
  });

  it('a picture hides toward the edge it sits on and grows away from it', () => {
    expect([hidingSwipe(PipCorner.TopEnd), growingSwipe(PipCorner.TopEnd)]).toEqual([VideoSwipe.Up, VideoSwipe.Down]);
    expect([hidingSwipe(PipCorner.BottomStart), growingSwipe(PipCorner.BottomStart)]).toEqual([VideoSwipe.Down, VideoSwipe.Up]);
  });

  it('the controls beside the video make room for the picture or for its tab', () => {
    expect(pipRoom(false, false)).toBe(0);
    expect(pipRoom(true, true)).toBeLessThan(pipRoom(true, false));
  });

  it('empty telemetry shows the units the pilot chose', () => {
    expect(placeholderUnits(0, 0, 2)).toEqual(PlaceholderUnits({ distance: 'ft', height: 'ft', speed: 'mph' }));
    expect(placeholderUnits(null, null, null)).toEqual(PlaceholderUnits({ distance: 'm', height: 'm', speed: 'm/s' }));
  });

  it('the video area says what it will show once the aircraft connects', () => {
    expect(awaitingAircraftText(true)).toBe('Synthetic view starts when your aircraft connects');
    expect(awaitingAircraftText(false)).toBe('Video starts when your aircraft connects');
  });
});
