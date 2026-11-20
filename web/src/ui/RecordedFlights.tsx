import { useContext, useState } from 'react';
import { LinkCommands } from '@/bridge/BridgeCommands';
import { isJson, optText } from '@/bridge/Json';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { delay } from '@/kotlin/Std';
import { FilledTonalButton, Text, TextButton } from '@/m3';
import { md } from '@/theme/Colors';
import { LocalAppNavigation } from '@/ui/AppNavigation';
import { useArmed, useHasVehicle } from '@/ui/FlyState';
import { SectionHeader } from '@/ui/SetupRow';

const FLIGHTS_POLL_MS = 2000;

export const RECORDED_BY_THIS_APP = 'Recorded by this app';
export const REVIEW_LAST_FLIGHT = 'Review last flight';
export const REVIEW_HINT = 'Review plays a flight back on Fly. It disconnects the aircraft, which saves the flight it just flew.';
export const LAND_TO_REVIEW = 'Land and disarm to review a flight.';
export const NO_FLIGHTS_YET = 'No flights recorded yet. Each armed flight is recorded while the aircraft is connected.';

export type RecordedFlight = { readonly name: string; readonly path: string };

export const recordedFlights = (result: unknown): readonly RecordedFlight[] =>
  Array.isArray(result) ? result.filter(isJson).map((it) => ({ name: optText(it, 'name'), path: optText(it, 'path') })) : [];

const STAMP = /^(\d{4})-(\d{2})-(\d{2}) (\d{2})-(\d{2})-(\d{2})/;

export const flightTitle = (file: string): string => {
  const stamp = STAMP.exec(file);
  return stamp === null ? file : `Flight on ${stamp[1]}-${stamp[2]}-${stamp[3]} at ${stamp[4]}:${stamp[5]}`;
};

export const reviewFlight = async (log: string, showFly: () => void): Promise<boolean> => {
  const opened = await LinkCommands.reviewFlight(log);
  if (opened) showFly();
  return opened;
};

export const RecordedFlights = () => {
  const navigation = useContext(LocalAppNavigation);
  const armed = useArmed();
  const connected = useHasVehicle();
  const [flights, setFlights] = useState<readonly RecordedFlight[]>([]);
  useLaunchedEffect([], async (scope) => {
    const poll = async (): Promise<void> => {
      if (!scope.isActive()) return;
      const read = recordedFlights(await LinkCommands.savedFlights());
      if (scope.isActive()) setFlights(read);
      await delay(FLIGHTS_POLL_MS);
      return poll();
    };
    await poll();
  });
  const review = (log: string) => void reviewFlight(log, () => navigation.showFly());
  return (
    <div className="flex w-full flex-col">
      <SectionHeader text={RECORDED_BY_THIS_APP} />
      <Text typography="bodySmall" color={md('onSurfaceVariant')} className="block px-4 pb-2">
        {armed ? LAND_TO_REVIEW : flights.length === 0 && !connected ? NO_FLIGHTS_YET : REVIEW_HINT}
      </Text>
      {connected || flights.length > 0 ? (
        <div className="flex px-4 pb-2">
          <FilledTonalButton enabled={!armed} onClick={() => review('')}>
            {REVIEW_LAST_FLIGHT}
          </FilledTonalButton>
        </div>
      ) : null}
      {flights.map((flight) => (
        <div key={flight.path} className="flex w-full items-center gap-2 px-4 py-1">
          <Text typography="bodyLarge" className="min-w-0 flex-1">
            {flightTitle(flight.name)}
          </Text>
          <TextButton enabled={!armed} onClick={() => review(flight.path)}>
            Review
          </TextButton>
        </div>
      ))}
    </div>
  );
};
