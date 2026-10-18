import { useMemo } from 'react';
import { PlanCommands } from '@/bridge/BridgeCommands';
import { type Json, isNull, optBoolean, optInt, optLong, optText } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { AlertDialog, Button, HorizontalDivider, OutlinedButton, Text, TextButton } from '@/m3';
import { aircast } from '@/theme/Colors';

export const MISSION_COMPLETE_PATH = 'view.missionComplete';

export type MissionComplete = {
  readonly id: number;
  readonly imagesTaken: number;
  readonly resumeFromWaypoint: number | null;
  readonly batteryWarning: boolean;
  readonly interrupted: boolean;
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
        interrupted: optBoolean(view, 'interrupted'),
        title: optText(view, 'title') || 'Flight plan complete',
        detail: optText(view, 'detail'),
        resumeLabel: optText(view, 'resumeLabel'),
      }
    : null;

export const imagesTakenText = (count: number): string | null => (count === 0 ? null : `${count} Images Taken`);

export const MissionCompleteDialog = () => {
  const view = useQgcPath(MISSION_COMPLETE_PATH);
  const notice = useMemo(() => missionComplete(view), [view]);
  if (notice === null) return null;
  const close = () => void PlanCommands.dismissMissionComplete(notice.id);
  const images = imagesTakenText(notice.imagesTaken);
  const waypoint = notice.resumeFromWaypoint;
  const remove = () =>
    void (async () => {
      await PlanCommands.removeAllFromVehicle();
      await PlanCommands.dismissMissionComplete(notice.id);
    })();
  const resume = (at: number) =>
    void (async () => {
      await PlanCommands.resumeMission(at);
      await PlanCommands.dismissMissionComplete(notice.id);
    })();

  if (notice.interrupted && waypoint !== null) {
    return (
      <AlertDialog
        onDismissRequest={close}
        title={notice.title}
        confirmButton={<TextButton onClick={close}>Close</TextButton>}
        text={
          <div className="flex flex-col gap-3">
            <Text typography="bodyMedium">{notice.detail}</Text>
            <Button className="w-full" onClick={() => resume(waypoint)}>
              {notice.resumeLabel || `Resume mission from waypoint ${waypoint}`}
            </Button>
            <OutlinedButton className="w-full" onClick={close}>
              Leave plan on vehicle
            </OutlinedButton>
            <TextButton className="w-full" onClick={remove}>
              Remove plan from vehicle
            </TextButton>
            {notice.batteryWarning ? (
              <Text typography="bodySmall" color={aircast('warning')}>
                If you are changing batteries for Resume Mission do not disconnect from the vehicle.
              </Text>
            ) : null}
          </div>
        }
      />
    );
  }

  return (
    <AlertDialog
      onDismissRequest={close}
      title={notice.title}
      confirmButton={<TextButton onClick={close}>Close</TextButton>}
      text={
        <div className="flex flex-col gap-3">
          {images !== null ? <Text typography="bodyMedium" textAlign="center" className="w-full">{images}</Text> : null}
          <Button
            className="w-full"
            onClick={() =>
              void (async () => {
                await PlanCommands.removeAllFromVehicle();
                await PlanCommands.dismissMissionComplete(notice.id);
              })()
            }
          >
            Remove plan from vehicle
          </Button>
          <OutlinedButton className="w-full" onClick={close}>
            Leave plan on vehicle
          </OutlinedButton>
          {waypoint !== null ? (
            <>
              <HorizontalDivider />
              <OutlinedButton
                className="w-full"
                onClick={() =>
                  void (async () => {
                    await PlanCommands.resumeMission(waypoint);
                    await PlanCommands.dismissMissionComplete(notice.id);
                  })()
                }
              >
                {`Resume mission from waypoint ${waypoint}`}
              </OutlinedButton>
              <Text typography="bodySmall">Resume Mission will rebuild the current mission from the last flown waypoint and upload it to the vehicle for the next flight.</Text>
            </>
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
