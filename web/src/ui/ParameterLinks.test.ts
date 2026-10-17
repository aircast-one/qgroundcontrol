import { describe, expect, it } from 'vitest';
import { makeFact } from '@/bridge/Qgc';
import { parseJson } from '@/bridge/Json';
import { factFromControl } from '@/ui/ParameterForm';
import { FORCE_EDIT_NOTE, READ_ONLY_NOTE, forceEditNote, manualEntryFact, paramLinkName, parameterDefault, valueDetailsNotes } from './ParameterLinks';

describe('ParameterLinksTest', () => {
  it('a param link names the parameter and other links are left to the browser', () => {
    expect(paramLinkName('param://COM_ARM_WO_GPS')).toBe('COM_ARM_WO_GPS');
    expect(paramLinkName('param://')).toBeNull();
    expect(paramLinkName('https://docs.px4.io')).toBeNull();
  });

  it('a read-only parameter says so until force edit is on', () => {
    expect(forceEditNote(false, false)).toBeNull();
    expect(forceEditNote(true, false)).toBe(READ_ONLY_NOTE);
    expect(forceEditNote(true, true)).toBe(FORCE_EDIT_NOTE);
  });

  it('reset offers the default only when the metadata has one', () => {
    expect(parameterDefault(parseJson('{"defaultValueAvailable":true,"defaultValue":20}'))).toBe(20);
    expect(parameterDefault(parseJson('{"defaultValueAvailable":false,"defaultValue":20}'))).toBeNull();
    expect(parameterDefault(parseJson('{"defaultValueAvailable":true,"defaultValue":null}'))).toBeNull();
  });

  it('manual entry drops the choice lists', () => {
    const fact = makeFact({ path: 'p', name: 'MODE', description: '', units: '', valueString: '1', value: 1, enumStrings: ['A', 'B'], enumValues: ['0', '1'], enumIndex: 1, isBool: false, isString: false, readOnly: false });
    expect(manualEntryFact(fact).isEnum).toBe(false);
  });

  it("value details read the control's description and default like the landing altitude dialog", () => {
    const control = parseJson(
      '{"path":"item.finalApproachAltitude","name":"FinalApproachAltitude","label":"Altitude","valueDetails":"Altitude to begin landing approach from.","units":"m","valueString":"40.0","value":40.0,"control":"number","readOnly":false,"minimumText":null,"maximumText":null,"defaultText":"40.0"}',
    );
    const fact = control === null ? null : factFromControl(control);
    expect(fact?.valueDetails).toBe('Altitude to begin landing approach from.');
    expect(fact === null ? null : valueDetailsNotes(fact)).toEqual(['Altitude to begin landing approach from.', 'default 40.0 m']);
  });
});
