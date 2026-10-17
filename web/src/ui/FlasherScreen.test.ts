import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { WifiInfo, phoneSsid, securedFrom } from '@/ui/PhoneNetwork';
import {
  FLASHER_PASSWORD_MODE,
  type FlasherDisk,
  FlasherForm,
  FlasherRelease,
  type FlasherState,
  FlasherStep,
  defaultSshMode,
  flasherCardText,
  flasherDownloadText,
  flasherPhaseLabel,
  flasherState,
  flasherStepFor,
  keepsScreenOn,
  meteredNote,
  nextSteps,
  previousStep,
  problemInMoreOptions,
  problemNote,
  readerToOpen,
  setupComplete,
  showsChannels,
  visibleProblems,
} from './FlasherScreen';

const view = parseJson(`
{"available":true,"channel":"stable","channels":["stable","staging","development"],
 "releases":{"state":"ready","error":null,"items":[{"version":"v0.3.5","label":"Aircast OS v0.3.5","prerelease":false,"published":"2026-08-22","size":576627168,"sizeText":"577 MB"}]},
 "selected":"v0.3.5",
 "form":{"hostname":"falcon-01","ssid":"field-net","wifiPassword":"","noWifi":false,"country":"GE","sshMode":"key-only","authorizedKey":"","devicePassword":"","authKey":"","controlServer":""},
 "problems":{"ssid":"Enter the WiFi network name, or choose no WiFi."},
 "keyIdentity":{"algorithm":"ssh-ed25519","comment":"operator@base","fingerprint":"SHA256:abc"},
 "summary":[{"label":"Hostname","value":"falcon-01.local","warn":false},{"label":"Device access","value":"Image default (pi / raspberry)","warn":true}],
 "disks":[{"id":"/dev/bus/usb/001/004","name":"Generic SD Reader"}],
 "card":{"state":"ready","id":"/dev/bus/usb/001/004","label":"Generic SD","capacity":16000000000,"capacityText":"16.0 GB"},
 "download":{"state":"running","version":"v0.3.5","done":10,"total":40,"percent":25.0},
 "job":{"phase":"writing","busy":true,"cancellable":true,"done":1,"total":4,"percent":25.0,"speed":12300000,"speedText":"12 MB/s","remainingText":"about 4 min left","error":null,"hostname":"falcon-01"},
 "firstProblem":{"field":"ssid","message":"Enter the WiFi network name, or choose no WiFi."},
 "canStart":false,"blocked":"A card is being written"}
`);

const parsed = (): FlasherState => {
  const read = flasherState(view);
  if (read === null) throw new Error('the view did not parse');
  return read;
};

const state = ({ phase = 'idle', busy = false, card = 'none', disks = [{ id: '/dev/bus/usb/001/004', name: 'Reader' }] }: { phase?: string; busy?: boolean; card?: string; disks?: FlasherDisk[] } = {}): FlasherState => ({
  ...parsed(),
  phase,
  busy,
  cardState: card,
  disks,
});

