import { type CSSProperties, type ReactNode, useEffect, useMemo, useRef, useState } from 'react';
import { Slider as BaseSlider } from '@base-ui/react';
import { type Json, optBoolean, optDouble } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { VideoCommands } from '@/bridge/BridgeCommands';
import { settingControl, useQgcBool, useQgcPath, useQgcValue } from '@/bridge/QgcState';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { coerceIn, delay } from '@/kotlin/Std';
import { FilterChip, Text, TextButton } from '@/m3';
import { AircastSheet } from '@/map/AircastSheet';
import { useKeyedState } from '@/map/SettingStepper';
import { VEHICLES_VIEW } from '@/map/VehiclePicker';
import { Color, md, withAlpha } from '@/theme/Colors';
import { activeVehicleId } from '@/ui/ActiveVehicle';
import { VIDEO_RECORDING_STATE } from '@/ui/CameraControlLayer';
import { OpenOnRequest } from '@/ui/FlyScreenState';
import { useHasVehicle } from '@/ui/FlyState';
import { type GimbalIndicatorState, gimbalIndicator, gimbalRefusal } from '@/ui/GimbalIndicator';
import { PWM_CENTER, PWM_MAX, PWM_MIN, rcSendDue } from '@/ui/RcControls';
import { cameraRcControls } from '@/ui/RcControlsLayer';
import { RecenterTarget, TiltReadout, recenter, recenterTarget } from '@/ui/RecenterCamera';
import { DEFAULT_TILT_DEG, SYNTHETIC_AIM, SYNTHETIC_VIEW, useRememberSyntheticAims } from '@/ui/SyntheticView';

const FLY_VIEW_SETTINGS = 'settings.flyViewSettings';
const RAIL_SLIDER_LENGTH = 140;
const RAIL_SLIDER_THICKNESS = 40;
const RAIL_SLIDER_SHORTEST = 64;
const GIMBAL_VIEW = 'view.gimbalIndicator';
export const GIMBAL_SHEET = 'gimbal';

export type RailSliderColors = { thumb: string; activeTrack: string; inactiveTrack: string };

export const RailSlider = ({
  value,
  onValueChange,
  onValueChangeFinished,
  valueRange,
  colors = null,
  label,
  style,
}: {
  value: number;
  onValueChange: (value: number) => void;
  onValueChangeFinished?: () => void;
  valueRange: [number, number];
  colors?: RailSliderColors | null;
  label?: string;
  style?: CSSProperties;
}) => (
  <BaseSlider.Root
    value={value}
    min={valueRange[0]}
    max={valueRange[1]}
    step={(valueRange[1] - valueRange[0]) / 1000}
    orientation="vertical"
    onValueChange={(next) => onValueChange(Array.isArray(next) ? (next[0] ?? value) : next)}
    onValueCommitted={() => onValueChangeFinished?.()}
    aria-label={label}
    className="h-full transition-opacity"
    style={{ width: RAIL_SLIDER_THICKNESS, ...style }}
  >
    <BaseSlider.Control className="flex h-full w-full touch-none justify-center">
      <BaseSlider.Track className={`relative h-full w-1 rounded-full ${colors === null ? 'bg-secondary-container' : ''}`} style={{ background: colors?.inactiveTrack }}>
        <BaseSlider.Indicator className={`w-full rounded-full ${colors === null ? 'bg-primary' : ''}`} style={{ background: colors?.activeTrack }} />
        <BaseSlider.Thumb className={`size-5 rounded-full shadow outline-none focus-visible:ring-4 focus-visible:ring-primary/30 ${colors === null ? 'bg-primary' : ''}`} style={{ background: colors?.thumb }} />
      </BaseSlider.Track>
    </BaseSlider.Control>
  </BaseSlider.Root>
);

export const RailSlot = ({ children }: { children: ReactNode }) => {
  const [slot, setSlot] = useState<HTMLDivElement | null>(null);
  const [room, setRoom] = useState(true);
  useEffect(() => {
    if (slot === null) return undefined;
    const observer = new ResizeObserver(([entry]) => setRoom((entry?.contentRect.height ?? 0) >= RAIL_SLIDER_SHORTEST));
    observer.observe(slot);
    return () => observer.disconnect();
  }, [slot]);
  return (
    <div ref={setSlot} className="flex justify-center" style={{ flex: `0 1 ${RAIL_SLIDER_LENGTH}px`, minHeight: 0, width: RAIL_SLIDER_THICKNESS }}>
      {room ? children : null}
    </div>
  );
};

