import { useEffect, useMemo, useState } from 'react';
import { type Json, jsonObjects, optJSONArray, optText } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { LinearProgressIndicator, Surface, Text, TextButton } from '@/m3';
import { Drawable } from '@/res/Icon';
import { md } from '@/theme/Colors';
import { hostDevice } from '@/app/HostDevice';
import { VEHICLES_VIEW, vehicleChoices } from '@/map/VehiclePicker';
import { useRememberVehicleLoading } from '@/ui/ActiveVehicle';
import { FLY_STATE, flyState } from '@/ui/FlyState';
import { LOST_LINK_HINT, SIGNAL_LOST } from '@/ui/InstrumentDetail';
import { LINKS_VIEW } from '@/ui/LinksScreen';

export const connectingTitle = (name: string | null): string => (name !== null && name.trim() !== '' ? `Connecting to ${name}` : 'Connecting');

export type LoadingWords = { title: string; detail: string; dismiss: string };

export const loadingWords = (name: string | null, lost: boolean): LoadingWords =>
  lost
    ? { title: SIGNAL_LOST, detail: `Loading stopped. It picks up where it left off when the aircraft answers. ${LOST_LINK_HINT}`, dismiss: 'Hide' }
    : { title: connectingTitle(name), detail: 'Loading its settings.', dismiss: 'Fly now, finish loading in background' };

export const ConnectingCard = ({ className }: { className?: string }) => {
  const loading = useRememberVehicleLoading();
  const [dismissed, setDismissed] = useState(false);
  const vehiclesJson = useQgcPath(VEHICLES_VIEW);
  const name = useMemo(() => vehicleChoices(vehiclesJson).active?.name ?? null, [vehiclesJson]);
  const flyJson = useQgcPath(FLY_STATE);
  const lost = useMemo(() => flyState(flyJson)?.contactLost === true, [flyJson]);
  const words = loadingWords(name, lost);

  useEffect(() => {
    if (loading === null) setDismissed(false);
  }, [loading === null]);

  if (loading === null || dismissed) return null;
  const progress = loading;
  return (
    <div className={`mx-auto w-full max-w-[560px] p-4 ${className ?? ''}`}>
      <Surface shape="extraLarge" color={md('surfaceContainer')} contentColor={md('onSurface')}>
        <div className="flex flex-col items-center gap-3 p-6">
          <div className="flex size-[72px] items-center justify-center rounded-full bg-primary-container text-on-primary-container">
            <Drawable name={lost ? 'ic_warning' : 'ic_link'} size={36} />
          </div>
          <Text typography="headlineSmall" textAlign="center" color={lost ? md('error') : md('onSurface')}>
            {words.title}
          </Text>
          <Text typography="bodyMedium" textAlign="center" color={md('onSurfaceVariant')}>
            {words.detail}
          </Text>
          <div className="flex w-full">
            <Text typography="titleSmall" className="flex-1">
              Parameters
            </Text>
            <Text typography="labelLarge" color={md('onSurfaceVariant')}>
              {`${Math.trunc(progress * 100)}%`}
            </Text>
          </div>
          <LinearProgressIndicator progress={progress} className="w-full" color={lost ? md('outline') : undefined} />
          <TextButton onClick={() => setDismissed(true)}>{words.dismiss}</TextButton>
        </div>
      </Surface>
    </div>
  );
};

export const LOOKING_TITLE = 'Looking for your aircraft';
export const LOOKING_HINT = 'Turn on the aircraft. A USB cable or telemetry radio connects by itself; for Wi-Fi, add a link.';
export const BUILT_IN_LOOKING_HINT = 'Turn on the aircraft. The built-in radio connects by itself.';

export const BUILT_IN_RADIO = 'Built-in radio';

export const builtInRadioProblem = (links: Json | null): string | null => {
  const radio = jsonObjects(optJSONArray(links, 'links')).find((it) => optText(it, 'name') === BUILT_IN_RADIO);
  const problem = radio === undefined ? '' : optText(radio, 'lastError');
  return problem.trim() === '' ? null : problem;
};

export const lookingHint = (device: string | null, problem: string | null): string => problem ?? (device !== null ? BUILT_IN_LOOKING_HINT : LOOKING_HINT);

export const useRememberLookingHint = (): string => {
  const [device] = useState(hostDevice);
  const links = useQgcPath(LINKS_VIEW);
  return lookingHint(device, builtInRadioProblem(links));
};
