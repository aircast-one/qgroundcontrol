import { describe, expect, it } from 'vitest';
import { makeFact } from '@/bridge/Qgc';
import type { Json } from '@/bridge/Json';
import { PRIVACY_POLICY_LINK, aboutLinks } from '@/ui/AboutPage';
import { LOOKING_HINT, LOOKING_TITLE } from '@/ui/ConnectingCard';
import { factFromControl } from '@/ui/ParameterForm';
import { FLIGHT_MODES_PAGE, SAFETY_SETUP_PAGE, SENSORS } from '@/ui/SetupPages';
import { SetupComponent } from '@/ui/SetupScreen';
import {
  CONNECTIONS_PAGE,
  blockInertNote,
  editOnDesktop,
  enumLabel,
  factKeyboard,
  factSubtitle,
  factValueLines,
  fieldText,
  inertNote,
  plainNumber,
  sentenceCase,
  sharedRebootNote,
  showsAsField,
  showsAsSegments,
  shownEnumLabel,
  shownUnits,
  truncationRefusal,
  typedValue,
  valueOnTheRight,
  FLY_VIEW_GROUP,
  GROUPS_WITH_A_HEAD_EDITOR,
  PAGES_WITHOUT_A_SCREEN,
  PAGE_LOOKS,
  SettingsGroup,
  SettingsPageEntry,
  type SettingsSectionRows,
  UNITS_GROUP,
  activeLinkCount,
  activeLinksGlance,
  activeLinksText,
  blockHeading,
  borrowedTitle,
  glanceText,
  matchesIn,
  pageLook,
  pageMatches,
  devicePageMatches,
  pageTitle,
  sectionsIn,
  RADIO_ON_USB_BLOCK,
  choiceOptions,
  ALLOW_USB_PORT,
  settingsPages,
  settingsSections,
  setupSearchHits,
  shownBreadcrumb,
  shownSubtitle,
  tabPages,
  tabSetupComponents,
} from './SettingsScreen';

const page = (title: string) => SettingsPageEntry({ title, showsLinks: false, showsVideoSources: false, sectionCount: 0 });

describe('SettingsGroupsTest', () => {
  const every = [page('General'), page('Remote ID'), page('Fly View'), page('ADSB Server'), page('Video'), page('Connections'), page('Something new')];

  it('eachTabListsItsPagesInDesignOrderAndUnknownPagesLandInGeneral', () => {
    expect(tabPages(SettingsGroup.Control, every).map((it) => it.title)).toEqual(['Connections', 'Fly View']);
    expect(tabPages(SettingsGroup.Transmission, every).map((it) => it.title)).toEqual(['Connections']);
    expect(tabPages(SettingsGroup.General, every).map((it) => it.title)).toEqual(['General', 'Something new']);
  });

  it('safetyOpensWithTheGuidedLimitsItBorrowsFromFlyView', () => {
    expect(tabPages(SettingsGroup.Safety, every).map((it) => it.title)).toEqual(['Fly View', 'ADSB Server', 'Remote ID']);
  });

  const block = (title: string) => ({
    title,
    facts: [
      makeFact({ path: `settings.flyViewSettings.${title}`, name: title, description: '', units: '', valueString: '', value: 0, enumStrings: [], enumIndex: -1, isBool: true, isString: false, readOnly: false }),
    ],
  });

  const flyView: SettingsSectionRows[] = [{ title: 'Fly View', group: 'flyViewSettings', note: '', blocks: [block('Guided Commands'), block('Map and compass')] }];

  it('guidedCommandsMoveToSafetyAndLeaveTheRestOfFlyViewInControl', () => {
    expect(sectionsIn(SettingsGroup.Safety, 'Fly View', flyView).flatMap((it) => it.blocks).map((it) => it.title)).toEqual(['Guided Commands']);
    expect(sectionsIn(SettingsGroup.Control, 'Fly View', flyView).flatMap((it) => it.blocks).map((it) => it.title)).toEqual(['Map and compass']);
    expect(sectionsIn(SettingsGroup.Camera, 'Fly View', flyView)).toEqual([]);
  });

  it('theRadioOnUsbMovesFromLinksToControlBesideTheOtherStickInputs', () => {
    const connections: SettingsSectionRows[] = [{ title: 'Auto Connect', group: 'autoConnectSettings', note: '', blocks: [block(''), block('NMEA GPS'), block(RADIO_ON_USB_BLOCK)] }];
    expect(sectionsIn(SettingsGroup.Control, 'Connections', connections).flatMap((it) => it.blocks).map((it) => it.title)).toEqual([RADIO_ON_USB_BLOCK]);
    expect(sectionsIn(SettingsGroup.Transmission, 'Connections', connections).flatMap((it) => it.blocks).map((it) => it.title)).toEqual(['', 'NMEA GPS']);
  });

  it('aPortPickerOffersToAllowAUsbPortOnlyWhereTheBrowserCan', () => {
    const port = makeFact({ path: 'settings.autoConnectSettings.usbRadioPort', name: 'usbRadioPort', description: 'Device', units: '', valueString: '', value: '', enumStrings: ['Off', 'EdgeTX radio (webserial-0)'], enumIndex: 0, isBool: false, isString: true, readOnly: false, portChoice: true });
    expect(choiceOptions(port, true)).toEqual(['Off', 'EdgeTX radio (webserial-0)', ALLOW_USB_PORT]);
    expect(choiceOptions(port, false)).toEqual(['Off', 'EdgeTX radio (webserial-0)']);
    expect(choiceOptions({ ...port, portChoice: false }, true)).toEqual(['Off', 'EdgeTX radio (webserial-0)']);
  });

  it('borrowedBlocksAreHeadedByTheirOwnNames', () => {
    expect(borrowedTitle(sectionsIn(SettingsGroup.Safety, 'Fly View', flyView))).toBe('Guided commands');
  });

  it('everyKnownPageCarriesItsOwnIconAndUnknownPagesFallBackToSettings', () => {
    expect([...PAGE_LOOKS.values()].filter((it) => it.icon.length > 0).length).toBe(PAGE_LOOKS.size);
    expect(pageLook('Something new').icon).toBe('ic_settings');
  });
});

