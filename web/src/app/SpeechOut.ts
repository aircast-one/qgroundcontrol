import { Qgc } from '@/bridge/Qgc';
import { coerceIn } from '@/kotlin/Std';
import { SPEECH_VIEW, speechBatch, speechPath } from '@/ui/VoiceAlerts';

const POLL_MS = 500;
const MAX_TEXT_QUEUE = 20;
const SPEECH_LANGUAGE = 'en-US';
const RAW = '../../../android/app/src/main/res/raw';

const clips = import.meta.glob('../../../android/app/src/main/res/raw/*.m4a', { query: '?url', import: 'default', eager: true }) as Record<string, string>;

const clipUrl = (clip: string): string | null => clips[`${RAW}/${clip}.m4a`] ?? null;

const speech: {
  started: boolean;
  ready: boolean;
  poller: ReturnType<typeof setTimeout> | null;
  after: number | null;
  waiting: number;
  generation: number;
  muted: boolean;
  queue: Promise<void>;
  stopCurrent: (() => void) | null;
} = { started: false, ready: false, poller: null, after: null, waiting: 0, generation: 0, muted: false, queue: Promise.resolve(), stopCurrent: null };

const flush = () => {
  speech.generation += 1;
  speech.stopCurrent?.();
  speech.stopCurrent = null;
  speechSynthesis.cancel();
  speech.waiting = 0;
};

const played = (text: string, clip: string, volume: number): Promise<void> =>
  new Promise((resolve) => {
    const url = clipUrl(clip);
    if (url !== null) {
      const audio = new Audio(url);
      audio.volume = volume;
      audio.onended = () => resolve();
      audio.onerror = () => resolve();
      speech.stopCurrent = () => {
        audio.pause();
        resolve();
      };
      return void audio.play().catch(() => resolve());
    }
    const utterance = new SpeechSynthesisUtterance(text);
    utterance.lang = SPEECH_LANGUAGE;
    utterance.volume = volume;
    utterance.onend = () => resolve();
    utterance.onerror = () => resolve();
    speech.stopCurrent = () => resolve();
    speechSynthesis.speak(utterance);
  });

const speak = (text: string, clip: string, volume: number) => {
  if (speech.waiting >= MAX_TEXT_QUEUE) flush();
  const generation = speech.generation;
  speech.waiting += 1;
  speech.queue = speech.queue
    .then(() => (generation === speech.generation ? played(text, clip, coerceIn(volume, 0, 1)) : undefined))
    .catch(() => undefined)
    .finally(() => {
      if (generation === speech.generation) speech.waiting = Math.max(0, speech.waiting - 1);
    });
};

const poll = async (): Promise<void> => {
  const batch = speechBatch(await Qgc.get(speech.after === null ? SPEECH_VIEW : speechPath(speech.after)).catch(() => null));
  if (!speech.started) return;
  speech.poller = setTimeout(() => void poll(), POLL_MS);
  if (batch === null) return;
  const seen = speech.after;
  speech.after = batch.last;
  if (batch.muted && !speech.muted && speech.ready) flush();
  speech.muted = batch.muted;
  if (seen === null || !speech.ready) return;
  batch.lines.filter((it) => it.sequence > seen).map((it) => speak(it.text, it.clip, it.volume));
};

export const SpeechOut = {
  start(): void {
    if (speech.started) return;
    speech.started = true;
    speech.ready = typeof speechSynthesis !== 'undefined';
    speech.poller = setTimeout(() => void poll(), POLL_MS);
  },

  say(text: string, clip: string): void {
    if (speech.started && speech.ready && !speech.muted) speak(text, clip, 1);
  },

  stop(): void {
    if (speech.poller !== null) clearTimeout(speech.poller);
    if (speech.ready) flush();
    speech.started = false;
    speech.ready = false;
    speech.poller = null;
    speech.after = null;
    speech.waiting = 0;
    speech.muted = false;
  },
};
