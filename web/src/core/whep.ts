export type WhepCounters = { bytes: number; lost: number; received: number; dropped: number; at: number };
export type WhepStats = { fps: number; kbps: number; jitterMs: number; lossPercent: number; skippedPerSecond: number; counters: WhepCounters };
export type WhepReport = { running: boolean; frames: number; width: number; height: number; error: string; restarted: boolean; stats: WhepStats | null };
type InboundVideo = { timestamp: number; framesPerSecond?: number; bytesReceived?: number; jitter?: number; packetsLost?: number; packetsReceived?: number; framesDropped?: number };

const ICE_GATHER_MS = 2000;
const REPORT_MS = 500;
const STATS_WINDOW_REPORTS = 5;

const iceGathered = (peer: RTCPeerConnection): Promise<void> =>
  new Promise((resolve) => {
    if (peer.iceGatheringState === 'complete') return resolve();
    const done = () => {
      peer.removeEventListener('icegatheringstatechange', changed);
      resolve();
    };
    const changed = () => (peer.iceGatheringState === 'complete' ? done() : undefined);
    peer.addEventListener('icegatheringstatechange', changed);
    setTimeout(done, ICE_GATHER_MS);
  });

const negotiate = async (peer: RTCPeerConnection, endpoint: string): Promise<string | null> => {
  await peer.setLocalDescription(await peer.createOffer());
  await iceGathered(peer);
  const answer = await fetch(endpoint, { method: 'POST', headers: { 'Content-Type': 'application/sdp' }, body: peer.localDescription?.sdp ?? '' });
  const sdp = await answer.text();
  if (!answer.ok) throw new Error(`Unexpected response: ${answer.status} - ${sdp.trim()}`);
  await peer.setRemoteDescription({ type: 'answer', sdp });
  const resource = answer.headers.get('Location');
  return resource === null ? null : new URL(resource, endpoint).href;
};

export const whepStats = (inbound: InboundVideo, oldest: WhepCounters | null): WhepStats => {
  const counters = { bytes: inbound.bytesReceived ?? 0, lost: inbound.packetsLost ?? 0, received: inbound.packetsReceived ?? 0, dropped: inbound.framesDropped ?? 0, at: inbound.timestamp };
  const since = oldest ?? counters;
  const elapsedMs = counters.at - since.at;
  const lost = counters.lost - since.lost;
  const packets = lost + counters.received - since.received;
  return {
    fps: Math.round(inbound.framesPerSecond ?? 0),
    kbps: elapsedMs > 0 ? Math.round(((counters.bytes - since.bytes) * 8) / elapsedMs) : 0,
    jitterMs: Math.round((inbound.jitter ?? 0) * 1000),
    lossPercent: packets > 0 ? (100 * lost) / packets : 0,
    skippedPerSecond: elapsedMs > 0 ? (1000 * (counters.dropped - since.dropped)) / elapsedMs : 0,
    counters,
  };
};

const inboundVideo = (report: RTCStatsReport): InboundVideo | null =>
  ([...report.values()] as (InboundVideo & { type: string; kind?: string })[]).find((stat) => stat.type === 'inbound-rtp' && stat.kind === 'video') ?? null;

export const playWhep = (video: HTMLVideoElement, endpoint: string, report: (reading: WhepReport) => void): (() => void) => {
  const peer = new RTCPeerConnection();
  const session = { resource: null as string | null, error: '', first: true, stats: null as WhepStats | null, history: [] as readonly WhepCounters[] };
  peer.addTransceiver('video', { direction: 'recvonly' });
  peer.ontrack = ({ streams, track }) => {
    video.srcObject = streams[0] ?? new MediaStream([track]);
  };
  negotiate(peer, endpoint).then(
    (resource) => {
      session.resource = resource;
    },
    (error: unknown) => {
      session.error = error instanceof Error ? error.message : String(error);
    },
  );
  const timer = setInterval(() => {
    void peer.getStats().then((report) => {
      const inbound = inboundVideo(report);
      session.stats = inbound === null ? null : whepStats(inbound, session.history[0] ?? null);
      session.history = session.stats === null ? [] : [...session.history, session.stats.counters].slice(-STATS_WINDOW_REPORTS);
    }, () => undefined);
    const frames = video.getVideoPlaybackQuality().totalVideoFrames;
    report({ running: peer.connectionState === 'connected', frames, width: video.videoWidth, height: video.videoHeight, error: session.error, restarted: session.first, stats: session.stats });
    session.first = false;
  }, REPORT_MS);
  return () => {
    clearInterval(timer);
    peer.close();
    video.srcObject = null;
    if (session.resource !== null) void fetch(session.resource, { method: 'DELETE' }).catch(() => undefined);
  };
};