describe('FlasherScreenTest', () => {
  it('the_core_view_becomes_screen_state', () => {
    const read = parsed();
    expect(read.releases).toEqual([FlasherRelease({ version: 'v0.3.5', label: 'Aircast OS v0.3.5', published: '2026-08-22', sizeText: '577 MB' })]);
    expect(read.form.hostname).toBe('falcon-01');
    expect(read.form.country).toBe('GE');
    expect(read.problems.get('ssid')).toBe('Enter the WiFi network name, or choose no WiFi.');
    expect(read.keyIdentity).toBe('ssh-ed25519 SHA256:abc operator@base');
    expect(read.summary[1]).toEqual({ label: 'Device access', value: 'Image default (pi / raspberry)', warn: true });
    expect(read.cardText).toBe('Generic SD · 16.0 GB');
    expect(read.downloadText).toBe('Downloading Aircast OS… 25%');
    expect(read.downloadState).toBe('running');
    expect(read.percent).toBe(25);
    expect(read.blocked).toBe('A card is being written');
  });

  it('a_missing_or_foreign_view_is_no_state', () => {
    expect(flasherState(null)).toBeNull();
    expect(flasherState(parseJson('{"ports":[]}'))).toBeNull();
  });

  it('card_and_download_states_read_as_sentences', () => {
    expect(flasherCardText(parseJson('{"state":"waiting"}'))).toBe('Tap OK on the prompt to let Aircast use the card reader');
    expect(flasherCardText(parseJson('{"state":"failed","error":"no card in the reader"}'))).toBe('no card in the reader');
    expect(flasherCardText(parseJson('{"state":"none"}'))).toBe('');
    expect(flasherDownloadText(parseJson('{"state":"failed","error":"offline"}'))).toBe('Download failed: offline');
    expect(flasherDownloadText(parseJson('{"state":"ready","imageSizeText":"3.9 GB"}'))).toBe('Aircast OS is downloaded');
  });

  it('every_job_phase_has_a_title', () => {
    expect(['downloading', 'preparing', 'writing', 'verifying', 'customizing', 'done', 'failed', 'cancelled'].filter((phase) => flasherPhaseLabel(phase).trim() === '')).toEqual([]);
    expect(flasherPhaseLabel('idle')).toBe('');
  });

  it('a_running_or_finished_job_always_shows_the_write_step', () => {
    expect(flasherStepFor(state({ phase: 'writing', busy: true }))).toBe(FlasherStep.Write);
    expect(flasherStepFor(state({ phase: 'done' }))).toBe(FlasherStep.Write);
    expect(flasherStepFor(state())).toBeNull();
    expect(flasherStepFor(null)).toBeNull();
  });

  it('a_single_reader_opens_by_itself_but_a_choice_is_left_to_the_user', () => {
    expect(readerToOpen(state())).toBe('/dev/bus/usb/001/004');
    expect(readerToOpen(state({ card: 'opening' }))).toBeNull();
    expect(
      readerToOpen(
        state({
          disks: [
            { id: 'a', name: 'A' },
            { id: 'b', name: 'B' },
          ],
        }),
      ),
    ).toBeNull();
    expect(readerToOpen(state({ disks: [] }))).toBeNull();
    expect(readerToOpen(state({ phase: 'done' }))).toBeNull();
  });

  it('problems_show_once_a_field_is_touched_or_continue_is_pressed', () => {
    const problems = new Map([
      ['ssid', 'Enter the WiFi network name, or choose no WiFi.'],
      ['hostname', 'Use lowercase letters'],
    ]);
    expect(visibleProblems(problems, new Set(), false)).toEqual(new Map());
    expect(new Set(visibleProblems(problems, new Set(['hostname']), false).keys())).toEqual(new Set(['hostname']));
    expect(visibleProblems(problems, new Set(), true)).toEqual(problems);
    expect(setupComplete(problems)).toBe(false);
    expect(setupComplete(new Map())).toBe(true);
  });

  it('a_phone_without_a_key_signs_in_with_a_password', () => {
    expect(defaultSshMode(FlasherForm())).toBe(FLASHER_PASSWORD_MODE);
    expect(defaultSshMode(FlasherForm({ authorizedKey: 'ssh-ed25519 AAAA' }))).toBeNull();
    expect(defaultSshMode(FlasherForm({ sshMode: 'disabled' }))).toBeNull();
  });

  it('mobile_data_is_warned_about_only_before_the_download_starts', () => {
    const release = FlasherRelease({ version: 'v0.3.5', label: 'Aircast OS v0.3.5', published: '2026-08-22', sizeText: '577 MB', recommended: true });
    expect(meteredNote(release, 'idle', true)).toBe('Aircast OS is a 577 MB download and this phone is on mobile data.');
    expect(meteredNote(release, 'idle', false)).toBeNull();
    expect(meteredNote(release, 'running', true)).toBeNull();
    expect(meteredNote(release, 'ready', true)).toBeNull();
    expect(meteredNote(null, 'idle', true)).toBeNull();
  });

  it('the_phones_wifi_name_is_cleaned_of_quotes_and_placeholders', () => {
    expect(phoneSsid('"field-net"')).toBe('field-net');
    expect(phoneSsid('<unknown ssid>')).toBeNull();
    expect(phoneSsid('""')).toBeNull();
    expect(phoneSsid(null)).toBeNull();
  });

  it('the_first_problem_is_named_by_its_field_and_opens_more_options_when_it_lives_there', () => {
    const read = parsed();
    expect(problemNote(read)).toBe('WiFi network: Enter the WiFi network name, or choose no WiFi.');
    expect(problemInMoreOptions(read)).toBe(false);
    expect(problemInMoreOptions({ ...read, firstProblemField: 'controlServer' })).toBe(true);
    expect(problemNote({ ...read, firstProblem: '' })).toBe('');
    expect(read.remainingText).toBe('about 4 min left');
  });

  it('internal_channels_stay_out_of_customer_builds', () => {
    expect(showsChannels(false, 'stable')).toBe(false);
    expect(showsChannels(true, 'stable')).toBe(true);
    expect(showsChannels(false, 'staging')).toBe(true);
  });

  it('the_screen_stays_on_while_downloading_or_writing', () => {
    expect(keepsScreenOn(state({ phase: 'writing', busy: true }))).toBe(true);
    expect(keepsScreenOn({ ...parsed(), busy: false, phase: 'idle', downloadState: 'running' })).toBe(true);
    expect(keepsScreenOn({ ...parsed(), busy: false, phase: 'idle', downloadState: 'ready' })).toBe(false);
    expect(keepsScreenOn(null)).toBe(false);
  });

  it('the_ending_says_what_happens_next', () => {
    const steps = nextSteps('field-net', 'falcon-01', false);
    expect(steps).toHaveLength(3);
    expect(steps[1]).toContain('joins field-net');
    expect(steps[2]).toContain('http://falcon-01.local');
    expect(nextSteps('', 'falcon-01', true)[1]).toContain('Ethernet or cellular');
  });

  it('only_an_open_network_counts_as_unsecured', () => {
    expect(securedFrom(WifiInfo.SECURITY_TYPE_OPEN)).toBe(false);
    expect(securedFrom(WifiInfo.SECURITY_TYPE_PSK)).toBe(true);
    expect(securedFrom(WifiInfo.SECURITY_TYPE_UNKNOWN)).toBeNull();
  });

  it('back_returns_to_the_previous_step_unless_writing_or_finished', () => {
    expect(previousStep(FlasherStep.Write, false, 'idle')).toBe(FlasherStep.Setup);
    expect(previousStep(FlasherStep.Setup, false, 'idle')).toBe(FlasherStep.Card);
    expect(previousStep(FlasherStep.Card, false, 'idle')).toBeNull();
    expect(previousStep(FlasherStep.Write, true, 'writing')).toBeNull();
    expect(previousStep(FlasherStep.Write, false, 'done')).toBeNull();
  });
});
