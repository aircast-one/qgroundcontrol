import { useState } from 'react';
import { type Json, jsonStrings, optJSONArray, optString } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { AlertDialog, DropdownMenu, DropdownMenuItem, OutlinedButton, OutlinedTextField, Text, TextButton } from '@/m3';
import { md } from '@/theme/Colors';

export const SAVE_PRESET = 'core.plan.savePreset';
export const APPLY_PRESET = 'core.plan.applyPreset';
export const DELETE_PRESET = 'core.plan.deletePreset';

export const presetKind = (view: Json | null): string | null => {
  const kind = view === null ? null : optString(view, 'presetKind');
  return kind !== null && kind.trim() !== '' && kind !== 'null' ? kind : null;
};

export const presetNames = (view: Json | null): readonly string[] => jsonStrings(optJSONArray(view, 'names'));

export const presetNameError = (typed: string): string | null =>
  typed.trim() === '' ? 'Preset name cannot be blank.' : typed.includes('/') ? 'Preset name cannot include the "/" character.' : null;

export const PatternPresets = ({ index, kind, onApplied }: { index: number; kind: string; onApplied: () => void }) => {
  const [revision, setRevision] = useState(0);
  const [names, setNames] = useState<readonly string[]>([]);
  const [chosen, setChosen] = useState<string | null>(null);
  const [picking, setPicking] = useState(false);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState<string | null>(null);
  const [refusal, setRefusal] = useState<string | null>(null);
  const [typed, setTyped] = useState('');

  useLaunchedEffect([kind, revision], async (scope) => {
    const listed = presetNames(await Qgc.get(`view.patternPresets(${kind})`));
    if (!scope.isActive()) return;
    setNames(listed);
    setChosen((previous) => (previous !== null && listed.includes(previous) ? previous : (listed[0] ?? null)));
  });

  const act = (path: string, args: readonly unknown[], then: () => void = () => undefined) =>
    void (async () => {
      const refused = await Qgc.refusalOf(path, ...args);
      setRefusal(refused);
      setRevision((it) => it + 1);
      if (refused === null) then();
    })();

  return (
    <>
      <div className="flex flex-col gap-1.5 px-5 py-2">
        <Text typography="titleSmall">Presets</Text>
        <div className="flex gap-2">
          <Text className="pt-3">Preset</Text>
          <div className="relative">
            <OutlinedButton enabled={names.length > 0} onClick={() => setPicking(true)}>
              {chosen ?? 'None'}
            </OutlinedButton>
            <DropdownMenu expanded={picking} onDismissRequest={() => setPicking(false)}>
              {names.map((name) => (
                <DropdownMenuItem
                  key={name}
                  text={name}
                  onClick={() => {
                    setChosen(name);
                    setPicking(false);
                  }}
                />
              ))}
            </DropdownMenu>
          </div>
        </div>
        <div className="flex gap-2">
          <TextButton enabled={chosen !== null} onClick={() => chosen !== null && act(APPLY_PRESET, [index, chosen], onApplied)}>
            Apply preset
          </TextButton>
          <TextButton enabled={chosen !== null} onClick={() => setDeleting(chosen)}>
            Delete preset
          </TextButton>
        </div>
        <TextButton
          className="self-start"
          onClick={() => {
            setTyped('');
            setSaving(true);
          }}
        >
          Save settings as new preset
        </TextButton>
        {refusal !== null ? <Text color={md('error')}>{refusal}</Text> : null}
      </div>
      {deleting !== null ? (
        <AlertDialog
          onDismissRequest={() => setDeleting(null)}
          title="Delete preset"
          text={`Are you sure you want to delete '${deleting}' preset?`}
          confirmButton={
            <TextButton
              onClick={() => {
                setDeleting(null);
                act(DELETE_PRESET, [kind, deleting]);
              }}
            >
              OK
            </TextButton>
          }
          dismissButton={<TextButton onClick={() => setDeleting(null)}>Cancel</TextButton>}
        />
      ) : null}
      {saving ? (
        <AlertDialog
          onDismissRequest={() => setSaving(false)}
          title="Save preset"
          text={
            <div className="flex flex-col gap-2">
              <Text>Save the current settings as a named preset.</Text>
              <OutlinedTextField value={typed} onValueChange={setTyped} label="Preset name" placeholder="Enter preset name" singleLine />
              {presetNameError(typed) !== null ? (
                <Text typography="bodySmall" color={md('error')}>
                  {presetNameError(typed)}
                </Text>
              ) : null}
            </div>
          }
          confirmButton={
            <TextButton
              enabled={presetNameError(typed) === null}
              onClick={() => {
                setSaving(false);
                act(SAVE_PRESET, [index, typed.trim()], () => setChosen(typed.trim()));
              }}
            >
              Save
            </TextButton>
          }
          dismissButton={<TextButton onClick={() => setSaving(false)}>Cancel</TextButton>}
        />
      ) : null}
    </>
  );
};
