import { useEffect, useMemo, useRef, useState } from 'react';
import { VehicleCommands } from '@/bridge/VehicleCommands';
import { useQgcPath } from '@/bridge/QgcState';
import { Button, FilterChip, Slider, Surface, Text } from '@/m3';
import { md, withAlpha } from '@/theme/Colors';
import { FLY_STATE, flyState, useHasVehicle } from '@/ui/FlyState';
import { PWM_CENTER, PWM_MAX, PWM_MIN, RC_CONTROLS_VIEW, type RcControlState, RcControlType, parseRcControlStates, rcSendDue, switch3Pwms } from '@/ui/RcControls';

const inOrder = (block: () => Promise<unknown>): void => void block();

export class RcHolder {
  private sent: ReadonlySet<number> = new Set();

  constructor(private readonly send: (block: () => Promise<unknown>) => void = inOrder) {}

  hold(channel: number, pwm: number): void {
    if (channel <= 0) return;
    this.sent = new Set([...this.sent, channel]);
    this.send(() => VehicleCommands.overrideRcChannel(channel, pwm));
  }

  release(): void {
    const held = [...this.sent].toSorted((a, b) => a - b);
    this.sent = new Set();
    if (held.length > 0) this.send(() => Promise.all(held.map((it) => VehicleCommands.releaseRcChannel(it))));
  }

  forget(): void {
    this.sent = new Set();
  }

  holding(): ReadonlySet<number> {
    return new Set(this.sent);
  }
}

export const customRcControls = new RcHolder();
export const cameraRcControls = new RcHolder();

const sendRcOverride = (channel: number, pwm: number) => customRcControls.hold(channel, pwm);

const releaseOverrides = () => {
  customRcControls.forget();
  cameraRcControls.forget();
  inOrder(() => VehicleCommands.clearRcOverrides());
};

const SWITCH3_LABELS = ['Low', 'Mid', 'High'];

const ControlLabel = ({ text }: { text: string }) => (
  <Text typography="labelMedium" className="w-[84px] shrink-0">
    {text}
  </Text>
);

const RcSlider = ({ control }: { control: RcControlState }) => {
  const [dragging, setDragging] = useState<number | null>(null);
  const shown = dragging ?? control.pwm ?? PWM_CENTER;
  const latest = useRef(shown);
  const lastSent = useRef(0);

  const send = (value: number, finished: boolean) => {
    const now = performance.now();
    if (!rcSendDue(now, lastSent.current, finished)) return;
    lastSent.current = now;
    sendRcOverride(control.channel, value);
  };

  return (
    <div className="flex items-center">
      <ControlLabel text={control.label} />
      <Slider
        value={shown}
        onValueChange={(raw) => {
          const next = Math.trunc(raw);
          if (next === shown) return;
          latest.current = next;
          setDragging(next);
          send(next, false);
        }}
        onValueChangeFinished={() => {
          send(latest.current, true);
          setDragging(null);
        }}
        valueRange={[PWM_MIN, PWM_MAX]}
        label={control.label}
        className="w-full"
      />
    </div>
  );
};

const RcButton = ({ control }: { control: RcControlState }) => {
  const on = control.on === true;
  return (
    <FilterChip
      selected={on}
      onClick={() => sendRcOverride(control.channel, on ? PWM_MIN : PWM_MAX)}
      label={control.label}
      className="self-start"
    />
  );
};

const RcSwitch3 = ({ control }: { control: RcControlState }) => (
  <div className="flex items-center">
    <ControlLabel text={control.label} />
    <div className="flex gap-1">
      {switch3Pwms().map((value, index) => (
        <FilterChip
          key={index}
          selected={control.position === index}
          onClick={() => sendRcOverride(control.channel, value)}
          label={SWITCH3_LABELS[index] ?? ''}
        />
      ))}
    </div>
  </div>
);

const PRESS_KEYS: ReadonlySet<string> = new Set([' ', 'Enter']);

const RcMomentary = ({ control }: { control: RcControlState }) => {
  const [pressed, setPressed] = useState(false);
  const everPressed = useRef(false);

  useEffect(() => {
    if (pressed) {
      everPressed.current = true;
      sendRcOverride(control.channel, PWM_MAX);
    } else if (everPressed.current) {
      sendRcOverride(control.channel, PWM_MIN);
    }
  }, [pressed]);

  return (
    <span
      onPointerDown={(event) => {
        event.currentTarget.setPointerCapture(event.pointerId);
        setPressed(true);
      }}
      onPointerUp={() => setPressed(false)}
      onPointerCancel={() => setPressed(false)}
      onKeyDown={(event) => PRESS_KEYS.has(event.key) && !event.repeat && setPressed(true)}
      onKeyUp={(event) => PRESS_KEYS.has(event.key) && setPressed(false)}
      onBlur={() => setPressed(false)}
      className="flex select-none self-start"
      style={{ touchAction: 'none' }}
    >
      <Button>{control.label}</Button>
    </span>
  );
};

const RcControlView = ({ control }: { control: RcControlState }) =>
  control.type === RcControlType.Slider ? (
    <RcSlider control={control} />
  ) : control.type === RcControlType.Button ? (
    <RcButton control={control} />
  ) : control.type === RcControlType.Switch3 ? (
    <RcSwitch3 control={control} />
  ) : (
    <RcMomentary control={control} />
  );

export const RcControlsLayer = ({ className }: { className?: string }) => {
  const hasVehicle = useHasVehicle();
  const view = useQgcPath(RC_CONTROLS_VIEW);
  const controls = useMemo(() => parseRcControlStates(view), [view]);
  const stateJson = useQgcPath(FLY_STATE);
  const overriding = useMemo(() => flyState(stateJson)?.rcOverride === true, [stateJson]);

  useEffect(() => () => customRcControls.release(), []);

  if (controls.length === 0 || !hasVehicle) return null;

  return (
    <Surface className={className} color={withAlpha(md('surface'), 0.85)}>
      <div className="flex flex-col gap-1.5 px-3 py-2">
        {controls.map((control, index) => (
          <RcControlView key={`${index}:${control.channel}`} control={control} />
        ))}
        {overriding ? (
          <div className="flex w-full items-center gap-2">
            <Text typography="labelMedium" className="flex-1">
              These channels are held by this tablet.
            </Text>
            <Button onClick={releaseOverrides}>Give back</Button>
          </div>
        ) : null}
      </div>
    </Surface>
  );
};