describe('LookingForAircraftTest', () => {
  it('the empty state says what to do, not what is missing', () => {
    expect(LOOKING_TITLE).toBe('Looking for your aircraft');
    expect(LOOKING_HINT.startsWith('Turn on the aircraft')).toBe(true);
  });
});

describe('TabSetupRowsTest', () => {
  const component = (name: string) => SetupComponent({ index: 0, name, needsAttention: false });

  it('a tab lists only the setup pages the vehicle reports, in tab order', () => {
    const reported = [component('Flight Modes'), component(SAFETY_SETUP_PAGE), component('Power')];
    expect(tabSetupComponents(SettingsGroup.Safety, reported).map((it) => it.name)).toEqual([SAFETY_SETUP_PAGE]);
    expect(tabSetupComponents(SettingsGroup.Control, reported).map((it) => it.name)).toEqual([FLIGHT_MODES_PAGE]);
    expect(tabSetupComponents(SettingsGroup.Camera, reported)).toEqual([]);
    expect(tabSetupComponents(SettingsGroup.Safety, [])).toEqual([]);
  });

  it('search finds setup pages by name and nothing for a blank query', () => {
    const reported = [component('Flight Modes'), component('Flight Behavior'), component(SENSORS)];
    expect(setupSearchHits(reported, 'flight').map((it) => it.name)).toEqual(['Flight Modes', 'Flight Behavior']);
    expect(setupSearchHits(reported, '  ')).toEqual([]);
  });
});

describe('ShownSubtitleTest', () => {
  it('the description hides behind help until opened and units stay', () => {
    expect(shownSubtitle('Loiter radius · m', 'Loiter radius', true, false)).toEqual({ text: 'm', hasHelp: true });
    expect(shownSubtitle('Loiter radius · m', 'Loiter radius', true, true)).toEqual({ text: 'Loiter radius · m', hasHelp: true });
    expect(shownSubtitle('Loiter radius', 'Loiter radius', true, false)).toEqual({ text: '', hasHelp: true });
  });

  it('a description containing the separator still hides whole', () => {
    expect(shownSubtitle('Speed · climb · m', 'Speed · climb', true, false)).toEqual({ text: 'm', hasHelp: true });
  });

  it('no help outside settings, without a description, or for a custom subtitle', () => {
    expect(shownSubtitle('Loiter radius · m', 'Loiter radius', false, false)).toEqual({ text: 'Loiter radius · m', hasHelp: false });
    expect(shownSubtitle('m', '', true, false)).toEqual({ text: 'm', hasHelp: false });
    expect(shownSubtitle('Custom', 'Loiter radius', true, false)).toEqual({ text: 'Custom', hasHelp: false });
  });
});

