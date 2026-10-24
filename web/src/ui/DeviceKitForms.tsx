import { useState } from 'react';
import { FilledTonalButton, OutlinedTextField, Text } from '@/m3';
import { md } from '@/theme/Colors';
import { type ActionConfirm, ConfirmDialog, type DeviceField, DeviceSelect, SecretField, useDeviceAction, useFormDraft } from '@/ui/DeviceKit';
import type { DeviceMethod } from '@/ui/DeviceView';

export type FormValues = Readonly<Record<string, string>>;

export type ShapedField = DeviceField & { readonly shownWhen?: (values: FormValues) => boolean };

export const shownFields = (fields: readonly ShapedField[], values: FormValues): readonly ShapedField[] => fields.filter((field) => field.shownWhen?.(values) ?? true);

export const ShapedForm = ({
  fields,
  method,
  path,
  submit,
  success = '',
  confirm = null,
  note = '',
  onDone = () => undefined,
}: {
  fields: readonly ShapedField[];
  method: DeviceMethod;
  path: string;
  submit: string;
  success?: string;
  confirm?: ActionConfirm | null;
  note?: string;
  onDone?: () => void;
}) => {
  const action = useDeviceAction();
  const { values: current, set, saved } = useFormDraft(fields, method);
  const [asking, setAsking] = useState(false);
  const send = () =>
    void action.run({ method, path, body: current }, success).then((done) => {
      if (!done) return;
      saved(current);
      onDone();
    });
  const refusedOn = (field: ShapedField) => action.refusal?.field === field.name;
  return (
    <div className="flex flex-col gap-3 px-list py-2">
      <div className="grid gap-3 sm:grid-cols-2">
        {shownFields(fields, current).map((field) => (
          <div key={field.name} className="min-w-0">
            {field.options !== undefined ? (
              <DeviceSelect label={field.label} value={current[field.name] ?? ''} options={field.options} onPick={set(field.name)} />
            ) : field.keyboard === 'password' ? (
              <SecretField
                label={field.label}
                value={current[field.name] ?? ''}
                placeholder={field.placeholder}
                isError={refusedOn(field)}
                supportingText={refusedOn(field) ? action.refusal?.message : field.hint}
                onValueChange={set(field.name)}
              />
            ) : (
              <OutlinedTextField
                label={field.label}
                ariaLabel={field.label}
                value={current[field.name] ?? ''}
                placeholder={field.placeholder}
                keyboardType={field.keyboard ?? 'text'}
                autoCorrect={false}
                isError={refusedOn(field)}
                supportingText={refusedOn(field) ? action.refusal?.message : field.hint}
                onValueChange={set(field.name)}
              />
            )}
          </div>
        ))}
      </div>
      {note !== '' ? (
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          {note}
        </Text>
      ) : null}
      {action.refusal !== null && !shownFields(fields, current).some(refusedOn) ? (
        <div role="alert">
          <Text typography="bodySmall" color={md('error')}>
            {action.refusal.message}
          </Text>
        </div>
      ) : null}
      <div>
        <FilledTonalButton enabled={!action.busy} onClick={() => (confirm === null ? send() : setAsking(true))}>
          {submit}
        </FilledTonalButton>
      </div>
      {asking && confirm !== null ? <ConfirmDialog {...confirm} onConfirm={send} onDismiss={() => setAsking(false)} /> : null}
    </div>
  );
};
