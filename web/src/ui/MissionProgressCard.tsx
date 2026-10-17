import { type Json, optBoolean, optDouble, optInt, optText } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { LinearProgressIndicator, Surface, Text, TextButton } from '@/m3';
import { Color, md } from '@/theme/Colors';
import { useRememberActiveVehicleId } from './ActiveVehicle';
import { SetWaypointSheet } from './MapClickMenu';
import { useOsdBackdrop } from './Osd';
import { useKeyedState } from '@/kotlin/Compose';

const MISSION_PROGRESS = 'view.missionProgress';
const CARD_MAX_WIDTH = 360;

export type MissionProgress = { readonly current: number; readonly last: number; readonly fraction: number; readonly distance: string; readonly skipTo: number | null };

export const missionProgress = (view: Json | null): MissionProgress | null =>
  view !== null && optBoolean(view, 'shown')
    ? {
        current: optInt(view, 'current'),
        last: optInt(view, 'last'),
        fraction: optDouble(view, 'fraction', 0.0),
        distance: [optText(view, 'distanceToNext'), optText(view, 'distanceUnits')].filter((part) => part.trim() !== '').join(' '),
        skipTo: optBoolean(view, 'canSkip') ? optInt(view, 'skipTo') : null,
      }
    : null;

export const missionProgressLine = (progress: MissionProgress): string =>
  [`To waypoint ${progress.current} of ${progress.last}`, progress.distance].filter((it) => it.trim() !== '').join(' · ');

export const MissionProgressCard = ({ className }: { className?: string }) => {
  const view = useQgcPath(MISSION_PROGRESS);
  const progress = missionProgress(view);
  return progress === null ? null : <MissionProgressShown progress={progress} className={className} />;
};

const MissionProgressShown = ({ progress, className }: { progress: MissionProgress; className?: string }) => {
  const vehicleId = useRememberActiveVehicleId();
  const [skipTarget, setSkipTarget] = useKeyedState<number | null>([vehicleId], () => null);
  const backdrop = useOsdBackdrop(md('surfaceContainerHigh'));
  return (
    <>
      <Surface className={className} style={{ maxWidth: CARD_MAX_WIDTH }} shape="medium" color={backdrop} contentColor={backdrop === Color.Transparent ? undefined : md('onSurface')}>
        <div className="flex flex-col gap-2 p-3">
          <div className="flex items-center">
            <Text typography="labelLarge" className="min-w-0 flex-1">
              {missionProgressLine(progress)}
            </Text>
            {progress.skipTo !== null ? <TextButton onClick={() => setSkipTarget(progress.skipTo)}>Skip</TextButton> : null}
          </div>
          <LinearProgressIndicator progress={progress.fraction} className="w-full" />
        </div>
      </Surface>
      {skipTarget !== null ? <SetWaypointSheet sequence={skipTarget} onDismiss={() => setSkipTarget(null)} /> : null}
    </>
  );
};