describe('SettingsViewTest', () => {
  const pages = (...raw: Json[]): Json => ({ pages: raw });

  it('a page the head draws nothing for is not offered', () => {
    const view = pages(
      { title: 'Empty', sections: [] },
      { title: 'Connections', showsLinks: true, sections: [{ title: 'Auto Connect', group: 'autoConnectSettings' }] },
      { title: 'Maps', sections: [{ title: 'Maps', group: 'mapsSettings' }] },
    );
    expect(settingsPages(view).map((it) => it.title)).toEqual(['Connections', 'Maps']);
  });

  it('the about page is offered with the help links the core serves', () => {
    const view = pages({
      title: 'About',
      showsAbout: true,
      sections: [],
      helpLinks: [{ name: 'QGroundControl User Guide', url: 'https://docs.qgroundcontrol.com', host: 'docs.qgroundcontrol.com' }],
    });
    const about = settingsPages(view);
    expect(about.map((it) => it.title)).toEqual(['About']);
    expect(about[0]?.helpLinks.map((it) => it.host)).toEqual(['docs.qgroundcontrol.com']);
  });

  it('a page with no sections is still offered when the head draws its own block', () => {
    expect(settingsPages(pages({ title: 'Connections', showsLinks: true, sections: [] })).map((it) => it.title)).toEqual(['Connections']);
  });

  it('a page this head deliberately leaves out is not offered', () => {
    const view = pages(...[...PAGES_WITHOUT_A_SCREEN.keys()].map((title) => ({ title, sections: [{ title, group: 'g' }] })));
    expect(settingsPages(view), 'each of these has a stated reason, and the generic renderer would happily draw all of them').toEqual([]);
    expect([...PAGES_WITHOUT_A_SCREEN.values()].every((it) => it.trim() !== '')).toBe(true);
  });

  it('the actions section keeps its heading but not the raw file path rows', () => {
    const view = {
      title: 'General',
      sections: [
        { title: 'Actions', group: 'mavlinkActionsSettings', subsections: [{ title: '', controls: [{ name: 'flyViewActionsFile', label: 'Fly View', control: 'text', path: 'p' }] }] },
        { title: 'Application', group: 'appSettings', subsections: [{ title: '', controls: [{ name: 'audioMuted', label: 'Mute', control: 'toggle', path: 'q' }] }] },
      ],
    };
    const sections = settingsSections(view);
    expect(sections.map((it) => it.title)).toEqual(['Actions', 'Application']);
    expect(sections[0]?.blocks.flatMap((it) => it.facts).map((it) => it.name), 'MavlinkActionsSection draws the pickers and the folder note').toEqual([]);
  });

  const general = {
    title: 'General',
    sections: [
      {
        title: 'Application',
        group: 'appSettings',
        note: '',
        subsections: [
          { title: 'Sound', controls: [{ name: 'audioMuted', label: 'Mute', control: 'toggle', path: 'settings.appSettings.audioMuted' }] },
          { title: 'Empty', controls: [] },
        ],
      },
      { title: 'Units', group: 'unitsSettings', note: '', subsections: [{ title: '', controls: [{ name: 'speedUnits', label: 'Speed', control: 'choice', path: 'settings.unitsSettings.speedUnits' }] }] },
    ],
  };

  it('a section reads as its subsections, and an empty one is dropped', () => {
    const read = settingsSections(general);
    expect(read.map((it) => it.title)).toEqual(['Application', 'Units']);
    expect(read[0]?.blocks.map((it) => it.title)).toEqual(['Sound']);
    expect(read[0]?.blocks[0]?.facts.map((it) => it.name)).toEqual(['audioMuted']);
  });

  it('a control keeps the path the write goes to', () => {
    const fact = settingsSections(general)[0]?.blocks[0]?.facts[0];
    expect(fact?.path).toBe('settings.appSettings.audioMuted');
    expect(fact?.isBool).toBe(true);
  });

  it('an inverted switch shows the opposite of the stored value', () => {
    const control = { name: 'apmStartMavlinkStreams', label: 'Controlled by Vehicle', control: 'toggle', value: true, inverted: true };
    expect(factFromControl(control)?.boolValue).toBe(false);
    expect(factFromControl({ ...control, inverted: false })?.boolValue).toBe(true);
  });

  it('the heading names the subsection, or the section when it says something the page title does not', () => {
    const read = settingsSections(general);
    expect(blockHeading('General', read[0]!, read[0]!.blocks[0]!)).toBe('Sound');
    expect(blockHeading('General', read[1]!, read[1]!.blocks[0]!)).toBe('Units');
    expect(
      blockHeading('Plan View', { title: 'Plan View', group: 'planViewSettings', note: '', blocks: [] }, { title: '', facts: [] }),
      'a page called Plan View does not need a heading called Plan View under it',
    ).toBe('');
  });

  it('a section with nothing in it is not drawn', () => {
    expect(settingsSections({ title: 'X', sections: [{ title: 'Gone', group: 'g', subsections: [{ title: '', controls: [] }] }] })).toEqual([]);
  });

  it('the video page carries the flag the extra-sources editor is drawn on', () => {
    expect(settingsPages(pages({ title: 'Video', showsVideoSources: true, sections: [{ title: 'Video', group: 'videoSettings' }] })).map((it) => it.showsVideoSources)).toEqual([true]);
    expect(
      settingsPages(pages({ title: 'Maps', sections: [{ title: 'Maps', group: 'mapsSettings' }] })).map((it) => it.showsVideoSources),
      'a page that does not ask for the block must not get it',
    ).toEqual([false]);
  });

  it('a search finds a setting by what it is called and by its name', () => {
    const read = settingsSections(general);
    expect(matchesIn('General', read, 'mute').map((it) => it.title)).toEqual(['General › Application']);
    expect(
      matchesIn('General', read, 'audioMuted').flatMap((section) => section.blocks.flatMap((it) => it.facts)).map((it) => it.name),
      'an operator who knows the fact\'s name should not have to guess the page',
    ).toEqual(['audioMuted']);
    expect(matchesIn('General', read, 'MUTE').length, 'a typed query is not case sensitive').toBe(1);
  });

  it('a section named like its page is not repeated in the breadcrumb', () => {
    expect(matchesIn('Application', settingsSections(general), 'mute').map((it) => it.title)).toEqual(['Application']);
  });

  it('an empty search matches nothing rather than everything', () => {
    const read = settingsSections(general);
    expect(matchesIn('General', read, '')).toEqual([]);
    expect(matchesIn('General', read, '   ')).toEqual([]);
  });

  it('a search result keeps the path its write goes to', () => {
    const hits = matchesIn('General', settingsSections(general), 'mute');
    expect(hits.flatMap((hit) => hit.blocks.flatMap((it) => it.facts)).map((it) => it.path)).toEqual(['settings.appSettings.audioMuted']);
    expect(hits[0]?.note, 'a note about facts the head drew elsewhere makes no sense beside one search hit').toBe('');
  });

  it('the unit rows are reached through their own section, never through search', () => {
    expect(
      matchesIn('General', settingsSections(general), 'speed'),
      'whether a single measurement may be set at all depends on the measurement system, and that gate lives in the section this head draws itself',
    ).toEqual([]);
  });

  it('the head suppresses the note only where it draws the editor itself', () => {
    expect(GROUPS_WITH_A_HEAD_EDITOR).toEqual(new Set([FLY_VIEW_GROUP]));
    expect(
      GROUPS_WITH_A_HEAD_EDITOR.has(UNITS_GROUP),
      "the core's note sends the operator to the desktop for on-screen RC controls, which is wrong on a head that has an editor for them",
    ).toBe(false);
  });

  it('theConnectionsRowCountsConnectedLinks', () => {
    const view = { links: [{ connected: true }, { connected: false }, { connected: true }] };
    expect(activeLinkCount(view)).toBe(2);
    expect(activeLinksText(2)).toBe('2 active');
    expect(activeLinksText(0)).toBe('');
    expect(activeLinkCount(null)).toBe(0);
  });

  it('aSearchBreadcrumbReadsInSentenceCase', () => {
    expect(shownBreadcrumb('Maps › Flight Map')).toBe('Maps › Flight map');
  });

  it('theConnectionsPageReadsAsLinksLikePenpot', () => {
    expect(pageTitle(CONNECTIONS_PAGE)).toBe('Links');
    expect(pageTitle('Fly View')).toBe('Fly view');
    expect(pageTitle('ADSB Server')).toBe('ADS-B server');
  });

  it('aGlanceJoinsTheSetValuesOnly', () => {
    expect(glanceText(['Bing', 'Hybrid'])).toBe('Bing · Hybrid');
    expect(glanceText(['', ' '])).toBe('');
    expect(glanceText(['Video Stream Disabled', 'Google', 'Satellite'])).toBe('Video stream disabled · Google · Satellite');
  });

  it('theConnectionsRowNamesItsLiveLinks', () => {
    const view = {
      links: [
        { name: 'UDP Link', connected: true, summary: 'UDP port 14550' },
        { name: 'Spare', connected: false, summary: 'COM3' },
        { name: 'TCP 127.0.0.1:5771', connected: true, summary: '' },
      ],
    };
    expect(activeLinksGlance(view)).toBe('UDP port 14550 · TCP 127.0.0.1:5771');
    expect(activeLinksGlance(null)).toBe('');
  });

  it('pages match on their QGC keywords and settings on theirs', () => {
    const telemetry = SettingsPageEntry({ title: 'MAVLink', showsLinks: false, showsVideoSources: false, sectionCount: 0, keywords: 'ground, station, signing, forwarding' });
    expect(pageMatches(telemetry, 'Signing')).toBe(true);
    expect(pageMatches(telemetry, 'mav')).toBe(true);
    expect(pageMatches(telemetry, 'offline')).toBe(false);
    expect(pageMatches(telemetry, ' ')).toBe(false);
  });

  it('device pages match on what their summary names, so rtsp and telemetry find the ground station links', () => {
    const links = { title: 'Ground station links', href: '/connect', icon: 'qr-code', summary: 'RTSP video, MAVLink telemetry', needs: false };
    expect(devicePageMatches(links, 'RTSP')).toBe(true);
    expect(devicePageMatches(links, 'telemetry')).toBe(true);
    expect(devicePageMatches(links, 'ground station')).toBe(true);
    expect(devicePageMatches(links, 'recordings')).toBe(false);
    expect(devicePageMatches(links, ' ')).toBe(false);
  });

  it('about lists the privacy policy after the support links', () => {
    const guide = { name: 'QGroundControl User Guide', url: 'https://docs.qgroundcontrol.com', host: 'docs.qgroundcontrol.com' };
    expect(aboutLinks([guide])).toEqual([guide, PRIVACY_POLICY_LINK]);
    expect(aboutLinks([]).map((it) => it.url)).toEqual(['https://aircast.one/privacy/android']);
  });
});

