import { type ReactNode, useContext, useMemo, useState } from 'react';
import { type Fact, Qgc, makeFact } from '@/bridge/Qgc';
import { type Json, has, isNull, opt, optBoolean } from '@/bridge/Json';
import { useKeyedState, useProduceState, useUpdatedRef } from '@/kotlin/Compose';
import { openUri } from '@/kotlin/Platform';
import { Button, Checkbox, OutlinedTextField, Text, TextButton, type Typography } from '@/m3';
import { AircastSheet } from '@/map/AircastSheet';
import { toDoubleOrNull } from '@/map/AltitudeEntry';
import { aircast, md } from '@/theme/Colors';
import { useAdvancedUiShown } from '@/ui/AdvancedUi';
import { parameterFact, parameterPath } from '@/ui/ParametersScreen';
import { FactRow, LocalBlockRebootNote, LocalSettingsList, LocalStagedEdits, factRebootNote, fieldText, useStagedEdit, useStagedEdits, useWriteNotice, writeForcedValue, writeValue } from '@/ui/SettingsScreen';
import { ValueControls, opensAsValue } from '@/ui/ValueRow';

const PARAM_SCHEME = 'param://';

export const paramLinkName = (url: string): string | null => {
  if (!url.startsWith(PARAM_SCHEME)) return null;
  const name = url.slice(PARAM_SCHEME.length);
  return name.trim() === '' ? null : name;
};

export const SETUP_DIALOG_OPENED = 'setup.dialogOpened';
export const SENSOR_SETTINGS_DIALOG = 'sensorSettings';
const PARAMETER_EDITOR_DIALOG = 'parameterEditor';

export const parameterLinkOpens = async (name: string): Promise<boolean> => {
  const answer = await Qgc.call(SETUP_DIALOG_OPENED, PARAMETER_EDITOR_DIALOG, name);
  return answer === null ? true : optBoolean(answer, 'exists', true);
};

const htmlNodes = (nodes: NodeListOf<ChildNode>, onLink: (url: string) => void, path: string): ReactNode[] =>
  [...nodes].map((node, index) => {
    const key = `${path}.${index}`;
    if (node.nodeType === Node.TEXT_NODE) return node.textContent;
    if (!(node instanceof Element)) return null;
    const children = htmlNodes(node.childNodes, onLink, key);
    switch (node.tagName.toLowerCase()) {
      case 'a': {
        const url = node.getAttribute('href') ?? '';
        return (
          <a
            key={key}
            href={url}
            className="underline"
            onClick={(event) => {
              event.preventDefault();
              onLink(url);
            }}
          >
            {children}
          </a>
        );
      }
      case 'b':
      case 'strong':
        return <strong key={key}>{children}</strong>;
      case 'i':
      case 'em':
      case 'cite':
      case 'dfn':
        return <em key={key}>{children}</em>;
      case 'u':
        return <u key={key}>{children}</u>;
      case 'br':
        return <br key={key} />;
      case 'p':
      case 'div':
        return (
          <span key={key} className="block">
            {children}
          </span>
        );
      default:
        return <span key={key}>{children}</span>;
    }
  });

export const LinkedText = ({ html, style, color, onParameter, className }: { html: string; style: Typography; color: string; onParameter: (name: string) => void; className?: string }) => {
  const opener = useUpdatedRef(onParameter);
  const text = useMemo(() => {
    const onLink = (url: string) => {
      const name = paramLinkName(url);
      if (name === null) return openUri(url);
      void (async () => {
        if (await parameterLinkOpens(name)) opener.current(name);
      })();
    };
    return htmlNodes(new DOMParser().parseFromString(html, 'text/html').body.childNodes, onLink, 'html');
  }, [html]);
  return (
    <Text typography={style} color={color} className={className}>
      {text}
    </Text>
  );
};

export const READ_ONLY_NOTE = 'This parameter is read-only and cannot be modified.';
export const FORCE_EDIT_NOTE = 'Warning: This parameter is read-only. Force edit is enabled.';

