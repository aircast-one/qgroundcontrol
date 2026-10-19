import { useContext, useEffect, useMemo, useRef, useState } from 'react';
import { isJson, optText } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { openDocument } from '@/kotlin/Platform';
import { AssistChip, Button, CircularProgressIndicator, OutlinedButton, OutlinedTextField, Text, TextButton } from '@/m3';
import { MaterialIcon } from '@/res/Icon';
import { aircast, md } from '@/theme/Colors';
import {
  type CameraWizardView,
  DEVICE_WIZARD,
  type DeviceWizard,
  NO_FORM,
  NO_WIZARD,
  PI_CAMERA,
  STEP_NAMES,
  type WizardCamera,
  type PortAddress,
  type WizardPort,
  type WizardRefusal,
  applyCamera,
  continueLeads,
  fixWizardPort,
  openWizard,
  parseDeviceWizard,
  restartWizard,
  sameCamera,
  scanForCameras,
  scanLeads,
  setUpCameraMake,
  signedInUrl,
  wizardForm,
  wizardNext,
} from '@/ui/DeviceCameraWizard';
import { DeviceSelect } from '@/ui/DeviceKit';
import { Fold, useFold } from '@/ui/DeviceKitCamera';
import { NO_ANSWER, deviceUpload } from '@/ui/DeviceView';
import { LocalChangeNotice } from '@/ui/ValueRow';

const errorOf = (json: unknown, fallback: string): string => optText(isJson(json) ? json : null, 'error') || fallback;

type PortInput = { readonly open: boolean; readonly text: string };

