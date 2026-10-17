import { useState } from 'react';
import { type Json, has, isNull, optInt } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { DropdownMenu, DropdownMenuItem, Text, TextButton } from '@/m3';
import { type AltitudeModeOffer, type AltitudeModesView, MISSION_CONTEXT, altitudeModesPath, altitudeModesView, refusalFor } from '@/map/AltitudeModes';

const PLAN_VIEW_PATH = 'view.plan';
const GLOBAL_ALTITUDE_MODE = 'plan.missionController.globalAltitudeMode';

export const missionFramePicks = (view: AltitudeModesView | null): readonly AltitudeModeOffer[] => view?.offers ?? [];

export const missionFrameChoice = (view: AltitudeModesView | null): boolean => missionFramePicks(view).filter((it) => it.enabled).length > 1;

export const globalAltitudeFrame = (plan: Json | null): number | null =>
  has(plan, 'globalAltitudeFrame') && !isNull(plan, 'globalAltitudeFrame') ? optInt(plan, 'globalAltitudeFrame') : null;

const MissionAltitudeFrameIn = ({ current }: { current: number }) => {
  const json = useQgcPath(altitudeModesPath(MISSION_CONTEXT, current));
  const view = altitudeModesView(json);
  const picks = missionFramePicks(view);
  const [open, setOpen] = useState(false);
  return (
    <div className="flex w-full items-center px-5">
      <Text typography="bodyMedium" className="min-w-0 flex-1">
        Altitude frame
      </Text>
      <div className="relative">
        <TextButton onClick={() => setOpen(true)} enabled={missionFrameChoice(view)}>
          {picks.find((it) => it.current)?.title ?? 'Frame'}
        </TextButton>
        <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
          {picks.map((offer) => {
            const note = refusalFor(view, offer.raw) ?? offer.help;
            return (
              <DropdownMenuItem
                key={offer.raw}
                text={
                  <span className="flex flex-col">
                    <span>{offer.title}</span>
                    {note.trim() !== '' ? <Text typography="labelSmall">{note}</Text> : null}
                  </span>
                }
                enabled={offer.enabled}
                onClick={() => {
                  setOpen(false);
                  void Qgc.set(GLOBAL_ALTITUDE_MODE, offer.raw);
                }}
              />
            );
          })}
        </DropdownMenu>
      </div>
    </div>
  );
};

export const MissionAltitudeFrame = () => {
  const plan = useQgcPath(PLAN_VIEW_PATH);
  const current = globalAltitudeFrame(plan);
  return current === null ? null : <MissionAltitudeFrameIn current={current} />;
};
