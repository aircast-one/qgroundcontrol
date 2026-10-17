import { describe, expect, it } from 'vitest';
import { powerCalcPath, powerCalculator } from './PowerCalc';

describe('PowerCalcTest', () => {
  it("a row's calculator is read and asks the core about its own battery and parameter", () => {
    const calculator = powerCalculator(JSON.parse('{"title":"Calculate Voltage Divider","measure":"voltage","batteryIndex":2,"param":"BAT2_V_DIV","button":"Calculate"}'));
    expect(calculator).not.toBeNull();
    expect(calculator === null ? null : powerCalcPath(calculator)).toBe('view.powerCalc(voltage,2,BAT2_V_DIV)');
    expect(powerCalculator(null)).toBeNull();
    expect(powerCalculator(JSON.parse('{"title":"x"}'))).toBeNull();
  });
});
