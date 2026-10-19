import { type PointerEvent, useContext, useEffect, useRef, useState } from 'react';
import type { Json } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { useProduceState } from '@/kotlin/Compose';
import { copyToClipboard } from '@/kotlin/Platform';
import { mutableStateOf, useValue } from '@/kotlin/State';
import { FilterChip, OutlinedButton, Text, TextButton } from '@/m3';
import { MaterialIcon } from '@/res/Icon';
import { aircast, md } from '@/theme/Colors';
import { ConfirmDialog, DeviceBadgeText, DeviceNotice, DeviceText, LinkButton, QrImage, levelColor, useCopy } from '@/ui/DeviceKit';
import {
  type ChartLane,
  type FlightChart,
  type FlightEntry,
  type FlightLine,
  type FlightPart,
  MIN_BRUSH,
  type SortieShare,
  type SortieVideo,
  type Zoom,
  composeZoom,
  flightChart,
  flightLink,
  hasLines,
  laneKeys,
  parseFlights,
  parseRender,
  rememberLanes,
  renderPath,
  requestRender,
  sampleAt,
  storedLanes,
  toggled,
} from '@/ui/DeviceFlights';
import type { DeviceScreenProps } from '@/ui/DeviceScreens';
import { DeviceLevel } from '@/ui/DeviceLevel';
import { useDesktopLayout } from '@/ui/DesktopLayout';
import { deviceUrl, useDeviceView } from '@/ui/DeviceView';
import { FootNote } from '@/ui/SetupRow';
import { LocalChangeNotice } from '@/ui/ValueRow';

const laneChoice = mutableStateOf<readonly string[] | null>(null);

const muted = md('onSurfaceVariant');

export const FlightsScreen = ({ query }: DeviceScreenProps) => {
  const view = useDeviceView('flights', parseFlights);
  const desktop = useDesktopLayout();
  if (view === null) return <FootNote text="Waiting for the device to answer…" />;
  const linked = query.get('flight');
  return (
    <>
      <DeviceText text="One entry per run of this device. None of it reaches you in the air — the ground station only gets the few lines worth interrupting a flight for." />
      {view.flights.length === 0 ? (
        <DeviceNotice notice={{ level: DeviceLevel.Neutral, text: 'No flights recorded yet. This device starts one every time it powers up.' }} />
      ) : desktop ? (
        <FlightsSplit flights={view.flights} linked={linked} />
      ) : (
        view.flights.map((flight) => <FlightRow key={flight.id} flight={flight} linked={linked === flight.id} />)
      )}
    </>
  );
};

const FlightHeader = ({ flight, open, icon, onClick }: { flight: FlightEntry; open: boolean; icon: string; onClick: () => void }) => (
  <button
    type="button"
    aria-expanded={open}
    onClick={onClick}
    className={`flex min-h-16 w-full items-center gap-3 px-list py-3 text-left hover:bg-[color-mix(in_srgb,var(--md-on-surface)_8%,transparent)] ${open && icon === 'chevron_right' ? 'bg-surface-container-high' : ''}`}
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
    <MaterialIcon name={icon} style={{ color: muted }} />
  </button>
);

const FlightRow = ({ flight, linked }: { flight: FlightEntry; linked: boolean }) => {
  const [open, setOpen] = useState(flight.open || linked);
  const [active, setActive] = useState<number | null>(null);
  const anchor = useRef<HTMLElement>(null);
  useEffect(() => void (linked ? anchor.current?.scrollIntoView({ block: 'start' }) : undefined), []);
  return (
    <section ref={anchor} aria-label={flight.title} className="border-b border-outline-variant">
      <FlightHeader flight={flight} open={open} icon={open ? 'expand_less' : 'expand_more'} onClick={() => setOpen(!open)} />
      {open ? <FlightBody flight={flight} active={active} onActive={setActive} /> : null}
    </section>
  );
};

