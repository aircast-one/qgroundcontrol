import { useCallback, useContext, useMemo, useState } from 'react';
import type { Json } from '@/bridge/Json';
import { VideoCommands } from '@/bridge/BridgeCommands';
import { useQgcDerived, useQgcPath } from '@/bridge/QgcState';
import { useUpdatedRef } from '@/kotlin/Compose';
import { useValue } from '@/kotlin/State';
import { DropdownMenu, DropdownMenuItem, HorizontalDivider, Surface, Text } from '@/m3';
import { Drawable, MaterialIcon } from '@/res/Icon';
import { Color, aircast, md, withAlpha } from '@/theme/Colors';
import { CAMERA_VIEW, trackingCanAim, trackingReading } from '@/ui/CameraControl';
import { CAMERA_SHEET } from '@/ui/CameraControlLayer';
import { CAMERAS_VIEW, type CameraEntry, CameraStatus, type CamerasReading, camerasReading } from '@/ui/Cameras';
import { LocalFlyScreenState } from '@/ui/FlyScreenState';
import { GIMBAL_INDICATOR_PATH, onScreenGimbal } from '@/ui/GimbalIndicator';
import { useOsdBackdrop } from '@/ui/Osd';
import { GIMBAL_SHEET } from '@/ui/RcCameraControls';
import { shownCameraStatus, useSyntheticPage } from '@/ui/SyntheticView';
import { VIDEO_VIEW, videoReading } from '@/ui/VideoView';

const SWITCH_SCRIM_ALPHA = 0.55;
const SWITCH_BORDER_ALPHA = 0.4;
const SWITCH_DOT = 8;
const SWITCH_BORDER = 1;
const CHIP_PAD_HORIZONTAL = 12;
const CHIP_PAD_VERTICAL = 8;
const CHIP_GAP = 6;
const CHIP_LABEL_MAX = 96;
const CHIP_GLYPH = 18;
const CHIP_ARROW = 18;
const cameraMenuSheets = (): readonly (readonly [string, string])[] => [
  [CAMERA_SHEET, 'Camera settings'],
  [GIMBAL_SHEET, 'Gimbal settings'],
];

export type CameraSwitchState = { shown: CameraEntry; cameras: readonly CameraEntry[] };

export const cameraSwitchState = (reading: CamerasReading | null): CameraSwitchState | null => {
  const cameras = (reading?.cameras ?? []).filter((it) => it.active || it.problem === null);
  const shown = cameras.find((it) => it.active) ?? cameras[0] ?? null;
  return shown !== null && cameras.length > 1 ? { shown, cameras } : null;
};

export const pipCamera = (reading: CamerasReading | null, streamOn: boolean): CameraEntry | null => {
  const pip = reading?.pip ?? null;
  return reading === null || pip === null || !streamOn || !pip.enabled || pip.slot === null ? null : (reading.cameras.find((it) => it.slot === pip.slot) ?? null);
};

export const splitToggleTarget = (reading: CamerasReading | null, room: boolean, streamOn: boolean): boolean | null => {
  const pip = reading?.pip ?? null;
  return pip !== null && streamOn && room && pip.slot !== null ? !pip.enabled : null;
};

const streamEnabledOf = (json: Json | null): boolean => videoReading(json)?.streamEnabled === true;

const useStreamOn = (): boolean => useQgcDerived(VIDEO_VIEW, streamEnabledOf);

export const useCameraStatusTint = (status: CameraStatus): string =>
  status === CameraStatus.Live ? aircast('success') : status === CameraStatus.Connecting ? aircast('warning') : status === CameraStatus.NoSignal ? md('error') : md('outline');

export const CameraStatusDot = ({ status, className }: { status: CameraStatus; className?: string }) => {
  const tint = useCameraStatusTint(status);
  return <span role="img" aria-label={status.label} className={`block shrink-0 rounded-full ${className ?? ''}`} style={{ width: SWITCH_DOT, height: SWITCH_DOT, background: tint }} />;
};

export const showCamera = (slot: number, onRefused: () => void = () => undefined): void =>
  void (async () => {
    if (!(await VideoCommands.setActiveSource(slot))) onRefused();
  })();

export const chipCamera = (reading: CamerasReading | null): CameraEntry | null =>
  reading === null ? null : (reading.cameras.find((it) => it.active) ?? reading.cameras[0] ?? null);

