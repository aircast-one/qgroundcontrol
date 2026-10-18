import { useEffect, useRef, useState } from 'react';
import { Qgc } from '@/bridge/Qgc';
import { useQgcBool, useQgcFacts } from '@/bridge/QgcState';
import { type Json, isNull, jsonObjects, jsonStrings, optBoolean, optDouble, optInt, optJSONArray, optText } from '@/bridge/Json';
import { requestSerialPort, serialAvailable } from '@/core/client';
import { stageFile } from '@/core/files';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { openDocument } from '@/kotlin/Platform';
import { delay, replaceFirstCharUpper, same } from '@/kotlin/Std';
import { Button, Checkbox, LinearProgressIndicator, ListItem, OutlinedButton, RadioButton, Surface, Text, TextButton } from '@/m3';
import { aircast, md } from '@/theme/Colors';
import { ChoiceField, FactRow } from '@/ui/SettingsScreen';

export const FIRMWARE_VIEW = 'view.firmwareUpgrade';
export const FIRMWARE_PORTS_VIEW = 'view.firmwarePorts';
export const FIRMWARE_FLASH = 'firmware.flash';
export const FIRMWARE_CANCEL = 'firmware.cancel';
export const FIRMWARE_CHOOSE = 'firmware.choose';
export const FIRMWARE_UPGRADE_SETTINGS = 'settings.firmwareUpgradeSettings';
export const APM_CHIBIOS = 'apmChibiOS';
export const FIRMWARE_POLL_MS = 500;
export const FIRMWARE_EXTENSIONS: readonly string[] = ['px4', 'apj', 'bin', 'ihx'];
const STAGED_PREFIX = '/cache/firmware-';
export const DEVICE_PORT = 'aircast-device';

export type FirmwarePort = { port: string; description: string; bootloader: boolean; boardType: string };

export const FirmwarePort = (fields: { port: string; description: string; bootloader: boolean; boardType?: string }): FirmwarePort => ({ boardType: '', ...fields });

export const MULTIPLE_DEVICES = 'Multiple devices detected. Make sure to select the correct one from the list.';

export const preselectedPort = (ports: readonly FirmwarePort[], current: string): string | null =>
  ports.find((it) => it.port === current && it.boardType.trim() !== '')?.port ??
  (ports.find((it) => it.boardType === 'Pixhawk') ?? ports.find((it) => it.boardType === 'SiK Radio'))?.port ??
  null;

export type FirmwareJob = {
  phase: string;
  busy: boolean;
  cancellable: boolean;
  progress: number;
  messages: readonly string[];
  error: string;
  choices: readonly (readonly [string, string])[];
  bestChoice: number;
  updateAvailable: string;
  px4StableVersion: string;
  px4BetaVersion: string;
};

export const FirmwareJob = (
  fields: Pick<FirmwareJob, 'phase' | 'busy' | 'cancellable' | 'progress' | 'messages' | 'error'> & Partial<Omit<FirmwareJob, 'phase' | 'busy' | 'cancellable' | 'progress' | 'messages' | 'error'>>,
): FirmwareJob => ({ choices: [], bestChoice: -1, updateAvailable: '', px4StableVersion: '', px4BetaVersion: '', ...fields });

export const firmwarePorts = (view: Json | null): FirmwarePort[] =>
  jsonObjects(optJSONArray(view, 'ports')).flatMap((entry) => {
    const port = optText(entry, 'port');
    return port.trim() === '' ? [] : [FirmwarePort({ port, description: optText(entry, 'description'), bootloader: optBoolean(entry, 'bootloader'), boardType: optText(entry, 'boardType') })];
  });

