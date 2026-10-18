import { type PointerEvent, type ReactNode, useContext, useEffect, useRef, useState } from 'react';
import { copyToClipboard } from '@/kotlin/Platform';
import { mutableStateOf, useValue } from '@/kotlin/State';
import { FilterChip, Text, TextButton } from '@/m3';
import { MaterialIcon } from '@/res/Icon';
import { aircast, md } from '@/theme/Colors';
import { ConfirmDialog, DeviceBadgeText, DeviceNotice, DeviceText, levelColor, qrSource, useCopy } from '@/ui/DeviceKit';
import {
  type FlightEntry,
  type FlightLine,
  type FlightPart,
  type FlightTrace,
  IDLE_RENDER,
  MIN_BRUSH,
  PLOT_BOX,
  RENDER_POLL_MS,
  type RenderJob,
  type SortieShare,
  type SortieVideo,
  type TraceLane,
  type Zoom,
  ceilingOf,
  clockAt,
  composeZoom,
  flightLink,
  gridLines,
  hasLines,
  laneKeys,
  markerLines,
  parseFlights,
  readRender,
  readoutAt,
  rememberLanes,
  renderExpiry,
  renderPercent,
  renderRunning,
  segments,
  spanLabel,
  startRender,
  storedLanes,
  ticks,
  toggled,
  zoomSlice,
} from '@/ui/DeviceFlights';
import type { DeviceScreenProps } from '@/ui/DeviceScreens';
import { DeviceLevel } from '@/ui/DeviceStatus';
import { deviceUrl, useDeviceView } from '@/ui/DeviceView';
import { FootNote } from '@/ui/SetupRow';
import { LocalChangeNotice } from '@/ui/ValueRow';

const laneChoice = mutableStateOf<readonly string[] | null>(null);

const muted = md('onSurfaceVariant');

export const FlightsScreen = ({ query }: DeviceScreenProps) => {
  const view = useDeviceView('flights', parseFlights);
  if (view === null) return <FootNote text="Waiting for the device to answer…" />;
  return (
    <>
      <DeviceText text="One entry per run of this device. None of it reaches you in the air — the ground station only gets the few lines worth interrupting a flight for." />
      {view.flights.length === 0 ? (
        <DeviceNotice notice={{ level: DeviceLevel.Neutral, text: 'No flights recorded yet. This device starts one every time it powers up.' }} />
      ) : (
        view.flights.map((flight) => <FlightRow key={flight.id} flight={flight} linked={query.get('flight') === flight.id} />)
      )}
    </>
  );
};

const FlightRow = ({ flight, linked }: { flight: FlightEntry; linked: boolean }) => {
  const [open, setOpen] = useState(flight.open || linked);
  const [active, setActive] = useState<number | null>(null);
  const anchor = useRef<HTMLElement>(null);
  useEffect(() => (linked ? anchor.current?.scrollIntoView({ block: 'start' }) : undefined), []);
  return (
    <section ref={anchor} aria-label={flight.title} className="border-b border-outline-variant">
      <button
        type="button"
        aria-expanded={open}
        onClick={() => setOpen(!open)}
        className="flex min-h-16 w-full items-center gap-3 px-list py-3 text-left hover:bg-[color-mix(in_srgb,var(--md-on-surface)_8%,transparent)]"
      >
        <span className="flex min-w-0 flex-1 flex-col">
          <Text typography="bodyLarge">{flight.title}</Text>
          {flight.summary !== '' ? (
            <Text typography="bodyMedium" color={muted}>
              {flight.summary}
            </Text>
          ) : null}
        </span>
        <DeviceBadgeText badge={flight.badge} />
        <MaterialIcon name={open ? 'expand_less' : 'expand_more'} style={{ color: muted }} />
      </button>
      {open ? <FlightBody flight={flight} active={active} onActive={setActive} /> : null}
    </section>
  );
};

