export type WhepReport = { running: boolean; frames: number; width: number; height: number; error: string; restarted: boolean };

const ICE_GATHER_MS = 2000;
const REPORT_MS = 500;

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

export const playWhep = (video: HTMLVideoElement, endpoint: string, report: (reading: WhepReport) => void): (() => void) => {
  const peer = new RTCPeerConnection();
  const session = { resource: null as string | null, error: '', first: true };
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
    const frames = video.getVideoPlaybackQuality().totalVideoFrames;
    report({ running: peer.connectionState === 'connected', frames, width: video.videoWidth, height: video.videoHeight, error: session.error, restarted: session.first });
    session.first = false;
  }, REPORT_MS);
  return () => {
    clearInterval(timer);
    peer.close();
    video.srcObject = null;
    if (session.resource !== null) void fetch(session.resource, { method: 'DELETE' }).catch(() => undefined);
  };
};