export const firmwareJob = (view: Json | null): FirmwareJob | null =>
  view === null || optText(view, 'class') !== 'FirmwareUpgrade'
    ? null
    : FirmwareJob({
        phase: optText(view, 'phase').trim() === '' ? 'idle' : optText(view, 'phase'),
        busy: optBoolean(view, 'busy'),
        cancellable: optBoolean(view, 'cancellable'),
        progress: optDouble(view, 'progress', 0),
        messages: jsonStrings(optJSONArray(view, 'messages')),
        error: optText(view, 'error'),
        choices: jsonObjects(optJSONArray(view, 'choices')).map((it) => [optText(it, 'name'), optText(it, 'url')] as const),
        updateAvailable: optText(view, 'updateAvailable'),
        bestChoice: isNull(view, 'bestChoice') ? -1 : optInt(view, 'bestChoice'),
        px4StableVersion: optText(view, 'px4StableVersion'),
        px4BetaVersion: optText(view, 'px4BetaVersion'),
      });

export const cancelledOnLeave = (phase: string | null): boolean => phase === 'connecting' || phase === 'choosing';

const PHASE_TEXT: Readonly<Record<string, string>> = {
  connecting: 'Waiting for the bootloader',
  choosing: 'Choose the firmware build',
  erasing: 'Erasing',
  programming: 'Programming',
  verifying: 'Verifying',
  complete: 'Upgrade complete',
  failed: 'Upgrade failed',
};

export const firmwarePhaseText = (phase: string): string => PHASE_TEXT[phase] ?? '';

export const FIRMWARE_FROM_FILE = 'file';
export const FLASH_FAIL_TEXT =
  'If upgrade failed, make sure to connect directly to a powered USB port on your computer, not through a USB hub. Also make sure you are only powered via USB not battery.';

const APM_VEHICLES: readonly string[] = ['copter', 'heli', 'plane', 'rover', 'sub'];

export const FIRMWARE_SOURCES: readonly (readonly [string, string])[] = [
  [FIRMWARE_FROM_FILE, 'A firmware file'],
  ['px4:stable', 'PX4 Pro, stable'],
  ['px4:beta', 'PX4 Pro, beta'],
  ['px4:dev', 'PX4 Pro, dev'],
  ['sik:stable', 'SiK radio, stable'],
  ...APM_VEHICLES.flatMap((vehicle) => ['stable', 'beta', 'dev'].map((build) => [`ardupilot:${vehicle}:${build}`, `ArduPilot ${replaceFirstCharUpper(vehicle)}, ${build}`] as const)),
];

export const DEFAULT_FIRMWARE_SOURCE = 'px4:stable';
const FIRMWARE_TYPE_PX4 = 12;
const FIRMWARE_TYPE_APM = 3;
const DEFAULT_FIRMWARE_TYPE = 'defaultFirmwareType';
const APM_VEHICLE_TYPE = 'apmVehicleType';

export const rememberedSource = (firmwareType: number | null, apmVehicleType: number | null): string =>
  firmwareType === FIRMWARE_TYPE_APM ? `ardupilot:${APM_VEHICLES[apmVehicleType ?? 0] ?? APM_VEHICLES[0]}:stable` : DEFAULT_FIRMWARE_SOURCE;

export const sourceSettings = (source: string): (readonly [string, number])[] => {
  const parts = source.split(':');
  if (parts[0] === 'px4') return [[DEFAULT_FIRMWARE_TYPE, FIRMWARE_TYPE_PX4]];
  if (parts[0] !== 'ardupilot') return [];
  const vehicle = APM_VEHICLES.indexOf(parts[1] ?? '');
  return [[DEFAULT_FIRMWARE_TYPE, FIRMWARE_TYPE_APM], ...(vehicle >= 0 ? [[APM_VEHICLE_TYPE, vehicle] as const] : [])];
};

export const APM_FIRMWARE = 'vehicle.apmFirmware';
export const FLASH_BOOTLOADER = 'vehicle.flashBootloader';

export const bootloaderOffered = (advanced: boolean, apmVehicle: boolean): boolean => advanced && apmVehicle;

export const firmwareSources = (advanced: boolean): readonly (readonly [string, string])[] => (advanced ? FIRMWARE_SOURCES : FIRMWARE_SOURCES.filter(([it]) => it.endsWith(':stable')));