export type RcCameraChannels = { tilt: number; pan: number; zoom: number; light: number; record: number; readonly any: boolean };

export const RcCameraChannels = (fields: Omit<RcCameraChannels, 'any'>): RcCameraChannels => ({
  ...fields,
  any: [fields.tilt, fields.pan, fields.zoom, fields.light, fields.record].some((it) => it > 0),
});

export const rcCameraChannels = (tilt: number, pan: number, zoom: number, light: number, record: number): RcCameraChannels =>
  RcCameraChannels({ tilt, pan: pan === tilt ? 0 : pan, zoom, light, record });

export const cameraRecording = (recordChannel: number, channelRecording: boolean, streamRecording: boolean): boolean => streamRecording || (recordChannel > 0 && channelRecording);

const send = (channel: number, pwm: number) => cameraRcControls.hold(channel, pwm);

const useChannelSetting = (name: string): number => {
  const value = useQgcValue(settingControl(`${FLY_VIEW_SETTINGS}.${name}`));
  return typeof value === 'number' ? Math.trunc(value) : 0;
};

const PwmSlider = ({ label, channel, pwm, onPwm }: { label: string; channel: number; pwm: number; onPwm: (pwm: number) => void }) => {
  const lastSent = useRef({ channel, at: 0 });
  const dragged = useRef<number | null>(null);
  return (
    <div className="flex min-h-0 flex-col items-center">
      <RailSlot>
        <RailSlider
          value={pwm}
          onValueChange={(raw) => {
            const next = Math.trunc(raw);
            dragged.current = next;
            onPwm(next);
            const now = performance.now();
            const sentAt = lastSent.current.channel === channel ? lastSent.current.at : 0;
            if (rcSendDue(now, sentAt, false)) {
              lastSent.current = { channel, at: now };
              send(channel, next);
            }
          }}
          onValueChangeFinished={() => {
            send(channel, dragged.current ?? pwm);
            dragged.current = null;
          }}
          valueRange={[PWM_MIN, PWM_MAX]}
          label={label}
        />
      </RailSlot>
      <Text typography="labelMedium">{label}</Text>
    </div>
  );
};

const sendTilt = (pitch: number) => void (async () => gimbalRefusal(await Qgc.call('gimbal.pitch', pitch)))();

const GIMBAL_REFUSAL_MS = 4000;
export const GIMBAL_TILT_MIN = -90;
export const GIMBAL_TILT_MAX = 30;
const GIMBAL_IDLE_MS = 3000;
const GIMBAL_TRACK_ALPHA = 0.8;
const GIMBAL_REST_ALPHA = 0.25;
const TILT_IDLE_ALPHA = 0.45;

const TILT_COLORS: RailSliderColors = {
  thumb: Color.White,
  activeTrack: withAlpha(Color.White, GIMBAL_TRACK_ALPHA),
  inactiveTrack: withAlpha(Color.White, GIMBAL_REST_ALPHA),
};

const aimSynthetic = (tilt: number, pan: number) => void Qgc.invoke(SYNTHETIC_AIM, tilt, pan);

const TiltSlider = ({ pitch, onRecenter, slider, onTilt }: { pitch: number | null; onRecenter: (() => void) | null; slider: boolean; onTilt: (tilt: number) => void }) => {
  const [dragging, setDragging] = useState<number | null>(null);
  const dragged = useRef<number | null>(null);
  const lastSent = useRef(0);
  const shown = dragging ?? coerceIn(pitch ?? 0, GIMBAL_TILT_MIN, GIMBAL_TILT_MAX);
  const [active, setActive] = useState(true);
  const [wake, setWake] = useState(0);
  useLaunchedEffect([shown, dragging, wake], async (scope) => {
    setActive(true);
    if (dragging === null) {
      await delay(GIMBAL_IDLE_MS);
      if (scope.isActive()) setActive(false);
    }
  });
  return (
    <div className="flex min-h-0 flex-col items-center">
      <TiltReadout degrees={shown} onRecenter={onRecenter} onTap={slider ? () => setWake((it) => it + 1) : null} tapLabel="Tilt the camera" />
      {slider ? (
        <RailSlot>
          <RailSlider
            value={shown}
            onValueChange={(next) => {
              dragged.current = next;
              setDragging(next);
              const now = performance.now();
              if (rcSendDue(now, lastSent.current, false)) {
                lastSent.current = now;
                onTilt(next);
              }
            }}
            onValueChangeFinished={() => {
              if (dragged.current !== null) onTilt(dragged.current);
              dragged.current = null;
              setDragging(null);
            }}
            valueRange={[GIMBAL_TILT_MIN, GIMBAL_TILT_MAX]}
            colors={TILT_COLORS}
            label="Camera tilt"
            style={{ opacity: active ? 1 : TILT_IDLE_ALPHA }}
          />
        </RailSlot>
      ) : null}
    </div>
  );
};

