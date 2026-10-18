import { type Json, isJson, jsonObjects, jsonStrings, optBoolean, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { type DeviceBadge, parseBadge } from '@/ui/DeviceKit';

export type PageLink = { readonly label: string; readonly href: string };

export type GcsChoice = { readonly value: string; readonly label: string; readonly hides: readonly string[] };

export type ConnectAddress = { readonly label: string; readonly url: string };

export type ConnectTarget = {
  readonly proto: string;
  readonly label: string;
  readonly url: string;
  readonly hint: string;
  readonly tag: string;
  readonly down: boolean;
  readonly reach: string;
  readonly qr: boolean;
  readonly alternatives: readonly ConnectAddress[];
};

export type ConnectCamera = { readonly name: string; readonly status: DeviceBadge; readonly notice: string; readonly fix: PageLink | null; readonly targets: readonly ConnectTarget[] };

export type ConnectGroup = { readonly label: string; readonly targets: readonly ConnectTarget[] };

export type ConnectTrouble = { readonly fallback: readonly ConnectTarget[]; readonly addresses: { readonly title: string; readonly groups: readonly ConnectGroup[] } | null };

export type ConnectView = {
  readonly choices: readonly GcsChoice[];
  readonly scope: string;
  readonly cameras: readonly ConnectCamera[];
  readonly noCamera: PageLink | null;
  readonly telemetry: { readonly status: DeviceBadge; readonly targets: readonly ConnectTarget[] } | null;
  readonly trouble: ConnectTrouble | null;
};

const parseLink = (json: Json | null): PageLink | null => (json === null ? null : { label: optText(json, 'label'), href: optText(json, 'href') });

const parseTargets = (json: Json | null, key: string): readonly ConnectTarget[] =>
  jsonObjects(optJSONArray(json, key)).map((it) => ({
    proto: optText(it, 'proto'),
    label: optText(it, 'label'),
    url: optText(it, 'url'),
    hint: optText(it, 'hint'),
    tag: optText(it, 'tag'),
    down: optBoolean(it, 'down'),
    reach: optText(it, 'reach'),
    qr: optBoolean(it, 'qr'),
    alternatives: jsonObjects(optJSONArray(it, 'alternatives')).map((alt) => ({ label: optText(alt, 'label'), url: optText(alt, 'url') })),
  }));

const parseTrouble = (json: Json | null): ConnectTrouble | null => {
  const addresses = optJSONObject(json, 'addresses');
  return json === null
    ? null
    : {
        fallback: parseTargets(json, 'fallback'),
        addresses:
          addresses === null
            ? null
            : { title: optText(addresses, 'title'), groups: jsonObjects(optJSONArray(addresses, 'groups')).map((group) => ({ label: optText(group, 'label'), targets: parseTargets(group, 'targets') })) },
      };
};

export const parseConnect = (json: unknown): ConnectView | null => {
  if (!isJson(json) || !Array.isArray(json.choices)) return null;
  const telemetry = optJSONObject(json, 'telemetry');
  return {
    choices: jsonObjects(optJSONArray(json, 'choices')).map((it) => ({ value: optText(it, 'value'), label: optText(it, 'label'), hides: jsonStrings(optJSONArray(it, 'hides')) })),
    scope: optText(json, 'scope'),
    cameras: jsonObjects(optJSONArray(json, 'cameras')).map((it) => ({
      name: optText(it, 'name'),
      status: parseBadge(it.status),
      notice: optText(it, 'notice'),
      fix: parseLink(optJSONObject(it, 'fix')),
      targets: parseTargets(it, 'targets'),
    })),
    noCamera: parseLink(optJSONObject(json, 'noCamera')),
    telemetry: telemetry === null ? null : { status: parseBadge(telemetry.status), targets: parseTargets(telemetry, 'targets') },
    trouble: parseTrouble(optJSONObject(json, 'trouble')),
  };
};

export const GCS_KEY = 'gcs';

export const chosenGcs = (choices: readonly GcsChoice[], saved: string | null): GcsChoice | null => choices.find((choice) => choice.value === saved) ?? choices[0] ?? null;

export const shownTargets = (targets: readonly ConnectTarget[], choice: GcsChoice | null): readonly ConnectTarget[] => targets.filter((target) => !(choice?.hides.includes(target.proto) ?? false));

export type Reach = 'ok' | 'bad';

export const REACH_TIMEOUT_MS = 5000;

export const REACH_RECHECK_MS = 30_000;

export const probeBlocked = (url: string, pageProtocol: string): boolean => url === '' || (pageProtocol === 'https:' && url.startsWith('http://'));

export const probeReach = async (url: string): Promise<Reach> => {
  const abort = new AbortController();
  const timer = setTimeout(() => abort.abort(), REACH_TIMEOUT_MS);
  const answered = await fetch(url, { method: 'HEAD', mode: 'no-cors', cache: 'no-store', signal: abort.signal }).then(
    () => true,
    () => false,
  );
  clearTimeout(timer);
  return answered ? 'ok' : 'bad';
};

export const readStored = (key: string): string | null => {
  try {
    return window.localStorage.getItem(key);
  } catch {
    return null;
  }
};

export const writeStored = (key: string, value: string): void => {
  try {
    window.localStorage.setItem(key, value);
  } catch {
    return;
  }
};