describe('FactKeyboardTest', () => {
  const fact = (min: string, { isString = false, isBool = false, whole = false } = {}) =>
    makeFact({ path: 'p', name: 'n', description: '', units: '', valueString: '0', value: 0, enumStrings: [], enumIndex: -1, isBool, isString, readOnly: false, minString: min, wholeNumbersOnly: whole });

  it('a parameter that cannot go negative gets the number pad', () => {
    expect(factKeyboard(fact('200'))).toBe('decimal');
    expect(factKeyboard(fact('0'))).toBe('decimal');
  });

  it('a parameter that can go negative keeps the full keyboard', () => {
    expect(factKeyboard(fact('-50'))).toBe('text');
    expect(factKeyboard(fact('-3.4e38'))).toBe('text');
  });

  it('an unstated minimum keeps the full keyboard rather than making a minus untypable', () => {
    expect(factKeyboard(fact(''))).toBe('text');
    expect(factKeyboard(fact('not a number'))).toBe('text');
  });

  it('text and boolean facts are never given a number pad', () => {
    expect(factKeyboard(fact('0', { isString: true }))).toBe('text');
    expect(factKeyboard(fact('0', { isBool: true }))).toBe('text');
  });

  it('a text value is readable without entering the control that writes it', () => {
    expect(
      factValueLines(fact('0', { isString: true })),
      'a comma list overflows a single-line field - Flight Modes shows six rows of ' +
        'Acro,Circle,Drift,Sport,Flip,Bra... - and reading the rest means tapping into an ' +
        'editable field, which on a phone also raises the keyboard over the page',
    ).toBe(4);
  });

  it('a number keeps one line, because a number does not overflow', () => {
    expect(factValueLines(fact('200'))).toBe(1);
    expect(factValueLines(fact('-50'))).toBe(1);
    expect(factValueLines(fact('0', { isBool: true }))).toBe(1);
  });

  it('a wrapping field must not let the return key into a settings value', () => {
    expect(typedValue('Acro,\nCircle'), 'the multi-line field is what puts Enter on the keyboard, so the newline is a hazard ' + 'this change introduces rather than one it found').toBe('Acro,Circle');
  });

  it('a fraction typed into a whole-number setting is refused, not silently truncated', () => {
    expect(
      truncationRefusal(fact('0', { whole: true }), '3.7'),
      'FactTextField validates the typed text, and QString("3.7").toInt fails, so ' + 'FactMetaData::convertAndValidateCooked answers Invalid number',
    ).toBe('Invalid number');
  });

  it('a whole number in a whole-number setting passes', () => {
    expect(truncationRefusal(fact('0', { whole: true }), '4')).toBeNull();
    expect(truncationRefusal(fact('0', { whole: true }), ' 12 ')).toBeNull();
    expect(truncationRefusal(fact('-5', { whole: true }), '-3')).toBeNull();
  });

  it('a real-typed setting still takes fractions', () => {
    expect(
      truncationRefusal(fact('0'), '3.7'),
      'decimalPlaces is a different question - a real fact declaring zero decimals is what ' +
        'you write for a percentage, and refusing fractions there would reject values the ' +
        'vehicle accepts',
    ).toBeNull();
  });

  it("text that is not a number is left to the vehicle's own validator", () => {
    expect(truncationRefusal(fact('0', { whole: true }), 'not a number')).toBeNull();
    expect(truncationRefusal(fact('0', { whole: true }), '')).toBeNull();
    expect(
      truncationRefusal(fact('-5', { whole: true }), 'NaN'),
      'Kotlin parses these to a Double, and NaN == floor(NaN) is false, so the fraction ' +
        'message fires for something that is not a fraction - and steals a specific ' +
        'complaint from the validator that can actually name the problem',
    ).toBeNull();
    expect(truncationRefusal(fact('-5', { whole: true }), 'Infinity')).toBeNull();
  });

  it('a whole-number setting that cannot go negative gets a keypad with no decimal point', () => {
    expect(factKeyboard(fact('0', { whole: true }))).toBe('number');
    expect(factKeyboard(fact('-5', { whole: true }))).toBe('text');
    expect(factKeyboard(fact('0'))).toBe('decimal');
  });
});

