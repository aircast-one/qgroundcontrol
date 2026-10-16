import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { powerStateText } from '@/app/GamepadInput';
import { JoystickBadge, joystickBadge } from './JoystickIndicator';

describe('JoystickIndicatorTest', () => {
  it("the badge reads the core's indicator and hides without a joystick", () => {
    const badge = joystickBadge(parseJson('{"indicator":{"heading":"Xbox","enabledText":"No","warn":true,"typeText":"Gamepad","inputsText":"6 axes, 15 buttons"}}'));
    expect(badge).toEqual(JoystickBadge('Xbox', 'No', true, 'Gamepad', '6 axes, 15 buttons'));
    expect(joystickBadge(parseJson('{"indicator":null}'))).toBeNull();
  });

  it('detail rows come through with their warning', () => {
    const badge = joystickBadge(parseJson('{"indicator":{"heading":"Pad","details":[{"label":"Battery:","value":"15%","warn":true}]}}'));
    expect(badge?.details).toEqual([{ label: 'Battery:', value: '15%', warn: true }]);
    expect(powerStateText(-1)).toBe('');
  });
});
