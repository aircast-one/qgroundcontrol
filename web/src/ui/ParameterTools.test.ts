import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { modifiedFilterOn, parameterCountLine, parameterShown } from '@/ui/ParametersScreen';
import { ParameterDiffRow, ParameterReview, checkedAll, confirmLabel, diffLine, diffRowKey, parameterReview, parameterTools, reviewSummary, reviewWarnings, sendableCount } from './ParameterTools';

const review = (text: string): ParameterReview => {
  const parsed = parameterReview(parseJson(text));
  if (parsed === null) throw new Error('the review did not parse');
  return parsed;
};

describe('ParameterToolsTest', () => {
  it("tools come from the core in the editor's order", () => {
    const tools = parameterTools(
      parseJson(
        '{"tools":[{"path":"parameterTools.refresh","label":"Refresh","confirm":false,"confirmTitle":"","confirmMessage":""},' +
          '{"path":"parameterTools.resetDefaults","label":"Reset all to firmware\'s defaults","confirm":true,"confirmTitle":"Reset All","confirmMessage":"m"},' +
          '{"path":"vehicle.rebootVehicle","label":"Reboot Vehicle","confirm":true,"confirmTitle":"Reboot Vehicle","confirmMessage":"Select Ok to reboot vehicle."}]}',
      ),
    );
    expect(tools.map((it) => it.label)).toEqual(['Refresh', "Reset all to firmware's defaults", 'Reboot Vehicle']);
    expect(tools[0]?.confirm).toBe(false);
    expect(tools[1] && confirmLabel(tools[1])).toBe('Reset');
    expect(tools[2] && confirmLabel(tools[2])).toBe('Ok');
  });

  it('the modified filter keeps only parameters changed from stock', () => {
    expect(parameterShown('RTL_ALT', [], '', true, new Set(['RTL_ALT']))).toBe(true);
    expect(parameterShown('RTL_SPEED', [], '', true, new Set(['RTL_ALT']))).toBe(false);
    expect(parameterShown('RTL_SPEED', [], 'rtl', false, new Set())).toBe(true);
  });

  it('the modified filter only applies to PX4 like ParameterEditor', () => {
    expect(modifiedFilterOn(true, true)).toBe(true);
    expect(modifiedFilterOn(true, false)).toBe(false);
    expect(modifiedFilterOn(false, true)).toBe(false);
  });

  it('a review lists the rows and warns about another vehicle', () => {
    const shown = review(
      '{"otherVehicle":true,"multipleComponents":false,"rows":[' +
        '{"name":"RTL_ALT","fileValue":"2000","vehicleValue":"1500","units":"cm","cannotSend":false},' +
        '{"name":"MP_ONLY","fileValue":"4","vehicleValue":"","units":"","cannotSend":true}]}',
    );
    expect(shown.rows.map((it) => it.name)).toEqual(['RTL_ALT', 'MP_ONLY']);
    expect(reviewWarnings(shown)).toEqual(['The parameters in the file are from a different vehicle.']);
    expect(shown.rows[0] && diffLine(shown.rows[0])).toBe('Vehicle 1500 · File 2000 · cm');
    expect(shown.rows[1] && diffLine(shown.rows[1])).toBe('Vehicle N/A — not on Vehicle · File 4');
  });

  it('the summary counts every clause like ParameterDiffDialog', () => {
    const shown = review(
      '{"parsed":9,"unchanged":3,"readOnly":1,"rows":[' + '{"name":"A","cannotSend":false},{"name":"B","cannotSend":false,"noVehicleValue":true},' + '{"name":"C","cannotSend":true,"noVehicleValue":true}]}',
    );
    expect(reviewSummary(shown)).toBe(
      'Loaded 9 parameters from file: 2 will be changed (including 1 not currently on the Vehicle), 3 already match the Vehicle, ' + '1 read-only parameter will not be sent, 1 not found on the Vehicle and cannot be sent.',
    );
    expect(sendableCount(shown)).toBe(2);
    expect(reviewSummary(ParameterReview({ rows: [], otherVehicle: false, multipleComponents: false, parsed: 1, unchanged: 1 }))).toBe('Loaded 1 parameter from file: 1 already matches the Vehicle.');
    expect(reviewSummary(ParameterReview({ rows: [], otherVehicle: false, multipleComponents: false, parsed: 2, readOnly: 2 }))).toBe('Loaded 2 parameters from file: 2 read-only parameters will not be sent.');
    expect(reviewSummary(ParameterReview({ rows: [], otherVehicle: false, multipleComponents: false, parsed: 0 }))).toBe('Loaded 0 parameters from file.');
  });

  it('check all toggles only the rows that can be sent', () => {
    const shown = review('{"rows":[{"name":"A","cannotSend":false},{"name":"C","cannotSend":true}]}');
    expect(checkedAll(shown, new Set(['1:A']), false)).toEqual(new Set());
    expect(checkedAll(shown, new Set(), true)).toEqual(new Set(['1:A']));
  });

  it('a file row is keyed by component and a parameter new to the vehicle says so', () => {
    const gimbal = ParameterDiffRow({ json: {}, name: 'MNT_TYPE', fileValue: '1', vehicleValue: '', units: '', cannotSend: false, componentId: 154, noVehicleValue: true });
    const autopilot = { ...gimbal, componentId: 1, noVehicleValue: false, vehicleValue: '0' };
    expect(diffRowKey(gimbal)).not.toBe(diffRowKey(autopilot));
    expect(diffLine(gimbal)).toBe('Vehicle N/A — new to Vehicle · File 1');
  });

  it('the count line reads like the Penpot header', () => {
    expect(parameterCountLine(1204, 3)).toBe('1,204 parameters · 3 changed');
    expect(parameterCountLine(1, 0)).toBe('1 parameter');
  });
});
