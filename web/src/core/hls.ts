import type Hls from 'hls.js';
import { type PlaybackReport, REPORT_MS, STOPPED, watched, whileShown } from './whep';

const connectHls = (video: HTMLVideoElement, url: string, report: (reading: PlaybackReport) => void): (() => void) => {
  const session: { error: string; first: boolean; stopped: boolean; attached: boolean; hls: Hls | null } = { error: '', first: true, stopped: false, attached: false, hls: null };
  const said = (error: unknown): string => (error instanceof Error ? error.message : String(error));
  void import('hls.js/light').then(
    ({ default: Player }) => {
      if (session.stopped) return;
      session.hls = Player.isSupported() ? new Player() : null;
      session.attached = true;
      session.hls?.on(Player.Events.ERROR, (_event, data) => {
        session.error = data.fatal ? `${data.type}: ${data.details}` : session.error;
      });
      session.hls?.loadSource(url);
      session.hls?.attachMedia(video);
      if (session.hls === null) video.src = url;
      video.muted = true;
      void video.play().catch((error: unknown) => {
        session.error = `The browser would not start the buffered video: ${said(error)}`;
      });
    },
    (error: unknown) => {
      session.error = `The buffered video player did not load: ${said(error)}`;
    },
  );
  const timer = setInterval(() => {
    const frames = video.getVideoPlaybackQuality().totalVideoFrames;
    report({ running: video.readyState >= HTMLMediaElement.HAVE_CURRENT_DATA, frames, width: video.videoWidth, height: video.videoHeight, error: session.error, restarted: session.first, stats: null });
    session.first = false;
  }, REPORT_MS);
  return () => {
    session.stopped = true;
    clearInterval(timer);
    session.hls?.destroy();
    if (!session.attached) return;
    video.removeAttribute('src');
    video.load();
  };
};

export const playHls = (video: HTMLVideoElement, url: string, report: (reading: PlaybackReport) => void): (() => void) =>
  whileShown(
    () => connectHls(video, url, report),
    () => watched(video),
    () => report(STOPPED),
  );
