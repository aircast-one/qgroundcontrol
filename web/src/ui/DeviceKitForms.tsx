import { useState } from 'react';
import { FilledTonalButton, IconButton, OutlinedTextField, Text } from '@/m3';
import { MaterialIcon } from '@/res/Icon';
import { md } from '@/theme/Colors';
import { type ActionConfirm, ConfirmDialog, type DeviceField, DeviceSelect, formBody, useDeviceAction } from '@/ui/DeviceKit';
import type { DeviceMethod } from '@/ui/DeviceView';

export type FormValues = Readonly<Record<string, string>>;

export type ShapedField = DeviceField & { readonly shownWhen?: (values: FormValues) => boolean };

export const shownFields = (fields: readonly ShapedField[], values: FormValues): readonly ShapedField[] => fields.filter((field) => field.shownWhen?.(values) ?? true);

export const SecretField = ({ label, value, placeholder, hint, isError = false, onValueChange }: { label: string; value: string; placeholder?: string; hint?: string; isError?: boolean; onValueChange: (value: string) => void }) => {
  const [revealed, setRevealed] = useState(false);
  return (
    <OutlinedTextField
      label={label}
      ariaLabel={label}
      value={value}
      placeholder={placeholder}
      keyboardType={revealed ? 'text' : 'password'}
      autoCorrect={false}
      isError={isError}
      supportingText={hint}
      onValueChange={onValueChange}
      trailingIcon={
        <IconButton label={revealed ? 'Hide password' : 'Show password'} onClick={() => setRevealed(!revealed)}>
          <MaterialIcon name={revealed ? 'visibility_off' : 'visibility'} />
        </IconButton>
      }
    />
  );
};

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
  const [values, setValues] = useState<FormValues>({});
  const [asking, setAsking] = useState(false);
  const current = formBody(fields, values);
  const set = (name: string) => (value: string) => setValues((it) => ({ ...it, [name]: value }));
  const send = () =>
    void action.run({ method, path, body: current }, success).then((done) => {
      if (!done) return;
      setValues({});
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
                hint={refusedOn(field) ? action.refusal?.message : field.hint}
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