export const forceEditNote = (readOnly: boolean, forced: boolean): string | null => {
  if (!readOnly) return null;
  return forced ? FORCE_EDIT_NOTE : READ_ONLY_NOTE;
};

const CheckRow = ({ text, checked, onChecked }: { text: string; checked: boolean; onChecked: (checked: boolean) => void }) => (
  <div className="flex items-center gap-3">
    <Checkbox checked={checked} onCheckedChange={onChecked} label={text} />
    <Text typography="bodySmall">{text}</Text>
  </div>
);

export const IN_FLIGHT_WARNING =
  'Warning: Modifying values while vehicle is in flight can lead to vehicle instability and possible vehicle loss. Make sure you know what you are doing and double-check your values before Save!';

export const parameterDefault = (json: Json | null): unknown =>
  json !== null && optBoolean(json, 'defaultValueAvailable') && has(json, 'defaultValue') && !isNull(json, 'defaultValue') ? opt(json, 'defaultValue') : null;

export const manualEntryFact = (fact: Fact): Fact => makeFact({ ...fact, enumStrings: [], enumValues: [], bitmaskStrings: [], bitmaskValues: [] });

const notBlank = (text: string): boolean => text.trim() !== '';

export const parameterRangeLine = (fact: Fact): string => {
  const units = notBlank(fact.units) && !fact.isEnum ? ` ${fact.units}` : '';
  const min = !fact.minIsDefaultForType && notBlank(fact.minString) ? fact.minString : null;
  const max = !fact.maxIsDefaultForType && notBlank(fact.maxString) ? fact.maxString : null;
  const bounds = min !== null && max !== null ? `Range ${min}–${max}${units}` : min !== null ? `Min ${min}${units}` : max !== null ? `Max ${max}${units}` : null;
  return [bounds, notBlank(fact.defaultValueString) ? `default ${fact.defaultValueString}${units}` : null].filter((it): it is string => it !== null).join(' · ');
};

export const parameterRebootNotes = (fact: Fact): string[] =>
  [fact.vehicleRebootRequired ? 'Vehicle reboot required after change' : null, fact.qgcRebootRequired ? 'Application restart required after change' : null].filter((it): it is string => it !== null);

const ForcedValueField = ({ fact, onWrite }: { fact: Fact; onWrite: () => void }) => {
  const [text, setText] = useState('');
  const [refusal, setRefusal] = useState<string | null>(null);
  const staged = useContext(LocalStagedEdits);
  const announce = useWriteNotice(writeForcedValue);
  const commit = async (): Promise<boolean> => {
    const entered = fact.isString ? text : (toDoubleOrNull(text.trim()) ?? text.trim());
    const refused = await writeForcedValue(fact.path, entered);
    setRefusal(refused);
    if (refused !== null) return false;
    setText('');
    onWrite();
    announce(fact, String(entered), fieldText(fact), onWrite);
    return true;
  };
  useStagedEdit(fact.path, notBlank(text) ? commit : null, [text, fact.valueString]);
  return (
    <>
      <OutlinedTextField value={text} onValueChange={setText} singleLine label="Value" onDone={staged?.save} />
      {refusal !== null ? (
        <Text typography="bodySmall" color={md('error')}>
          {refusal}
        </Text>
      ) : null}
    </>
  );
};