const finiteOrNull = (value: number): number | null => (Number.isFinite(value) ? value : null);

export const RcCameraControls = ({ className, tiltSlider = true }: { className?: string; tiltSlider?: boolean }) => {
  const channels = rcCameraChannels(
    useChannelSetting('gimbalTiltChannel'),
    useChannelSetting('gimbalPanChannel'),
    useChannelSetting('cameraZoomChannel'),
    useChannelSetting('cameraLightChannel'),
    useChannelSetting('cameraRecordChannel'),
  );
  const gimbalJson = useQgcPath(GIMBAL_VIEW);
  const gimbalManager = optBoolean(gimbalJson, 'shown');
  const vehicleId = activeVehicleId(useQgcPath(VEHICLES_VIEW));
  const streamRecording = useQgcBool(VIDEO_RECORDING_STATE);
  const syntheticAims = useRememberSyntheticAims();
  const syntheticJson = useQgcPath(SYNTHETIC_VIEW);
  const [tilt, setTilt] = useKeyedState([vehicleId], () => PWM_CENTER);
  const [pan, setPan] = useKeyedState([vehicleId], () => PWM_CENTER);
  const [zoom, setZoom] = useKeyedState([vehicleId], () => PWM_CENTER);
  const [lightOn, setLightOn] = useKeyedState([vehicleId], () => false);
  const [channelRecording, setChannelRecording] = useKeyedState([vehicleId], () => false);
  useEffect(() => () => cameraRcControls.release(), [vehicleId]);
  const hasVehicle = useHasVehicle();
  if (!hasVehicle || (!channels.any && !gimbalManager && !syntheticAims)) return null;
  return (
    <RcCameraControlsShown
      className={className}
      tiltSlider={tiltSlider}
      channels={channels}
      gimbalJson={gimbalJson}
      gimbalManager={gimbalManager}
      streamRecording={streamRecording}
      syntheticAims={syntheticAims}
      syntheticJson={syntheticJson}
      pwm={{ tilt, pan, zoom }}
      setPwm={{ tilt: setTilt, pan: setPan, zoom: setZoom }}
      lightOn={lightOn}
      setLightOn={setLightOn}
      channelRecording={channelRecording}
      setChannelRecording={setChannelRecording}
    />
  );
};

type PwmTriple<T> = { tilt: T; pan: T; zoom: T };

