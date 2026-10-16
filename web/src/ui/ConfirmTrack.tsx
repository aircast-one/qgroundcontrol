import { useMemo } from 'react';
import { type Json, optBoolean, optJSONArray, arrayJSONObject, optString, optText } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { Checkbox, Surface, Text, TextButton } from '@/m3';
import { summaryRow } from '@/map/MissionSummary';
import { sentenceCase } from '@/map/SentenceCase';
import { useKeyedState } from '@/map/SettingStepper';
import { aircast, md } from '@/theme/Colors';
import { HoldToConfirm, holdLabel } from '@/ui/HoldToConfirm';
import { planStatusText, planTitle } from '@/ui/PlanFileRules';
import type { GuidedAction } from '@/ui/VehicleUi';

export const sentIsStillShowing = (name: string | null, snapshotAtSend: string | null, live: string | null): boolean =>
  name !== null && snapshotAtSend !== null && snapshotAtSend === live;

export const sentText = (name: string): string => `Sent · ${name}`;

export const SENT_SHOWN_MS = 3_000;

export const LAND_FROM = 'view.instruments(altitudeRelative)';

export const landFrom = (view: Json | null): readonly [string, string] | null => {
  const item = arrayJSONObject(optJSONArray(view, 'items'), 0);
  if (item === null || optBoolean(item, 'missing')) return null;
  const value = optString(item, 'value');
  return value.trim() !== '' && value !== '—' ? [value, optString(item, 'units')] : null;
};

export const MISSION_ACTIONS: ReadonlySet<string> = new Set(['startMission', 'continueMission']);

export const MISSION_ON_DRONE = 'The mission already on the drone';

export type MissionIdentity = { title: string; facts: string; warning: string | null };

export const MissionIdentity = (fields: MissionIdentity): MissionIdentity => ({ ...fields });

export const missionIdentity = (plan: Json | null, summary: Json | null): MissionIdentity => {
  if (!optBoolean(plan, 'hasMissionItems')) return MissionIdentity({ title: MISSION_ON_DRONE, facts: '', warning: "It isn't open in Plan, so its route can't be shown here." });
  if (optBoolean(plan, 'dirty')) return MissionIdentity({ title: MISSION_ON_DRONE, facts: '', warning: `${planTitle(optText(plan, 'file'))} has changes that aren't on the drone. Upload it first to fly it.` });
  const status = planStatusText(plan);
  return MissionIdentity({
    title: planTitle(optText(plan, 'file')),
    facts: [status.trim() === '' ? null : status, summaryRow(summary, 'Distance'), summaryRow(summary, 'Time')].filter((it): it is string => it !== null).join(' · '),
    warning: null,
  });
};

const MissionIdentityCard = () => {
  const planJson = useQgcPath('view.plan');
  const summaryJson = useQgcPath('view.missionSummary');
  const mission = useMemo(() => missionIdentity(planJson, summaryJson), [planJson, summaryJson]);
  return (
    <Surface shape="medium" color={md('surfaceContainerHigh')}>
      <div className="flex w-full flex-col gap-1 px-4 py-3">
        <Text typography="titleMedium">{mission.title}</Text>
        {mission.facts.trim() !== '' ? (
          <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
            {mission.facts}
          </Text>
        ) : null}
        {mission.warning !== null ? (
          <Text typography="bodyMedium" color={aircast('warning')}>
            {mission.warning}
          </Text>
        ) : null}
      </div>
    </Surface>
  );
};

const LandFromTile = () => {
  const from = landFrom(useQgcPath(LAND_FROM));
  return from === null ? null : <FactTile label="FROM" value={from[0]} units={from[1]} />;
};

export const ConfirmTrack = ({ action, onSent, onCancel, className }: { action: GuidedAction; onSent: () => void; onCancel: () => void; className?: string }) => {
  const [optionChecked, setOptionChecked] = useKeyedState([action.name, action.confirm, action.destructive, action.offerId, action.option?.label], () => false);
  const confirm = () => {
    if (action.option) action.option.run(optionChecked);
    else action.run();
    onSent();
  };
  return (
    <div className={`flex w-full flex-col gap-2 ${className ?? ''}`}>
      <Text typography="titleLarge">{sentenceCase(action.name)}</Text>
      {action.confirm.trim() !== '' ? (
        <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
          {action.confirm}
        </Text>
      ) : null}
      {action.offerId !== null && MISSION_ACTIONS.has(action.offerId) ? <MissionIdentityCard /> : null}
      {action.offerId === 'land' ? <LandFromTile /> : null}
      {action.option ? (
        <label className="flex min-h-12 w-full cursor-pointer items-center gap-3">
          <Checkbox checked={optionChecked} onCheckedChange={setOptionChecked} label={action.option.label} />
          <Text typography="labelLarge">{action.option.label}</Text>
        </label>
      ) : null}
      <HoldToConfirm label={holdLabel(action.name)} destructive={action.destructive} className="mt-2" onConfirm={confirm} />
      <TextButton onClick={onCancel} className="self-center">
        Cancel
      </TextButton>
    </div>
  );
};

export const HoldOrCancel = ({ label, onConfirm, onCancel }: { label: string; onConfirm: () => void; onCancel: () => void }) => (
  <div className="flex w-full flex-col gap-1">
    <HoldToConfirm label={holdLabel(label)} className="mt-2" onConfirm={onConfirm} />
    <TextButton onClick={onCancel} className="self-center">
      Cancel
    </TextButton>
  </div>
);

export const SentNotice = ({ name, onDismiss, className }: { name: string; onDismiss: () => void; className?: string }) => (
  <button type="button" onClick={onDismiss} className={`w-full text-left ${className ?? ''}`}>
    <Text typography="bodyMedium" fontWeight={700} color={md('primary')}>
      {sentText(name)}
    </Text>
  </button>
);

const FactTile = ({ label, value, units }: { label: string; value: string; units: string }) => (
  <div className="flex flex-col self-start rounded-shape-md bg-surface-container-high px-3 py-1.5">
    <Text typography="labelSmall" color={md('onSurfaceVariant')}>
      {label}
    </Text>
    <div className="flex items-end gap-1">
      <Text typography="titleLarge">{value}</Text>
      {units.trim() !== '' ? (
        <Text typography="labelMedium" color={md('onSurfaceVariant')}>
          {units}
        </Text>
      ) : null}
    </div>
  </div>
);
