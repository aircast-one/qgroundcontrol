import { useState } from 'react';
import { type Json, jsonObjects, optBoolean, optInt, optJSONArray, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { copyToClipboard } from '@/kotlin/Platform';
import { delay } from '@/kotlin/Std';
import { AlertDialog, FilledTonalButton, OutlinedTextField, RadioButton, Text, TextButton } from '@/m3';
import { md } from '@/theme/Colors';
import { SectionHeader } from '@/ui/SetupRow';

export const MAVLINK_GROUP = 'mavlinkSettings';
export const SIGNING_AFTER_BLOCK = 'Ground Station';
export const SIGNING_KEYS_VIEW = 'view.signingKeys';
export const SIGNING_ADD_PASSPHRASE = 'signingKeys.addPassphrase';
export const SIGNING_ADD_RAW = 'signingKeys.addRaw';
export const SIGNING_GENERATE = 'signingKeys.generate';
export const SIGNING_REMOVE = 'signingKeys.remove';
export const SIGNING_EXPORT = 'signingKeys.export';
export const SIGNING_ENABLE = 'signing.enable';
export const SIGNING_DISABLE = 'signing.disable';
const SIGNING_POLL_MS = 500;
export const RAW_KEY_HEX_LENGTH = 64;
const CLIPBOARD_WIPE_MS = 30_000;

export type SigningKeyRow = { name: string; inUse: boolean; activeOnVehicle: boolean };

export type SigningKeys = {
  available: boolean;
  vehicle: boolean;
  armed: boolean;
  state: string;
  linkName: string;
  activeKey: string;
  minPassphraseLength: number;
  keys: readonly SigningKeyRow[];
};

export const signingKeys = (view: Json | null): SigningKeys | null =>
  view !== null && optBoolean(view, 'available')
    ? {
        available: true,
        vehicle: optBoolean(view, 'vehicle'),
        armed: optBoolean(view, 'armed'),
        state: optText(view, 'state'),
        linkName: optText(view, 'linkName'),
        activeKey: optText(view, 'activeKey'),
        minPassphraseLength: optInt(view, 'minPassphraseLength', 8),
        keys: jsonObjects(optJSONArray(view, 'keys')).map((row) => ({ name: optText(row, 'name'), inUse: optBoolean(row, 'inUse'), activeOnVehicle: optBoolean(row, 'activeOnVehicle') })),
      }
    : null;

export type KeyButtons = { enable: boolean; disable: boolean; otherActive: boolean; pending: boolean };

export const keyButtons = (keys: SigningKeys, row: SigningKeyRow): KeyButtons => {
  const anyActive = keys.vehicle && keys.activeKey !== 'None';
  return {
    enable: !anyActive,
    disable: keys.vehicle && row.activeOnVehicle,
    otherActive: anyActive && !row.activeOnVehicle,
    pending: keys.state === 'enabling' || keys.state === 'disabling',
  };
};

const HEX_CHARACTER = /^[0-9a-fA-F]$/;

export const isHexKey = (text: string): boolean => text.length === RAW_KEY_HEX_LENGTH && [...text].every((character) => HEX_CHARACTER.test(character));

export const canAddKey = (name: string, raw: boolean, secret: string, minPassphrase: number): boolean => name !== '' && (raw ? isHexKey(secret) : secret.length >= minPassphrase);

const wipeClipboard = () => void copyToClipboard('').catch(() => undefined);

export const SigningKeysSection = () => {
  const [revision, setRevision] = useState(0);
  const [read, setRead] = useState<SigningKeys | null>(null);
  const [adding, setAdding] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);
  const [exported, setExported] = useState<string | null>(null);
  const [wipeAfterExport, setWipeAfterExport] = useState(0);
  const [confirmEnable, setConfirmEnable] = useState<string | null>(null);
  const [armedWarning, setArmedWarning] = useState(false);
  const [refused, setRefused] = useState<string | null>(null);
  const bump = () => setRevision((count) => count + 1);

  useLaunchedEffect([revision], async (scope) => {
    const next = signingKeys(await Qgc.get(SIGNING_KEYS_VIEW));
    if (!scope.isActive()) return;
    setRead(next);
    if (next?.state === 'enabling' || next?.state === 'disabling') {
      await delay(SIGNING_POLL_MS);
      if (scope.isActive()) bump();
    }
  });

  const change = (path: string, ...args: unknown[]) =>
    void (async () => {
      setRefused(await Qgc.refusalOf(path, ...args));
      bump();
    })();

  useLaunchedEffect([wipeAfterExport], async (scope) => {
    if (wipeAfterExport <= 0) return;
    await delay(CLIPBOARD_WIPE_MS);
    if (scope.isActive()) wipeClipboard();
  });

  if (read === null) return null;
  const keys = read;

  return (
    <>
      <SectionHeader text="MAVLink 2 signing" />
      <div className="flex w-full flex-col gap-1.5 px-4 py-2">
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          Signing keys should only be sent to the vehicle over secure links (e.g. USB).
        </Text>
        {keys.vehicle ? (
          <div className="flex w-full">
            <Text typography="bodyLarge" className="flex-1">
              Active key
            </Text>
            <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
              {keys.activeKey}
            </Text>
          </div>
        ) : null}
        {keys.keys.map((key) => {
          const buttons = keyButtons(keys, key);
          return (
            <div key={key.name} className="flex w-full items-center">
              <Text typography="bodyLarge">{key.name}</Text>
              {buttons.otherActive ? (
                <Text typography="bodySmall" color={md('onSurfaceVariant')}>
                  {' (another key active)'}
                </Text>
              ) : null}
              <div className="flex-1" />
              {buttons.enable ? (
                <TextButton enabled={keys.vehicle && !buttons.pending} onClick={() => setConfirmEnable(key.name)}>
                  {buttons.pending ? 'Configuring…' : 'Enable'}
                </TextButton>
              ) : null}
              {buttons.disable ? (
                <TextButton enabled={!buttons.pending} onClick={() => (keys.armed ? setArmedWarning(true) : change(SIGNING_DISABLE))}>
                  {buttons.pending ? 'Disabling…' : 'Disable'}
                </TextButton>
              ) : null}
              {!key.inUse ? (
                <>
                  <TextButton
                    onClick={() =>
                      void (async () => {
                        const hex = await Qgc.invokeResult(SIGNING_EXPORT, key.name);
                        if (typeof hex !== 'string' || hex === '') return;
                        const copied = await copyToClipboard(hex).then(
                          () => true,
                          () => false,
                        );
                        if (!copied) return;
                        setWipeAfterExport((count) => count + 1);
                        setExported(key.name);
                      })()
                    }
                  >
                    Export
                  </TextButton>
                  <TextButton onClick={() => setConfirmDelete(key.name)}>Delete</TextButton>
                </>
              ) : null}
            </div>
          );
        })}
        {refused !== null ? (
          <Text typography="bodySmall" color={md('error')}>
            {refused}
          </Text>
        ) : null}
        {keys.keys.length === 0 ? (
          <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
            No keys configured
          </Text>
        ) : null}
        <div>
          <FilledTonalButton onClick={() => setAdding(true)}>Add key</FilledTonalButton>
        </div>
      </div>

      {confirmEnable !== null ? (
        <AlertDialog
          onDismissRequest={() => setConfirmEnable(null)}
          title="Send signing key"
          text={`This will transmit key '${confirmEnable}' to the vehicle over '${read.linkName}'. Only proceed if this link is secure (USB or trusted local network).`}
          confirmButton={
            <TextButton
              onClick={() => {
                const name = confirmEnable;
                setConfirmEnable(null);
                change(SIGNING_ENABLE, name);
              }}
            >
              OK
            </TextButton>
          }
          dismissButton={<TextButton onClick={() => setConfirmEnable(null)}>Cancel</TextButton>}
        />
      ) : null}
      {armedWarning ? (
        <AlertDialog
          onDismissRequest={() => setArmedWarning(false)}
          title="Disable signing while armed?"
          text={
            <span className="whitespace-pre-line">
              {'Vehicle is armed. ArduPilot will refuse to disable signing while armed and PX4 will not accept the disable packet without a valid signature. The disable attempt will likely time out and leave the link in an inconsistent state.\n\nDisarm the vehicle first.'}
            </span>
          }
          confirmButton={<TextButton onClick={() => setArmedWarning(false)}>Cancel</TextButton>}
        />
      ) : null}
      {adding ? (
        <AddKeyDialog
          minPassphrase={keys.minPassphraseLength}
          onDone={() => {
            setAdding(false);
            bump();
          }}
        />
      ) : null}
      {confirmDelete !== null ? (
        <AlertDialog
          onDismissRequest={() => setConfirmDelete(null)}
          title="Delete signing key"
          text={
            <span className="whitespace-pre-line">
              {`Are you sure you want to delete '${confirmDelete}'?\n\nIf a vehicle still has this key configured, you will no longer be able to communicate with it over a signed connection. Raw or generated keys cannot be recovered — Export the hex first if you may need it later.`}
            </span>
          }
          confirmButton={
            <TextButton
              onClick={() => {
                const name = confirmDelete;
                setConfirmDelete(null);
                void (async () => {
                  await Qgc.invoke(SIGNING_REMOVE, name);
                  bump();
                })();
              }}
            >
              OK
            </TextButton>
          }
          dismissButton={<TextButton onClick={() => setConfirmDelete(null)}>Cancel</TextButton>}
        />
      ) : null}
      {exported !== null ? (
        <AlertDialog
          onDismissRequest={() => setExported(null)}
          title={`Export key: ${exported}`}
          text="Key copied to clipboard. Store it securely — it will be cleared from the clipboard in 30 seconds."
          confirmButton={<TextButton onClick={() => setExported(null)}>OK</TextButton>}
        />
      ) : null}
    </>
  );
};