export const ParameterEditDialog = ({ name, title = name, onWrite = () => undefined, onDismiss }: { name: string; title?: string; onWrite?: () => void; onDismiss: () => void }) => {
  const [revision, setRevision] = useState(0);
  const [advanced, setAdvanced] = useState(false);
  const [forced, setForced] = useState(false);
  const [manual, setManual] = useState(false);
  const [forceSave, setForceSave] = useState(false);
  const [rejected, setRejected] = useState(false);
  const [refusal, setRefusal] = useState<string | null>(null);
  const staged = useStagedEdits(onDismiss);
  const writer = forced ? writeForcedValue : writeValue;
  const announce = useWriteNotice(writer);
  const forceAllowed = useAdvancedUiShown();
  const fact = useProduceState<Fact | null>(null, [name, revision], async (set) => set(await parameterFact(name)));
  const defaultValue = useProduceState<unknown>(null, [name, revision], async (set) => set(parameterDefault(await Qgc.get(parameterPath(name)))));
  const written = () => {
    setRevision((it) => it + 1);
    onWrite();
  };

  const body = (loaded: Fact) => {
    const description = notBlank(loaded.longDescription) ? loaded.longDescription : loaded.description;
    const editable = !loaded.readOnly || forced;
    const note = forceEditNote(loaded.readOnly, forced);
    const shown = manual ? manualEntryFact(loaded) : loaded;
    const range = parameterRangeLine(loaded);
    const hasChoices = loaded.isEnum || loaded.isBitmask;
    return (
      <>
        {notBlank(description) ? (
          <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
            {description}
          </Text>
        ) : null}
        {note !== null ? (
          <Text typography="bodySmall" color={forced ? aircast('warning') : md('onSurface')}>
            {note}
          </Text>
        ) : null}
        <LocalBlockRebootNote.Provider value={factRebootNote(loaded)}>
          <FactRow fact={shown} title={notBlank(loaded.heading) ? loaded.heading : 'Value'} subtitle="" fieldModifier="w-full py-2" onWrite={written} onRejected={() => setRejected(true)} />
        </LocalBlockRebootNote.Provider>
        {notBlank(range) ? (
          <Text typography="bodySmall" color={md('onSurfaceVariant')}>
            {range}
          </Text>
        ) : null}
        {parameterRebootNotes(loaded).map((it) => (
          <Text key={it} typography="bodySmall">
            {it}
          </Text>
        ))}
        {editable ? (
          <Text typography="bodySmall" color={aircast('warning')}>
            {IN_FLIGHT_WARNING}
          </Text>
        ) : null}
        {(loaded.readOnly && forceAllowed) || (editable && hasChoices) ? (
          <>
            <CheckRow
              text="Advanced settings"
              checked={advanced}
              onChecked={(on) => {
                setAdvanced(on);
                setForced(forced && on);
                setManual(manual && on);
              }}
            />
            {advanced && loaded.readOnly && forceAllowed ? <CheckRow text="Force edit read-only param" checked={forced} onChecked={setForced} /> : null}
            {advanced && editable && hasChoices ? <CheckRow text="Manual entry" checked={manual} onChecked={setManual} /> : null}
          </>
        ) : null}
        {forced || (editable && (manual || !hasChoices) && !loaded.isString && !loaded.isBool) ? (
          <>
            {!forced && rejected && forceAllowed ? <CheckRow text="Force save (dangerous!)" checked={forceSave} onChecked={setForceSave} /> : null}
            {forceSave || forced ? <ForcedValueField fact={loaded} onWrite={written} /> : null}
          </>
        ) : null}
      </>
    );
  };

  const resettable = fact !== null && (!fact.readOnly || forced) && defaultValue !== null ? fact : null;
  const reset = (loaded: Fact) =>
    void (async () => {
      const refused = await writer(loaded.path, defaultValue);
      setRefusal(refused);
      if (refused !== null) return;
      announce(loaded, String(defaultValue), fieldText(loaded), onWrite);
      onDismiss();
    })();

  return (
    <AircastSheet onDismissRequest={onDismiss} label={title}>
      <LocalStagedEdits.Provider value={staged}>
        <div className="flex w-full flex-col gap-2 px-sheet pb-6">
          <Text typography="titleLarge">{title}</Text>
          {fact !== null ? (
            body(fact)
          ) : (
            <Text typography="bodySmall">{`${name} is not a parameter on this vehicle.`}</Text>
          )}
          {refusal !== null ? (
            <Text typography="bodySmall" color={md('error')}>
              {refusal}
            </Text>
          ) : null}
          <SheetActions onReset={resettable !== null ? () => reset(resettable) : null} onCancel={onDismiss} onSave={fact !== null ? staged.save : null} />
        </div>
      </LocalStagedEdits.Provider>
    </AircastSheet>
  );
};