const FlightsSplit = ({ flights, linked }: { flights: readonly FlightEntry[]; linked: string | null }) => {
  const [chosen, setChosen] = useState(linked ?? flights.find((flight) => flight.open)?.id ?? null);
  const [active, setActive] = useState<number | null>(null);
  const shown = flights.find((flight) => flight.id === chosen);
  const pick = (id: string) => {
    setChosen(id);
    setActive(null);
  };
  return (
    <div className="grid grid-cols-[minmax(0,2fr)_minmax(0,3fr)] items-start gap-x-6">
      <div className="flex min-w-0 flex-col">
        {flights.map((flight) => (
          <div key={flight.id} className="border-b border-outline-variant">
            <FlightHeader flight={flight} open={flight.id === shown?.id} icon="chevron_right" onClick={() => pick(flight.id)} />
          </div>
        ))}
      </div>
      {shown !== undefined ? (
        <section aria-label={shown.title} className="min-w-0 rounded-shape-md bg-surface-container-low pt-2">
          <FlightBody key={shown.id} flight={shown} active={active} onActive={setActive} />
        </section>
      ) : (
        <div className="grid min-h-40 place-items-center rounded-shape-md bg-surface-container-low px-list">
          <Text typography="bodyMedium" color={muted}>
            Pick a run to see what happened.
          </Text>
        </div>
      )}
    </div>
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

const SortieVideoLinks = ({ video }: { video: SortieVideo }) => (
  <>
    <LinkButton href={deviceUrl(video.url)} look="outlined" download title="The sortie's video with its telemetry as a subtitle track — turn subtitles on in your player to see it">
      <MaterialIcon name="download" size={18} />
      {video.label}
    </LinkButton>
    {video.share !== null ? <SortieShareRow share={video.share} /> : null}
  </>
);

const SortieShareRow = ({ share }: { share: SortieShare }) => {
  const copy = useCopy();
  const render = parseRender(useQgcPath(renderPath(share)));
  const [asking, setAsking] = useState(false);
  return (
    <div className="flex flex-wrap items-center gap-2">
      {render.canStart ? (
        <OutlinedButton title="Burns the telemetry into the picture in Aircast cloud, so it survives Telegram, WhatsApp and any player" onClick={() => setAsking(true)}>
          {share.label}
        </OutlinedButton>
      ) : null}
      {render.progress !== '' ? (
        <Text typography="bodySmall" color={muted}>
          {render.progress}
        </Text>
      ) : null}
      {render.error !== '' ? (
        <div role="alert">
          <Text typography="bodySmall" color={md('error')}>
            {render.error}
          </Text>
        </div>
      ) : null}
      {render.url !== '' ? (
        <div role="group" aria-label="Share version" className="flex basis-full flex-wrap items-center gap-3">
          <QrImage text={render.url} alt="QR code for the share link" className="size-24 rounded-shape-xs bg-white p-1" />
          <div className="flex flex-col items-start gap-1">
            <a href={render.url} target="_blank" rel="noopener noreferrer" className="text-label-large text-primary underline">
              Open the share version
            </a>
            <TextButton onClick={() => copy(render.url)}>Copy link</TextButton>
            <Text typography="labelSmall" color={muted}>
              {render.expiry}
            </Text>
          </div>
        </div>
      ) : null}
      {asking ? <ConfirmDialog title={share.label} text={share.confirm} confirm="Send" onConfirm={() => void requestRender(share)} onDismiss={() => setAsking(false)} /> : null}
    </div>
  );
};

type Brush = { readonly from: number; readonly to: number };

type Plot = {
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

const FlightLanes = ({ trace, active }: { trace: Json; active: number | null }) => {
  const chosen = useValue(laneChoice);
  const [zoom, setZoom] = useState<Zoom | null>(null);
  const [cursor, setCursor] = useState<number | null>(null);
  const [brush, setBrush] = useState<Brush | null>(null);
  const chart = useProduceState<FlightChart | null>(null, [trace, zoom], async (set) => set(await flightChart(trace, zoom)));
  if (chart === null) return null;
  const keys = laneKeys(
    chosen ?? storedLanes(),
    chart.lanes.filter((lane) => lane.default).map((lane) => lane.key),
  );
  const toggle = (key: string) => {
    const next = toggled(keys, key);
    rememberLanes(next);
    laneChoice.value = next;
  };
  const plot: Plot = {
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
  const shown = chart.lanes.filter((lane) => keys.includes(lane.key));
  return (
    <div className="mt-2 border-t border-outline-variant pt-3">
      <div role="group" aria-label="Lanes" className="flex flex-wrap items-center gap-2 px-list">
        {chart.lanes.map((lane) => (
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
      {chart.spans.length > 0 || zoom !== null ? (
        <div role="group" aria-label="Zoom" className="flex flex-wrap items-center gap-2 px-list pt-2">
          {chart.spans.map((span) => (
            <FilterChip key={span.label} selected={zoom !== null && zoom[0] === span.from && zoom[1] === span.to} onClick={() => setZoom([span.from, span.to])} label={span.label} />
          ))}
          {zoom !== null ? <TextButton onClick={() => setZoom(null)}>Whole flight</TextButton> : null}
        </div>
      ) : null}
      {shown.map((lane, index) => (
        <LaneChart key={lane.key} lane={lane} plot={plot} last={index === shown.length - 1} />
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

const LaneChart = ({ lane, plot, last }: { lane: ChartLane; plot: Plot; last: boolean }) => {
  const sample = plot.cursor === null ? null : sampleAt(lane.readings.length, plot.cursor);
  return (
    <div role="group" aria-label={lane.label} className="px-list pt-4">
      <Text typography="labelSmall" color={muted} className="tracking-wide uppercase">
        {lane.label}
      </Text>
      <div className="relative mt-1 h-28 touch-none select-none" onPointerDown={plot.down} onPointerMove={plot.move} onPointerUp={plot.up} onPointerLeave={plot.leave}>
        <svg viewBox="0 0 100 100" preserveAspectRatio="none" className="absolute inset-0 size-full" role="img" aria-label={`${lane.label} over the flight`}>
          {lane.grid.map((line) => (
            <line key={line.y} x1="0" x2="100" y1={line.y} y2={line.y} stroke={md('onSurface')} strokeOpacity={0.12} strokeWidth="1" vectorEffect="non-scaling-stroke" />
          ))}
          {lane.markers.map((marker) => (
            <line
              key={marker.index}
              x1={marker.x}
              x2={marker.x}
              y1="0"
              y2="100"
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
          {lane.lines.map((points) => (
            <polyline key={points} points={points} fill="none" stroke={md('primary')} strokeWidth="1.5" vectorEffect="non-scaling-stroke" />
          ))}
        </svg>
        <div aria-label={`${lane.label} scale`} className="pointer-events-none absolute inset-y-0 right-0">
          {lane.grid.map((line) => (
            <span key={line.y} className="absolute right-0 -translate-y-1/2 rounded-shape-xs bg-surface/80 px-1 whitespace-nowrap" style={{ top: `${line.y}%` }}>
              <Text typography="labelSmall" monospace color={muted}>
                {line.label}
              </Text>
            </span>
          ))}
        </div>
        {plot.brush !== null ? (
          <div className="pointer-events-none absolute inset-y-0 bg-primary/15" style={{ left: percent(Math.min(plot.brush.from, plot.brush.to)), width: percent(Math.abs(plot.brush.to - plot.brush.from)) }} />
        ) : null}
        {sample !== null ? (
          <>
            <div className="pointer-events-none absolute inset-y-0 w-px bg-on-surface/40" style={{ left: `${sample.x}%` }} />
            <div
              role="tooltip"
              className={`pointer-events-none absolute top-0 z-10 rounded-shape-xs border border-outline-variant bg-surface-container-highest px-2 py-1 whitespace-nowrap shadow-sm ${readoutAlign(sample.x)}`}
              style={{ left: `${sample.x}%` }}
            >
              <Text typography="labelSmall" monospace>
                {lane.readings[sample.index]}
              </Text>
            </div>
          </>
        ) : null}
      </div>
      {last ? (
        <>
          <div className="relative h-5" aria-label="Time">
            {lane.ticks.map((tick, index, all) => (
              <span key={tick.x} className={`absolute top-0.5 ${index === 0 ? '' : index === all.length - 1 ? '-translate-x-full' : '-translate-x-1/2'}`} style={{ left: `${tick.x}%` }}>
                <Text typography="labelSmall" monospace color={muted}>
                  {tick.label}
                </Text>
              </span>
            ))}
          </div>
          <Text typography="labelSmall" color={muted}>
            {lane.span}
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
        <LinkButton href={deviceUrl(flight.csv)} look="outlined" download>
          Samples as CSV
        </LinkButton>
        <LinkButton href={deviceUrl(flight.json)} look="text" download>Everything as JSON</LinkButton>
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
