import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { Surface, Text } from '@/m3';
import { md } from '@/theme/Colors';
import { liveSlotText, liveSwitchesText, MODE_SLOTS, modeSlotsView } from '@/ui/ModeSlots';
import { ParameterForm } from '@/ui/ParameterForm';
import { PwmBar } from '@/ui/RadioScreen';
import { RADIO_VIEW, radioCalAction, radioView } from '@/ui/RadioView';
import { FLIGHT_MODES_PAGE } from '@/ui/SetupPages';

export const FlightModesSetup = ({ className }: { className?: string }) => {
  const view = useQgcPath(MODE_SLOTS);
  const slots = modeSlotsView(view);
  const joined = [liveSlotText(slots), liveSwitchesText(slots)].filter((it): it is string => it !== null).join('\n');
  const live = joined.trim() === '' ? null : joined;

  return (
    <div className={`flex flex-col ${className ?? ''}`}>
      {live !== null ? (
        <Surface className="w-full" color={md('surfaceVariant')}>
          <Text typography="bodyMedium" className="block whitespace-pre-line px-4 py-2.5">
            {live}
          </Text>
        </Surface>
      ) : null}
      <ParameterForm page={FLIGHT_MODES_PAGE} className="w-full" highlighted={slots?.activeParams ?? new Set()} />
      {slots?.channelMonitor === true ? <ChannelMonitor /> : null}
    </div>
  );
};

const ChannelMonitor = () => {
  const json = useQgcPath(RADIO_VIEW);
  const channels: readonly { label: string; fraction: number }[] = radioView(json)?.channels ?? [];
  useLaunchedEffect([], async () => {
    await Qgc.invoke(radioCalAction('start'));
  });
  return (
    <div className="flex w-full flex-col px-5 py-2">
      <Text typography="titleSmall">Channel monitor</Text>
      {channels.map((channel, index) => (
        <div key={index} className="flex w-full items-center py-1">
          <Text typography="bodySmall" className="w-7 shrink-0">
            {channel.label}
          </Text>
          <PwmBar fraction={channel.fraction} className="flex-1" />
        </div>
      ))}
    </div>
  );
};
