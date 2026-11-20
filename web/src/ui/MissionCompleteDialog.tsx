import { useContext, useMemo } from 'react';
import { PlanCommands } from '@/bridge/BridgeCommands';
import { type Json, isNull, optBoolean, optInt, optLong, optText } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { AlertDialog, Button, OutlinedButton, Text, TextButton } from '@/m3';
import { aircast } from '@/theme/Colors';
import { LocalAppNavigation } from '@/ui/AppNavigation';
import { useArmed } from '@/ui/FlyState';
import { reviewFlight } from '@/ui/RecordedFlights';

export const MISSION_COMPLETE_PATH = 'view.missionComplete';

export type MissionComplete = {
  readonly id: number;
  readonly imagesTaken: number;
  readonly resumeFromWaypoint: number | null;
  readonly batteryWarning: boolean;
  readonly title: string;
  readonly detail: string;
  readonly resumeLabel: string;
};

export const missionComplete = (view: Json | null): MissionComplete | null =>
  view !== null && optBoolean(view, 'open')
    ? {
        id: optLong(view, 'id'),
        imagesTaken: optInt(view, 'imagesTaken'),
        resumeFromWaypoint: isNull(view, 'resumeFromWaypoint') ? null : optInt(view, 'resumeFromWaypoint'),
        batteryWarning: optBoolean(view, 'batteryWarning'),
        title: optText(view, 'title') || 'Flight plan complete',
        detail: optText(view, 'detail'),
        resumeLabel: optText(view, 'resumeLabel'),
      }
    : null;

export const imagesTakenText = (count: number): string | null => (count === 0 ? null : `${count} Images Taken`);

export const MissionCompleteDialog = () => {
  const view = useQgcPath(MISSION_COMPLETE_PATH);
  const notice = useMemo(() => missionComplete(view), [view]);
  const navigation = useContext(LocalAppNavigation);
  const armed = useArmed();
  if (notice === null) return null;
  const close = () => void PlanCommands.dismissMissionComplete(notice.id);
  const images = imagesTakenText(notice.imagesTaken);
  const waypoint = notice.resumeFromWaypoint;
  const remove = () =>
    void (async () => {
      await PlanCommands.removeAllFromVehicle();
      await PlanCommands.dismissMissionComplete(notice.id);
    })();
  const review = () =>
    void (async () => {
      await PlanCommands.dismissMissionComplete(notice.id);
      await reviewFlight('', () => navigation.showFly());
    })();
  const resume = (at: number) =>
    void (async () => {
      await PlanCommands.resumeMission(at);
      await PlanCommands.dismissMissionComplete(notice.id);
    })();

  return (
    <AlertDialog
      onDismissRequest={close}
      title={notice.title}
      confirmButton={<TextButton onClick={close}>Close</TextButton>}
      text={
        <div className="flex flex-col gap-3">
          {notice.detail !== '' ? <Text typography="bodyMedium">{notice.detail}</Text> : null}
          {images !== null ? <Text typography="bodyMedium">{images}</Text> : null}
          {waypoint !== null ? (
            <Button className="w-full" onClick={() => resume(waypoint)}>
              {notice.resumeLabel}
            </Button>
          ) : null}
          <OutlinedButton className="w-full" onClick={close}>
            Leave plan on vehicle
          </OutlinedButton>
          <TextButton className="w-full" onClick={remove}>
            Remove plan from vehicle
          </TextButton>
          {!armed ? (
            <TextButton className="w-full" onClick={review}>
              Review this flight
            </TextButton>
          ) : null}
          {notice.batteryWarning ? (
            <Text typography="bodySmall" color={aircast('warning')}>
              If you are changing batteries for Resume Mission do not disconnect from the vehicle.
            </Text>
          ) : null}
        </div>
      }
    />
  );
};