const SheetActions = ({ onReset, onCancel, onSave }: { onReset: (() => void) | null; onCancel: () => void; onSave: (() => void) | null }) => (
  <div className="flex w-full flex-wrap items-center justify-end gap-2">
    {onReset !== null ? (
      <TextButton onClick={onReset} className="mr-auto">
        Reset to default
      </TextButton>
    ) : null}
    <TextButton onClick={onCancel}>Cancel</TextButton>
    {onSave !== null ? <Button onClick={onSave}>Save</Button> : null}
  </div>
);

export const VALUE_DETAILS_TITLE = 'Value Details';

const APP_SETTING_PREFIX = 'settings.';

export const SENT_TO_AIRCRAFT_NOTE = 'Changes go to the aircraft straight away, even in flight.';

export const EDIT_PARAMETER_TITLE = 'Edit parameter';

export const valueDetailsNotes = (fact: Fact): string[] => [
  ...[notBlank(fact.longDescription) ? fact.longDescription : fact.valueDetails, parameterRangeLine(fact)].filter(notBlank),
  ...parameterRebootNotes(fact),
];

export const ValueDetailsSheet = ({ fact, title = VALUE_DETAILS_TITLE, onWrite, onDismiss }: { fact: Fact; title?: string; onWrite: () => void; onDismiss: () => void }) => {
  const [refusal, setRefusal] = useKeyedState<string | null>([fact.path], () => null);
  const staged = useStagedEdits(onDismiss);
  const announce = useWriteNotice();
  const parsedDefault = toDoubleOrNull(fact.defaultValueString);
  const defaultValue = parsedDefault !== null && !fact.readOnly ? parsedDefault : null;
  const typedEntry = opensAsValue(fact) ? (
    <FactRow fact={{ ...fact, slider: null }} title="Type a value" subtitle="" fieldModifier="w-full py-2" onWrite={onWrite} />
  ) : (
    <FactRow fact={fact} title={fact.heading} subtitle="" fieldModifier="w-full py-2" onWrite={onWrite} />
  );
  const reset = (value: number) =>
    void (async () => {
      const refused = await writeValue(fact.path, value);
      setRefusal(refused);
      if (refused !== null) return;
      onWrite();
      announce(fact, fact.defaultValueString, fieldText(fact), onWrite);
      onDismiss();
    })();
  return (
    <AircastSheet onDismissRequest={onDismiss} label={title}>
      <LocalSettingsList.Provider value={false}>
        <LocalStagedEdits.Provider value={staged}>
          <div className="flex w-full flex-col gap-2 px-sheet pb-6">
            <Text typography="titleLarge">{title}</Text>
            {opensAsValue(fact) ? <ValueControls fact={fact} onWrite={onWrite} typedEntry={typedEntry} /> : typedEntry}
            {valueDetailsNotes(fact).map((it) => (
              <Text key={it} typography="bodySmall" color={md('onSurfaceVariant')}>
                {it}
              </Text>
            ))}
            {!fact.readOnly && !fact.path.startsWith(APP_SETTING_PREFIX) ? (
              <Text typography="bodySmall" color={aircast('warning')}>
                {SENT_TO_AIRCRAFT_NOTE}
              </Text>
            ) : null}
            {refusal !== null ? (
              <Text typography="bodySmall" color={md('error')}>
                {refusal}
              </Text>
            ) : null}
            <SheetActions onReset={defaultValue !== null ? () => reset(defaultValue) : null} onCancel={onDismiss} onSave={staged.save} />
          </div>
        </LocalStagedEdits.Provider>
      </LocalSettingsList.Provider>
    </AircastSheet>
  );
};
