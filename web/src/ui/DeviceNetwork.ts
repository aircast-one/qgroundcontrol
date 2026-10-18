import { type Json, isJson, jsonObjects, jsonStrings, optBoolean, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { type DeviceBadge, type DeviceChartView, type DeviceOption, parseBadge, parseBadges, parseChart, parseOptions } from '@/ui/DeviceKit';

export type DeviceFact = { readonly label: string; readonly value: string };
export type DeviceProblem = { readonly text: string; readonly detail: string };
export type ConfirmedAction = { readonly label: string; readonly confirm: string };
export type CardHead = { readonly status: DeviceBadge; readonly summary: string };

export type UplinkCard = {
  readonly icon: string;
  readonly title: string;
  readonly status: DeviceBadge;
  readonly verdict: DeviceBadge;
  readonly detail: string;
  readonly addr: string;
  readonly meta: string;
  readonly warning: string;
  readonly rate: string;
  readonly failover: readonly { readonly label: string; readonly status: DeviceBadge }[];
  readonly switchText: string;
  readonly backup: string;
};

export type SmsMessage = { readonly from: string; readonly text: string; readonly when: string };

export type SimSettings = { readonly summary: string; readonly apn: string; readonly placeholder: string; readonly metered: string; readonly options: readonly DeviceOption[] };

export type ModemCard = CardHead & {
  readonly notices: readonly DeviceBadge[];
  readonly operator: string;
  readonly detail: string;
  readonly signal: DeviceChartView | null;
  readonly facts: readonly DeviceFact[];
  readonly messages: { readonly summary: string; readonly items: readonly SmsMessage[] };
  readonly sim: SimSettings | null;
};

export type StarlinkDemo = { readonly on: boolean; readonly note: string; readonly condition: string; readonly conditions: readonly DeviceOption[]; readonly first: boolean };

export type StarlinkCard = CardHead & {
  readonly demo: StarlinkDemo | null;
  readonly notices: readonly DeviceBadge[];
  readonly pollError: string;
  readonly facts: readonly DeviceFact[];
  readonly firmware: string;
  readonly address: { readonly value: string; readonly fallback: string; readonly defaultFirst: boolean; readonly open: boolean };
};

export type WifiCard = CardHead & { readonly joined: DeviceBadge | null; readonly reopen: { readonly host: string; readonly hostname: string } };

export type IPv4Settings = { readonly method: string; readonly metric: string; readonly metricPlaceholder: string; readonly address: string; readonly gateway: string; readonly dns: string };

export type LinkEditor = { readonly iface: string; readonly ipv4: IPv4Settings; readonly confirm: string; readonly uuid: string; readonly deactivateConfirm: string; readonly deleteConfirm: string };

export type NetLink = { readonly name: string; readonly kind: string; readonly role: DeviceBadge | null; readonly where: string; readonly local: boolean; readonly usage: string; readonly editor: LinkEditor | null };

export type SavedProfile = { readonly uuid: string; readonly name: string; readonly kind: string; readonly ipv4: IPv4Settings; readonly activateConfirm: string; readonly deleteConfirm: string };

export type InterfacesCard = CardHead & {
  readonly problem: DeviceProblem | null;
  readonly links: readonly NetLink[];
  readonly saved: readonly SavedProfile[];
  readonly other: readonly { readonly label: string; readonly month: string; readonly total: string }[];
  readonly methods: readonly DeviceOption[];
  readonly add: { readonly types: readonly DeviceOption[]; readonly ports: readonly DeviceOption[]; readonly autoconnect: readonly DeviceOption[] };
};

export type BudgetMeter = { readonly label: string; readonly used: string; readonly percent: number; readonly level: number; readonly remaining: string; readonly note: string };

export type DataGauge = {
  readonly badges: readonly DeviceBadge[];
  readonly hero: string;
  readonly heroCaption: string;
  readonly uplinkPercent: number;
  readonly uplink: string;
  readonly downlink: string;
  readonly meters: readonly BudgetMeter[];
  readonly stats: readonly { readonly label: string; readonly value: string; readonly note: string }[];
};

export type BudgetSettings = { readonly summary: string; readonly monthlyCapGB: string; readonly flightCapMB: string; readonly warnPercent: string };

export type DataCard = CardHead & { readonly gauge: DataGauge | null; readonly budget: BudgetSettings };

export type TailscaleControl = {
  readonly title: string;
  readonly open: boolean;
  readonly server: string;
  readonly authKeyPlaceholder: string;
  readonly authKeySet: boolean;
  readonly switchError: string;
  readonly pending: string;
  readonly switchTo: ConfirmedAction | null;
  readonly logOutToSwitch: ConfirmedAction | null;
  readonly hint: string;
};

export type TailscaleCard = CardHead & {
  readonly problem: DeviceProblem | null;
  readonly banner: { readonly title: string; readonly subtitle: string; readonly status: DeviceBadge } | null;
  readonly facts: readonly DeviceFact[];
  readonly addresses: readonly string[];
  readonly signIn: { readonly label: string; readonly href: string } | null;
  readonly connect: string;
  readonly cancel: boolean;
  readonly logOut: string;
  readonly hint: string;
  readonly control: TailscaleControl | null;
};

export type IceServer = { readonly url: string; readonly username: string; readonly password: string };

export type RelayCard = CardHead & { readonly on: boolean; readonly configured: boolean; readonly reason: string; readonly notice: DeviceBadge; readonly servers: readonly IceServer[] };

export type NetworkView = {
  readonly uplink: UplinkCard;
  readonly modem: ModemCard | null;
  readonly starlink: StarlinkCard;
  readonly wifi: WifiCard;
  readonly interfaces: InterfacesCard;
  readonly data: DataCard;
  readonly tailscale: TailscaleCard;
  readonly relay: RelayCard;
};

const objects = (json: Json | null, key: string): Json[] => jsonObjects(optJSONArray(json, key));

const optBadge = (json: Json | null, key: string): DeviceBadge | null => {
  const it = optJSONObject(json, key);
  return it === null ? null : parseBadge(it);
};

const parseHead = (json: Json | null): CardHead => ({ status: parseBadge(json?.status), summary: optText(json, 'summary') });

const parseFacts = (json: Json | null): readonly DeviceFact[] => objects(json, 'facts').map((it) => ({ label: optText(it, 'label'), value: optText(it, 'value') }));

const parseProblem = (json: Json | null): DeviceProblem | null => (json === null ? null : { text: optText(json, 'text'), detail: optText(json, 'detail') });

const parseConfirmed = (json: Json | null): ConfirmedAction | null => (json === null ? null : { label: optText(json, 'label'), confirm: optText(json, 'confirm') });

const parseIPv4 = (json: Json | null): IPv4Settings => ({
  method: optText(json, 'method'),
  metric: optText(json, 'metric'),
  metricPlaceholder: optText(json, 'metricPlaceholder'),
  address: optText(json, 'address'),
  gateway: optText(json, 'gateway'),
  dns: optText(json, 'dns'),
});

const parseUplink = (json: Json | null): UplinkCard => ({
  icon: optText(json, 'icon'),
  title: optText(json, 'title'),
  status: parseBadge(json?.status),
  verdict: parseBadge(json?.verdict),
  detail: optText(json, 'detail'),
  addr: optText(json, 'addr'),
  meta: optText(json, 'meta'),
  warning: optText(json, 'warning'),
  rate: optText(json, 'rate'),
  failover: objects(json, 'failover').map((it) => ({ label: optText(it, 'label'), status: parseBadge(it.status) })),
  switchText: optText(json, 'switch'),
  backup: optText(json, 'backup'),
});

const parseSim = (json: Json | null): SimSettings | null =>
  json === null ? null : { summary: optText(json, 'summary'), apn: optText(json, 'apn'), placeholder: optText(json, 'placeholder'), metered: optText(json, 'metered'), options: parseOptions(json, 'options') };

const parseModem = (json: Json | null): ModemCard | null => {
  const messages = optJSONObject(json, 'messages');
  return json === null
    ? null
    : {
        ...parseHead(json),
        notices: parseBadges(json, 'notices'),
        operator: optText(json, 'operator'),
        detail: optText(json, 'detail'),
        signal: parseChart(json.signal),
        facts: parseFacts(json),
        messages: { summary: optText(messages, 'summary'), items: objects(messages, 'items').map((it) => ({ from: optText(it, 'from'), text: optText(it, 'text'), when: optText(it, 'when') })) },
        sim: parseSim(optJSONObject(json, 'sim')),
      };
};

const parseDemo = (json: Json | null): StarlinkDemo | null =>
  json === null ? null : { on: optBoolean(json, 'on'), note: optText(json, 'note'), condition: optText(json, 'condition'), conditions: parseOptions(json, 'conditions'), first: optBoolean(json, 'first') };

const parseStarlink = (json: Json | null): StarlinkCard => {
  const address = optJSONObject(json, 'address');
  return {
    ...parseHead(json),
    demo: parseDemo(optJSONObject(json, 'demo')),
    notices: parseBadges(json, 'notices'),
    pollError: optText(json, 'pollError'),
    facts: parseFacts(json),
    firmware: optText(json, 'firmware'),
    address: { value: optText(address, 'value'), fallback: optText(address, 'default'), defaultFirst: optBoolean(address, 'defaultFirst'), open: optBoolean(address, 'open') },
  };
};

const parseWifi = (json: Json | null): WifiCard => {
  const reopen = optJSONObject(json, 'reopen');
  return { ...parseHead(json), joined: optBadge(json, 'joined'), reopen: { host: optText(reopen, 'host'), hostname: optText(reopen, 'hostname') } };
};

const parseEditor = (json: Json | null): LinkEditor | null =>
  json === null
    ? null
    : {
        iface: optText(json, 'interface'),
        ipv4: parseIPv4(optJSONObject(json, 'ipv4')),
        confirm: optText(json, 'confirm'),
        uuid: optText(json, 'uuid'),
        deactivateConfirm: optText(json, 'deactivateConfirm'),
        deleteConfirm: optText(json, 'deleteConfirm'),
      };

const parseInterfaces = (json: Json | null): InterfacesCard => {
  const add = optJSONObject(json, 'add');
  return {
    ...parseHead(json),
    problem: parseProblem(optJSONObject(json, 'problem')),
    links: objects(json, 'links').map((it) => ({
      name: optText(it, 'name'),
      kind: optText(it, 'kind'),
      role: optBadge(it, 'role'),
      where: optText(it, 'where'),
      local: optBoolean(it, 'local'),
      usage: optText(it, 'usage'),
      editor: parseEditor(optJSONObject(it, 'editor')),
    })),
    saved: objects(json, 'saved').map((it) => ({
      uuid: optText(it, 'uuid'),
      name: optText(it, 'name'),
      kind: optText(it, 'kind'),
      ipv4: parseIPv4(optJSONObject(it, 'ipv4')),
      activateConfirm: optText(it, 'activateConfirm'),
      deleteConfirm: optText(it, 'deleteConfirm'),
    })),
    other: objects(json, 'other').map((it) => ({ label: optText(it, 'label'), month: optText(it, 'month'), total: optText(it, 'total') })),
    methods: parseOptions(json, 'methods'),
    add: { types: parseOptions(add, 'types'), ports: parseOptions(add, 'ports'), autoconnect: parseOptions(add, 'autoconnect') },
  };
};

const parseGauge = (json: Json | null): DataGauge | null =>
  json === null
    ? null
    : {
        badges: parseBadges(json, 'badges'),
        hero: optText(json, 'hero'),
        heroCaption: optText(json, 'heroCaption'),
        uplinkPercent: optInt(json, 'uplinkPercent'),
        uplink: optText(json, 'uplink'),
        downlink: optText(json, 'downlink'),
        meters: objects(json, 'meters').map((it) => ({
          label: optText(it, 'label'),
          used: optText(it, 'used'),
          percent: optInt(it, 'percent'),
          level: optInt(it, 'level'),
          remaining: optText(it, 'remaining'),
          note: optText(it, 'note'),
        })),
        stats: objects(json, 'stats').map((it) => ({ label: optText(it, 'label'), value: optText(it, 'value'), note: optText(it, 'note') })),
      };

const parseData = (json: Json | null): DataCard => {
  const budget = optJSONObject(json, 'budget');
  return {
    ...parseHead(json),
    gauge: parseGauge(optJSONObject(json, 'gauge')),
    budget: { summary: optText(budget, 'summary'), monthlyCapGB: optText(budget, 'monthlyCapGB'), flightCapMB: optText(budget, 'flightCapMB'), warnPercent: optText(budget, 'warnPercent') },
  };
};

const parseControl = (json: Json | null): TailscaleControl | null =>
  json === null
    ? null
    : {
        title: optText(json, 'title'),
        open: optBoolean(json, 'open'),
        server: optText(json, 'server'),
        authKeyPlaceholder: optText(json, 'authKeyPlaceholder'),
        authKeySet: optBoolean(json, 'authKeySet'),
        switchError: optText(json, 'switchError'),
        pending: optText(json, 'pending'),
        switchTo: parseConfirmed(optJSONObject(json, 'switch')),
        logOutToSwitch: parseConfirmed(optJSONObject(json, 'logOutToSwitch')),
        hint: optText(json, 'hint'),
      };

const parseTailscale = (json: Json | null): TailscaleCard => {
  const banner = optJSONObject(json, 'banner');
  const signIn = optJSONObject(json, 'signIn');
  return {
    ...parseHead(json),
    problem: parseProblem(optJSONObject(json, 'problem')),
    banner: banner === null ? null : { title: optText(banner, 'title'), subtitle: optText(banner, 'subtitle'), status: parseBadge(banner.status) },
    facts: parseFacts(json),
    addresses: jsonStrings(optJSONArray(json, 'addresses')),
    signIn: signIn === null ? null : { label: optText(signIn, 'label'), href: optText(signIn, 'href') },
    connect: optText(json, 'connect'),
    cancel: optBoolean(json, 'cancel'),
    logOut: optText(json, 'logOut'),
    hint: optText(json, 'hint'),
    control: parseControl(optJSONObject(json, 'control')),
  };
};

const parseRelay = (json: Json | null): RelayCard => ({
  ...parseHead(json),
  on: optBoolean(json, 'on'),
  configured: optBoolean(json, 'configured'),
  reason: optText(json, 'reason'),
  notice: parseBadge(json?.notice),
  servers: objects(json, 'servers').map((it) => ({ url: optText(it, 'url'), username: optText(it, 'username'), password: optText(it, 'password') })),
});

export const parseNetwork = (json: unknown): NetworkView | null =>
  !isJson(json) || !isJson(json.uplink)
    ? null
    : {
        uplink: parseUplink(json.uplink),
        modem: parseModem(optJSONObject(json, 'modem')),
        starlink: parseStarlink(optJSONObject(json, 'starlink')),
        wifi: parseWifi(optJSONObject(json, 'wifi')),
        interfaces: parseInterfaces(optJSONObject(json, 'interfaces')),
        data: parseData(optJSONObject(json, 'data')),
        tailscale: parseTailscale(optJSONObject(json, 'tailscale')),
        relay: parseRelay(optJSONObject(json, 'relay')),
      };

const UPLINK_ICONS: ReadonlyMap<string, string> = new Map([
  ['satellite-dish', 'satellite_alt'],
  ['signal', 'signal_cellular_alt'],
  ['wifi', 'wifi'],
  ['ethernet-port', 'settings_ethernet'],
]);

export const uplinkIcon = (icon: string): string => UPLINK_ICONS.get(icon) ?? 'lan';

export const GOOGLE_STUN = 'stun:stun.l.google.com:19302';

export const emptyIceServer: IceServer = { url: '', username: '', password: '' };

export const iceRows = (servers: readonly IceServer[]): readonly IceServer[] => (servers.length > 0 ? servers : [emptyIceServer]);

export const serializeIceServers = (rows: readonly IceServer[]): string =>
  rows
    .map((row) => ({ url: row.url.trim(), username: row.username.trim(), password: row.password.trim() }))
    .filter((row) => row.url !== '')
    .map((row) => (row.username !== '' || row.password !== '' ? `${row.url} | ${row.username} | ${row.password}` : row.url))
    .join('\n');

export const manualAddressing = (method: string): boolean => method === 'manual';

export const connectionFieldShown = (type: string, field: string): boolean => {
  const only = ({ ssid: 'wifi', psk: 'wifi', apn: 'gsm', pin: 'gsm' } as Readonly<Record<string, string>>)[field];
  return only === undefined || only === type;
};
