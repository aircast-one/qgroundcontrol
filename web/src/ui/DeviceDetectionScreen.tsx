import { useContext, useState } from 'react';
import { Checkbox, FilledTonalButton, OutlinedTextField, Text, TextButton } from '@/m3';
import { md } from '@/theme/Colors';
import { type DetectCamera, type DetectModel, type DetectionView, NO_LABELS, addLabels, detecting, modelPath, parseDetection, uploadModel } from '@/ui/DeviceDetection';
import { ActionButton, DeviceBadgeText, DeviceColumns, DeviceDisclosure, DeviceNotice, DeviceSection, DeviceSwitchRow, DeviceText, RefusalText } from '@/ui/DeviceKit';
import { StackedFact, StatusLine } from '@/ui/DeviceKitSettings';
import type { DeviceScreenProps } from '@/ui/DeviceScreens';
import { DeviceLevel } from '@/ui/DeviceLevel';
import { type DeviceRefusal, useDeviceView } from '@/ui/DeviceView';
import { FootNote } from '@/ui/SetupRow';
import { LocalChangeNotice } from '@/ui/ValueRow';

const UNAVAILABLE = "This device has no AI accelerator, so .hef models can't run. Switch to an .onnx model to detect on the CPU. Draw-a-box tracking still works on every camera.";

export const DetectionScreen = ({ onOpen }: DeviceScreenProps) => {
  const view = useDeviceView('detection', parseDetection);
  if (view === null) return <FootNote text="Waiting for the device to answer…" />;
  return (
    <DeviceColumns
      left={
        <>
          <RunningSection view={view} />
          <CamerasSection cameras={view.cameras} onOpen={onOpen} />
        </>
      }
      right={
        <>
          <ModelsSection view={view} />
          <StatusLine text="Detection runs on the device and sends boxes as data — the picture itself is untouched. Pick the model for what the camera is looking at: an aerial model hallucinates trucks on a desk. Draw-a-box tracking works whichever model is active." />
        </>
      }
    />
  );
};

const RunningSection = ({ view }: { view: DetectionView }) => (
  <DeviceSection title="Running">
    <StackedFact label="In use" value={view.status.text} />
    {view.unavailable ? <DeviceNotice notice={{ level: DeviceLevel.Warn, text: UNAVAILABLE }} /> : null}
  </DeviceSection>
);

const CamerasSection = ({ cameras, onOpen }: { cameras: readonly DetectCamera[]; onOpen: (href: string) => void }) => (
  <DeviceSection title="Cameras">
    {cameras.length === 0 ? (
      <>
        <DeviceText text="No cameras yet." />
        <div className="px-list">
          <TextButton onClick={() => onOpen('/streaming')}>Cameras →</TextButton>
        </div>
      </>
    ) : (
      cameras.map((camera) => (
        <DeviceSwitchRow
          key={camera.name}
          title={`Draw boxes on ${camera.name}`}
          detail={camera.status.text}
          checked={camera.on}
          on={{ method: 'PUT', path: `/api/streams/${camera.name}/detect` }}
          off={{ method: 'DELETE', path: `/api/streams/${camera.name}/detect` }}
          success={`Detection turned on for ${camera.name}.`}
          offSuccess={`Detection turned off for ${camera.name}.`}
        />
      ))
    )}
  </DeviceSection>
);

const ModelsSection = ({ view }: { view: DetectionView }) => {
  const [round, setRound] = useState(0);
  return (
    <DeviceSection title="Models">
      {view.models.length === 0 ? <DeviceText text="No models installed yet." /> : view.models.map((model) => <ModelRow key={model.name} model={model} />)}
      <DeviceDisclosure key={round} title="Add a model" summary="A .hef for the Hailo-8 or an .onnx for the CPU.">
        <ModelUploadForm cameraDetecting={detecting(view)} onUploaded={() => setRound(round + 1)} />
      </DeviceDisclosure>
    </DeviceSection>
  );
};

