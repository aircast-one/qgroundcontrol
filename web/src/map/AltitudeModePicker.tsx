import { FilterChip, Text } from '@/m3';
import { ITEM_CONTEXT, FRAME_UNKNOWN, altitudeModesPath, altitudeModesView, choosable, offersChoice, refusalFor } from './AltitudeModes';
import { useMapPath } from './MapBridge';
import type { MissionItem } from './PlanBridge';
import { sentenceCase } from './SentenceCase';

export const GLOBAL_FRAME_MIXED = 0;
export const GLOBAL_FRAME_RELATIVE = 1;

export const itemReferenceShown = (globalFrame: number | null): boolean => globalFrame !== GLOBAL_FRAME_RELATIVE;

export const itemReferenceSelectable = (globalFrame: number | null): boolean => globalFrame === null || globalFrame === GLOBAL_FRAME_MIXED;

export const AltitudeModePicker = ({ item, onPick, className, globalFrameMixed = true }: { item: MissionItem; onPick: (raw: number) => void; className?: string; globalFrameMixed?: boolean }) => {
  const view = altitudeModesView(useMapPath(altitudeModesPath(ITEM_CONTEXT, item.altitudeMode)));
  const picks = choosable(view);
  const live = globalFrameMixed && offersChoice(view);
  const notes = picks
    .filter((it) => !it.current)
    .flatMap((offer) => {
      const reason = refusalFor(view, offer.raw);
      return reason === null ? [] : [`${offer.title}: ${reason}`];
    });
  return (
    <div className={`flex flex-col gap-0.5 ${className ?? ''}`}>
      {picks.length === 0 ? <Text typography="labelMedium">{item.altitudeFrameText.trim() === '' ? FRAME_UNKNOWN : item.altitudeFrameText}</Text> : null}
      <div className="flex flex-wrap gap-2">
        {picks.map((offer) => (
          <FilterChip
            key={offer.raw}
            selected={offer.current}
            enabled={(live && offer.enabled) || offer.current}
            onClick={() => {
              if (live && !offer.current) onPick(offer.raw);
            }}
            label={sentenceCase(offer.title)}
          />
        ))}
      </div>
      {live
        ? notes.map((it) => (
            <Text key={it} typography="labelSmall">
              {it}
            </Text>
          ))
        : null}
    </div>
  );
};
