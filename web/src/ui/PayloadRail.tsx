import { useState } from 'react';
import { type Json, jsonObjects, optBoolean, optInt, optJSONArray, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { delay } from '@/kotlin/Std';
import { Surface, Text } from '@/m3';
import { Drawable } from '@/res/Icon';
import { Color, aircast, md, withAlpha } from '@/theme/Colors';
import { osdShadow } from '@/ui/Osd';

export const PAYLOAD_RAIL_VIEW = 'view.payloadRail';
const ARM = 'payloadRail.arm';
const SELECT = 'payloadRail.select';
const DROP = 'payloadRail.drop';
const TILE = 48;
const DISABLED_ALPHA = 0.38;
const MAX_BAYS = 6;
const REFUSAL_SHOWN_MS = 4000;

export type PayloadRailOwner = 'radio' | 'app';

export type PayloadRail = {
  readonly shown: boolean;
  readonly armed: boolean;
  readonly armedBy: PayloadRailOwner;
  readonly selected: number;
  readonly selectedBy: PayloadRailOwner;
  readonly bays: readonly { readonly bay: number; readonly loaded: boolean }[];
  readonly dropReady: boolean;
  readonly dropLabel: string;
  readonly reason: string | null;
};

const owner = (text: string): PayloadRailOwner => (text === 'radio' ? 'radio' : 'app');

export const payloadRailOf = (json: Json | null): PayloadRail | null =>
  json === null || !optBoolean(json, 'shown')
    ? null
    : {
        shown: true,
        armed: optBoolean(json, 'armed'),
        armedBy: owner(optText(json, 'armedBy')),
        selected: optInt(json, 'selected'),
        selectedBy: owner(optText(json, 'selectedBy')),
        bays: jsonObjects(optJSONArray(json, 'bays'))
          .slice(0, MAX_BAYS)
          .map((bay) => ({ bay: optInt(bay, 'bay'), loaded: optBoolean(bay, 'loaded') })),
        dropReady: optBoolean(json, 'dropReady'),
        dropLabel: optText(json, 'dropLabel'),
        reason: optText(json, 'reason') || null,
      };

export const payloadNote = (rail: PayloadRail, refusal: string | null): string | null => refusal ?? (rail.dropReady || !rail.armed ? null : rail.reason);

export const lockLabel = (rail: PayloadRail): string => (rail.armed ? `Payload armed${rail.armedBy === 'radio' ? ' by radio' : ', tap to make safe'}` : `Payload safe${rail.armedBy === 'radio' ? ' by radio' : ', tap to arm'}`);

export const bayTileLabel = (rail: PayloadRail, bay: number, loaded: boolean): string => `Bay ${bay}, ${loaded ? 'loaded' : 'empty'}${rail.selected === bay ? ', selected' : ''}`;

const Tile = ({ label, enabled, ring, fill, onClick, children }: { label: string; enabled: boolean; ring: boolean; fill: string; onClick: () => void; children: React.ReactNode }) => (
  <Surface
    shape="full"
    color={fill}
    contentColor={aircast('outdoorForeground')}
    border={ring ? `3px solid ${aircast('outdoorAccent')}` : `1px solid ${withAlpha(aircast('outdoorForeground'), 0.5)}`}
    className="flex shrink-0 cursor-pointer items-center justify-center outline-none focus-visible:ring-2 focus-visible:ring-primary"
    style={{ width: TILE, height: TILE, opacity: enabled ? 1 : DISABLED_ALPHA }}
    onClick={onClick}
    label={label}
  >
    {children}
  </Surface>
);

const Labelled = ({ text, tone, children }: { text: string; tone: string; children: React.ReactNode }) => (
  <div className="flex flex-col items-center gap-1">
    {children}
    <Text typography="labelSmall" color={tone} maxLines={1} className={osdShadow()}>
      {text}
    </Text>
  </div>
);

export const PayloadRailStrip = () => {
  const rail = payloadRailOf(useQgcPath(PAYLOAD_RAIL_VIEW) as Json | null);
  const [refusal, setRefusal] = useState<string | null>(null);
  useLaunchedEffect([refusal], async (scope) => {
    if (refusal === null) return;
    await delay(REFUSAL_SHOWN_MS);
    if (scope.isActive()) setRefusal(null);
  });
  if (rail === null) return null;
  const run = (path: string, ...args: unknown[]) => void Qgc.refusalOf(path, ...args).then(setRefusal);
  const note = payloadNote(rail, refusal);
  const dim = withAlpha(aircast('outdoorBackground'), 0.4);
  return (
    <div role="group" aria-label="Payload" className="flex flex-col items-center gap-1">
      <div className="flex items-start gap-2">
        <Labelled text={rail.armed ? 'Armed' : 'Safe'} tone={rail.armed ? md('error') : aircast('outdoorForeground')}>
          <Tile label={lockLabel(rail)} enabled={rail.armedBy === 'app'} ring={false} fill={rail.armed ? md('error') : dim} onClick={() => run(ARM, !rail.armed)}>
            <Drawable name="ic_shield" size={24} className={osdShadow()} />
          </Tile>
        </Labelled>
        {rail.bays.map(({ bay, loaded }) => (
          <Tile key={bay} label={bayTileLabel(rail, bay, loaded)} enabled={loaded} ring={rail.selected === bay} fill={loaded ? withAlpha(aircast('outdoorBackground'), 0.6) : 'transparent'} onClick={() => run(SELECT, bay)}>
            <Text typography="titleLarge" color={aircast('outdoorForeground')} className={osdShadow()}>
              {bay}
            </Text>
          </Tile>
        ))}
        <Labelled text={rail.dropLabel} tone={aircast('outdoorForeground')}>
          <Tile label={rail.dropLabel} enabled={rail.dropReady} ring={false} fill={rail.dropReady ? md('error') : dim} onClick={() => run(DROP)}>
            <Drawable name="ic_drop" size={28} className={osdShadow()} />
          </Tile>
        </Labelled>
      </div>
      {note === null ? null : (
        <Text typography="bodyMedium" color={aircast('outdoorForeground')} maxLines={2} className="rounded-full px-3 py-1" style={{ background: withAlpha(Color.Black, 0.65) }}>
          {note}
        </Text>
      )}
    </div>
  );
};