export const CameraSwitch = ({ splitRoom, className }: { splitRoom: boolean; className?: string }) => {
  const json = useQgcPath(CAMERAS_VIEW);
  const reading = useMemo(() => camerasReading(json), [json]);
  const switchState = cameraSwitchState(reading);
  const splitTo = splitToggleTarget(reading, splitRoom, useStreamOn());
  const shown = switchState?.shown ?? chipCamera(reading);
  const flyScreen = useContext(LocalFlyScreenState);
  const offered = useValue(flyScreen.offeredSheets);
  const settings = cameraMenuSheets().filter(([sheet]) => offered.includes(sheet));
  const [open, setOpen] = useState(false);
  const white = aircast('outdoorForeground');
  const backdrop = useOsdBackdrop(withAlpha(Color.Black, SWITCH_SCRIM_ALPHA));
  const page = useSyntheticPage();
  const hidden = (switchState === null && splitTo === null && settings.length === 0) || shown === null;
  if (hidden && open) setOpen(false);
  if (hidden) return null;
  return (
    <div className={`relative ${className ?? ''}`}>
      <Surface
        onClick={() => setOpen(true)}
        shape="full"
        color={backdrop}
        contentColor={white}
        border={`${SWITCH_BORDER}px solid ${withAlpha(white, SWITCH_BORDER_ALPHA)}`}
        label={`Cameras, showing ${shown.title}`}
      >
        <span className="flex items-center" style={{ paddingInline: CHIP_PAD_HORIZONTAL, paddingBlock: CHIP_PAD_VERTICAL, gap: CHIP_GAP }}>
          <CameraStatusDot status={shownCameraStatus(shown, true, page)} />
          <Text typography="labelLarge" maxLines={1} ellipsis style={{ maxWidth: CHIP_LABEL_MAX }}>
            {shown.short}
          </Text>
          {splitTo === false ? <Drawable name="ic_split_view" label="Split view on" size={CHIP_GLYPH} /> : null}
          <Drawable name="ic_arrow_drop_down" size={CHIP_ARROW} />
        </span>
      </Surface>
      <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
        {switchState?.cameras.map((camera) => {
          const showing = camera.slot === shown.slot;
          return (
            <DropdownMenuItem
              key={camera.slot}
              text={
                <Text typography="labelLarge" maxLines={1} ellipsis>
                  {camera.title}
                </Text>
              }
              leadingIcon={<CameraStatusDot status={shownCameraStatus(camera, showing, page)} />}
              trailingIcon={showing ? <MaterialIcon name="check" filled label="Showing" /> : undefined}
              onClick={() => {
                setOpen(false);
                if (!showing) showCamera(camera.slot);
              }}
            />
          );
        })}
        {splitTo !== null ? (
          <>
            {switchState !== null ? <HorizontalDivider /> : null}
            <DropdownMenuItem
              text="Split view"
              checked={!splitTo}
              leadingIcon={<Drawable name="ic_split_view" size={CHIP_GLYPH} />}
              trailingIcon={!splitTo ? <MaterialIcon name="check" filled label="On" /> : undefined}
              onClick={() => {
                setOpen(false);
                void VideoCommands.setPictureInPicture(splitTo);
              }}
            />
          </>
        ) : null}
        {settings.length > 0 && (switchState !== null || splitTo !== null) ? <HorizontalDivider /> : null}
        {settings.map(([sheet, label]) => (
          <DropdownMenuItem
            key={sheet}
            text={label}
            leadingIcon={<Drawable name={sheet === CAMERA_SHEET ? 'ic_settings' : 'ic_tune'} size={CHIP_GLYPH} />}
            onClick={() => {
              setOpen(false);
              flyScreen.requestedSheet.value = sheet;
            }}
          />
        ))}
      </DropdownMenu>
    </div>
  );
};

export const useShownSecondCamera = (): CameraEntry | null => {
  const streamOn = useStreamOn();
  const read = useCallback((json: Json | null) => pipCamera(camerasReading(json), streamOn), [streamOn]);
  return useQgcDerived(CAMERAS_VIEW, read);
};

export const neighbourCamera = (state: CameraSwitchState | null, step: number): CameraEntry | null => {
  const cameras = state?.cameras ?? [];
  if (state === null || cameras.length <= 1) return null;
  const at = cameras.indexOf(state.shown) + step;
  return cameras[((at % cameras.length) + cameras.length) % cameras.length] ?? null;
};

export const useRememberCameraSwiper = (): ((step: number) => boolean) => {
  const json = useQgcPath(CAMERAS_VIEW);
  const state = useUpdatedRef(cameraSwitchState(camerasReading(json)));
  return useCallback((step: number) => {
    const next = neighbourCamera(state.current, step);
    if (next === null) return false;
    showCamera(next.slot);
    return true;
  }, []);
};

export const useRememberDragAims = (): boolean => {
  const view = useQgcPath(GIMBAL_INDICATOR_PATH);
  const camera = useQgcPath(CAMERA_VIEW);
  return useMemo(() => onScreenGimbal(view) !== null || trackingCanAim(trackingReading(camera)), [view, camera]);
};
