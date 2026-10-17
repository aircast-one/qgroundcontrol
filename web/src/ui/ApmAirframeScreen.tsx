import { useState } from 'react';
import { type Json, isNull, jsonObjects, optBoolean, optInt, optJSONArray, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { delay } from '@/kotlin/Std';
import { FilterChip, Surface, Text } from '@/m3';
import { md } from '@/theme/Colors';
import { airframeImageAsset } from '@/ui/Px4AirframeScreen';

export const APM_AIRFRAME_SCREEN = 'apmAirframe';
export const APM_AIRFRAME_VIEW = 'view.apmAirframe';
export const APM_AIRFRAME_PICK_CLASS = 'apmAirframe.pickClass';
export const APM_AIRFRAME_PICK_TYPE = 'apmAirframe.pickType';
const APM_AIRFRAME_POLL_MS = 1000;

export type FrameTypeChoice = { name: string; value: number };

export type FrameClassCard = { name: string; value: number; chosen: boolean; image: string; types: readonly FrameTypeChoice[]; valid: boolean };

export type ApmAirframe = { help: string; frameType: number | null; invalidText: string; classes: readonly FrameClassCard[] };

export const apmAirframe = (view: Json | null): ApmAirframe | null =>
  view === null || !optBoolean(view, 'available')
    ? null
    : {
        help: optText(view, 'help'),
        frameType: isNull(view, 'frameType') ? null : optInt(view, 'frameType'),
        invalidText: optText(view, 'invalidText'),
        classes: jsonObjects(optJSONArray(view, 'classes')).map((c) => ({
          name: optText(c, 'name'),
          value: optInt(c, 'value'),
          chosen: optBoolean(c, 'chosen'),
          image: optText(c, 'image'),
          types: jsonObjects(optJSONArray(c, 'types')).map((t) => ({ name: optText(t, 'name'), value: optInt(t, 'value') })),
          valid: optBoolean(c, 'valid', true),
        })),
      };

export const ApmAirframeScreen = ({ className }: { className?: string }) => {
  const [revision, setRevision] = useState(0);
  const [read, setRead] = useState<ApmAirframe | null>(null);
  const [refusal, setRefusal] = useState<string | null>(null);
  useLaunchedEffect([revision], async (scope) => {
    const next = apmAirframe(await Qgc.get(APM_AIRFRAME_VIEW));
    if (!scope.isActive()) return;
    setRead(next);
    await delay(APM_AIRFRAME_POLL_MS);
    if (scope.isActive()) setRevision((it) => it + 1);
  });
  const act = (path: string, value: number) =>
    void (async () => {
      setRefusal(await Qgc.refusalOf(path, value));
      setRead(apmAirframe(await Qgc.get(APM_AIRFRAME_VIEW)));
    })();
  const state = read;
  if (state === null) {
    return <Text className={`block p-4 ${className ?? ''}`}>This page is for an ArduPilot vehicle with a FRAME_CLASS parameter.</Text>;
  }
  const chosen = state.classes.find((it) => it.chosen && it.valid && it.types.length > 0) ?? null;
  return (
    <div className={`flex w-full flex-col gap-3 p-4 ${className ?? ''}`}>
      <Surface color={md('surfaceContainerHigh')} shape="large" className="w-full">
        <Text typography="bodyMedium" className="block p-4">
          {state.help}
        </Text>
      </Surface>
      {refusal !== null ? <Text color={md('error')}>{refusal}</Text> : null}
      {chosen !== null ? <FrameTypeChips types={chosen.types} frameType={state.frameType} onPick={(it) => act(APM_AIRFRAME_PICK_TYPE, it)} /> : null}
      <div className="grid min-h-0 flex-1 content-start gap-2 overflow-y-auto" style={{ gridTemplateColumns: `repeat(auto-fill, minmax(${FRAME_TILE_MIN}px, 1fr))` }}>
        {state.classes.map((card) => (
          <FrameClassTile key={card.value} card={card} invalidText={state.invalidText} onPickClass={() => act(APM_AIRFRAME_PICK_CLASS, card.value)} />
        ))}
      </div>
    </div>
  );
};

const FRAME_TILE_MIN = 104;

const FrameClassTile = ({ card, invalidText, onPickClass }: { card: FrameClassCard; invalidText: string; onPickClass: () => void }) => {
  const asset = airframeImageAsset(card.image);
  return (
    <Surface
      onClick={onPickClass}
      enabled={!card.chosen || !card.valid}
      shape="large"
      color={card.chosen ? md('primaryContainer') : md('surfaceContainerHigh')}
      border={card.chosen ? `2px solid ${md('primary')}` : undefined}
      className="relative"
      style={{ opacity: 1 }}
    >
      <div className="flex flex-col items-center gap-1 p-2" style={{ opacity: card.valid ? 1 : 0.5 }}>
        {asset !== null ? <img src={asset} alt={card.name} className="aspect-square w-full object-contain" /> : <span className="aspect-square w-full" />}
        <Text typography="labelLarge" maxLines={1}>
          {card.name}
        </Text>
      </div>
      {!card.valid ? (
        <span className="absolute inset-0 flex items-center justify-center p-1">
          <Text typography="labelSmall" color={md('error')} textAlign="center">
            {invalidText}
          </Text>
        </span>
      ) : null}
    </Surface>
  );
};

const FrameTypeChips = ({ types, frameType, onPick }: { types: readonly FrameTypeChoice[]; frameType: number | null; onPick: (value: number) => void }) => (
  <div className="flex flex-col gap-1">
    <Text typography="labelLarge" color={md('onSurfaceVariant')}>
      Frame type
    </Text>
    <div className="flex flex-wrap gap-2">
      {types.map((type) => (
        <FilterChip
          key={type.value}
          selected={type.value === frameType}
          onClick={() => {
            if (type.value !== frameType) onPick(type.value);
          }}
          label={type.name}
        />
      ))}
    </div>
  </div>
);