const FlightBody = ({ flight, active, onActive }: { flight: FlightEntry; active: number | null; onActive: (index: number | null) => void }) => {
  const [alertsOnly, setAlertsOnly] = useState(false);
  return (
    <div className="pb-4">
      {hasLines(flight) ? (
        <>
          <div role="group" aria-label="Lines" className="flex flex-wrap items-center gap-2 px-list py-2">
            <FilterChip selected={!alertsOnly} onClick={() => setAlertsOnly(false)} label="Everything" />
            <FilterChip selected={alertsOnly} onClick={() => setAlertsOnly(true)} label="Alerts only" />
          </div>
          {flight.sections.map((part, index) => (
            <FlightPartRows key={`${index}-${part.title}`} part={part} alertsOnly={alertsOnly} active={active} onActive={onActive} />
          ))}
        </>
      ) : (
        <DeviceText text="Nothing worth recording — the device had no faults to report." />
      )}
      {flight.trace !== null ? <FlightLanes trace={flight.trace} active={active} /> : null}
      {flight.dropped !== '' ? <DeviceNotice notice={{ level: DeviceLevel.Warn, text: flight.dropped }} /> : null}
      <FlightFiles flight={flight} />
    </div>
  );
};

const FlightPartRows = ({ part, alertsOnly, active, onActive }: { part: FlightPart; alertsOnly: boolean; active: number | null; onActive: (index: number | null) => void }) => (
  <div role="group" aria-label={part.title === '' ? 'Lines' : part.title} className="px-list">
    {part.title !== '' ? (
      <div className="flex flex-wrap items-center gap-x-3 gap-y-2 pt-4 pb-1">
        <Text typography="labelMedium" color={muted} className="tracking-wide uppercase">
          {part.title}
        </Text>
        {part.span !== '' ? (
          <Text typography="labelSmall" monospace color={muted}>
            {part.span}
          </Text>
        ) : null}
      </div>
    ) : null}
    {part.videos.length > 0 ? (
      <div className="flex flex-wrap items-center gap-2 py-1">
        {part.videos.map((video) => (
          <SortieVideoLinks key={video.camera} video={video} />
        ))}
      </div>
    ) : null}
    <ol className="flex flex-col">
      {part.lines
        .filter((line) => !alertsOnly || line.alert)
        .map((line) => (
          <LineRow key={line.index} line={line} active={active === line.index} onActive={onActive} />
        ))}
    </ol>
  </div>
);

const lineColor = (line: FlightLine): string => (line.heldBack ? muted : line.level === DeviceLevel.Neutral ? md('onSurface') : levelColor(line.level));

const LineRow = ({ line, active, onActive }: { line: FlightLine; active: boolean; onActive: (index: number | null) => void }) => (
  <li
    className={`-mx-2 flex flex-wrap items-baseline gap-x-3 gap-y-0.5 rounded-shape-xs px-2 py-1 ${active ? 'bg-surface-container-highest' : ''}`}
    onPointerEnter={() => onActive(line.index)}
    onPointerLeave={() => onActive(null)}
    onClick={() => onActive(line.index)}
  >
    <Text typography="bodySmall" monospace color={muted} className="w-16 shrink-0">
      {line.clock}
    </Text>
    <Text typography="bodyMedium" color={lineColor(line)} className="min-w-0 flex-1">
      {line.text}
    </Text>
    {line.repeats !== '' ? (
      <Text typography="labelSmall" monospace color={muted}>
        {line.repeats}
      </Text>
    ) : null}
    {line.heldBack ? (
      <span className="rounded-full border border-outline-variant px-2 py-0.5">
        <Text typography="labelSmall" color={muted}>
          pilot not told
        </Text>
      </span>
    ) : null}
  </li>
);

const linkLook = 'inline-flex h-10 select-none items-center whitespace-nowrap justify-center gap-2 rounded-full px-4 text-label-large text-primary no-underline hover:bg-[color-mix(in_srgb,var(--md-primary)_8%,transparent)]';

const LinkButton = ({ href, outlined = false, title, children }: { href: string; outlined?: boolean; title?: string; children: ReactNode }) => (
  <a href={deviceUrl(href)} download title={title} className={`${linkLook} ${outlined ? 'border border-outline' : ''}`}>
    {children}
  </a>
);

const SortieVideoLinks = ({ video }: { video: SortieVideo }) => (
  <>
    <LinkButton href={video.url} outlined title="The sortie's video with its telemetry as a subtitle track — turn subtitles on in your player to see it">
      <MaterialIcon name="download" size={18} />
      {video.label}
    </LinkButton>
    {video.share !== null ? <SortieShareRow share={video.share} /> : null}
  </>
);

