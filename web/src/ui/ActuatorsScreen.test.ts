import { describe, expect, it } from 'vitest';
import { GeometryCell, actuatorOutputs, bitsetChecked, bitsetWritten, geometry, geometryCell, mixerEditable, motorAssignment, plainMessage, signWritten } from './ActuatorsScreen';

describe('ActuatorsScreenTest', () => {
  it("bitset and true-if-positive write the raw value as QGC's wrapper facts do", () => {
    expect(bitsetChecked(5.0, 2)).toBe(true);
    expect(bitsetChecked(5.0, 1)).toBe(false);
    expect(bitsetWritten(5.0, 1, true)).toBe(7);
    expect(bitsetWritten(5.0, 2, false)).toBe(1);
    expect(signWritten(-1500.0, true)).toBe(1500.0);
    expect(signWritten(1500.0, false)).toBe(-1500.0);
  });

  it('groups subgroups and channel configs read from the view', () => {
    const read = actuatorOutputs(
      JSON.parse(
        '{"class":"ActuatorOutputs","available":true,"showUi":true,"groups":[{"label":"MAIN","enable":null,"groupsVisible":true,"params":[],' +
          '"subgroups":[{"label":"MAIN 1-4","primary":null,"params":[],"columns":[{"label":"Function","advanced":false,"visible":true}],' +
          '"channels":[{"label":"MAIN 1","configs":[null]}]}]}]}',
      ),
    );
    expect(read).not.toBeNull();
    expect(read?.groups).toHaveLength(1);
    const subgroup = read?.groups[0]?.subgroups[0];
    expect(read?.groups[0]?.subgroups).toHaveLength(1);
    expect(subgroup?.columns).toHaveLength(1);
    expect(subgroup?.columns[0]?.label).toBe('Function');
    expect(subgroup?.channels).toHaveLength(1);
    expect(subgroup?.channels[0]?.configs).toEqual([null]);
  });
});

describe('ActuatorGeometryTest', () => {
  it('fixed geometry cells read as read-only values', () => {
    const read = geometry(
      JSON.parse(
        '{"title":"Geometry: Tiltrotor","helpUrl":"","groups":[{"label":"Motors","count":null,"params":[],' +
          '"channels":[{"label":"Rear Motor (Motor 3)","cells":[{"fixed":true,"label":"Position X","valueString":"-0.7500","advanced":false},null]}]}]}',
      ),
    );
    expect(read?.groups).toHaveLength(1);
    expect(read?.groups[0]?.channels).toHaveLength(1);
    const channel = read?.groups[0]?.channels[0];
    expect(channel?.label).toBe('Rear Motor (Motor 3)');
    expect(channel?.cells[0]).toEqual(GeometryCell.Fixed('Position X', '-0.7500', false));
    expect(channel?.cells[1]).toBeNull();
  });
});

describe('ActuatorMixerCellTest', () => {
  it('an axis cell and a rule-hidden cell read from the geometry', () => {
    const axis = geometryCell(JSON.parse('{"axis":true,"options":["Custom","Upwards"],"index":1,"params":["CA_MC_R0_AX","CA_MC_R0_AY","CA_MC_R0_AZ"],"hidden":false,"disabled":false,"advanced":true}'));
    expect(axis).toEqual(GeometryCell.Axis(['Custom', 'Upwards'], 1, ['CA_MC_R0_AX', 'CA_MC_R0_AY', 'CA_MC_R0_AZ'], true, false, false));
    const hidden = geometryCell(JSON.parse('{"fixed":true,"label":"Pitch Torque","valueString":"0.0000","advanced":false,"hidden":true}'));
    expect(hidden?.hidden).toBe(true);
  });

  it('the mixer is editable only while sliders are off and no motor assignment runs', () => {
    expect(mixerEditable(false, false)).toBe(true);
    expect(mixerEditable(true, false)).toBe(false);
    expect(mixerEditable(false, true)).toBe(false);
  });

  it('a cell with a missing parameter reads as not available', () => {
    const missing = geometryCell(JSON.parse('{"unavailable":true,"label":"Roll Torque","advanced":false,"hidden":false,"disabled":false,"channelFunction":201,"param":"CA_SV_CS0_TRQ_R"}'));
    expect(missing).toEqual(GeometryCell.Unavailable('Roll Torque', false, false));
  });
});

describe('MotorAssignmentHeadTest', () => {
  it('the assignment message loses its markup and the highlighted motors read as a set', () => {
    expect(plainMessage('a<br /><br /><b>Warning</b><br />b')).toBe('a\n\nWarning\nb');
    const state = motorAssignment(JSON.parse('{"multirotor":true,"enabled":true,"active":true,"message":"m","highlighted":[1,3]}'));
    expect(state.highlighted).toEqual(new Set([1, 3]));
    expect(state.active).toBe(true);
  });
});
