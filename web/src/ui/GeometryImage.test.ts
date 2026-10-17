import { describe, expect, it } from 'vitest';
import { type GeometryMotor, geometryLayout, motorAt } from './GeometryImage';

const motor = (index: number, label: number, x: number, y: number, counterClockwise: boolean): GeometryMotor => ({ index, label, x, y, counterClockwise });

const quad = [motor(0, 1, 1.0, 1.0, true), motor(1, 2, -1.0, -1.0, true), motor(2, 3, 1.0, -1.0, false), motor(3, 4, -1.0, 1.0, false)];

describe('GeometryImageTest', () => {
  it('the quad is laid out nose up, centred and inside the image', () => {
    const layout = geometryLayout(quad, 320, 160)!;
    const front = layout.motors.find((it) => it.motor.label === 1)!;
    const back = layout.motors.find((it) => it.motor.label === 2)!;
    expect(front.center.y < back.center.y, 'front is above back, as +x is drawn upwards').toBe(true);
    expect(front.center.x > back.center.x, '+y is drawn to the right').toBe(true);
    expect(Math.abs(layout.origin.x - (front.center.x + back.center.x) / 2)).toBeLessThanOrEqual(0.01);
    expect(layout.motors.every((it) => it.center.x - layout.rotorDiameter / 2 >= 0 && it.center.y + layout.rotorDiameter / 2 <= 160)).toBe(true);
    expect(geometryLayout(quad.slice(0, 1), 320, 160), 'one motor draws nothing').toBeNull();
  });

  it('a coaxial pair draws the lower motor offset and a tap picks only a highlighted motor', () => {
    const coax = [...quad, motor(4, 5, 1.0, 1.0, false)];
    const layout = geometryLayout(coax, 320, 160)!;
    const lower = layout.motors.find((it) => it.motor.label === 5)!;
    expect(lower.coax).toBe(true);
    expect(layout.extraYMargin > 0).toBe(true);
    const target = layout.motors.find((it) => it.motor.label === 3)!;
    expect(motorAt(layout, target.textCenter, new Set([2]))).toBe(2);
    expect(motorAt(layout, target.textCenter, new Set())).toBeNull();
    expect(motorAt(layout, { x: -50, y: -50 }, new Set([0, 1, 2, 3]))).toBeNull();
  });
});