const SortieShareRow = ({ share }: { share: SortieShare }) => {
  const copy = useCopy();
  const [job, setJob] = useState<RenderJob>(IDLE_RENDER);
  const [asking, setAsking] = useState(false);
  useEffect(() => {
    if (!renderRunning(job) || job.id === '') return undefined;
    const abort = new AbortController();
    const timer = setTimeout(() => void readRender(job.id, abort.signal).then((next) => (abort.signal.aborted ? undefined : setJob(next))), RENDER_POLL_MS);
    return () => {
      clearTimeout(timer);
      abort.abort();
    };
  }, [job]);
  return (
    <div className="flex flex-wrap items-center gap-2">
      {job.state === '' || job.state === 'failed' ? (
        <button
          type="button"
          title="Burns the telemetry into the picture in Aircast cloud, so it survives Telegram, WhatsApp and any player"
          onClick={() => setAsking(true)}
          className={`${linkLook} border border-outline`}
        >
          {share.label}
        </button>
      ) : null}
      {job.state === 'uploading' ? (
        <Text typography="bodySmall" color={muted}>
          {`Sending to Aircast cloud… ${renderPercent(job)}%`}
        </Text>
      ) : null}
      {job.state === 'rendering' ? (
        <Text typography="bodySmall" color={muted}>
          Burning the telemetry in… this takes a few minutes; you can leave this page.
        </Text>
      ) : null}
      {job.state === 'failed' ? (
        <div role="alert">
          <Text typography="bodySmall" color={md('error')}>
            {job.error}
          </Text>
        </div>
      ) : null}
      {job.state === 'done' ? (
        <div role="group" aria-label="Share version" className="flex basis-full flex-wrap items-center gap-3">
          <img src={qrSource(job.url)} alt="QR code for the share link" className="size-24 rounded-shape-xs bg-white p-1" />
          <div className="flex flex-col items-start gap-1">
            <a href={job.url} target="_blank" rel="noopener noreferrer" className="text-label-large text-primary underline">
              Open the share version
            </a>
            <TextButton onClick={() => copy(job.url)}>Copy link</TextButton>
            <Text typography="labelSmall" color={muted}>
              {renderExpiry(job)}
            </Text>
          </div>
        </div>
      ) : null}
      {asking ? <ConfirmDialog title={share.label} text={share.confirm} confirm="Send" onConfirm={() => void startRender(share).then(setJob)} onDismiss={() => setAsking(false)} /> : null}
    </div>
  );
};

type Brush = { readonly from: number; readonly to: number };

type Plot = {
  readonly zoom: Zoom | null;
  readonly cursor: number | null;
  readonly brush: Brush | null;
  readonly active: number | null;
  readonly down: (event: PointerEvent<HTMLElement>) => void;
  readonly move: (event: PointerEvent<HTMLElement>) => void;
  readonly up: (event: PointerEvent<HTMLElement>) => void;
  readonly leave: (event: PointerEvent<HTMLElement>) => void;
};

const fractionIn = (event: PointerEvent<HTMLElement>): number => {
  const box = event.currentTarget.getBoundingClientRect();
  return box.width > 0 ? Math.max(0, Math.min(1, (event.clientX - box.left) / box.width)) : 0;
};

