import { describe, expect, it } from 'vitest';
import { type Json } from '@/bridge/Json';
import { makeFact } from '@/bridge/Qgc';
import { format } from '@/kotlin/Std';
import { parameterPath } from '@/ui/ParametersScreen';
import { bitmaskEntryEnabled, bitmaskToggled, factConstraintNote, factKeyboard, factRebootNote, inertNote, isAddress, ratioText, ratioValue, validationMessage } from '@/ui/SettingsScreen';
import { sectionImageAsset } from '@/ui/SetupRow';
import { setupGroups, setupPage, setupPagePath } from '@/ui/SetupView';
import { KNOWN_CONTROL_KINDS, ParameterRows, bitmaskRaw, bitmaskSummary, controlIsUnderstood, factFromControl, factFromParameter, readOnlyNote, sectionMatches, sliderSnapped, sliderTitle, sliderValue } from './ParameterForm';

const control = (extra: Json = {}): Json => ({
  kind: 'control',
  path: 'settings.appSettings.enforceChecklist',
  name: 'enforceChecklist',
  label: 'Enforce checklist',
  control: 'toggle',
  value: true,
  ...extra,
});

const ranged = ({ min = '', max = '', minDefault = true, maxDefault = true, fallback = '', vehicleReboot = false, qgcReboot = false } = {}) =>
  makeFact({
    path: 'p', name: 'p', description: '', units: '', valueString: '1',
    value: 1, enumStrings: [], enumIndex: 0,
    isBool: false, isString: false, readOnly: false,
    minString: min, maxString: max,
    minIsDefaultForType: minDefault, maxIsDefaultForType: maxDefault,
    defaultValueString: fallback,
    vehicleRebootRequired: vehicleReboot, qgcRebootRequired: qgcReboot,
  });

const required = <T,>(value: T | null): T => {
  if (value === null) throw new Error('expected a value');
  return value;
};