export const CameraWizard = ({
  view,
  scan = false,
  fixPort = null,
  onCancel = null,
  onStep = () => undefined,
  onDone,
}: {
  view: CameraWizardView;
  scan?: boolean;
  fixPort?: PortAddress | null;
  onCancel?: (() => void) | null;
  onStep?: (step: 1 | 2 | 3) => void;
  onDone: (name: string) => void;
}) => {
  const notice = useContext(LocalChangeNotice);
  const [step, setStep] = useState<1 | 2 | 3>(1);
  const [rtsp, setRtsp] = useState(view.rtsp);
  const [name, setName] = useState(view.suggestedName);
  const address = useFold(view.addressOpen);
  const naming = useFold(false);
  const tech = useFold(fixPort !== null);
  const portFold = useFold(false);
  const fileFold = useFold(false);
  const [refusal, setRefusal] = useState<WizardRefusal | null>(null);
  const [dupAck, setDupAck] = useState('');
  const [inputs, setInputs] = useState<Readonly<Record<string, PortInput>>>(fixPort === null ? {} : { [fixPort.port]: { open: true, text: fixPort.address } });
  const [picked, setPicked] = useState<WizardCamera | null>(null);
  const [editRtsp, setEditRtsp] = useState(false);
  const [base, setBase] = useState('');
  const [signIn, setSignIn] = useState({ shown: false, asked: false });
  const [user, setUser] = useState('');
  const [pass, setPass] = useState('');
  const [iface, setIface] = useState(view.iface);
  const [ifaceTouched, setIfaceTouched] = useState(false);
  const [ip, setIp] = useState('');
  const [transport, setTransport] = useState(view.transport);
  const [form, setForm] = useState(NO_FORM);
  const [session, setSession] = useState(-1);
  const [file, setFile] = useState<File | null>(null);
  const [uploading, setUploading] = useState(false);
  const seen = useRef<DeviceWizard>(NO_WIZARD);
  const handling = useRef<Promise<void>>(Promise.resolve());
  const errorLine = useRef<HTMLDivElement>(null);
  const shown = useQgcPath(DEVICE_WIZARD);
  const wizard = useMemo(() => {
    const parsed = parseDeviceWizard(shown);
    return parsed.session === session ? parsed : NO_WIZARD;
  }, [shown, session]);

  const locked = picked !== null && picked.pathFound && !editRtsp;
  const cameraName = view.isNew ? name.trim().toLowerCase() : view.name;
  const current = view.interfaces.find((option) => option.value === iface)?.address ?? '';
  const applyField = wizard.apply.state === 'failed' ? wizard.apply.field : '';

  useEffect(() => {
    if (refusal !== null) errorLine.current?.scrollIntoView({ block: 'nearest' });
  }, [refusal]);

  useEffect(() => {
    onStep(step);
  }, [step]);

  useEffect(() => {
    void openWizard(view, scan).then((opened) => {
      setSession(opened);
      if (fixPort !== null) void fixWizardPort(fixPort.port, fixPort.address);
    });
  }, []);

  useEffect(() => {
    void wizardForm(rtsp, iface, ip, current).then(setForm);
  }, [rtsp, iface, ip, current, wizard.scans, wizard.scanning]);

  const pick = async (cam: WizardCamera): Promise<string> => {
    const keep = cam.needsAuth;
    setPicked(cam);
    setEditRtsp(false);
    if (!cam.pathFound) address.onToggle(true);
    setBase(cam.url);
    setSignIn({ shown: cam.needsAuth, asked: false });
    if (!keep) {
      setUser('');
      setPass('');
    }
    const url = await signedInUrl(cam.url, keep ? user : '', keep ? pass : '');
    if (cam.url !== '') setRtsp(url);
    return url;
  };

  const askSignIn = (url: string, reason: string) => {
    setBase(url);
    setSignIn({ shown: true, asked: true });
    setStep(1);
    setRefusal({ text: reason, warn: false, field: null });
  };

  const apply = (url: string, camera: string, port: string, portIp: string, how: string) => {
    setStep(3);
    void applyCamera(url, camera, port, portIp, how);
  };

  const advance = async (typed: string, typedName: string): Promise<void> => {
    const out = await wizardNext(typed, typedName, dupAck);
    if (out === null) return;
    setRtsp(out.rtsp);
    if (out.name !== null) setName(out.name);
    setDupAck(out.dupAck);
    if (out.refusal !== null) {
      setRefusal(out.refusal);
      if (out.refusal.field === 'rtsp') {
        setEditRtsp(true);
        address.onToggle(true);
      }
      if (out.refusal.field === 'name') naming.onToggle(true);
      return;
    }
    setRefusal(null);
    if (out.kind === 'apply') {
      setIface('');
      setIp('');
      apply(out.rtsp, out.camera, '', '', transport);
      return;
    }
    if (out.iface !== '' && !ifaceTouched) setIface(out.iface);
    if (ip === '') setIp(out.ip);
    setStep(2);
  };

  const happened = async (before: DeviceWizard, now: DeviceWizard): Promise<void> => {
    if (now.scans > before.scans) {
      setPicked(null);
      setBase('');
      setSignIn({ shown: false, asked: false });
      setInputs({});
      if (now.adopt !== null) await pick(now.adopt);
    }
    if (now.template.n > before.template.n) {
      const found = now.template.camera;
      if (found === null) setRefusal({ text: now.template.refusal, warn: false, field: null });
      else {
        const url = await pick(found);
        const camera = now.template.name || name;
        setName(camera);
        await advance(url, camera);
      }
    }
    if (now.notice.n > before.notice.n) notice.show(now.notice.text, null);
    if (now.apply.n > before.apply.n && now.apply.state === 'live') onDone(now.apply.camera);
    if (now.apply.n > before.apply.n && now.apply.state === 'signIn') askSignIn(now.apply.url, now.apply.text);
  };

  useEffect(() => {
    const before = seen.current;
    seen.current = wizard;
    handling.current = handling.current.then(() => happened(before, wizard));
  }, [wizard]);

  const restart = () => {
    void restartWizard();
    setRtsp('');
    setIp('');
    setTransport(view.transports[0]?.value ?? view.transport);
    setRefusal(null);
    setPicked(null);
    setBase('');
    setSignIn({ shown: false, asked: false });
    setEditRtsp(false);
    setStep(1);
  };

  const credentials = (nextUser: string, nextPass: string) => {
    setUser(nextUser);
    setPass(nextPass);
    if (base !== '') void signedInUrl(base, nextUser, nextPass).then(setRtsp);
  };

  const input = (port: WizardPort): PortInput => inputs[port.iface] ?? { open: port.open, text: port.suggest };
  const changeInput = (port: WizardPort, change: Partial<PortInput>) => setInputs((all) => ({ ...all, [port.iface]: { ...input(port), ...change } }));

  const upload = async () => {
    if (cameraName === '') return notice.show('Name the camera first.', null);
    if (file === null) return notice.show('Choose a video file.', null);
    setUploading(true);
    const { refusal, result } = await deviceUpload('PUT', `/api/streams/${encodeURIComponent(cameraName)}/file`, [{ name: 'file', file }]);
    setUploading(false);
    if (refusal === NO_ANSWER) return notice.show('Upload failed — check the file size and try again.', null);
    if (refusal !== null) return notice.show(errorOf(result, 'Upload failed.'), null);
    notice.show('Video source added.', null);
    onDone(cameraName);
  };

  const Scan = scanLeads(rtsp, picked !== null) ? Button : OutlinedButton;
  const Next = continueLeads(rtsp) ? Button : OutlinedButton;

  return (
    <div className="flex flex-col gap-3 px-list py-2" role="group" aria-label={view.isNew ? 'New camera' : `Set up ${view.name}`}>
      <StepNames step={step} skipsPort={step === 1 && form.skipsPort} />
      {step === 1 ? (
        <>
          {view.piCamera !== null ? (
            <div className="flex flex-wrap items-center gap-3 rounded-shape-md bg-surface-container-high px-4 py-3">
              <Text typography="bodyMedium" className="min-w-0 flex-1">
                {view.piCamera.note}
              </Text>
              <Button onClick={() => void advance(PI_CAMERA, name)}>{view.piCamera.action}</Button>
            </div>
          ) : null}
          {wizard.templates.length > 0 ? (
            <div className="flex flex-col gap-2" role="group" aria-label="Camera makes">
              {wizard.templates.map((t) => (
                <button
                  key={t.id}
                  type="button"
                  disabled={wizard.templateBusy}
                  onClick={() => {
                    setRefusal(null);
                    void setUpCameraMake(t.id);
                  }}
                  className="flex w-full items-center gap-3 rounded-shape-md border border-outline-variant px-4 py-3 text-left hover:bg-[color-mix(in_srgb,var(--md-on-surface)_8%,transparent)] disabled:opacity-60"
                >
                  <span className="flex min-w-0 flex-1 flex-col">
                    <Text typography="bodyLarge">{t.label}</Text>
                    <Text typography="bodySmall" color={md('onSurfaceVariant')}>
                      {t.sub}
                    </Text>
                  </span>
                  <Text typography="labelLarge" color={md('primary')}>
                    {t.action}
                  </Text>
                </button>
              ))}
              {wizard.templateNote !== '' ? <Hint text={wizard.templateNote} /> : null}
            </div>
          ) : null}
          {view.lookingFor !== '' ? (
            <Text typography="titleSmall" color={aircast('warning')}>
              {view.lookingFor}
            </Text>
          ) : null}
          {wizard.checklist === '' ? (
            <div className="flex flex-col items-start gap-2">
              <Hint text="Plug the camera in and look for it." />
              <Scan enabled={!wizard.scanning} onClick={() => void scanForCameras()}>
                {wizard.scanning ? 'Looking…' : 'Scan for cameras'}
              </Scan>
            </div>
          ) : null}
          {wizard.note !== '' ? <Hint text={wizard.note} /> : null}
          {wizard.checklist !== '' ? (
            <div className="flex flex-col gap-2 rounded-shape-md border border-outline-variant px-4 py-3" role="group" aria-label="Checklist">
              <Text typography="titleSmall">{wizard.checklist}</Text>
              <ul className="flex list-none flex-col gap-1 text-body-medium text-on-surface-variant">
                <li>☐ The cable goes from the camera to this device's camera port</li>
                <li>☐ The camera has power — its light is on</li>
                <li>☐ Give it 30 seconds after power-on</li>
              </ul>
              <div>
                <Button enabled={!wizard.scanning} onClick={() => void scanForCameras()}>
                  {wizard.scanning ? 'Looking…' : 'Scan again'}
                </Button>
              </div>
            </div>
          ) : null}
          {wizard.cams.length > 0 ? (
            <div className="flex flex-col gap-2">
              <Text typography="labelLarge" color={md('onSurfaceVariant')}>
                {wizard.camHeading}
              </Text>
              {wizard.cams.map((cam) => {
                const selected = sameCamera(picked, cam);
                return (
                  <button
                    key={`${cam.host} ${cam.url}`}
                    type="button"
                    aria-pressed={selected}
                    aria-label={`${cam.host} — ${cam.note}`}
                    onClick={() => void pick(cam)}
                    className={`flex w-full items-center gap-3 rounded-shape-md border px-4 py-3 text-left ${selected ? 'border-primary bg-[color-mix(in_srgb,var(--md-primary)_8%,transparent)]' : 'border-outline-variant hover:bg-[color-mix(in_srgb,var(--md-on-surface)_8%,transparent)]'}`}
                  >
                    <span className="flex min-w-0 flex-1 flex-col">
                      <Text typography="bodyLarge" monospace>
                        {cam.host}
                      </Text>
                      <Text typography="bodySmall" color={md('onSurfaceVariant')}>
                        {cam.note}
                      </Text>
                    </span>
                    <Text typography="labelLarge" color={md('primary')}>
                      {selected ? 'Selected' : cam.action}
                    </Text>
                  </button>
                );
              })}
              {wizard.camsHint !== '' ? <Hint text={wizard.camsHint} /> : null}
            </div>
          ) : null}
          {wizard.details !== '' ? (
            <Fold title={wizard.details} {...tech}>
              <ScanDetails wizard={wizard} coverage={view.coverage} input={input} onInput={changeInput} onFix={(port) => void fixWizardPort(port.iface, input(port).text.trim())} />
            </Fold>
          ) : null}
          {signIn.shown ? (
            <div className="flex flex-col gap-3 rounded-shape-md border border-outline-variant px-4 py-3" role="group" aria-label="Camera sign-in">
              <Hint text="This camera asked for a sign-in. Enter the username and password set in the camera's own app." />
              <div className="grid grid-cols-2 gap-3 max-sm:grid-cols-1">
                <OutlinedTextField label="Camera username" value={user} autoFocus={signIn.asked} autoCorrect={false} onValueChange={(next) => credentials(next, pass)} />
                <OutlinedTextField label="Camera password" value={pass} keyboardType="password" onValueChange={(next) => credentials(user, next)} />
              </div>
            </div>
          ) : null}
          {locked ? (
            <div className="flex min-w-0 items-baseline gap-2" role="group" aria-label="Stream">
              <Text typography="labelLarge" color={md('onSurfaceVariant')}>
                Stream
              </Text>
              <Text typography="bodyMedium" monospace className="min-w-0 flex-1 truncate">
                {base}
              </Text>
              <TextButton
                onClick={() => {
                  setEditRtsp(true);
                  address.onToggle(true);
                }}
              >
                Change
              </TextButton>
            </div>
          ) : (
            <Fold title="I know my camera's address" {...address}>
              <OutlinedTextField
                label="Camera RTSP URL"
                value={rtsp}
                placeholder={view.rtspHint}
                keyboardType="uri"
                autoCorrect={false}
                className="max-w-md"
                isError={refusal?.field === 'rtsp' || applyField === 'rtsp'}
                supportingText={picked === null ? "Find it in the camera's app or manual — for example rtsp://192.168.8.10:554/stream" : undefined}
                onValueChange={setRtsp}
              />
            </Fold>
          )}
          {view.isNew ? (
            <Fold title={`Named ${name.trim().toLowerCase() || '…'} — change`} {...naming}>
              <OutlinedTextField
                label="Camera name"
                value={name}
                placeholder="cam"
                autoCorrect={false}
                className="max-w-md"
                isError={refusal?.field === 'name' || applyField === 'name'}
                supportingText={`Used in the stream URLs (rtsp://…/${view.suggestedName}) — lowercase letters, numbers, dashes.`}
                onValueChange={setName}
              />
            </Fold>
          ) : null}
          {refusal !== null ? (
            <div ref={errorLine} role="alert" className="flex items-start gap-2">
              <MaterialIcon name="warning" size={18} style={{ color: refusal.warn ? aircast('warning') : md('error') }} />
              <Text typography="bodyMedium" color={refusal.warn ? aircast('warning') : md('error')}>
                {refusal.text}
              </Text>
            </div>
          ) : null}
          <div className="flex flex-wrap items-center gap-2">
            <Next onClick={() => void advance(rtsp, name)}>{form.next}</Next>
            {onCancel !== null ? <TextButton onClick={onCancel}>Cancel</TextButton> : null}
          </div>
          <Fold title="No camera? Use a video file" {...fileFold}>
            <Hint text="Upload a video and the device loops it as a stream — a stand-in camera for trying out detection and the overlays." />
            <div className="flex flex-wrap items-center gap-2">
              <OutlinedButton onClick={() => void openDocument(['video/*']).then((chosen) => chosen !== null && setFile(chosen))}>Choose a video file</OutlinedButton>
              <Text typography="bodyMedium" color={md('onSurfaceVariant')} className="min-w-0 truncate">
                {file?.name ?? 'H.264 .mp4 works best'}
              </Text>
            </div>
            <div>
              <OutlinedButton enabled={!uploading} onClick={() => void upload()}>
                {uploading ? 'Uploading…' : 'Upload & start'}
              </OutlinedButton>
            </div>
          </Fold>
        </>
      ) : null}
      {step === 2 ? (
        <>
          <DeviceSelect
            label="Which port is the camera plugged into?"
            value={iface}
            options={[{ value: '', label: 'Select a port' }, ...view.interfaces]}
            onPick={(next) => {
              setIface(next);
              setIfaceTouched(true);
            }}
          />
          <Hint text="Usually the Ethernet port your camera's cable connects to. Each port shows its own address — the camera has to be on that same subnet to be reachable." />
          <Fold title="Port address & transport" summary="This device's address must be on the same subnet as the camera's URL above — change it only if yours differs." {...portFold}>
            <OutlinedTextField
              label="This device's IP on the camera subnet"
              value={ip}
              placeholder="192.168.8.2/24"
              keyboardType="uri"
              autoCorrect={false}
              className="max-w-md"
              isError={applyField === 'address'}
              supportingText="Suggested from the camera's address — edit if your subnet differs."
              onValueChange={setIp}
            />
            <div className="max-w-md">
              <DeviceSelect label="RTSP transport" value={transport} options={view.transports} onPick={setTransport} />
            </div>
            <Hint text="Leave on Automatic; switch to TCP if the video stays black or choppy." />
          </Fold>
          <div className="flex flex-wrap items-center justify-between gap-2">
            <OutlinedButton onClick={() => setStep(1)}>← Back</OutlinedButton>
            <Button onClick={() => apply(rtsp, cameraName, iface, ip, transport)}>Set up & verify</Button>
          </div>
          {form.applyNote !== '' ? <Hint text={form.applyNote} /> : null}
        </>
      ) : null}
      {step === 3 ? (
        <>
          <div role="status" aria-live="polite" className="flex items-center gap-3">
            {wizard.apply.state !== 'failed' ? <CircularProgressIndicator size={20} strokeWidth={3} label="Working" /> : <MaterialIcon name="error" size={20} style={{ color: md('error') }} />}
            <Text typography="bodyLarge" color={wizard.apply.state === 'failed' ? md('error') : md('onSurface')}>
              {wizard.apply.text || 'Applying…'}
            </Text>
          </div>
          <div className="flex flex-wrap items-center justify-between gap-2">
            <TextButton onClick={restart}>Start over</TextButton>
            {wizard.apply.state === 'failed' ? (
              <span className="flex flex-wrap gap-2">
                <OutlinedButton onClick={() => setStep(2)}>Change settings</OutlinedButton>
                <Button onClick={() => apply(rtsp, cameraName, iface, ip, transport)}>Try again</Button>
              </span>
            ) : null}
          </div>
        </>
      ) : null}
    </div>
  );
};

