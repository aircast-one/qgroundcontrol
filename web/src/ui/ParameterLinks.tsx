import { type ReactNode, useCallback, useMemo, useRef, useState } from 'react';
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
import { FactRow, LocalBlockRebootNote, LocalSettingsList, LocalTypedDiscarded, factRebootNote } from '@/ui/SettingsScreen';
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

export const ParameterEditDialog = ({ name, title = name, onDismiss }: { name: string; title?: string; onDismiss: () => void }) => {
  const [revision, setRevision] = useState(0);
  const [advanced, setAdvanced] = useState(false);
  const [forced, setForced] = useState(false);
  const [manual, setManual] = useState(false);
  const [forceSave, setForceSave] = useState(false);
  const [rejected, setRejected] = useState(false);
  const [forcedText, setForcedText] = useState('');
  const [forceRefusal, setForceRefusal] = useState<string | null>(null);
  const resetting = useRef(false);
  const discarded = useCallback(() => resetting.current, []);
  const forceAllowed = useAdvancedUiShown();
  const fact = useProduceState<Fact | null>(null, [name, revision], async (set) => set(await parameterFact(name)));
  const defaultValue = useProduceState<unknown>(null, [name, revision], async (set) => set(parameterDefault(await Qgc.get(parameterPath(name)))));

  const body = (loaded: Fact) => {
    const description = notBlank(loaded.longDescription) ? loaded.longDescription : loaded.description;
    const editable = !loaded.readOnly || forced;
    const note = forceEditNote(loaded.readOnly, forced);
    const shown = manual ? manualEntryFact(loaded) : loaded;
    const range = parameterRangeLine(loaded);
    const hasChoices = loaded.isEnum || loaded.isBitmask;
    const saveForced = () =>
      void (async () => {
        const entered = loaded.isString ? forcedText : (toDoubleOrNull(forcedText.trim()) ?? forcedText.trim());
        const refused = await Qgc.writeForcedRefusal(loaded.path, entered);
        setForceRefusal(refused);
        if (refused === null) setRevision((it) => it + 1);
      })();
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
          <LocalTypedDiscarded.Provider value={discarded}>
            <FactRow
              fact={shown}
              title={notBlank(loaded.heading) ? loaded.heading : 'Value'}
              subtitle=""
              fieldModifier="w-full py-2"
              onWrite={() => setRevision((it) => it + 1)}
              onRejected={() => setRejected(true)}
            />
          </LocalTypedDiscarded.Provider>
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
            {forceSave || forced ? (
              <>
                <OutlinedTextField value={forcedText} onValueChange={setForcedText} singleLine label="Value" />
                {forceRefusal !== null ? (
                  <Text typography="bodySmall" color={md('error')}>
                    {forceRefusal}
                  </Text>
                ) : null}
                <div>
                  <TextButton enabled={notBlank(forcedText)} onClick={saveForced}>
                    Save
                  </TextButton>
                </div>
              </>
            ) : null}
          </>
        ) : null}
      </>
    );
  };

  const resettable = fact !== null && (!fact.readOnly || forced) && defaultValue !== null ? fact : null;
  const reset = (loaded: Fact) => {
    resetting.current = true;
    void (async () => {
      const refused = forced ? await Qgc.writeForcedRefusal(loaded.path, defaultValue) : await Qgc.writeRefusal(loaded.path, defaultValue);
      setForceRefusal(refused);
      resetting.current = refused === null;
      if (refused === null) onDismiss();
    })();
  };

  return (
    <AircastSheet onDismissRequest={onDismiss}>
      <div className="flex w-full flex-col gap-2 px-sheet pb-6">
        <Text typography="titleLarge">{title}</Text>
        {fact !== null ? (
          body(fact)
        ) : (
          <Text typography="bodySmall">{`${name} is not a parameter on this vehicle.`}</Text>
        )}
        <div className="flex w-full justify-end gap-2">
          {resettable !== null ? <TextButton onClick={() => reset(resettable)}>Reset to default</TextButton> : null}
          <Button onClick={onDismiss}>Done</Button>
        </div>
      </div>
    </AircastSheet>
  );
};

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
  const resetting = useRef(false);
  const discarded = useCallback(() => resetting.current, []);
  const parsedDefault = toDoubleOrNull(fact.defaultValueString);
  const defaultValue = parsedDefault !== null && !fact.readOnly ? parsedDefault : null;
  const typedEntry = <FactRow fact={fact} title={opensAsValue(fact) ? 'Type a value' : fact.heading} subtitle="" fieldModifier="w-full py-2" onWrite={onWrite} />;
  const reset = (value: number) => {
    resetting.current = true;
    void (async () => {
      const refused = await Qgc.writeRefusal(fact.path, value);
      setRefusal(refused);
      resetting.current = refused === null;
      if (refused === null) {
        onWrite();
        onDismiss();
      }
    })();
  };
  return (
    <AircastSheet onDismissRequest={onDismiss}>
      <LocalSettingsList.Provider value={false}>
        <LocalTypedDiscarded.Provider value={discarded}>
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
            <div className="flex w-full justify-end gap-2">
              {defaultValue !== null ? <TextButton onClick={() => reset(defaultValue)}>Reset to default</TextButton> : null}
              <Button onClick={onDismiss}>Done</Button>
            </div>
          </div>
        </LocalTypedDiscarded.Provider>
      </LocalSettingsList.Provider>
    </AircastSheet>
  );
};
