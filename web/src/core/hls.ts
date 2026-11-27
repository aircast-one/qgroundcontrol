import Hls from 'hls.js';
import { type WhepReport, whileShown } from './whep';

const REPORT_MS = 500;

const connectHls = (video: HTMLVideoElement, url: string, report: (reading: WhepReport) => void): (() => void) => {
  const session = { error: '', first: true };
  const hls = Hls.isSupported() ? new Hls({ lowLatencyMode: true }) : null;
  hls?.on(Hls.Events.ERROR, (_event, data) => {
    session.error = data.fatal ? `${data.type}: ${data.details}` : session.error;
  });
  hls?.loadSource(url);
  hls?.attachMedia(video);
  video.src = hls === null ? url : video.src;
  video.muted = true;
  void video.play().catch(() => undefined);
  const timer = setInterval(() => {
    const frames = video.getVideoPlaybackQuality().totalVideoFrames;
    report({ running: video.readyState >= HTMLMediaElement.HAVE_CURRENT_DATA, frames, width: video.videoWidth, height: video.videoHeight, error: session.error, restarted: session.first, stats: null });
    session.first = false;
  }, REPORT_MS);
  return () => {
    clearInterval(timer);
    hls?.destroy();
    video.removeAttribute('src');
    video.load();
  };
};

export const playHls = (video: HTMLVideoElement, url: string, report: (reading: WhepReport) => void): (() => void) =>
  whileShown(() => connectHls(video, url, report), () => document.pictureInPictureElement === video);