const FlightLanes = ({ trace, active }: { trace: FlightTrace; active: number | null }) => {
  const chosen = useValue(laneChoice);
  const keys = laneKeys(
    chosen ?? storedLanes(),
    trace.lanes.filter((lane) => lane.default).map((lane) => lane.key),
  );
  const [zoom, setZoom] = useState<Zoom | null>(null);
  const [cursor, setCursor] = useState<number | null>(null);
  const [brush, setBrush] = useState<Brush | null>(null);
  const toggle = (key: string) => {
    const next = toggled(keys, key);
    rememberLanes(next);
    laneChoice.value = next;
  };
  const plot: Plot = {
    zoom,
    cursor,
    brush,
    active,
    down: (event) => {
      if (event.button > 0) return;
      const at = fractionIn(event);
      event.currentTarget.setPointerCapture(event.pointerId);
      event.preventDefault();
      setBrush({ from: at, to: at });
      setCursor(at);
    },
    move: (event) => (brush !== null ? setBrush({ ...brush, to: fractionIn(event) }) : setCursor(fractionIn(event))),
    up: (event) => {
      if (brush === null) return;
      const at = fractionIn(event);
      setBrush(null);
      if (Math.abs(at - brush.from) >= MIN_BRUSH) setZoom(composeZoom(zoom, brush.from, at));
    },
    leave: (event) => (event.pointerType === 'mouse' && brush === null ? setCursor(null) : undefined),
  };
  const shown = trace.lanes.filter((lane) => keys.includes(lane.key));
  return (
    <div className="mt-2 border-t border-outline-variant pt-3">
      <div role="group" aria-label="Lanes" className="flex flex-wrap items-center gap-2 px-list">
        {trace.lanes.map((lane) => (
          <FilterChip
            key={lane.key}
            selected={keys.includes(lane.key)}
            onClick={() => toggle(lane.key)}
            label={
              <span title={lane.label} className="flex items-baseline gap-1.5">
                {lane.chip}
                {lane.peak !== '' ? (
                  <Text typography="labelSmall" monospace color={aircast('warning')}>
                    {lane.peak}
                  </Text>
                ) : null}
              </span>
            }
          />
        ))}
      </div>
      {trace.spans.length > 0 || zoom !== null ? (
        <div role="group" aria-label="Zoom" className="flex flex-wrap items-center gap-2 px-list pt-2">
          {trace.spans.map((span) => (
            <FilterChip key={span.label} selected={zoom !== null && zoom[0] === span.from && zoom[1] === span.to} onClick={() => setZoom([span.from, span.to])} label={span.label} />
          ))}
          {zoom !== null ? <TextButton onClick={() => setZoom(null)}>Whole flight</TextButton> : null}
        </div>
      ) : null}
      {shown.map((lane, index) => (
        <LaneChart key={lane.key} lane={lane} trace={trace} plot={plot} last={index === shown.length - 1} />
      ))}
      <div className="px-list pt-3">
        <Text typography="bodySmall" color={muted}>
          Pick the lanes you want above. One cursor across them: point at any lane to read every sample at that moment. Drag across a lane to zoom them all, or jump to one sortie with the chips above. Dashed lines are the moments listed above, in their
          colour; point at a line to pick out its mark.
        </Text>
      </div>
    </div>
  );
};

const markerColor = (level: number): string => (level === DeviceLevel.Neutral ? muted : levelColor(level));

const percent = (fraction: number): string => `${(fraction * 100).toFixed(1)}%`;

const readoutAlign = (x: number): string => (x < 15 ? '' : x > 85 ? '-translate-x-full' : '-translate-x-1/2');