describe('PlainNumberTest', () => {
  it('a limit in scientific notation is spelled out', () => {
    expect(plainNumber('3e+05')).toBe('300000');
    expect(plainNumber('-1e3')).toBe('-1000');
  });

  it('a whole number loses its decimal tail', () => {
    expect(plainNumber('200')).toBe('200');
    expect(plainNumber('200.000')).toBe('200');
  });

  it('a fractional limit keeps its digits', () => {
    expect(plainNumber('0.05')).toBe('0.05');
    expect(plainNumber('1.50')).toBe('1.5');
  });

  it('a limit too large to spell out is left alone', () => {
    expect(plainNumber('3.4e38')).toBe('3.4e38');
  });

  it('anything that is not a number passes through untouched', () => {
    expect(plainNumber('')).toBe('');
    expect(plainNumber('Auto')).toBe('Auto');
  });
});

describe('EnumLabelTest', () => {
  const fact = (value: string, strings: string[], index: number) =>
    makeFact({ path: 'p', name: 'n', description: '', units: '', valueString: value, value, enumStrings: strings, enumIndex: index, isBool: false, isString: false, readOnly: true });

  it('an enum reads as its name, not its number', () => {
    expect(enumLabel(fact('6', ['Stabilize', 'Land', 'RTL'], 2))).toBe('RTL');
  });

  it('a plain value reads as itself', () => {
    expect(enumLabel(fact('42', [], -1))).toBe('42');
  });

  it('an index outside the list falls back to the value', () => {
    expect(enumLabel(fact('99', ['A', 'B'], 7))).toBe('99');
    expect(enumLabel(fact('99', ['A', 'B'], -1))).toBe('99');
  });

  it('a settings choice reads in sentence case but an unlisted value stays as typed', () => {
    expect(shownEnumLabel(fact('0', ['Video Stream Disabled', 'UDP h.264 Video Stream'], 0))).toBe('Video stream disabled');
    expect(shownEnumLabel(fact('1', ['Fit Width', 'Fit Height'], 1))).toBe('Fit height');
    expect(shownEnumLabel(fact('My Value', [], -1))).toBe('My Value');
  });
});