const Hint = ({ text }: { text: string }) => (
  <Text typography="bodySmall" color={md('onSurfaceVariant')}>
    {text}
  </Text>
);

const StepNames = ({ step, skipsPort }: { step: number; skipsPort: boolean }) => (
  <ol className="flex flex-wrap items-center gap-2" aria-label="Steps">
    {STEP_NAMES.map((label, index) => (
      <li key={label} aria-current={index + 1 === step ? 'step' : undefined} className="flex items-center gap-2">
        <span className={`h-1.5 w-6 rounded-full ${index + 1 <= step ? 'bg-primary' : 'bg-outline-variant'} ${index === 1 && skipsPort ? 'opacity-40' : ''}`} />
        <Text typography="labelMedium" color={index + 1 === step ? md('onSurface') : md('onSurfaceVariant')} className={index === 1 && skipsPort ? 'line-through' : ''}>
          {label}
        </Text>
      </li>
    ))}
  </ol>
);

const PortRow = ({ label, detail }: { label: string; detail: string }) => (
  <div className="flex flex-col rounded-shape-md border border-outline-variant px-4 py-3" role="group" aria-label={label}>
    <Text typography="bodyLarge">{label}</Text>
    <Text typography="bodySmall" color={md('onSurfaceVariant')}>
      {detail}
    </Text>
  </div>
);