const LaneChart = ({ lane, trace, plot, last }: { lane: TraceLane; trace: FlightTrace; plot: Plot; last: boolean }) => {
  const slice = zoomSlice(lane.values.length, plot.zoom);
  const shown = lane.values.slice(slice.from, slice.to + 1);
  const start = trace.startUnix + slice.from * trace.step;
  const grid = gridLines(ceilingOf(shown, lane.max), lane.unit);
  const markers = markerLines(trace.markers, trace.step, shown.length, slice.from * trace.step);
  const readout = plot.cursor === null ? null : readoutAt(shown, lane.notes.slice(slice.from, slice.to + 1), plot.cursor, lane.unit);
  return (
    <div role="group" aria-label={lane.label} className="px-list pt-4">
      <Text typography="labelSmall" color={muted} className="tracking-wide uppercase">
        {lane.label}
      </Text>
      <div className="relative mt-1 h-28 touch-none select-none" onPointerDown={plot.down} onPointerMove={plot.move} onPointerUp={plot.up} onPointerLeave={plot.leave}>
        <svg viewBox={`0 0 100 ${PLOT_BOX}`} preserveAspectRatio="none" className="absolute inset-0 size-full" role="img" aria-label={`${lane.label} over the flight`}>
          {grid.map((line) => (
            <line key={line.y} x1="0" x2="100" y1={line.y} y2={line.y} stroke={md('onSurface')} strokeOpacity={0.12} strokeWidth="1" vectorEffect="non-scaling-stroke" />
          ))}
          {markers.map((marker) => (
            <line
              key={marker.index}
              x1={marker.x}
              x2={marker.x}
              y1="0"
              y2={PLOT_BOX}
              stroke={markerColor(marker.level)}
              strokeWidth="1"
              vectorEffect="non-scaling-stroke"
              strokeDasharray={marker.index === plot.active ? undefined : '2 2'}
              strokeOpacity={marker.index === plot.active ? 1 : 0.7}
              aria-current={marker.index === plot.active ? 'true' : undefined}
              className={marker.level === DeviceLevel.Neutral && marker.index !== plot.active ? 'max-sm:hidden' : ''}
            >
              <title>{marker.label}</title>
            </line>
          ))}
          {segments(shown, lane.max).map((points) => (
            <polyline key={points} points={points} fill="none" stroke={md('primary')} strokeWidth="1.5" vectorEffect="non-scaling-stroke" />
          ))}
        </svg>
        <div aria-label={`${lane.label} scale`} className="pointer-events-none absolute inset-y-0 right-0">
          {grid.map((line) => (
            <span key={line.y} className="absolute right-0 -translate-y-1/2 rounded-shape-xs bg-surface/80 px-1 whitespace-nowrap" style={{ top: percent(line.y / PLOT_BOX) }}>
              <Text typography="labelSmall" monospace color={muted}>
                {line.label}
              </Text>
            </span>
          ))}
        </div>
        {plot.brush !== null ? (
          <div className="pointer-events-none absolute inset-y-0 bg-primary/15" style={{ left: percent(Math.min(plot.brush.from, plot.brush.to)), width: percent(Math.abs(plot.brush.to - plot.brush.from)) }} />
        ) : null}
        {readout !== null ? (
          <>
            <div className="pointer-events-none absolute inset-y-0 w-px bg-on-surface/40" style={{ left: `${readout.x}%` }} />
            <div
              role="tooltip"
              className={`pointer-events-none absolute top-0 z-10 rounded-shape-xs border border-outline-variant bg-surface-container-highest px-2 py-1 whitespace-nowrap shadow-sm ${readoutAlign(readout.x)}`}
              style={{ left: `${readout.x}%` }}
            >
              <Text typography="labelSmall" monospace>
                {`${clockAt(start + readout.index * trace.step, trace.tzOffset, true)} · ${readout.text}`}
              </Text>
            </div>
          </>
        ) : null}
      </div>
      {last ? (
        <>
          <div className="relative h-5" aria-label="Time">
            {ticks(start, trace.step, shown.length, (unix) => clockAt(unix, trace.tzOffset, false)).map((tick, index, all) => (
              <span key={tick.x} className={`absolute top-0.5 ${index === 0 ? '' : index === all.length - 1 ? '-translate-x-full' : '-translate-x-1/2'}`} style={{ left: `${tick.x}%` }}>
                <Text typography="labelSmall" monospace color={muted}>
                  {tick.label}
                </Text>
              </span>
            ))}
          </div>
          <Text typography="labelSmall" color={muted}>
            {shown.length > 1 ? `${spanLabel(trace.step * (shown.length - 1))} of flight` : ''}
          </Text>
        </>
      ) : null}
    </div>
  );
};

const FlightFiles = ({ flight }: { flight: FlightEntry }) => {
  const notice = useContext(LocalChangeNotice);
  const copyLink = () =>
    void copyToClipboard(flightLink(location.origin + location.pathname, flight.id)).then(
      () => notice.show('Link copied — it opens straight on this flight.', null),
      () => notice.show('Could not copy — select the text instead', null),
    );
  return (
    <div className="mt-3 flex flex-col gap-2 border-t border-outline-variant px-list pt-3">
      <div className="flex flex-wrap items-center gap-2">
        <LinkButton href={flight.csv} outlined>
          Samples as CSV
        </LinkButton>
        <LinkButton href={flight.json}>Everything as JSON</LinkButton>
        <TextButton onClick={copyLink}>
          <MaterialIcon name="link" size={18} />
          Copy link to this flight
        </TextButton>
      </div>
      <Text typography="bodySmall" color={muted}>
        The CSV is one row per sample — every lane above and more besides: signal and radio, the aircraft's height, distance, battery and satellites, video per camera, viewers, what was dropped, and the box's own load and memory. The JSON adds every line
        above.
      </Text>
    </div>
  );
};