describe('FieldRunsTest', () => {
  const choice = (name: string, options: string[] = ['Metric', 'Imperial', 'SI', 'Nautical', 'Other'], label: string = name) =>
    makeFact({
      path: `settings.${name}`,
      name,
      description: `${name} long description`,
      units: '',
      valueString: options[0] ?? '',
      value: 0,
      enumStrings: options,
      enumIndex: 0,
      isBool: false,
      isString: false,
      readOnly: false,
      shortLabel: label,
    });

  const text = (name: string, isString = true, units = '') =>
    makeFact({ path: `settings.${name}`, name, description: name, units, valueString: '', value: '', enumStrings: [], enumIndex: -1, isBool: false, isString, readOnly: false, shortLabel: name });

  it('aSettingWithNoEffectYetStillDrawsAsAField', () => {
    const host = makeFact({ ...text('host', false), enabled: false, disabledReason: 'Has no effect while the ADS-B server connection is off.' });
    expect(showsAsField(host)).toBe(true);
    expect(showsAsField(makeFact({ ...host, readOnly: true }))).toBe(false);
  });

  it('numbersAndChoicesSitRightOfTheirLabelWhileTextTakesTheWholeRow', () => {
    expect(valueOnTheRight(text('alt', false, 'm'))).toBe(true);
    expect(valueOnTheRight(choice('units'))).toBe(true);
    expect(valueOnTheRight(text('url'))).toBe(false);
  });

  it('aNumberFieldDropsThePaddedDecimalsButTextIsLeftAlone', () => {
    expect(fieldText(makeFact({ ...text('d', false), valueString: '300.000' }))).toBe('300');
    expect(fieldText(makeFact({ ...text('d', false), valueString: '2.50' }))).toBe('2.5');
    expect(fieldText(makeFact({ ...text('s'), valueString: '007.10' }))).toBe('007.10');
  });

  it('aLabelThatNamesAnOptionIsNotShownAsTheSubtitle', () => {
    const fence = makeFact({ ...choice('FENCE_ENABLE', ['Disabled', 'Enabled'], 'Fence enable/disable'), description: 'Enabled' });
    expect(fence.detail).toBe('');
    expect(fence.heading).toBe('Fence enable/disable');
    expect(makeFact({ ...fence, description: 'Allows the fence' }).detail).toBe('Allows the fence');
  });

  it('aLabelEndingInAColonLosesIt', () => {
    expect(choice('RTL', undefined, 'Return at specified altitude:').heading).toBe('Return at specified altitude');
    expect(makeFact({ ...text('t'), description: 'Time Offset (seconds):', shortLabel: '' }).heading).toBe('Time Offset (seconds)');
  });

  it('aNoteThatOnlyRestatesTheLabelIsDropped', () => {
    const multiplier = makeFact({ ...text('BATT_VOLT_MULT', false), shortLabel: 'Voltage Multiplier', description: 'Voltage multiplier' });
    const relabelled = (shortLabel: string, description: string) => makeFact({ ...multiplier, shortLabel, description }).detail;
    expect(multiplier.detail).toBe('');
    expect(relabelled('Battery monitoring', 'Battery monitor')).toBe('');
    expect(relabelled('Required arming voltage', 'Minimum arming voltage')).toBe('Minimum arming voltage');
    expect(relabelled('ArduPilot support host', 'Ardupilot Support Host name')).toBe('');
    expect(relabelled('Color scheme', 'Application color scheme')).toBe('');
    expect(relabelled('RTL loiter time', 'Loiter time')).toBe('');
    expect(relabelled('RTL final altitude', 'Final land stage altitude')).toBe('Final land stage altitude');
    expect(relabelled('Attitude control input time constant', 'RC Roll/Pitch Feel')).toBe('RC Roll/Pitch feel');
    expect(relabelled('Confirm', 'Only while in Guided mode.')).toBe('Only while in Guided mode.');
  });

  it('aRebootNoticeSharedByTheBlockIsSaidOnce', () => {
    const reboot = makeFact({ ...text('a'), vehicleRebootRequired: true });
    expect(sharedRebootNote([reboot, text('plain'), makeFact({ ...reboot, name: 'b' })])).toBe('Reboot vehicle for changes to take effect.');
    expect(sharedRebootNote([reboot, text('plain')])).toBeNull();
    expect(sharedRebootNote([reboot, makeFact({ ...text('c'), qgcRebootRequired: true })])).toBeNull();
  });

  it('aRowLabelReadsInSentenceCaseKeepingAcronymsAndNames', () => {
    expect(sentenceCase('Mute Audio Output')).toBe('Mute audio output');
    expect(sentenceCase('Use Preflight Checklist')).toBe('Use preflight checklist');
    expect(sentenceCase('Forward MAVLink To UDP Host')).toBe('Forward MAVLink to UDP host');
    expect(sentenceCase('PX4 Pro')).toBe('PX4 Pro');
    expect(sentenceCase('3DR Solo (requires restart)')).toBe('3DR Solo (requires restart)');
    expect(sentenceCase('Yuneec Mantis G')).toBe('Yuneec Mantis G');
    expect(sentenceCase('Herelink Hotspot')).toBe('Herelink Hotspot');
    expect(sentenceCase('Mapbox Token')).toBe('Mapbox token');
    expect(sentenceCase('Automatically connect to a Pixhawk board')).toBe('Automatically connect to a Pixhawk board');
    expect(sentenceCase('Time Offset (seconds)')).toBe('Time offset (seconds)');
    expect(sentenceCase('Auto-Center throttle')).toBe('Auto-center throttle');
    expect(sentenceCase('Left-Handed mode')).toBe('Left-handed mode');
  });

  it('aBlockWhoseFieldsAreAllOffForOneReasonSaysItOnce', () => {
    const off = makeFact({ ...text('basicID'), enabled: false, disabledReason: 'Has no effect while Basic ID broadcast is off.' });
    const toggle = text('sendBasicID');
    expect(blockInertNote([toggle, off, makeFact({ ...off, name: 'uaType' })])).toBe('Has no effect while Basic ID broadcast is off.');
    expect(blockInertNote([toggle, off])).toBeNull();
  });

  it('secondsReadAsS', () => {
    expect(shownUnits('secs')).toBe('s');
    expect(shownUnits('Seconds')).toBe('s');
    expect(shownUnits('m/s')).toBe('m/s');
  });
});