const AddKeyDialog = ({ minPassphrase, onDone }: { minPassphrase: number; onDone: () => void }) => {
  const [name, setName] = useState('');
  const [raw, setRaw] = useState(false);
  const [passphrase, setPassphrase] = useState('');
  const [hex, setHex] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const secret = raw ? hex : passphrase;

  return (
    <AlertDialog
      onDismissRequest={() => {
        if (!saving) onDone();
      }}
      title="Add signing key"
      text={
        <div className="flex flex-col gap-2">
          <Text>Key name</Text>
          <OutlinedTextField value={name} onValueChange={setName} singleLine placeholder="Enter a friendly name" />
          <div className="flex items-center gap-2">
            <RadioButton selected={!raw} onClick={() => setRaw(false)} label="Passphrase" />
            <Text>Passphrase</Text>
            <RadioButton selected={raw} onClick={() => setRaw(true)} label="Raw key (hex)" />
            <Text>Raw key (hex)</Text>
          </div>
          {!raw ? (
            <>
              <OutlinedTextField value={passphrase} onValueChange={setPassphrase} singleLine keyboardType="password" autoCorrect={false} placeholder={`Enter passphrase (min ${minPassphrase} chars)`} />
              {passphrase !== '' && passphrase.length < minPassphrase ? (
                <Text typography="bodySmall" color={md('error')}>
                  {`Passphrase too short (${passphrase.length}/${minPassphrase})`}
                </Text>
              ) : null}
            </>
          ) : (
            <>
              <div className="flex items-center">
                <OutlinedTextField
                  className="flex-1"
                  value={hex}
                  onValueChange={(typed) => setHex([...typed].filter((character) => HEX_CHARACTER.test(character)).slice(0, RAW_KEY_HEX_LENGTH).join(''))}
                  singleLine
                  autoCorrect={false}
                  placeholder="64 hex characters"
                />
                <TextButton
                  onClick={() =>
                    void (async () => {
                      const generated = await Qgc.invokeResult(SIGNING_GENERATE);
                      setHex(typeof generated === 'string' ? generated : '');
                    })()
                  }
                >
                  Generate
                </TextButton>
              </div>
              {hex !== '' && hex.length !== RAW_KEY_HEX_LENGTH ? (
                <Text typography="bodySmall" color={md('error')}>
                  {`${hex.length}/64 hex characters`}
                </Text>
              ) : null}
            </>
          )}
          {error !== null ? (
            <Text typography="bodySmall" color={md('error')}>
              {error}
            </Text>
          ) : null}
        </div>
      }
      confirmButton={
        <TextButton
          enabled={!saving && canAddKey(name, raw, secret, minPassphrase)}
          onClick={() => {
            setSaving(true);
            void (async () => {
              const reason = await Qgc.refusalOf(raw ? SIGNING_ADD_RAW : SIGNING_ADD_PASSPHRASE, name, secret);
              setSaving(false);
              if (reason === null) onDone();
              else setError(reason);
            })();
          }}
        >
          {saving ? 'Adding…' : 'OK'}
        </TextButton>
      }
      dismissButton={
        <TextButton enabled={!saving} onClick={onDone}>
          Cancel
        </TextButton>
      }
    />
  );
};
