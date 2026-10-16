import { useEffect, useMemo, useState } from 'react';
import { type Json, has, isNull, optBoolean, optInt, optJSONObject, optLong } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { AlertDialog, LinearProgressIndicator, Text, TextButton } from '@/m3';
import { OPERATOR_CONTROL_VIEW, REQUEST_CONTROL } from './OperatorControl';

const REVERT_ALLOW_TAKEOVER = 'vehicle.startTimerRevertAllowTakeover';

export type IncomingControlRequest = { readonly systemId: number; readonly timeoutMs: number; readonly remainingMs: number };

export type ControlPrompt = { readonly incoming: IncomingControlRequest | null; readonly revertMs: number | null };

const incomingOf = (it: Json | null): IncomingControlRequest | null =>
  it === null ? null : { systemId: optInt(it, 'systemId'), timeoutMs: optLong(it, 'timeoutMs'), remainingMs: optLong(it, 'remainingMs') };

export const controlPrompt = (view: Json | null): ControlPrompt => ({
  incoming: incomingOf(optJSONObject(view, 'incomingRequest')),
  revertMs: view !== null && has(view, 'takeoverRevertMs') && !isNull(view, 'takeoverRevertMs') && optBoolean(view, 'inControl') ? optLong(view, 'takeoverRevertMs') : null,
});

export const secondsLeft = (ms: number): number => Math.trunc(Math.ceil(ms / 1000.0));

export const ControlRequestPrompt = () => {
  const view = useQgcPath(OPERATOR_CONTROL_VIEW);
  const prompt = useMemo(() => controlPrompt(view), [view]);
  const [ignored, setIgnored] = useState<number | null>(null);
  const [revertIgnored, setRevertIgnored] = useState(false);
  const [allowedFrom, setAllowedFrom] = useState<number | null>(null);
  const incomingGone = prompt.incoming === null;
  const revertGone = prompt.revertMs === null;
  useEffect(() => {
    if (incomingGone) setIgnored(null);
  }, [incomingGone]);
  useEffect(() => {
    if (revertGone) setRevertIgnored(false);
  }, [revertGone]);

  const incoming = prompt.incoming !== null && prompt.incoming.systemId !== ignored ? prompt.incoming : null;
  if (incoming !== null) {
    return (
      <AlertDialog
        onDismissRequest={() => setIgnored(incoming.systemId)}
        title={`GCS ${incoming.systemId} is requesting control`}
        text={
          <div className="flex flex-col">
            <Text typography="bodyMedium">{`Ignoring automatically in ${secondsLeft(incoming.remainingMs)} seconds`}</Text>
            <LinearProgressIndicator progress={incoming.timeoutMs > 0 ? incoming.remainingMs / incoming.timeoutMs : 0} className="w-full" />
          </div>
        }
        confirmButton={
          <TextButton
            onClick={() => {
              setAllowedFrom(incoming.systemId);
              setRevertIgnored(false);
              void (async () => {
                await Qgc.invoke(REQUEST_CONTROL, true, 0);
                await Qgc.invoke(REVERT_ALLOW_TAKEOVER);
              })();
            }}
          >
            Allow takeover
          </TextButton>
        }
        dismissButton={<TextButton onClick={() => setIgnored(incoming.systemId)}>Ignore</TextButton>}
      />
    );
  }

  const revert = prompt.revertMs !== null && !revertIgnored ? prompt.revertMs : null;
  if (revert === null) return null;
  return (
    <AlertDialog
      onDismissRequest={() => setRevertIgnored(true)}
      text={`Reverting back to takeover not allowed if GCS ${allowedFrom ?? ''} doesn't take control in ${secondsLeft(revert)} seconds ...`}
      confirmButton={<TextButton onClick={() => setRevertIgnored(true)}>Ignore</TextButton>}
    />
  );
};