describe('SegmentChoiceTest', () => {
  it('shortWritableEnumsBecomeSegments', () => {
    expect(showsAsSegments(true, false, true, ['Indoor', 'Outdoor', 'System'])).toBe(true);
  });

  it('longOrManyOptionsStayADropdown', () => {
    expect(showsAsSegments(true, false, true, ['Feet/Second', 'Meters/Second', 'Miles/Hour', 'Knots', 'Km/Hour'])).toBe(false);
    expect(showsAsSegments(true, false, true, ['A very long first option', 'And a second one'])).toBe(false);
  });

  it('readOnlyOrOffListValuesStayAsTheyAre', () => {
    expect(showsAsSegments(true, false, false, ['On', 'Off'])).toBe(false);
    expect(showsAsSegments(true, true, true, ['On', 'Off'])).toBe(false);
    expect(showsAsSegments(false, false, true, [])).toBe(false);
  });
});

describe('InertNoteTest', () => {
  const fact = (name: string, { enabled = true, readOnly = false, isBool = true } = {}) =>
    makeFact({ path: `settings.x.${name}`, name, description: '', units: '', valueString: '', value: null, enumStrings: [], enumIndex: -1, isBool, isString: false, enabled, readOnly });

  it("the core's own disabled reason is the note", () => {
    expect(inertNote(makeFact({ ...fact('enforceChecklist', { enabled: false }), disabledReason: 'Has no effect while the preflight checklist is off.' }))).toBe(
      'Has no effect while the preflight checklist is off.',
    );
    expect(inertNote(fact('enforceChecklist', { enabled: false }))).toBe('Has no effect yet');
  });

  it('a read-only fact says so', () => {
    expect(inertNote(fact('useChecklist', { readOnly: true }))).toBe('Read-only');
  });

  it('every setting is built here, so none is called inert while enabled', () => {
    expect(inertNote(fact('displayPresetsTabFirst'))).toBe('Read-only');
    expect(showsAsField(fact('displayPresetsTabFirst', { isBool: false }))).toBe(true);
  });

  it('an unknown control kind says edit on desktop', () => {
    expect(editOnDesktop(makeFact({ ...fact('useChecklist'), controlKind: 'slider' }))).toBe(true);
  });

  it('a subtitle carries only the units', () => {
    expect(factSubtitle(makeFact({ ...fact('someLiveNumber', { isBool: false }), units: 'MB' }))).toBe('MB');
    expect(factSubtitle(fact('someLiveToggle'))).toBe('');
  });
});

describe('FactSubtitleTest', () => {
  const fact = (units: string, { enums = [] as string[], bits = [] as string[], bool = false } = {}) =>
    makeFact({
      path: 'p',
      name: 'N',
      description: 'd',
      units,
      valueString: '0.15',
      value: '0.15',
      enumStrings: enums,
      enumIndex: 0,
      bitmaskStrings: bits,
      bitmaskValues: bits.map((_, at) => 2 ** at),
      isBool: bool,
      isString: false,
      readOnly: false,
    });

  it('a number keeps the unit it is measured in', () => {
    expect(factSubtitle(fact('m'))).toBe('m');
  });

  it('a row showing a word drops the unit, because Medium is not measured in seconds', () => {
    expect(factSubtitle(fact('s', { enums: ['Soft', 'Medium'] }))).toBe('');
    expect(factSubtitle(fact('s', { bits: ['Alt', 'Circle'] }))).toBe('');
    expect(factSubtitle(fact('s', { bool: true }))).toBe('');
  });
});