describe('ParameterFormTest', () => {
  it('a control served without the gate is writable, because every older core omits it', () => {
    const fact = required(factFromControl(control()));
    expect(
      fact.enabled,
      'enabled and disabledReason are additive; every control from a core that predates them arrives with neither, and a head that read a missing gate as false would grey out the whole of Settings',
    ).toBe(true);
    expect(fact.acceptsWrite).toBe(true);
  });

  it("a section's icon is read from the asset folder the build copies it into", () => {
    expect(sectionImageAsset('Battery.svg')).not.toBe('');
    expect(sectionImageAsset('NotCopied.svg')).toBe('');
  });

  it('an indented small-font help row keeps both cues', () => {
    const help = required(factFromControl(control({ indent: true, smallFont: true })));
    expect(help.indent && help.smallFont).toBe(true);
    const plain = required(factFromControl(control()));
    expect(plain.indent || plain.smallFont).toBe(false);
  });

  it('a control the core says is inert is not offered, and says why', () => {
    const fact = required(factFromControl(control({ enabled: false, disabledReason: 'Turn on Use preflight checklist first' })));
    expect(fact.acceptsWrite).toBe(false);
    expect(inertNote(fact)).toBe('Turn on Use preflight checklist first');
  });

  it('read-only and inert are different answers and say different things', () => {
    const readOnly = required(factFromControl(control({ readOnly: true })));
    expect(inertNote(readOnly)).toBe('Read-only');
    const inert = required(factFromControl(control({ enabled: false })));
    expect(inertNote(inert), 'the flag decides, never the presence of the string - a reason drawn whenever it is non-empty can explain why a row is off while it is on').toBe('Has no effect yet');
  });

  it('a fact response becomes a fact addressed by its parameter path', () => {
    const json = { kind: 'fact', name: 'RTL_ALT', shortDescription: 'Return altitude', units: 'cm', valueString: '1500', value: 1500, typeIsBool: false, readOnly: false };
    const fact = required(factFromParameter('RTL_ALT', json));
    expect(fact.path).toBe(parameterPath('RTL_ALT'));
    expect(fact.name).toBe('RTL_ALT');
    expect(fact.description).toBe('Return altitude');
    expect(fact.units).toBe('cm');
  });

  it('a parameter with no description is labelled by its name', () => {
    const fact = required(factFromParameter('MNT_ANGMIN_PAN', { kind: 'fact', name: 'MNT_ANGMIN_PAN', shortDescription: '', valueString: '0' }));
    expect(fact.name).toBe('MNT_ANGMIN_PAN');
    expect(fact.title).toBe('MNT_ANGMIN_PAN');
  });

  it('a description still wins over the name', () => {
    expect(factFromParameter('RTL_ALT', { kind: 'fact', name: 'RTL_ALT', shortDescription: 'Return altitude' })?.title).toBe('Return altitude');
  });

  it('a parameter the vehicle does not have is skipped', () => {
    expect(factFromParameter('NOPE', { kind: 'value', value: null })).toBeNull();
    expect(factFromParameter('NOPE', { kind: 'null' })).toBeNull();
    expect(factFromParameter('NOPE', {})).toBeNull();
    expect(
      factFromParameter('RTL_RETURN_ALT', { kind: 'fact', name: '', shortDescription: '', valueString: '0', value: 0 }),
      'the core answers a parameter it does not have with an unnamed 0, which must not pass for a value',
    ).toBeNull();
  });

  it('an empty error from QGC means the value is acceptable', () => {
    expect(validationMessage('')).toBeNull();
    expect(validationMessage('   ')).toBeNull();
  });

  it("QGC's own wording is passed through unchanged", () => {
    expect(validationMessage('Value must be within 0 and 100')).toBe('Value must be within 0 and 100');
  });

  it('a bridge call that returned nothing does not block the write', () => {
    expect(validationMessage(null)).toBeNull();
  });

  it('a non-string result is not treated as an error', () => {
    expect(validationMessage(false)).toBeNull();
    expect(validationMessage(0)).toBeNull();
  });

  it("a limit that is only the type's own limit is not shown", () => {
    expect(factConstraintNote(ranged({ min: '0', max: '4294967295' }))).toBeNull();
  });

  it('a real constraint is shown', () => {
    expect(factConstraintNote(ranged({ max: '100', maxDefault: false }))).toBe('Max 100');
  });

  it('min max and default read together', () => {
    expect(factConstraintNote(ranged({ min: '6', max: '48', minDefault: false, maxDefault: false, fallback: '14' }))).toBe('Min 6 · Max 48 · Default 14');
  });

  it('a vehicle reboot is named ahead of an app restart', () => {
    expect(factRebootNote(ranged({ vehicleReboot: true, qgcReboot: true }))).toBe('Reboot vehicle for changes to take effect.');
  });

  it('an app restart is named when only that is required', () => {
    expect(factRebootNote(ranged({ qgcReboot: true }))).toBe('Restart Aircast for this to take effect.');
  });

  it('a parameter needing no restart says nothing', () => {
    expect(factRebootNote(ranged())).toBeNull();
  });

  it('a bitmask on a setup page names its bits instead of showing a number', () => {
    const fact = required(
      factFromControl({
        control: 'bitmask', name: 'SIMPLE', label: 'Simple mode bitmask', path: 'p',
        value: 5, valueString: '5', display: '5',
        bits: [{ label: 'SwitchPos1', raw: '1', set: true }, { label: 'SwitchPos2', raw: '2', set: false }, { label: 'SwitchPos3', raw: '4', set: true }],
      }),
    );
    expect(fact.isBitmask).toBe(true);
    expect(fact.bitmaskValues).toEqual([1, 2, 4]);
    expect(bitmaskSummary(fact)).toBe('SwitchPos1, SwitchPos3');
  });

  it('a fact the core called a choice stays a choice even when it carries bits', () => {
    const fact = required(
      factFromControl({
        control: 'choice', name: 'FS_OPTIONS', label: 'Failsafe options', path: 'p',
        value: 1, valueString: '1', display: 'Continue',
        options: [{ label: 'None', raw: '0' }, { label: 'Continue', raw: '1' }],
        bits: [{ label: 'RC', raw: '1', set: true }],
      }),
    );
    expect(fact.isBitmask, 'the core decides the control kind, the head does not re-derive it').toBe(false);
    expect(fact.isEnum).toBe(true);
  });
});