export const sourceAfterAdvanced = (source: string, advanced: boolean): string => {
  if (advanced || source.endsWith(':stable')) return source;
  const stem = source.includes(':') ? source.slice(0, source.lastIndexOf(':')) : '';
  return stem === '' ? DEFAULT_FIRMWARE_SOURCE : `${stem}:stable`;
};

export const BETA_WARNING =
  'WARNING: BETA FIRMWARE. This firmware version is ONLY intended for beta testers. Although it has received FLIGHT TESTING, it represents actively changed code. Do NOT use for normal operation.';
export const DEV_WARNING =
  'WARNING: CONTINUOUS BUILD FIRMWARE. This firmware has NOT BEEN FLIGHT TESTED. It is only intended for DEVELOPERS. Run bench tests without props first. Do NOT fly this without additional safety precautions. Follow the forums actively when using it.';

export const firmwareWarning = (source: string): string | null => {
  if (source.endsWith(':beta')) return BETA_WARNING;
  if (source.endsWith(':dev')) return DEV_WARNING;
  return null;
};

export const firmwareHeading = (ports: readonly FirmwarePort[]): string =>
  ports.some((it) => it.port === DEVICE_PORT) ? 'Flash the flight controller on this Aircast device, or a board over USB' : ports.length === 0 ? 'No USB serial devices attached' : 'Flash a board over USB';

export const flashingLabel = (ports: readonly FirmwarePort[], port: string): string => {
  const description = ports.find((it) => it.port === port)?.description ?? '';
  return `Flashing - ${description.trim() === '' ? port : description}`;
};

export const sourceLabel = (source: string, label: string, stable: string, beta: string): string => {
  if (source === 'px4:stable' && stable.trim() !== '') return `PX4 Pro ${stable}`;
  if (source === 'px4:beta' && beta.trim() !== '') return `PX4 Pro ${beta}`;
  return label;
};

export const firmwareChoice = (source: string, file: string | null): string | null => (source === FIRMWARE_FROM_FILE ? file : source);

export const firmwareFileAccepted = (name: string): boolean => FIRMWARE_EXTENSIONS.some((it) => name.toLowerCase().endsWith(`.${it}`));

type PortSelection = { ports: readonly FirmwarePort[]; port: string };


