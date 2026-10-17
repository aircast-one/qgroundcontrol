import { useState } from 'react';
import { format, same } from '@/kotlin/Std';
import { AlertDialog, OutlinedTextField, TextButton } from '@/m3';
import { md } from '@/theme/Colors';
import { typedNumber } from './AltitudeEntry';
import type { BreachReturn } from './FenceBridge';

export const breachAltitudeText = (altitude: number | null): string => (altitude === null ? '' : format('%.1f', altitude));

export const BreachReturnDialog = ({ breach, onDismiss, onAltitude, onRemove }: { breach: BreachReturn; onDismiss: () => void; onAltitude: (altitude: number) => void; onRemove: () => void }) => {
  const [typedFor, setTypedFor] = useState(() => ({ breach, typed: breachAltitudeText(breach.altitude) }));
  const typed = same(typedFor.breach, breach) ? typedFor.typed : breachAltitudeText(breach.altitude);
  const value = typedNumber(typed);
  return (
    <AlertDialog
      onDismissRequest={onDismiss}
      title="Breach return point"
      text={
        <div className="flex flex-col gap-2">
          <OutlinedTextField value={typed} onValueChange={(it) => setTypedFor({ breach, typed: it })} label="Altitude" suffix={breach.units} singleLine isError={value === null} keyboardType="decimal" />
          <TextButton onClick={onRemove} contentColor={md('error')} className="self-start">
            Remove breach return point
          </TextButton>
        </div>
      }
      confirmButton={
        <TextButton onClick={() => value !== null && onAltitude(value)} enabled={value !== null && breach.altitudePath !== ''}>
          Set
        </TextButton>
      }
      dismissButton={<TextButton onClick={onDismiss}>Close</TextButton>}
    />
  );
};
