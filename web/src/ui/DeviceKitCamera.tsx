import { type ReactNode, useEffect, useRef, useState } from 'react';
import { type WhepReport, playWhep } from '@/core/whep';
import { CircularProgressIndicator, Text, TextButton } from '@/m3';
import { MaterialIcon } from '@/res/Icon';
import { Color, md } from '@/theme/Colors';
import { DeviceDisclosure, DeviceNotice } from '@/ui/DeviceKit';
import type { LinkedNotice, SetupPart } from '@/ui/DeviceCameras';

const RETRY_MS = [2000, 4000, 8000, 15000] as const;

export const keepWhepPlaying = (video: HTMLVideoElement, endpoint: string, onPlaying: (playing: boolean) => void): (() => void) => {
  const session = {
    hangUp: () => undefined as void,
    attempt: 0,
    connected: false,
    timer: undefined as ReturnType<typeof setTimeout> | undefined,
  };
  const report = (reading: WhepReport) => {
    onPlaying(reading.running && reading.frames > 0);
    if (reading.running) {
      session.attempt = 0;
      session.connected = true;
      return;
    }
    if (reading.error === '' && !session.connected) return;
    session.hangUp();
    session.hangUp = () => undefined;
    session.timer = setTimeout(start, RETRY_MS[Math.min(session.attempt, RETRY_MS.length - 1)]);
    session.attempt += 1;
  };
  const start = () => {
    session.connected = false;
    session.hangUp = playWhep(video, endpoint, report);
  };
  start();
  return () => {
    clearTimeout(session.timer);
    session.hangUp();
  };
};

const useOnScreen = (target: { readonly current: Element | null }): boolean => {
  const [seen, setSeen] = useState(false);
  const [shown, setShown] = useState(!document.hidden);
  useEffect(() => {
    const element = target.current;
    if (element === null) return;
    const observer = new IntersectionObserver((entries) => setSeen(entries.some((entry) => entry.isIntersecting)));
    observer.observe(element);
    const visibility = () => setShown(!document.hidden);
    document.addEventListener('visibilitychange', visibility);
    return () => {
      observer.disconnect();
      document.removeEventListener('visibilitychange', visibility);
    };
  }, []);
  return seen && shown;
};

export const WhepPreview = ({ endpoint, watch, name }: { endpoint: string; watch: string; name: string }) => {
  const box = useRef<HTMLAnchorElement>(null);
  const video = useRef<HTMLVideoElement>(null);
  const [playing, setPlaying] = useState(false);
  const onScreen = useOnScreen(box);
  useEffect(() => {
    if (!onScreen || video.current === null) return;
    const stop = keepWhepPlaying(video.current, new URL(endpoint, window.location.href).href, setPlaying);
    return () => {
      stop();
      setPlaying(false);
    };
  }, [onScreen, endpoint]);
  return (
    <a ref={box} href={watch} target="_blank" rel="noopener" aria-label={`Open the ${name} camera`} className="relative block aspect-video overflow-hidden rounded-shape-md bg-black">
      <video ref={video} autoPlay playsInline muted className="size-full object-contain" />
      {playing ? null : (
        <span className="absolute inset-0 grid place-items-center bg-black/35">
          <span className="flex flex-col items-center gap-2">
            <CircularProgressIndicator size={32} color={Color.White} label="Connecting" />
            <Text typography="labelMedium" color={Color.White}>
              Connecting…
            </Text>
          </span>
        </span>
      )}
      <span className="absolute right-2 bottom-2 flex items-center gap-1 rounded-shape-xs bg-black/60 px-2 py-1">
        <MaterialIcon name="open_in_new" size={14} style={{ color: Color.White }} />
        <Text typography="labelSmall" color={Color.White}>
          Open camera
        </Text>
      </span>
    </a>
  );
};

export const PreviewPlaceholder = ({ text }: { text: string }) => (
  <div className="grid aspect-video place-items-center rounded-shape-md bg-black/80 px-6 text-center">
    <span className="flex flex-col items-center gap-2">
      <MaterialIcon name="videocam_off" size={28} style={{ color: md('onSurfaceVariant') }} />
      <Text typography="labelMedium" color={md('onSurfaceVariant')}>
        {text}
      </Text>
    </span>
  </div>
);

export const PartDisclosure = ({ part, id, opened, children }: { part: SetupPart; id: string; opened: string; children: ReactNode }) => {
  const head = useRef<HTMLElement>(null);
  useEffect(() => {
    if (opened === id) head.current?.scrollIntoView({ block: 'start' });
  }, []);
  return (
    <section ref={head} id={id} aria-label={part.title}>
      <DeviceDisclosure title={part.title} summary={part.summary} badge={part.badge} initiallyOpen={part.open || opened === id}>
        <div className="pb-2">{children}</div>
      </DeviceDisclosure>
    </section>
  );
};

export const Fold = ({ title, summary = '', open, onToggle, children }: { title: string; summary?: string; open: boolean; onToggle: (open: boolean) => void; children: ReactNode }) => (
  <div>
    <button type="button" aria-expanded={open} onClick={() => onToggle(!open)} className="flex min-h-12 w-full items-center gap-3 py-2 text-left">
      <span className="flex min-w-0 flex-1 flex-col">
        <Text typography="labelLarge" color={md('onSurfaceVariant')}>
          {title}
        </Text>
        {summary !== '' ? (
          <Text typography="bodySmall" color={md('onSurfaceVariant')}>
            {summary}
          </Text>
        ) : null}
      </span>
      <MaterialIcon name={open ? 'expand_less' : 'expand_more'} style={{ color: md('onSurfaceVariant') }} />
    </button>
    {open ? <div className="flex flex-col gap-3 pb-2">{children}</div> : null}
  </div>
);

export const useFold = (initially: boolean): { readonly open: boolean; readonly onToggle: (open: boolean) => void } => {
  const [open, onToggle] = useState(initially);
  return { open, onToggle };
};

export const LinkedNoticeView = ({ notice, onOpen }: { notice: LinkedNotice; onOpen: (href: string) => void }) =>
  notice.text === '' ? null : (
    <>
      <DeviceNotice notice={notice} />
      {notice.link !== null ? (
        <div className="px-list">
          <TextButton onClick={() => notice.link !== null && onOpen(notice.link.href)}>{`${notice.link.label} →`}</TextButton>
        </div>
      ) : null}
    </>
  );