export const FirmwareScreen = ({ className }: { className?: string }) => {
  const [job, setJob] = useState<FirmwareJob | null>(null);
  const [selection, setSelection] = useState<PortSelection>({ ports: [], port: '' });
  const [file, setFile] = useState<File | null>(null);
  const [advanced, setAdvanced] = useState(false);
  const apmVehicle = useQgcBool(APM_FIRMWARE);
  const upgradeSettings = useQgcFacts(FIRMWARE_UPGRADE_SETTINGS);
  const [source, setSource] = useState(DEFAULT_FIRMWARE_SOURCE);
  const [sourceRestored, setSourceRestored] = useState(false);
  const [refusal, setRefusal] = useState('');
  const [flashingName, setFlashingName] = useState('');
  const { ports, port } = selection;

  useEffect(() => {
    if (sourceRestored || upgradeSettings.length === 0) return;
    const setting = (name: string): number | null => {
      const value = upgradeSettings.find((it) => it.name === name)?.value;
      return typeof value === 'number' ? Math.trunc(value) : null;
    };
    setSource(rememberedSource(setting(DEFAULT_FIRMWARE_TYPE), setting(APM_VEHICLE_TYPE)));
    setSourceRestored(true);
  }, [upgradeSettings]);

  const chooseSource = (chosen: string) => {
    setSource(chosen);
    setSourceRestored(true);
    void Promise.all(sourceSettings(chosen).map(([name, value]) => Qgc.writeRefusal(`${FIRMWARE_UPGRADE_SETTINGS}.${name}`, value)));
  };

  useLaunchedEffect([], async (scope) => {
    const poll = async (): Promise<void> => {
      if (!scope.isActive()) return;
      const readJob = firmwareJob(await Qgc.get(FIRMWARE_VIEW));
      const readPorts = firmwarePorts(await Qgc.get(FIRMWARE_PORTS_VIEW));
      if (!scope.isActive()) return;
      setJob(readJob);
      setSelection((previous) => ({
        ports: readPorts,
        port:
          !same(readPorts, previous.ports) && readJob?.busy !== true
            ? (preselectedPort(readPorts, previous.port) ?? (readPorts.some((it) => it.port === previous.port) ? previous.port : ''))
            : previous.port,
      }));
      await delay(FIRMWARE_POLL_MS);
      return poll();
    };
    await poll();
  });

  const pick = async () => {
    const chosen = await openDocument(['*/*']);
    if (chosen === null) return;
    if (!firmwareFileAccepted(chosen.name)) {
      setRefusal('Choose a .px4, .apj, .bin or .ihx firmware file.');
      return;
    }
    const staged = await stageFile(`${STAGED_PREFIX}${chosen.name}`, chosen).catch(() => false);
    setRefusal(staged ? '' : 'That file could not be read.');
    if (staged) setFile(chosen);
  };

  const busy = job?.busy === true;
  const phase = useRef<string | null>(null);
  phase.current = job?.phase ?? null;
  useEffect(
    () => () => {
      if (cancelledOnLeave(phase.current)) void Qgc.invoke(FIRMWARE_CANCEL);
    },
    [],
  );

  const offered = firmwareSources(advanced);
  const known = FIRMWARE_SOURCES.find(([it]) => it === source);
  const label = (entry: readonly [string, string]) => sourceLabel(entry[0], entry[1], job?.px4StableVersion ?? '', job?.px4BetaVersion ?? '');
  const chibios = source.startsWith('ardupilot:') ? (upgradeSettings.find((it) => it.name === APM_CHIBIOS) ?? null) : null;
  const choice = firmwareChoice(source, file === null ? null : `${STAGED_PREFIX}${file.name}`);
  const warning = firmwareWarning(source);
  const refuse = (block: () => Promise<string | null>) =>
    void (async () => {
      setRefusal((await block()) ?? '');
    })();

  return (
    <div className={`h-full w-full overflow-auto ${className ?? ''}`}>
      <div className="w-full px-4 py-2">
        <Surface color={md('primaryContainer')} contentColor={md('onPrimaryContainer')} shape="medium">
          <div className="flex flex-col gap-2 p-4">
            {job !== null && job.updateAvailable.trim() !== '' ? (
              <Text typography="bodyMedium" color={aircast('warning')}>
                {job.updateAvailable}
              </Text>
            ) : null}
            <Text typography="titleSmall">{firmwareHeading(ports)}</Text>
            <Text typography="bodyMedium">
              Plug in your device via USB, choose its port and either a release, downloaded once the board is identified, or a firmware file, then press Flash. A board running its firmware is asked to be
              unplugged and plugged back in so its bootloader starts.
            </Text>
            {serialAvailable() ? (
              <div>
                <TextButton onClick={() => void requestSerialPort().catch(() => undefined)}>Allow a USB port</TextButton>
              </div>
            ) : null}
          </div>
        </Surface>
      </div>
      {!busy && ports.filter((it) => it.boardType.trim() !== '').length > 1 ? (
        <Text typography="bodyMedium" color={aircast('warning')} className="block px-4">
          {MULTIPLE_DEVICES}
        </Text>
      ) : null}
      {ports.map((entry) => (
        <div key={entry.port} className={busy ? '' : 'cursor-pointer'} onClick={() => !busy && setSelection((previous) => ({ ...previous, port: entry.port }))}>
          <ListItem
            headlineContent={entry.description.trim() === '' ? entry.port : entry.description}
            supportingContent={entry.bootloader ? 'In its bootloader' : [entry.boardType, entry.port].filter((it) => it.trim() !== '').join(' · ')}
            leadingContent={<RadioButton selected={entry.port === port} onClick={null} enabled={!busy} label={entry.description.trim() === '' ? entry.port : entry.description} />}
          />
        </div>
      ))}
      <div className="flex w-full flex-col gap-2 px-4 py-3">
        <ChoiceField label="Firmware" value={known === undefined ? source : label(known)} options={offered.map(label)} className="w-full" onPick={(index) => !busy && chooseSource(offered[index]?.[0] ?? source)} />
        {chibios !== null ? <FactRow fact={chibios} fieldModifier="w-full" /> : null}
        <div className="flex items-center gap-3">
          <Checkbox
            checked={advanced}
            enabled={!busy}
            label="Advanced settings"
            onCheckedChange={(it) => {
              setAdvanced(it);
              setSource(sourceAfterAdvanced(source, it));
            }}
          />
          <Text typography="bodyMedium">Advanced settings</Text>
        </div>
        {bootloaderOffered(advanced, apmVehicle) ? (
          <div>
            <OutlinedButton enabled={!busy} onClick={() => void Qgc.invoke(FLASH_BOOTLOADER)}>
              Flash ChibiOS bootloader
            </OutlinedButton>
          </div>
        ) : null}
        {source === FIRMWARE_FROM_FILE ? <Text typography="bodyMedium">{file?.name ?? 'No firmware file chosen'}</Text> : null}
        {warning !== null ? (
          <Text typography="bodyMedium" color={md('error')}>
            {warning}
          </Text>
        ) : null}
        {busy ? (
          <Text typography="bodyMedium" maxLines={1} ellipsis>
            {flashingName.trim() === '' ? flashingLabel(ports, port) : flashingName}
          </Text>
        ) : null}
        <div className="flex w-full items-center justify-end gap-3">
          {source === FIRMWARE_FROM_FILE ? (
            <OutlinedButton enabled={!busy} onClick={() => void pick()}>
              Choose file
            </OutlinedButton>
          ) : null}
          <Button
            enabled={!busy && port.trim() !== '' && choice !== null}
            onClick={() => {
              if (choice === null) return;
              setFlashingName(flashingLabel(ports, port));
              refuse(() => Qgc.refusalOf(FIRMWARE_FLASH, port, choice));
            }}
          >
            Flash
          </Button>
          {busy ? (
            <OutlinedButton enabled={job?.cancellable === true} onClick={() => refuse(() => Qgc.refusalOf(FIRMWARE_CANCEL))}>
              Cancel
            </OutlinedButton>
          ) : null}
        </div>
        {job !== null ? (
          <>
            {job.busy ? <LinearProgressIndicator progress={job.progress} /> : null}
            {firmwarePhaseText(job.phase) !== '' ? <Text>{firmwarePhaseText(job.phase)}</Text> : null}
            {job.phase === 'choosing'
              ? job.choices.map(([name, url], index) =>
                  index === job.bestChoice ? (
                    <Button key={url} className="w-full" onClick={() => refuse(() => Qgc.refusalOf(FIRMWARE_CHOOSE, url))}>
                      {name}
                    </Button>
                  ) : (
                    <OutlinedButton key={url} className="w-full" onClick={() => refuse(() => Qgc.refusalOf(FIRMWARE_CHOOSE, url))}>
                      {name}
                    </OutlinedButton>
                  ),
                )
              : null}
            {job.error.trim() !== '' ? <Text color={md('error')}>{job.error}</Text> : null}
            {job.phase === 'failed' ? <Text typography="bodyMedium">{FLASH_FAIL_TEXT}</Text> : null}
          </>
        ) : null}
        {refusal.trim() !== '' ? <Text color={md('error')}>{refusal}</Text> : null}
        {job !== null && job.messages.length > 0 ? (
          <Text typography="bodySmall" monospace className="whitespace-pre-wrap">
            {job.messages.join('\n')}
          </Text>
        ) : null}
      </div>
    </div>
  );
};
