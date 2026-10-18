import { useContext, useEffect, useRef, useState } from 'react';
import { isJson, optText } from '@/bridge/Json';
import { openDocument } from '@/kotlin/Platform';
import { delay } from '@/kotlin/Std';
import { AssistChip, Button, CircularProgressIndicator, OutlinedButton, OutlinedTextField, Text, TextButton } from '@/m3';
import { MaterialIcon } from '@/res/Icon';
import { aircast, md } from '@/theme/Colors';
import {
  type CameraTemplate,
  type CameraWizardView,
  type FoundCamera,
  NO_ANSWER_CHECKLIST,
  type PortFixer,
  SCANNING_NOTE,
  SCAN_FAILED_NOTE,
  STEP_NAMES,
  type ScanOutcome,
  type ScanPort,
  type WizardRefusal,
  adoptedCamera,
  applyNote,
  camHeading,
  camURL,
  checklistHint,
  commitsNow,
  continueLeads,
  fixerGoLabel,
  fixerHint,
  fixersFor,
  nextLabel,
  nextStep,
  parseCameraStatus,
  parseFoundCameras,
  parseScanPorts,
  portDetail,
  portHasSubnet,
  portToSuggest,
  rowNote,
  rtspHost,
  scanLeads,
  scanOutcome,
  subnetOptions,
  suggestIP,
  templateAction,
  templateHolderError,
  templatePort,
  templateReadyPort,
  templateSub,
  todoHeading,
} from '@/ui/DeviceCameraWizard';
import { DeviceSelect } from '@/ui/DeviceKit';
import { Fold, useFold } from '@/ui/DeviceKitCamera';
import { type DeviceMethod, type DeviceRefusal, NO_ANSWER, deviceRead, deviceUrl, refusalOf } from '@/ui/DeviceView';
import { LocalChangeNotice } from '@/ui/ValueRow';

const VERIFY_ATTEMPTS = 10;
const VERIFY_POLL_MS = 1500;
const PORT_UP_ATTEMPTS = 15;
const PORT_UP_POLL_MS = 1000;

type Sent = { readonly json: unknown; readonly refusal: DeviceRefusal | null };