describe('ReadOnlyNoteTest', () => {
  const fact = (name: string, readOnly: boolean) =>
    makeFact({ path: `p/${name}`, name, description: '', units: '', valueString: '0', value: 0, enumStrings: [], enumIndex: -1, isBool: false, isString: false, readOnly });

  it('nothing read-only says nothing', () => {
    expect(readOnlyNote([fact('FLTMODE1', false), fact('FLTMODE2', false)])).toBeNull();
  });

  it('all read-only says so without naming them', () => {
    expect(readOnlyNote([fact('A', true), fact('B', true)])).toBe('This firmware reports all of these as read-only, so they are shown for reference.');
  });

  it('some read-only names which ones', () => {
    expect(readOnlyNote([fact('FLTMODE1', false), fact('FLTMODE2', true), fact('FLTMODE3', true)])).toBe('This firmware reports FLTMODE2, FLTMODE3 as read-only, so they are shown but cannot be changed here.');
  });

  it('the setup pages come from the core, grouped and flagged', () => {
    const view = {
      groups: [
        { title: 'Vehicle', pages: [{ name: 'Frame', openable: true, blockedReason: null, parameterSections: true }, { name: 'Motors', openable: true, blockedReason: null, parameterSections: false }] },
        { title: 'Support', pages: [{ name: 'Remote Support', openable: true, blockedReason: null, parameterSections: false }] },
      ],
    };

    expect(setupGroups(view).map((it) => it.title)).toEqual(['Vehicle', 'Support']);
    expect(setupPage(view, 'Frame')?.parameterSections).toBe(true);
    expect(setupPage(view, 'Remote Support')?.parameterSections).toBe(false);
    expect(setupPage(view, 'A page this firmware does not have')).toBeNull();
  });

  it("a control becomes a row with the core's label and options", () => {
    const fact = required(
      factFromControl({
        class: 'Control', control: 'choice', label: 'Frame Class',
        name: 'FRAME_CLASS', display: 'Quad', value: 1, valueString: '1', units: '',
        path: 'vehicle.parameterManager.getParameter(-1,FRAME_CLASS)', readOnly: false,
        rebootRequired: true, vehicleRebootRequired: true, applicationRestartRequired: false,
        options: [{ label: 'Undefined', raw: '0' }, { label: 'Quad', raw: '1' }],
      }),
    );

    expect(fact.title).toBe('Frame Class');
    expect(fact.enumStrings).toEqual(['Undefined', 'Quad']);
    expect(fact.enumIndex).toBe(1);
    expect(fact.vehicleRebootRequired).toBe(true);
  });

  it('an app setting that needs an app restart does not ask for a vehicle reboot', () => {
    const fact = required(
      factFromControl({
        class: 'Control', control: 'text', label: 'Host name', name: 'forwardMavlinkHostName',
        display: 'localhost:14445', value: 'localhost:14445', valueString: 'localhost:14445', units: '',
        path: 'settings.mavlinkSettings.forwardMavlinkHostName', readOnly: false,
        rebootRequired: true, vehicleRebootRequired: false, applicationRestartRequired: true,
      }),
    );

    expect(fact.vehicleRebootRequired).toBe(false);
    expect(fact.qgcRebootRequired).toBe(true);
    expect(factRebootNote(fact)).toBe('Restart Aircast for this to take effect.');
  });

  it('a control the vehicle does not have is dropped rather than shown blank', () => {
    expect(
      factFromControl({ class: 'Control', control: 'number', label: '', name: '', display: '0', value: 0, valueString: '0', units: '', path: 'vehicle.parameterManager.getParameter(-1,BATT_MONITOR)' }),
    ).toBeNull();
  });

  it('the page path carries the name and no separators the watch would split', () => {
    expect(setupPagePath('Flight Modes')).toBe('view.setup(Flight Modes)');
    expect(setupPagePath('Flight Modes').includes(',')).toBe(false);
  });
});

