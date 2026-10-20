import { type Json, optText } from '@/bridge/Json';
import { Checkbox, Text } from '@/m3';
import { ChoiceField } from '@/ui/SettingsScreen';

export const MOCK_VEHICLES: readonly (readonly [string, string])[] = [
  ['px4', 'PX4 Vehicle'],
  ['apmCopter', 'APM ArduCopter Vehicle'],
  ['apmPlane', 'APM ArduPlane Vehicle'],
  ['apmSub', 'APM ArduSub Vehicle'],
  ['apmRover', 'APM ArduRover Vehicle'],
  ['generic', 'Generic Vehicle'],
];

export const MOCK_VIDEO_STREAMS: readonly string[] = ['Disabled', 'RTP/UDP H.264', 'RTP/UDP H.265', 'RTSP (H.264)', 'MPEG-TS (UDP)', 'MPEG-TS (TCP)'];

export type MockLinkChoices = {
  sendStatusText: boolean;
  camera: boolean;
  gimbal: boolean;
  proximity: boolean;
  freshParams: boolean;
  vehicle: number;
  videoStream: number;
};

export const MockLinkChoices = (fields: Partial<MockLinkChoices> = {}): MockLinkChoices => ({
  sendStatusText: false,
  camera: false,
  gimbal: false,
  proximity: false,
  freshParams: false,
  vehicle: 0,
  videoStream: 0,
  ...fields,
});

export const preferredMockChoices = (linksView: Json | null): MockLinkChoices =>
  MockLinkChoices({ vehicle: Math.max(0, MOCK_VEHICLES.findIndex(([key]) => key === optText(linksView, 'mockVehicle'))) });

const vehicleKey = (choices: MockLinkChoices): string => MOCK_VEHICLES[choices.vehicle]?.[0] ?? '';

const vehicleLabel = (choices: MockLinkChoices): string => MOCK_VEHICLES[choices.vehicle]?.[1] ?? '';

export const mockVehicleIsApm = (choices: MockLinkChoices): boolean => vehicleKey(choices).startsWith('apm');

export const mockLinkArguments = (choices: MockLinkChoices): readonly unknown[] => [
  vehicleKey(choices),
  choices.sendStatusText,
  choices.camera,
  choices.gimbal,
  choices.proximity,
  choices.freshParams && mockVehicleIsApm(choices),
  choices.videoStream,
];

export const withMockVehicle = (choices: MockLinkChoices, vehicle: number): MockLinkChoices => {
  const picked = { ...choices, vehicle };
  return mockVehicleIsApm(picked) ? picked : { ...picked, freshParams: false };
};

const MockCheck = ({ text, checked, onChecked }: { text: string; checked: boolean; onChecked: (checked: boolean) => void }) => (
  <label className="flex w-full cursor-pointer items-center">
    <span className="flex p-3">
      <Checkbox checked={checked} onCheckedChange={onChecked} />
    </span>
    <Text typography="bodyLarge">{text}</Text>
  </label>
);

export const MockLinkFields = ({ choices, onChange }: { choices: MockLinkChoices; onChange: (choices: MockLinkChoices) => void }) => (
  <div className="flex flex-col gap-1">
    <ChoiceField label="Vehicle Type" value={vehicleLabel(choices)} options={MOCK_VEHICLES.map((it) => it[1])} className="w-full" onPick={(it) => onChange(withMockVehicle(choices, it))} />
    <MockCheck text="Send status text + voice" checked={choices.sendStatusText} onChecked={(it) => onChange({ ...choices, sendStatusText: it })} />
    <MockCheck text="Enable camera" checked={choices.camera} onChecked={(it) => onChange({ ...choices, camera: it })} />
    <MockCheck text="Enable gimbal" checked={choices.gimbal} onChecked={(it) => onChange({ ...choices, gimbal: it })} />
    <MockCheck text="Enable proximity sensors" checked={choices.proximity} onChecked={(it) => onChange({ ...choices, proximity: it })} />
    {mockVehicleIsApm(choices) ? (
      <MockCheck text="Start with fresh firmware parameters (setup required)" checked={choices.freshParams} onChecked={(it) => onChange({ ...choices, freshParams: it })} />
    ) : null}
    {choices.camera ? (
      <ChoiceField
        label="Served Video Stream"
        value={MOCK_VIDEO_STREAMS[choices.videoStream] ?? ''}
        options={MOCK_VIDEO_STREAMS}
        className="w-full pt-2"
        onPick={(it) => onChange({ ...choices, videoStream: it })}
      />
    ) : null}
  </div>
);
