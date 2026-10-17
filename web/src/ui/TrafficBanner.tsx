import { useMemo, useState } from 'react';
import { useQgcPath } from '@/bridge/QgcState';
import { Surface, Text } from '@/m3';
import { AircastSheet } from '@/map/AircastSheet';
import { Drawable } from '@/res/Icon';
import { aircast, md } from '@/theme/Colors';
import { OpenOnRequest, TRAFFIC_SHEET } from './FlyScreenState';
import {
  TRAFFIC_VIEW,
  type TrafficReading,
  TrafficLevel,
  trafficAlert,
  trafficCaption,
  trafficContactText,
  trafficContactUrgent,
  trafficReading,
  trafficSummary,
} from './TrafficView';

const TRAFFIC_BANNER_MAX_WIDTH = 560;
const TRAFFIC_BANNER_CORNER = 12;

export const TrafficBanner = ({ className }: { className?: string }) => {
  const view = useQgcPath(TRAFFIC_VIEW);
  const reading = useMemo(() => trafficReading(view), [view]);
  const [listed, setListed] = useState(false);
  const opener = <OpenOnRequest name={TRAFFIC_SHEET} open={() => setListed(true)} />;
  const sheet = reading !== null && listed ? <TrafficSheet reading={reading} onDismiss={() => setListed(false)} /> : null;

  const alert = reading === null ? null : trafficAlert(reading);
  if (reading === null || alert === null) {
    return (
      <>
        {opener}
        {sheet}
      </>
    );
  }
  const urgent = alert.level.ordinal >= TrafficLevel.Warning.ordinal;
  return (
    <>
      {opener}
      {sheet}
      <Surface
        onClick={reading.contacts.length > 0 ? () => setListed(true) : undefined}
        className={`w-full ${className ?? ''}`}
        style={{ maxWidth: TRAFFIC_BANNER_MAX_WIDTH }}
        shape={TRAFFIC_BANNER_CORNER}
        color={urgent ? md('errorContainer') : aircast('warningContainer')}
        contentColor={urgent ? md('onErrorContainer') : md('onSurface')}
      >
        <div className="flex min-h-12 items-center gap-3 px-3 py-2">
          <Drawable name={urgent ? 'ic_flight' : 'ic_warning'} size={24} className="shrink-0" style={{ color: urgent ? md('error') : aircast('warning') }} />
          <div className="flex min-w-0 flex-1 flex-col">
            <Text typography="titleSmall" maxLines={1} ellipsis>
              {alert.title}
            </Text>
            {alert.detail.trim() !== '' ? (
              <Text typography="bodyMedium" maxLines={1} ellipsis>
                {alert.detail}
              </Text>
            ) : null}
          </div>
        </div>
      </Surface>
    </>
  );
};

const TrafficSheet = ({ reading, onDismiss }: { reading: TrafficReading; onDismiss: () => void }) => {
  const urgent = md('error');
  return (
    <AircastSheet onDismissRequest={onDismiss}>
      <Text typography="titleSmall" className="block px-sheet">
        {trafficSummary(reading)}
      </Text>
      <Text typography="bodySmall" color={md('onSurfaceVariant')} className="block px-sheet py-1">
        {trafficCaption(reading)}
      </Text>
      <div className="w-full overflow-auto pb-6">
        {reading.contacts.map((contact) => {
          const colour = trafficContactUrgent(contact) ? urgent : undefined;
          return (
            <div key={contact.icaoAddress} className="flex w-full gap-3 px-sheet py-1.5">
              <Text typography="labelLarge" color={colour}>
                {contact.name}
              </Text>
              <Text typography="bodyMedium" color={colour} className="flex-1">
                {trafficContactText(contact, reading.units)}
              </Text>
            </div>
          );
        })}
      </div>
    </AircastSheet>
  );
};