describe('ControlBoundsTest', () => {
  const shown = (value: number | null) => (value === null ? null : format('%.2f', value));

  const bounded = (minimum: number | null, maximum: number | null, fallback: number | null) =>
    required(
      factFromControl({
        kind: 'object', class: 'Control', control: 'number', name: 'FENCE_ALT_MAX',
        label: 'Fence maximum altitude', path: 'p', units: 'm', value: 100.0,
        valueString: '100.00', options: [], bits: [], readOnly: false,
        minimum, maximum, defaultValue: fallback,
        minimumText: shown(minimum), maximumText: shown(maximum),
        defaultText: shown(fallback),
      }),
    );

  it('a control that declares bounds carries them into the fact', () => {
    expect(factConstraintNote(bounded(10.0, 1000.0, 100.0))).toBe('Min 10 · Max 1000 · Default 100');
  });

  it('a bound the fact does not declare is not printed as the type\'s extreme', () => {
    const fact = bounded(10.0, null, 100.0);

    expect(factConstraintNote(fact)).toBe('Min 10 · Default 100');
    expect(fact.maxIsDefaultForType).toBe(true);
    expect(fact.minIsDefaultForType).toBe(false);
  });

  it('a control with no bounds at all has no note rather than an empty one', () => {
    expect(factConstraintNote(bounded(null, null, null))).toBeNull();
  });

  it('a bound that can go negative asks for a keyboard that can type a minus', () => {
    expect(factKeyboard(bounded(-180.0, 180.0, 0.0))).toBe('text');
    expect(factKeyboard(bounded(0.0, 180.0, 0.0))).toBe('decimal');
  });

  it('a stream address gets the URL keyboard and no autocorrect', () => {
    const address = makeFact({ ...bounded(null, null, null), name: 'whepUrl', isString: true });
    expect(factKeyboard(address)).toBe('uri');
    expect(isAddress(address)).toBe(true);
    expect(isAddress(makeFact({ ...address, name: 'primaryCameraName' }))).toBe(false);
  });

  it('an aspect ratio reads and accepts width colon height', () => {
    expect(ratioText(1.777777)).toBe('16:9');
    expect(ratioText(1.333333)).toBe('4:3');
    expect(ratioText(1.9)).toBeNull();
    expect(ratioValue('16:9')).toBe('1.777778');
    expect(ratioValue(' 4 : 3 ')).toBe('1.333333');
    expect(ratioValue('1.85')).toBeNull();
    expect(ratioValue('16:0')).toBeNull();
  });

  it('a tuning slider reads its setup label and the live value like the Penpot tuning frame', () => {
    const fact = makeFact({ ...bounded(0.0, 1.0, 0.5), description: 'Climb Sensitivity', shortLabel: 'Acceleration (vertical) controller P gain' });
    expect(sliderTitle(fact)).toBe('Climb sensitivity');
    expect(sliderValue(0.1349, 3, '')).toBe('0.135');
    expect(sliderValue(38, 0, '%')).toBe('38 %');
  });

  it('a slider over a whole-number fact lands on whole numbers', () => {
    expect(sliderSnapped(12345.68, 0)).toBe(12346);
    expect(sliderSnapped(0.1351, 2)).toBe(0.14);
  });
});