const send = async (method: DeviceMethod, path: string, body?: unknown): Promise<Sent> => {
  const response = await fetch(deviceUrl(path), {
    method,
    headers: body === undefined ? { Accept: 'application/json' } : { Accept: 'application/json', 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  }).catch(() => null);
  if (response === null) return { json: null, refusal: NO_ANSWER };
  const json: unknown = await response.json().catch(() => null);
  return { json, refusal: response.ok ? null : refusalOf(response.status, json) };
};

const errorOf = (json: unknown, fallback: string): string => optText(isJson(json) ? json : null, 'error') || fallback;

const fetchPorts = async (): Promise<readonly ScanPort[]> => (await deviceRead('/api/streams/scan/ports', parseScanPorts)) ?? [];

const waitForPort = async (iface: string, want: string, attempt = 0): Promise<void> => {
  if (attempt >= PORT_UP_ATTEMPTS) return;
  const port = (await deviceRead('/api/streams/scan/ports', parseScanPorts))?.find((it) => it.iface === iface);
  if (port === undefined || (want !== '' ? portHasSubnet(port, want) : port.state !== 'configuring')) return;
  await delay(PORT_UP_POLL_MS);
  return waitForPort(iface, want, attempt + 1);
};

const setPortAddress = async (iface: string, address: string): Promise<string> => {
  const { json, refusal } = await send('POST', '/api/setup/camera/port', {
    interface: iface,
    address,
  });
  return refusal === null ? '' : refusal === NO_ANSWER ? refusal.message : errorOf(json, 'Could not set up this port.');
};

type Status = { readonly state: 'loading' | 'error'; readonly text: string };

const APPLYING: Status = { state: 'loading', text: 'Applying…' };

const NO_FIXER: PortFixer = { open: false, input: '', suggest: '', status: '', busy: false };

export const CameraWizard = ({ view, scan = false, onCancel = null, onDone }: { view: CameraWizardView; scan?: boolean; onCancel?: (() => void) | null; onDone: (name: string) => void }) => {
  const notice = useContext(LocalChangeNotice);
  const [step, setStep] = useState<1 | 2 | 3>(1);
  const [rtsp, setRtsp] = useState(view.rtsp);
  const [name, setName] = useState(view.suggestedName);
  const address = useFold(view.addressOpen);
  const naming = useFold(false);
  const tech = useFold(false);
  const portFold = useFold(false);
  const fileFold = useFold(false);
  const [refusal, setRefusal] = useState<WizardRefusal | null>(null);
  const [dupAck, setDupAck] = useState('');
  const [scanning, setScanning] = useState(false);
  const [scanNote, setScanNote] = useState('');
  const [outcome, setOutcome] = useState<ScanOutcome | null>(null);
  const [fixers, setFixers] = useState<Readonly<Record<string, PortFixer>>>({});
  const [picked, setPicked] = useState<FoundCamera | null>(null);
  const [editRtsp, setEditRtsp] = useState(false);
  const [base, setBase] = useState('');
  const [signIn, setSignIn] = useState({ shown: false, asked: false });
  const [user, setUser] = useState('');
  const [pass, setPass] = useState('');
  const [iface, setIface] = useState(view.iface);
  const [ifaceTouched, setIfaceTouched] = useState(false);
  const [ip, setIp] = useState('');
  const [transport, setTransport] = useState(view.transport);
  const [status, setStatus] = useState<Status>(APPLYING);
  const [applyField, setApplyField] = useState('');
  const [added, setAdded] = useState(false);
  const [template, setTemplate] = useState({ busy: '', note: '' });
  const [noPort, setNoPort] = useState(false);
  const [file, setFile] = useState<File | null>(null);
  const [uploading, setUploading] = useState(false);
  const generation = useRef(0);
  const errorLine = useRef<HTMLDivElement>(null);

  const ports = outcome?.ports ?? [];
  const cams = outcome?.cams ?? [];
  const locked = picked !== null && picked.pathFound && !editRtsp;
  const cameraName = view.isNew ? name.trim().toLowerCase() : view.name;

  useEffect(() => {
    if (refusal !== null) errorLine.current?.scrollIntoView({ block: 'nearest' });
  }, [refusal]);

  const pick = (cam: FoundCamera): string => {
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
    const url = camURL(cam.url, keep ? user : '', keep ? pass : '');
    if (cam.url !== '') setRtsp(url);
    return url;
  };

  const runScan = async (auto: boolean, typed: string): Promise<ScanOutcome | null> => {
    setScanning(true);
    setScanNote(SCANNING_NOTE);
    setOutcome(null);
    const { json, refusal: refused } = await send('POST', '/api/streams/scan');
    setScanning(false);
    if (refused !== null) {
      setScanNote(SCAN_FAILED_NOTE);
      return null;
    }
    const result = scanOutcome(parseFoundCameras(json, view.name), parseScanPorts(json), rtspHost(view.source));
    setOutcome(result);
    setFixers(fixersFor(result, view.subnets));
    setScanNote(result.note);
    setPicked(null);
    setBase('');
    setSignIn({ shown: false, asked: false });
    const adopt = adoptedCamera(result, auto, typed);
    if (adopt !== null) pick(adopt);
    return result;
  };

  useEffect(() => {
    if (view.isNew) {
      void fetchPorts().then((found) => setNoPort(found.length > 0 && !found.some((port) => !port.isUplink && port.type === 'ethernet')));
      void runScan(true, view.rtsp);
    } else if (scan) void runScan(false, view.rtsp);
  }, []);

  const askSignIn = (url: string, reason: string) => {
    setBase(url);
    setSignIn({ shown: true, asked: true });
    setStep(1);
    setRefusal({ text: reason, warn: false, field: null });
  };

  const verify = async (gen: number, url: string, camera: string, attempt: number, reason: string): Promise<void> => {
    if (generation.current !== gen) return;
    if (attempt >= VERIFY_ATTEMPTS) {
      setStatus({
        state: 'error',
        text: reason || 'No video yet — check the cable, IP and RTSP URL.',
      });
      return;
    }
    const answer = await deviceRead(`/api/setup/camera/status?source=${encodeURIComponent(url)}&name=${encodeURIComponent(camera)}`, parseCameraStatus);
    if (generation.current !== gen) return;
    if (answer?.state === 'live') {
      notice.show(view.isNew ? `Camera ${camera} added.` : 'Stream is live — receiving video from the camera.', null);
      onDone(camera);
      return;
    }
    if (answer?.state === 'needs_auth') {
      askSignIn(url, answer.reason);
      return;
    }
    setStatus({ state: 'loading', text: 'Verifying the stream is live…' });
    await delay(VERIFY_POLL_MS);
    return verify(gen, url, camera, attempt + 1, answer === null ? reason : answer.reason);
  };

  const apply = async (url: string, camera: string, port: string, portIp: string, how: string) => {
    const gen = generation.current + 1;
    generation.current = gen;
    setStep(3);
    setApplyField('');
    setStatus({ state: 'loading', text: 'Applying network and stream settings…' });
    const { json, refusal: refused } = await send('POST', '/api/setup/camera', {
      name: camera,
      new: view.isNew && !added,
      rtsp: url,
      interface: port,
      address: portIp.trim(),
      transport: how,
    });
    if (generation.current !== gen) return;
    if (refused !== null) {
      setStatus({
        state: 'error',
        text: refused === NO_ANSWER ? 'Network error — try again.' : errorOf(json, 'Setup failed.'),
      });
      setApplyField(refused.field);
      return;
    }
    setAdded(true);
    setStatus({ state: 'loading', text: 'Verifying the stream is live…' });
    await verify(gen, url, camera, 0, '');
  };

  const advance = (typed: string, typedName: string, known: readonly ScanPort[]) => {
    const out = nextStep({
      rtsp: typed,
      name: view.isNew ? typedName : null,
      others: view.others,
      ports: known,
      dupAck,
    });
    setRtsp(out.rtsp);
    if (out.name !== null) setName(out.name);
    if (out.kind === 'refuse') {
      setRefusal(out.refusal);
      setDupAck(out.dupAck);
      if (out.refusal.field === 'rtsp') {
        setEditRtsp(true);
        address.onToggle(true);
      }
      if (out.refusal.field === 'name') naming.onToggle(true);
      return;
    }
    setRefusal(null);
    const camera = out.name ?? view.name;
    if (out.kind === 'apply') {
      setIface('');
      setIp('');
      void apply(out.rtsp, camera, '', '', transport);
      return;
    }
    const suggested = portToSuggest(known);
    if (suggested !== '' && !ifaceTouched) setIface(suggested);
    if (ip === '') setIp(suggestIP(out.rtsp));
    setStep(2);
  };

  const restart = () => {
    generation.current += 1;
    setRtsp('');
    setIp('');
    setTransport(view.transports[0]?.value ?? view.transport);
    setRefusal(null);
    setAdded(false);
    setPicked(null);
    setBase('');
    setSignIn({ shown: false, asked: false });
    setEditRtsp(false);
    setStatus(APPLYING);
    setStep(1);
  };

  const credentials = (nextUser: string, nextPass: string) => {
    setUser(nextUser);
    setPass(nextPass);
    if (base !== '') setRtsp(camURL(base, nextUser, nextPass));
  };

  const setUpTemplate = async (t: CameraTemplate) => {
    if (template.busy !== '') return;
    const fail = (text: string) => {
      setTemplate({ busy: '', note: '' });
      setRefusal({ text, warn: false, field: null });
    };
    setRefusal(null);
    setTemplate({ busy: t.id, note: `Finding the Ethernet port for the ${t.label}…` });
    const known = await fetchPorts();
    const port = templatePort(known, t);
    if (port === null) return fail(`This device has no Ethernet port to plug the ${t.label} into.`);
    if (!portHasSubnet(port, t.cidr)) {
      const holder = known.find((other) => other.iface !== port.iface && portHasSubnet(other, t.cidr));
      if (holder !== undefined) return fail(templateHolderError(holder, t));
      setTemplate({
        busy: t.id,
        note: `Putting ${port.iface} on the ${t.label}'s network (${t.cidr})…`,
      });
      const refused = await setPortAddress(port.iface, t.cidr);
      if (refused !== '') return fail(refused);
      await waitForPort(port.iface, t.cidr);
    }
    setTemplate({ busy: t.id, note: `Looking for the ${t.label} at ${t.host}…` });
    const result = await runScan(false, rtsp);
    const found = result?.cams.find((cam) => cam.host === t.host && cam.pathFound);
    if (result === null || found === undefined) return fail(`No ${t.label} answered at ${t.host} — check its Ethernet cable and that it has power, then try again.`);
    const url = pick(found);
    const camera = view.others.some((other) => other.name.toLowerCase() === t.name) ? name : t.name;
    setName(camera);
    if (t.gimbal !== '') {
      void send('PUT', '/api/gimbal/config', { vendor: t.gimbal, cameraAddr: t.gimbalAddr }).then(({ refusal: refused }) =>
        refused === null ? undefined : notice.show(`Gimbal control was not switched to this camera — ${refused.message}`, null),
      );
    }
    setTemplate({ busy: '', note: '' });
    advance(url, camera, result.ports);
  };

  const fixer = (port: ScanPort): PortFixer => fixers[port.iface] ?? NO_FIXER;
  const changeFixer = (port: ScanPort, change: Partial<PortFixer>) => setFixers((all) => ({ ...all, [port.iface]: { ...(all[port.iface] ?? NO_FIXER), ...change } }));

  const fixPort = async (port: ScanPort) => {
    const want = fixer(port).input.trim();
    changeFixer(port, { busy: true, status: `Setting up ${port.iface}…` });
    const refused = await setPortAddress(port.iface, want);
    if (refused !== '') {
      changeFixer(port, { busy: false, status: refused });
      return;
    }
    changeFixer(port, { status: `Waiting for ${port.iface} to come up on ${want}…` });
    await waitForPort(port.iface, want);
    await runScan(false, rtsp);
  };

  const upload = async () => {
    if (cameraName === '') return notice.show('Name the camera first.', null);
    if (file === null) return notice.show('Choose a video file.', null);
    setUploading(true);
    const form = new FormData();
    form.append('file', file);
    const response = await fetch(deviceUrl(`/api/streams/${encodeURIComponent(cameraName)}/file`), {
      method: 'PUT',
      headers: { Accept: 'application/json' },
      body: form,
    }).catch(() => null);
    setUploading(false);
    if (response === null) return notice.show('Upload failed — check the file size and try again.', null);
    const json: unknown = await response.json().catch(() => null);
    if (!response.ok) return notice.show(errorOf(json, 'Upload failed.'), null);
    notice.show('Video source added.', null);
    onDone(cameraName);
  };

  const commits = commitsNow(rtsp, ports);
  const fresh = cams.filter((cam) => cam.used === '');
  const checklist = scanNote !== '' && cams.length === 0 && ports.length > 0;
  const Scan = scanLeads(rtsp, picked !== null) ? Button : OutlinedButton;
  const Next = continueLeads(rtsp) ? Button : OutlinedButton;
  const current = view.interfaces.find((option) => option.value === iface)?.address ?? '';

  return (
    <div className="flex flex-col gap-3 px-list py-2" role="group" aria-label={view.isNew ? 'New camera' : `Set up ${view.name}`}>
      <StepNames step={step} skipsPort={step === 1 && commits} />
      {step === 1 ? (
        <>
          {view.piCamera !== null ? (
            <div className="flex flex-wrap items-center gap-3 rounded-shape-md bg-surface-container-high px-4 py-3">
              <Text typography="bodyMedium" className="min-w-0 flex-1">
                {view.piCamera.note}
              </Text>
              <Button onClick={() => advance('rpiCamera', name, ports)}>{view.piCamera.action}</Button>
            </div>
          ) : null}
          {view.templates.length > 0 && !noPort ? (
            <div className="flex flex-col gap-2" role="group" aria-label="Camera makes">
              {view.templates.map((t) => {
                const ready = templateReadyPort(t, cams, ports);
                return (
                  <button
                    key={t.id}
                    type="button"
                    disabled={template.busy !== ''}
                    onClick={() => void setUpTemplate(t)}
                    className="flex w-full items-center gap-3 rounded-shape-md border border-outline-variant px-4 py-3 text-left hover:bg-[color-mix(in_srgb,var(--md-on-surface)_8%,transparent)] disabled:opacity-60"
                  >
                    <span className="flex min-w-0 flex-1 flex-col">
                      <Text typography="bodyLarge">{t.label}</Text>
                      <Text typography="bodySmall" color={md('onSurfaceVariant')}>
                        {templateSub(t, ready)}
                      </Text>
                    </span>
                    <Text typography="labelLarge" color={md('primary')}>
                      {templateAction(template.busy === t.id, ready)}
                    </Text>
                  </button>
                );
              })}
              {template.note !== '' ? <Hint text={template.note} /> : null}
            </div>
          ) : null}
          {view.lookingFor !== '' ? (
            <Text typography="titleSmall" color={aircast('warning')}>
              {view.lookingFor}
            </Text>
          ) : null}
          {!checklist ? (
            <div className="flex flex-col items-start gap-2">
              <Hint text="Plug the camera in and look for it." />
              <Scan enabled={!scanning} onClick={() => void runScan(false, rtsp)}>
                {scanning ? 'Looking…' : 'Scan for cameras'}
              </Scan>
            </div>
          ) : null}
          {scanNote !== '' && !(ports.length > 0 && cams.length === 0) ? <Hint text={scanNote} /> : null}
          {checklist ? (
            <div className="flex flex-col gap-2 rounded-shape-md border border-outline-variant px-4 py-3" role="group" aria-label="Checklist">
              <Text typography="titleSmall">{checklistHint(ports, view.subnets) || NO_ANSWER_CHECKLIST}</Text>
              <ul className="flex list-none flex-col gap-1 text-body-medium text-on-surface-variant">
                <li>☐ The cable goes from the camera to this device's camera port</li>
                <li>☐ The camera has power — its light is on</li>
                <li>☐ Give it 30 seconds after power-on</li>
              </ul>
              <div>
                <Button enabled={!scanning} onClick={() => void runScan(false, rtsp)}>
                  {scanning ? 'Looking…' : 'Scan again'}
                </Button>
              </div>
            </div>
          ) : null}
          {cams.length > 0 ? (
            <div className="flex flex-col gap-2">
              <Text typography="labelLarge" color={md('onSurfaceVariant')}>
                {camHeading(cams)}
              </Text>
              {cams.map((cam) => (
                <button
                  key={`${cam.host} ${cam.url}`}
                  type="button"
                  aria-pressed={picked === cam}
                  aria-label={`${cam.host} — ${rowNote(cam)}`}
                  onClick={() => pick(cam)}
                  className={`flex w-full items-center gap-3 rounded-shape-md border px-4 py-3 text-left ${picked === cam ? 'border-primary bg-[color-mix(in_srgb,var(--md-primary)_8%,transparent)]' : 'border-outline-variant hover:bg-[color-mix(in_srgb,var(--md-on-surface)_8%,transparent)]'}`}
                >
                  <span className="flex min-w-0 flex-1 flex-col">
                    <Text typography="bodyLarge" monospace>
                      {cam.host}
                    </Text>
                    <Text typography="bodySmall" color={md('onSurfaceVariant')}>
                      {rowNote(cam)}
                    </Text>
                  </span>
                  <Text typography="labelLarge" color={md('primary')}>
                    {picked === cam ? 'Selected' : cam.used !== '' ? 'Add again' : 'Use'}
                  </Text>
                </button>
              ))}
              {fresh.length === 0 ? <Hint text="Every camera that answered is already set up on this device — a newly plugged-in one has not answered yet." /> : null}
            </div>
          ) : null}
          {ports.length > 0 && outcome !== null ? (
            <Fold title={cams.length > 0 ? 'Scan details' : 'Details for a technician'} {...tech}>
              <ScanDetails outcome={outcome} view={view} fixer={fixer} onFixer={changeFixer} onFix={(port) => void fixPort(port)} />
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
            <Next onClick={() => advance(rtsp, name, ports)}>{nextLabel(rtsp, ports, view.isNew)}</Next>
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
            <Button onClick={() => void apply(rtsp, cameraName, iface, ip, transport)}>Set up & verify</Button>
          </div>
          {applyNote(iface, ip, current) !== '' ? <Hint text={applyNote(iface, ip, current)} /> : null}
        </>
      ) : null}
      {step === 3 ? (
        <>
          <div role="status" aria-live="polite" className="flex items-center gap-3">
            {status.state === 'loading' ? <CircularProgressIndicator size={20} strokeWidth={3} label="Working" /> : <MaterialIcon name="error" size={20} style={{ color: md('error') }} />}
            <Text typography="bodyLarge" color={status.state === 'error' ? md('error') : md('onSurface')}>
              {status.text}
            </Text>
          </div>
          <div className="flex flex-wrap items-center justify-between gap-2">
            <TextButton onClick={restart}>Start over</TextButton>
            {status.state === 'error' ? (
              <span className="flex flex-wrap gap-2">
                <OutlinedButton onClick={() => setStep(2)}>Change settings</OutlinedButton>
                <Button onClick={() => void apply(rtsp, cameraName, iface, ip, transport)}>Try again</Button>
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
  outcome,
  view,
  fixer,
  onFixer,
  onFix,
}: {
  outcome: ScanOutcome;
  view: CameraWizardView;
  fixer: (port: ScanPort) => PortFixer;
  onFixer: (port: ScanPort, change: Partial<PortFixer>) => void;
  onFix: (port: ScanPort) => void;
}) => {
  const restFold = useFold(false);
  const checkedFold = useFold(false);
  const host = rtspHost(view.source);
  const rest = outcome.rest.map((it) => <PortRow key={it.port.iface} label={it.port.label} detail={portDetail(it, view.subnets, host)} />);
  const coverage = <Hint text={view.coverage} />;
  return (
    <>
      {outcome.todo.length > 0 ? (
        <div className="flex flex-col gap-2">
          <Text typography="labelLarge" color={md('onSurfaceVariant')}>
            {todoHeading(outcome.todo, view.subnets)}
          </Text>
          {outcome.todo.map((it) => (
            <PortFix
              key={it.port.iface}
              port={it.port}
              detail={portDetail(it, view.subnets, host)}
              ports={outcome.ports}
              view={view}
              fixer={fixer(it.port)}
              onFixer={(change) => onFixer(it.port, change)}
              onFix={() => onFix(it.port)}
            />
          ))}
        </div>
      ) : null}
      {outcome.rest.length > 0 && outcome.todo.length > 0 ? (
        <Fold title={`${outcome.rest.map((it) => it.port.label).join(', ')} — nothing answered`} {...restFold}>
          {rest}
        </Fold>
      ) : null}
      {outcome.rest.length > 0 && outcome.todo.length === 0 && outcome.cams.length > 0 ? (
        <>
          {rest}
          {coverage}
        </>
      ) : null}
      {outcome.rest.length > 0 && outcome.todo.length === 0 && outcome.cams.length === 0 ? (
        <>
          <Text typography="labelLarge" color={md('onSurfaceVariant')}>
            Ports checked
          </Text>
          {rest}
        </>
      ) : null}
      {outcome.cams.length === 0 ? (
        <Fold title="What was checked?" {...checkedFold}>
          {coverage}
        </Fold>
      ) : null}
    </>
  );
};

const PortFix = ({
  port,
  detail,
  ports,
  view,
  fixer,
  onFixer,
  onFix,
}: {
  port: ScanPort;
  detail: string;
  ports: readonly ScanPort[];
  view: CameraWizardView;
  fixer: PortFixer;
  onFixer: (change: Partial<PortFixer>) => void;
  onFix: () => void;
}) => {
  const Go = fixer.suggest !== '' ? Button : OutlinedButton;
  return (
    <div className="flex flex-col gap-2 rounded-shape-md border border-primary px-4 py-3" role="group" aria-label={port.label}>
      <Text typography="bodyLarge">{port.label}</Text>
      <Text typography="bodySmall" color={md('onSurfaceVariant')}>
        {detail}
      </Text>
      {fixer.open ? (
        <>
          <OutlinedTextField
            label="This port's address"
            value={fixer.input}
            placeholder="192.168.144.2/24"
            keyboardType="uri"
            autoCorrect={false}
            className="max-w-xs"
            onValueChange={(input) => onFixer({ input })}
          />
          <div className="flex flex-wrap gap-2">
            {subnetOptions(port, ports, view.subnets).map((option) => (
              <AssistChip key={option.cidr} onClick={() => onFixer({ input: option.cidr })} label={option.label !== '' ? `${option.cidr} · ${option.label}` : option.cidr} />
            ))}
          </div>
          <Hint text={fixerHint(fixer)} />
          <div>
            <Go enabled={!fixer.busy && fixer.input.trim() !== ''} onClick={onFix}>
              {fixerGoLabel(port, fixer)}
            </Go>
          </div>
          {fixer.status !== '' ? <Hint text={fixer.status} /> : null}
        </>
      ) : (
        <div>
          <OutlinedButton onClick={() => onFixer({ open: true })}>Change this port's address</OutlinedButton>
        </div>
      )}
    </div>
  );
};