const RcCameraControlsShown = ({
  className,
  tiltSlider,
  channels,
  gimbalJson,
  gimbalManager,
  streamRecording,
  syntheticAims,
  syntheticJson,
  pwm,
  setPwm,
  lightOn,
  setLightOn,
  channelRecording,
  setChannelRecording,
}: {
  className?: string;
  tiltSlider: boolean;
  channels: RcCameraChannels;
  gimbalJson: Json | null;
  gimbalManager: boolean;
  streamRecording: boolean;
  syntheticAims: boolean;
  syntheticJson: Json | null;
  pwm: PwmTriple<number>;
  setPwm: PwmTriple<(pwm: number) => void>;
  lightOn: boolean;
  setLightOn: (on: boolean) => void;
  channelRecording: boolean;
  setChannelRecording: (recording: boolean) => void;
}) => {
  const gimbal: GimbalIndicatorState | null = useMemo(() => gimbalIndicator(gimbalJson), [gimbalJson]);
  const [gimbalRefused, setGimbalRefused] = useState<string | null>(null);
  const [options, setOptions] = useState(false);
  const syntheticTilt = finiteOrNull(optDouble(syntheticJson, 'tilt'));
  const syntheticPan = finiteOrNull(optDouble(syntheticJson, 'pan'));
  const target = recenterTarget(syntheticAims, syntheticTilt, syntheticPan, gimbal);
  const onRecenter = (aimed: RecenterTarget): (() => void) | null => (target === aimed ? () => recenter(aimed, setGimbalRefused) : null);
  useLaunchedEffect([gimbalRefused], async (scope) => {
    if (gimbalRefused !== null) {
      await delay(GIMBAL_REFUSAL_MS);
      if (scope.isActive()) setGimbalRefused(null);
    }
  });
  const recording = cameraRecording(channels.record, channelRecording, streamRecording);
  const rcGimbal = !gimbalManager && (channels.tilt > 0 || channels.pan > 0);
  const gimbalCall = (path: string, ...args: unknown[]) => void (async () => setGimbalRefused(gimbalRefusal(await Qgc.call(path, ...args))))();
  return (
    <div className={`flex min-h-0 flex-col items-center gap-1 p-1 ${className ?? ''}`}>
      {channels.light > 0 || gimbal !== null || rcGimbal ? <OpenOnRequest name={GIMBAL_SHEET} open={() => setOptions(true)} /> : null}
      <div className="flex min-h-0 shrink gap-1">
        {syntheticAims ? (
          <TiltSlider pitch={syntheticTilt ?? DEFAULT_TILT_DEG} onRecenter={onRecenter(RecenterTarget.Synthetic)} slider={tiltSlider} onTilt={(it) => aimSynthetic(it, syntheticPan ?? 0.0)} />
        ) : gimbal !== null ? (
          <TiltSlider pitch={gimbal.pitchDegrees} onRecenter={onRecenter(RecenterTarget.Gimbal)} slider={tiltSlider} onTilt={sendTilt} />
        ) : null}
        {!gimbalManager && channels.tilt > 0 ? <PwmSlider label="Tilt" channel={channels.tilt} pwm={pwm.tilt} onPwm={setPwm.tilt} /> : null}
        {!gimbalManager && channels.pan > 0 ? <PwmSlider label="Pan" channel={channels.pan} pwm={pwm.pan} onPwm={setPwm.pan} /> : null}
        {channels.zoom > 0 ? <PwmSlider label="Zoom" channel={channels.zoom} pwm={pwm.zoom} onPwm={setPwm.zoom} /> : null}
      </div>
      {channels.record > 0 ? (
        <FilterChip
          selected={recording}
          onClick={() => {
            const next = !recording;
            setChannelRecording(next);
            send(channels.record, next ? PWM_MAX : PWM_MIN);
            void VideoCommands.setRecording(next);
          }}
          label="Record"
        />
      ) : null}
      {options ? (
        <AircastSheet onDismissRequest={() => setOptions(false)}>
          <div className="flex w-full flex-col gap-2 px-4 py-2">
            <Text typography="titleLarge">Gimbal</Text>
            <div className="flex items-center gap-2">
              {channels.light > 0 ? (
                <FilterChip
                  selected={lightOn}
                  onClick={() => {
                    const next = !lightOn;
                    setLightOn(next);
                    send(channels.light, next ? PWM_MAX : PWM_MIN);
                  }}
                  label="Light"
                />
              ) : null}
              {gimbal !== null && gimbal.yawLockOffered ? <FilterChip selected={gimbal.yawLocked} onClick={() => gimbalCall('gimbal.yawLock', !gimbal.yawLocked)} label={gimbal.yawLockLabel} /> : null}
              {gimbal !== null ? <TextButton onClick={() => gimbalCall('gimbal.center')}>Recenter</TextButton> : null}
              {rcGimbal ? (
                <TextButton
                  onClick={() => {
                    setPwm.tilt(PWM_CENTER);
                    setPwm.pan(PWM_CENTER);
                    send(channels.tilt, PWM_CENTER);
                    send(channels.pan, PWM_CENTER);
                  }}
                >
                  Recenter
                </TextButton>
              ) : null}
            </div>
            {gimbalRefused !== null ? (
              <Text typography="bodySmall" color={md('error')}>
                {gimbalRefused}
              </Text>
            ) : null}
          </div>
        </AircastSheet>
      ) : null}
      {gimbalRefused !== null ? (
        <Text typography="labelSmall" color={md('error')}>
          {gimbalRefused}
        </Text>
      ) : null}
    </div>
  );
};