describe('BitmaskTest', () => {
  const arming = (value: number) =>
    makeFact({
      path: 'p', name: 'ARMING_CHECK', description: 'Arm Checks to Perform', units: '',
      valueString: `${value}`, value,
      enumStrings: [], enumIndex: -1,
      isBool: false, isString: false, readOnly: false,
      bitmaskStrings: ['All', 'Barometer', 'Compass', 'GPS lock'],
      bitmaskValues: [1, 2, 4, 8],
    });

  it('a bitmask is not an enum, so it does not get a single-choice picker', () => {
    const fact = arming(0);

    expect(fact.isBitmask).toBe(true);
    expect(fact.isEnum).toBe(false);
  });

  it('no bits set says so instead of showing zero', () => {
    expect(bitmaskSummary(arming(0))).toBe('None');
  });

  it('the set checks are named, which is the whole point', () => {
    expect(bitmaskSummary(arming(2 | 8))).toBe('Barometer, GPS lock');
  });

  it('every bit set reads as All rather than a list of everything', () => {
    expect(bitmaskSummary(arming(1 | 2 | 4 | 8))).toBe('All');
  });

  it('a value arriving as a string is still read', () => {
    const asText = makeFact({ ...arming(6), value: null, valueString: '6.000' });

    expect(bitmaskSummary(asText)).toBe('Barometer, Compass');
  });

  it('toggling a bit flips only that bit', () => {
    const raw = bitmaskRaw(arming(2 | 8));

    expect(raw ^ 4).toBe(2 | 4 | 8);
    expect(raw ^ 2).toBe(8);
  });

  it('a fact whose bit names and values disagree is not treated as a bitmask', () => {
    const ragged = makeFact({ ...arming(0), bitmaskValues: [1, 2] });

    expect(ragged.isBitmask).toBe(false);
  });

  it('checking All clears and locks the rest, like FactBitmask firstEntryIsAll', () => {
    const fact = makeFact({ ...arming(2 | 8), firstEntryIsAll: true });
    expect(bitmaskToggled(fact, 2 | 8, 0)).toBe(1);
    expect(bitmaskEntryEnabled(fact, 1, 2)).toBe(false);
    expect(bitmaskEntryEnabled(fact, 0, 2)).toBe(true);
    expect(bitmaskToggled(fact, 1, 0), 'unchecking All only clears its own bit').toBe(0);
    expect(bitmaskToggled(arming(2), 2, 0), 'without the flag All is an ordinary bit').toBe(2 | 1);
  });
});

describe('ControlKindTest', () => {
  it('the head knows exactly the kinds the core enumerates', () => {
    expect(KNOWN_CONTROL_KINDS).toEqual(new Set(['toggle', 'choice', 'bitmask', 'text', 'number', 'label', 'dialog', 'button']));
  });

  it('every enumerated kind is rendered rather than refused', () => {
    expect([...KNOWN_CONTROL_KINDS].filter((it) => !controlIsUnderstood(it))).toEqual([]);
  });

  it('a kind the core adds later is not given an editable control', () => {
    expect(controlIsUnderstood('slider')).toBe(false);
    expect(controlIsUnderstood('colour')).toBe(false);
  });

  it('a control with no kind at all is still editable, because that is every other path', () => {
    expect(controlIsUnderstood('')).toBe(true);
  });

  it('the kind is carried from the projection onto the fact', () => {
    expect(factFromControl({ label: 'Arm Checks', name: 'ARMING_CHECK', path: 'p', control: 'bitmask', value: 82, valueString: '82' })?.controlKind).toBe('bitmask');
  });
});

describe('OptionalFactTest', () => {
  it('an optional field reads switched off while it holds no number', () => {
    const off = required(factFromControl({ name: 'Yaw', path: 'p', optional: true, value: null, valueText: '' }));
    expect(off.optional).toBe(true);
    expect(off.optionalSet).toBe(false);
    const on = required(factFromControl({ name: 'Yaw', path: 'p', optional: true, value: 15.0, valueText: '15.0' }));
    expect(on.optionalSet).toBe(true);
  });
});

describe('SectionMatchesTest', () => {
  const gcs = ParameterRows({ title: 'Ground Station Failsafe', facts: [], note: '', keywords: ['heartbeat', 'fs_gcs_timeout'] });

  it('matchesTitleOrKeywordIgnoringCase', () => {
    expect(sectionMatches(gcs, '  station ')).toBe(true);
    expect(sectionMatches(gcs, 'HEARTBEAT')).toBe(true);
    expect(sectionMatches(gcs, '')).toBe(true);
    expect(sectionMatches(gcs, 'battery')).toBe(false);
  });
});
