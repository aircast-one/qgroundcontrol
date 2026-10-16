import { describe, expect, it } from 'vitest';
import { FlyWait, NoVideoAction, activeCameraStatus, elapsedText, flyWait, noVideoDetail, unavailableVideoState, videoSourcesButton, whileArmed } from '@/ui/NoVideoPanel';
import { AppNavigationState } from '@/ui/AppNavigation';
import { VIDEO_SOURCES_PAGE } from '@/ui/Cameras';
import { Rect } from '@/ui/FlyShell';
import { centredRegion, looksForAircraft, messageRegion } from '@/ui/VideoSurface';
import { VideoReading } from '@/ui/VideoView';

describe('NoVideoPanelTest', () => {
  const video = VideoReading({ available: true, decoding: false, sourceSize: null, summary: 'Waiting for a stream.', activeSource: 0, cameras: [], noVideoText: 'No video on UDP port 5600' });

  it('the detail names the source and how long it has failed', () => {
    expect(noVideoDetail(video, 12)).toBe('No video on UDP port 5600 for 12 s');
    expect(noVideoDetail({ ...video, streaming: true }, 125)).toBe('Receiving data — waiting for video for 2 min');
    expect(elapsedText(3900)).toBe('1 h 5 min');
  });

  it('a set-up camera shows its own state while no aircraft is connected', () => {
    expect(looksForAircraft(false, video)).toBe(false);
    expect(looksForAircraft(false, { ...video, available: false })).toBe(true);
    expect(looksForAircraft(false, null)).toBe(true);
    expect(looksForAircraft(true, null)).toBe(false);
  });

  it('a stalled stream says why instead of how long', () => {
    const reason = "No answer from 192.168.1.50:8889. Check the address and that this device is on the drone's network.";
    expect(noVideoDetail({ ...video, noVideoReason: reason }, 70)).toBe(reason);
  });

  it("the panel reads the active camera's own status like FlightDisplayViewVideo", () => {
    const failing = { ...video, cameras: [{ slot: 0, status: 'Connection failed, retrying', configured: true }] };
    expect(activeCameraStatus(failing)).toBe('Connection failed, retrying');
    expect(activeCameraStatus(video)).toBeNull();
  });

  const unavailable = (sourceChosen: boolean, configured: boolean, streamEnabled = true) =>
    VideoReading({
      available: false,
      decoding: false,
      sourceSize: null,
      summary: 'No stream URL is set.',
      activeSource: 0,
      cameras: [{ slot: 0, status: '', configured }],
      sourceChosen,
      streamEnabled,
    });

  it('anUnavailableStreamSaysWhatIsMissingAndWhereToFixIt', () => {
    expect(unavailableVideoState(unavailable(false, false))?.action).toBe(NoVideoAction.SetUp);
    expect(unavailableVideoState(unavailable(true, false))?.title).toBe('No stream address');
    expect(unavailableVideoState(unavailable(true, true))?.action).toBe(NoVideoAction.Settings);
    expect(unavailableVideoState({ ...unavailable(true, true), available: true })).toBeNull();
  });

  it('a stream switched off offers to turn it on rather than sending the pilot to settings', () => {
    const off = unavailableVideoState(unavailable(true, true, false));
    expect(off?.title).toBe('Video off');
    expect(off?.action).toBe(NoVideoAction.TurnOn);
    expect(unavailableVideoState(unavailable(false, false, false))?.action).toBe(NoVideoAction.TurnOn);
  });

  it('while armed only the one-tap turn-on survives, never a trip to settings', () => {
    const off = whileArmed(unavailableVideoState(unavailable(true, true, false))!);
    expect(off.action).toBe(NoVideoAction.TurnOn);
    expect(off.detail).toBe('');
    expect(whileArmed(unavailableVideoState(unavailable(true, false))!).action).toBe(NoVideoAction.None);
    expect(whileArmed(unavailableVideoState(unavailable(false, false))!).action).toBe(NoVideoAction.None);
  });

  it('the missing-video copy points at the Video sources page on the Transmission tab', () => {
    expect(unavailableVideoState(unavailable(false, false))?.detail).toBe('Add a camera in Settings › Transmission › Video sources.');
    expect(unavailableVideoState(unavailable(true, false))?.detail).toBe('Enter the stream address in Settings › Transmission › Video sources.');
  });

  it('the video button is named for the Video sources page it opens', () => {
    const navigation = new AppNavigationState();
    const button = videoSourcesButton(navigation);
    button.onClick();
    expect(button.label).toBe('Video sources');
    expect(navigation.settingsPage.value).toBe(VIDEO_SOURCES_PAGE);
  });

  it('the message takes the largest free side of the instruments, or the whole area when they are elsewhere', () => {
    const free = Rect(0, 100, 1000, 1500);
    expect(messageRegion(free, [Rect(0, 100, 500, 220)], 10, 200, 50)).toEqual(Rect(0, 230, 1000, 1500));
    expect(messageRegion(free, [Rect(0, 100, 300, 1500)], 10, 200, 50)).toEqual(Rect(310, 100, 1000, 1500));
    expect(messageRegion(free, [Rect(300, 1600, 700, 1700)], 10, 200, 50)).toEqual(free);
    expect(messageRegion(free, [], 10, 200, 50)).toEqual(free);
  });

  it('the message clears every widget, looking past the first side it finds', () => {
    const free = Rect(0, 100, 1000, 600);
    const tape = Rect(0, 100, 400, 200);
    const fleet = Rect(0, 210, 900, 600);
    expect(messageRegion(free, [tape, fleet], 10, 200, 50)).toEqual(Rect(410, 100, 1000, 200));
  });

  it('with no gap big enough for the pill there is no message rather than a squeezed one', () => {
    const free = Rect(0, 100, 1000, 600);
    expect(messageRegion(free, [Rect(0, 100, 950, 600)], 10, 200, 50)).toBeNull();
    expect(messageRegion(Rect(0, 0, 150, 600), [], 10, 200, 50)).toBeNull();
  });

  it('the synthetic view waits for the aircraft instead of offering to retry a stream it never had', () => {
    expect(looksForAircraft(false, video, true)).toBe(true);
    expect(flyWait(false, video, true, false)).toBe(FlyWait.Aircraft);
    expect(flyWait(true, video, true, false)).toBe(FlyWait.Position);
    expect(flyWait(true, video, true, true)).toBeNull();
    expect(flyWait(false, video, false, false), 'a stream camera keeps saying why it has no picture').toBeNull();
    expect(flyWait(false, { ...video, decoding: true }, true, false)).toBeNull();
  });
});

describe('CentredRegionTest', () => {
  it('a message beside the mini-map still centres on the screen when it fits', () => {
    const room = Rect(410, 100, 1990, 900);
    expect(centredRegion(room, 1000, 400)).toEqual(Rect(410, 100, 1590, 900));
  });

  it('a room too lopsided to centre in keeps its own shape', () => {
    const room = Rect(900, 100, 1990, 900);
    expect(centredRegion(room, 1000, 400)).toEqual(room);
  });
});