const ModelRow = ({ model }: { model: DetectModel }) => (
  <div className="flex flex-wrap items-center gap-x-3 gap-y-1 px-list py-2" role="group" aria-label={model.name}>
    <div className="flex min-w-0 flex-1 basis-48 flex-col">
      <Text typography="bodyLarge" className="break-all">
        {model.name}
      </Text>
      {model.about !== '' ? (
        <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
          {model.about}
        </Text>
      ) : null}
      <span className="flex items-center gap-3">
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          {model.size}
        </Text>
        <DeviceBadgeText badge={model.classes} />
        {model.engine !== '' ? (
          <Text typography="bodySmall" color={md('onSurfaceVariant')}>
            {model.engine}
          </Text>
        ) : null}
      </span>
      {model.classes.text === NO_LABELS ? <AddLabels name={model.name} /> : null}
    </div>
    {model.active ? (
      <DeviceBadgeText badge={{ level: DeviceLevel.Ready, text: 'Active' }} />
    ) : (
      <span className="flex items-center gap-1">
        <ActionButton label="Use" request={{ method: 'PUT', path: '/api/stream/detect/settings', body: { hef: model.path } }} success="Model switched — detection restarts on the new model." />
        <ActionButton
          label="Delete"
          tonal={false}
          request={{ method: 'DELETE', path: modelPath(model.name) }}
          success="Model deleted."
          confirm={{ title: `Delete ${model.name}?`, text: `Delete ${model.name}? Anyone relying on it loses it from this device.`, confirm: 'Delete' }}
        />
      </span>
    )}
  </div>
);

const AddLabels = ({ name }: { name: string }) => {
  const notice = useContext(LocalChangeNotice);
  const [busy, setBusy] = useState(false);
  const [refusal, setRefusal] = useState<DeviceRefusal | null>(null);
  const add = (file: File | undefined) => {
    if (file === undefined) return;
    setBusy(true);
    void addLabels(name, file).then((refused) => {
      setBusy(false);
      setRefusal(refused);
      if (refused === null) notice.show(`Labels added to ${name}.`, null);
    });
  };
  return (
    <>
      <label className="w-fit cursor-pointer py-1">
        <input type="file" accept=".labels,.txt" aria-label={`Labels for ${name}`} className="sr-only" disabled={busy} onChange={(event) => add(event.target.files?.[0])} />
        <Text typography="labelLarge" color={md('primary')}>
          {busy ? 'Adding labels…' : 'Add labels'}
        </Text>
      </label>
      <RefusalText refusal={refusal} />
    </>
  );
};

const fileInput =
  'block w-full max-w-md cursor-pointer rounded-shape-xs border border-outline text-body-small text-on-surface-variant file:mr-3 file:cursor-pointer file:border-0 file:bg-secondary-container file:px-3 file:py-2 file:text-label-large file:text-on-secondary-container';

const ModelUploadForm = ({ cameraDetecting, onUploaded }: { cameraDetecting: boolean; onUploaded: () => void }) => {
  const notice = useContext(LocalChangeNotice);
  const [hef, setHef] = useState<File | null>(null);
  const [labels, setLabels] = useState<File | null>(null);
  const [about, setAbout] = useState('');
  const [use, setUse] = useState(true);
  const [busy, setBusy] = useState(false);
  const [refusal, setRefusal] = useState<DeviceRefusal | null>(null);
  const upload = () => {
    if (hef === null) return setRefusal({ message: 'Choose a .hef or .onnx model file.', field: 'hef' });
    setBusy(true);
    setRefusal(null);
    void uploadModel({ hef, labels, about, use }, cameraDetecting).then((outcome) => {
      setBusy(false);
      setRefusal(outcome.refusal);
      if (outcome.message !== '') notice.show(outcome.message, null);
      if (outcome.refusal === null) onUploaded();
    });
  };
  return (
    <div className="flex flex-col gap-3 px-list py-2">
      <label className="grid gap-label">
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          Model (.hef for the Hailo-8, .onnx for the CPU)
        </Text>
        <input type="file" accept=".hef,.onnx" aria-label="Model file" className={fileInput} onChange={(event) => setHef(event.target.files?.[0] ?? null)} />
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          Trained with Ultralytics? Export it first: yolo export model=best.pt format=onnx — the .onnx carries its class names.
        </Text>
      </label>
      <RefusalText refusal={refusal} field="hef" />
      <label className="grid gap-label">
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          Class labels (.labels, one per line — optional). An Ultralytics .onnx brings its own; without labels, boxes are named by class number.
        </Text>
        <input type="file" accept=".labels,.txt" aria-label="Class labels file" className={fileInput} onChange={(event) => setLabels(event.target.files?.[0] ?? null)} />
      </label>
      <OutlinedTextField label="What it's for (shown next to the model — optional)" ariaLabel="What it's for" value={about} placeholder="Aerial — vehicles at 400 ft" onValueChange={setAbout} />
      <span className="flex items-center gap-3">
        <Checkbox checked={use} label="Switch to this model after upload" onCheckedChange={setUse} />
        <Text typography="bodyMedium">Switch to this model after upload</Text>
      </span>
      <RefusalText refusal={refusal} />
      <div>
        <FilledTonalButton enabled={!busy} onClick={upload}>
          {busy ? 'Uploading…' : 'Upload model'}
        </FilledTonalButton>
      </div>
    </div>
  );
};
