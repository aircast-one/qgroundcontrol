import { useState } from 'react';
import { connectOffered, DeviceLink } from '@/app/DeviceLink';
import { useQgcPath } from '@/bridge/QgcState';
import { useValue } from '@/kotlin/State';
import { AlertDialog, Button, TextButton } from '@/m3';
import { FIRST_RUN_PATH, firstRun } from '@/ui/FirstRunDialog';

export const HANDOFF_TITLE = 'Connect to this device?';

export const handoffText = (origin: string): string =>
  `A link asks this app to connect to the Aircast device at ${new URL(origin).host}. Connect only if you opened it from your own device.`;

export const HandoffDialog = () => {
  const origin = useValue(connectOffered);
  const welcoming = firstRun(useQgcPath(FIRST_RUN_PATH)) !== null;
  const [busy, setBusy] = useState(false);
  const [refusal, setRefusal] = useState<string | null>(null);
  if (origin === null || welcoming) return null;
  const connect = () => {
    setBusy(true);
    void DeviceLink.acceptHandoff(origin).then((refused) => {
      setBusy(false);
      setRefusal(refused);
    });
  };
  const decline = () => DeviceLink.declineHandoff(origin);
  return (
    <AlertDialog
      onDismissRequest={decline}
      dismissOnClickOutside={false}
      title={HANDOFF_TITLE}
      text={refusal === null ? handoffText(origin) : refusal}
      confirmButton={
        <Button enabled={!busy} onClick={connect}>
          {refusal === null ? 'Connect' : 'Try again'}
        </Button>
      }
      dismissButton={<TextButton onClick={decline}>Not now</TextButton>}
    />
  );
};