const ScanDetails = ({
  wizard,
  coverage,
  input,
  onInput,
  onFix,
}: {
  wizard: DeviceWizard;
  coverage: string;
  input: (port: WizardPort) => PortInput;
  onInput: (port: WizardPort, change: Partial<PortInput>) => void;
  onFix: (port: WizardPort) => void;
}) => {
  const restFold = useFold(false);
  const checkedFold = useFold(false);
  const rest = wizard.rest.map((it) => <PortRow key={it.iface} label={it.label} detail={it.detail} />);
  const covered = <Hint text={coverage} />;
  return (
    <>
      {wizard.todo.length > 0 ? (
        <div className="flex flex-col gap-2">
          <Text typography="labelLarge" color={md('onSurfaceVariant')}>
            {wizard.todoHeading}
          </Text>
          {wizard.todo.map((it) => (
            <PortFix key={it.iface} port={it} input={input(it)} onInput={(change) => onInput(it, change)} onFix={() => onFix(it)} />
          ))}
        </div>
      ) : null}
      {wizard.rest.length > 0 && wizard.todo.length > 0 ? (
        <Fold title={wizard.restTitle} {...restFold}>
          {rest}
        </Fold>
      ) : null}
      {wizard.rest.length > 0 && wizard.todo.length === 0 && wizard.cams.length > 0 ? (
        <>
          {rest}
          {covered}
        </>
      ) : null}
      {wizard.rest.length > 0 && wizard.todo.length === 0 && wizard.cams.length === 0 ? (
        <>
          <Text typography="labelLarge" color={md('onSurfaceVariant')}>
            Ports checked
          </Text>
          {rest}
        </>
      ) : null}
      {wizard.cams.length === 0 ? (
        <Fold title="What was checked?" {...checkedFold}>
          {covered}
        </Fold>
      ) : null}
    </>
  );
};

