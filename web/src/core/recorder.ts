const MIME_TYPES: readonly string[] = ['video/mp4;codecs=avc1', 'video/mp4', 'video/webm;codecs=vp9', 'video/webm'];
const CHUNK_MS = 1000;

export const recordedName = (file: string, mime: string): string => {
  const leaf = file.slice(file.lastIndexOf('/') + 1);
  const stem = leaf.includes('.') ? leaf.slice(0, leaf.lastIndexOf('.')) : leaf;
  return `${stem}.${mime.startsWith('video/mp4') ? 'mp4' : 'webm'}`;
};

export const recordStream = (stream: MediaStream, file: string, save: (recording: File) => void): (() => void) => {
  const mimeType = MIME_TYPES.find((type) => MediaRecorder.isTypeSupported(type));
  const recorder = new MediaRecorder(stream, mimeType === undefined ? {} : { mimeType });
  const held: { chunks: readonly Blob[] } = { chunks: [] };
  recorder.ondataavailable = ({ data }) => {
    if (data.size > 0) held.chunks = [...held.chunks, data];
  };
  recorder.onstop = () => save(new File([...held.chunks], recordedName(file, recorder.mimeType), { type: recorder.mimeType }));
  recorder.start(CHUNK_MS);
  return () => {
    if (recorder.state !== 'inactive') recorder.stop();
  };
};