const PortFix = ({ port, input, onInput, onFix }: { port: WizardPort; input: PortInput; onInput: (change: Partial<PortInput>) => void; onFix: () => void }) => {
  const Go = port.suggest !== '' ? Button : OutlinedButton;
  return (
    <div className="flex flex-col gap-2 rounded-shape-md border border-primary px-4 py-3" role="group" aria-label={port.label}>
      <Text typography="bodyLarge">{port.label}</Text>
      <Text typography="bodySmall" color={md('onSurfaceVariant')}>
        {port.detail}
      </Text>
      {input.open ? (
        <>
          <OutlinedTextField
            label="This port's address"
            value={input.text}
            placeholder="192.168.144.2/24"
            keyboardType="uri"
            autoCorrect={false}
            className="max-w-xs"
            onValueChange={(text) => onInput({ text })}
          />
          <div className="flex flex-wrap gap-2">
            {port.options.map((option) => (
              <AssistChip key={option.cidr} onClick={() => onInput({ text: option.cidr })} label={option.text} />
            ))}
          </div>
          <Hint text={port.hint} />
          <div>
            <Go enabled={!port.busy && input.text.trim() !== ''} onClick={onFix}>
              {port.go}
            </Go>
          </div>
          {port.status !== '' ? <Hint text={port.status} /> : null}
        </>
      ) : (
        <div>
          <OutlinedButton onClick={() => onInput({ open: true })}>Change this port's address</OutlinedButton>
        </div>
      )}
    </div>
  );
};
