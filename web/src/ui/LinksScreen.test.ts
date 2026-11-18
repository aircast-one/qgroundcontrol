import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import {
  AddOutcome,
  dialFailure,
  dialState,
  parsedAddress,
  rememberedByDefault,
  withParsedAddress,
  CREATABLE_LINK_TYPES,
  DEFAULT_BAUD,
  LinkDraft,
  LinkEditing,
  LinkRow,
  LinkType,
  LinkTypeEntries,
  PILOT_LINK_ORDER,
  RC_TRANSMITTER,
  REMEDY_EDIT_ADDRESS,
  REPLAY_LINK_NAME,
  SerialFraming,
  TOOL_LINK_ORDER,
  addFailure,
  addOutcome,
  addableLinkTypes,
  autoLinkName,
  autoLinkStatus,
  autoLinkSubtitle,
  autoLinks,
  autoSerialName,
  bluetoothState,
  draftError,
  editError,
  editShowsAdvanced,
  editWrites,
  editedPort,
  framingWrites,
  linkEdit,
  linkEditingFrom,
  linkFlagWrites,
  linkFormError,
  linkFormErrorField,
  linkIcon,
  linkIsEditable,
  linkSections,
  LINK_WAITING,
  linkKind,
  linkRows,
  linkTypeFrom,
  linkTypeLabel,
  pilotLinkKinds,
  portFor,
  primaryLinkAction,
  remedyText,
  rowConnected,
  serialFormError,
  serialPortChoices,
  serverFormError,
  SERVER_FORMAT,
  suggestedLinkName,
  toolLinkKinds,
  udpServer,
  uniqueLinkName,
  withPluggedPort,
  withServer,
  withTypedServer,
  webSocketUrlError,
  SIMULATOR_WEBSOCKET_URL,
  webSocketAddress,
  webSocketLinkName,
} from './LinksScreen';
import { sheetDrilled } from './SettingsSheet';

const json = (text: string): Json => parseJson(text) ?? {};

describe('LinksScreenTest', () => {
  const view = (...configured: string[]) => json(`{"configured":[${configured.join(',')}]}`);

  const heardTcp = `{"index":1,"name":"Pi","statusLine":"Connected · TCP 10.0.0.4:5760",
            "connected":true,"heardVehicle":true,"lastError":""}`;
  const waitingUdp = `{"index":2,"name":"Bench","statusLine":"Waiting for the vehicle · UDP port 14551",
            "connected":true,"heardVehicle":false,"lastError":""}`;
  const idle = `{"index":3,"name":"Old","statusLine":"Not connected","connected":false,
            "heardVehicle":false,"lastError":"Connection refused"}`;

  it("a link's connected state is read by its core index, not its place in the list", () => {
    const rows = linkRows(view(waitingUdp, idle));
    expect(rowConnected(rows, 2)).toBe(true);
    expect(rowConnected(rows, 3)).toBe(false);
    expect(rowConnected(rows, 0)).toBeNull();
  });

  it("a udp link's server addresses and a serial link's framing are read from the row", () => {
    const udp = '{"index":4,"name":"U","statusLine":"","connected":false,"heardVehicle":false,"lastError":"","hostList":["10.0.0.2:14550"]}';
    const serial = '{"index":5,"name":"S","statusLine":"","connected":false,"heardVehicle":false,"lastError":"","dataBits":7,"stopBits":2,"parity":3,"flowControl":1}';
    const rows = linkRows(view(udp, serial));
    expect(rows[0]!.servers).toEqual(['10.0.0.2:14550']);
    expect(rows[1]!.framing).toEqual(SerialFraming({ dataBits: 7, stopBits: 2, parity: 3, flowControl: 1 }));
    expect(linkRows(view(heardTcp))[0]!.framing).toEqual(SerialFraming());
  });

  it('the sentence comes from the core, not from the head', () => {
    const rows = linkRows(view(heardTcp, waitingUdp, idle));
    expect(rows.map((it) => it.statusLine)).toEqual(['Connected · TCP 10.0.0.4:5760', 'Waiting for the vehicle · UDP port 14551', 'Not connected']);
  });

  it('a row keeps the index the core gave it, not its position in the list', () => {
    expect(linkRows(view(heardTcp, waitingUdp, idle)).map((it) => it.index)).toEqual([1, 2, 3]);
  });

  it('heard is what decides emphasis, and it is not the same as connected', () => {
    const rows = linkRows(view(heardTcp, waitingUdp));
    expect(rows.map((it) => it.connected)).toEqual([true, true]);
    expect(rows.map((it) => it.heard)).toEqual([true, false]);
  });

  it('the core has already dropped the automatic links', () => {
    const withDynamic = json(`{"links":[{"index":0,"name":"UDP Link (AutoConnect)","dynamic":true}],
                "configured":[${heardTcp}]}`);
    expect(linkRows(withDynamic).map((it) => it.name)).toEqual(['Pi']);
  });

  it('a last error is carried through', () => {
    const rows = linkRows(view(idle));
    expect(rows).toHaveLength(1);
    expect(rows[0]!.lastError).toBe('Connection refused');
  });

  it('an absent view yields no rows', () => {
    expect(linkRows(null)).toEqual([]);
    expect(linkRows(json('{}'))).toEqual([]);
  });

  it('reading the unfiltered list instead of the filtered one yields nothing', () => {
    const onlyLinks = json(`{"links":[${heardTcp}]}`);
    expect(linkRows(onlyLinks)).toHaveLength(0);
  });

  it('a payload using invented names for the sentence leaves it blank', () => {
    const invented = '{"index":0,"name":"X","status":"Connected","state":"Connected","connected":true}';
    const rows = linkRows(view(invented));
    expect(rows).toHaveLength(1);
    expect(rows[0]!.statusLine).toBe('');
  });

  it('a link names itself from what it points at', () => {
    expect(autoLinkName(LinkType.Udp, '', '14550')).toBe('UDP 14550');
    expect(autoLinkName(LinkType.Udp, '10.0.0.4', '14550')).toBe('UDP 14550');
    expect(autoLinkName(LinkType.Tcp, '10.0.0.4', '5760')).toBe('TCP 10.0.0.4:5760');
    expect(autoLinkName(LinkType.Tcp, '', '5760')).toBe('TCP');
  });

  it('a suggested name already in use gets a number, like LinkSettings _uniqueName', () => {
    expect(uniqueLinkName('UDP 14550', ['TCP'])).toBe('UDP 14550');
    expect(uniqueLinkName('UDP 14550', ['UDP 14550'])).toBe('UDP 14550 (2)');
    expect(uniqueLinkName('UDP 14550', ['UDP 14550', 'UDP 14550 (2)'])).toBe('UDP 14550 (3)');
  });

  it('a port outside the valid range is refused', () => {
    expect(linkFormError(LinkType.Udp, '', '0')).toBe('Enter a port between 1 and 65535, or leave it blank for 14550');
    expect(linkFormError(LinkType.Udp, '', '70000')).toBe('Enter a port between 1 and 65535, or leave it blank for 14550');
    expect(linkFormError(LinkType.Udp, '', 'abc')).toBe('Enter a port between 1 and 65535, or leave it blank for 14550');
  });

  it('tcp without an address is refused and udp without one is fine', () => {
    expect(linkFormError(LinkType.Tcp, '', '5760')).toBe('A TCP link needs the address of the device to call.');
    expect(linkFormError(LinkType.Udp, '', '14550')).toBeNull();
  });
});

describe('SerialLinkFormTest', () => {
  const ports = (...pairs: (readonly [string, string])[]) => json(`{"kind":"object","serialPorts":[${pairs.map(([port, label]) => `{"port":"${port}","label":"${label}"}`).join(',')}]}`);

  it('each port keeps the label the core paired it with', () => {
    const choices = serialPortChoices(ports(['/dev/ttyUSB0', 'FTDI UART'], ['/dev/ttyACM0', '/dev/ttyACM0']));
    expect(choices.map((it) => it.label)).toEqual(['FTDI UART', '/dev/ttyACM0']);
    expect(choices[0]!.port).toBe('/dev/ttyUSB0');
  });

  it('a blank label or port does not blank the row', () => {
    const choices = serialPortChoices(ports(['/dev/ttyUSB0', ''], ['', 'ghost']));
    expect(choices).toEqual([{ port: '/dev/ttyUSB0', label: '/dev/ttyUSB0' }]);
    expect(serialPortChoices(null)).toEqual([]);
  });

  it('no port picked is refused', () => {
    expect(serialFormError('', DEFAULT_BAUD, [], '', true)).toBe('Pick the port the radio is plugged into.');
  });

  it('a typed duplicate name is refused, a blank one is numbered instead', () => {
    expect(serialFormError('/dev/ttyUSB0', DEFAULT_BAUD, ['Serial ttyUSB0'], 'Serial ttyUSB0', true)).toBe('A link with that name already exists.');
    expect(serialFormError('/dev/ttyUSB0', DEFAULT_BAUD, ['Serial ttyUSB0'], '', true)).toBeNull();
  });

  it('a good serial form has nothing to say', () => {
    expect(serialFormError('/dev/ttyUSB0', DEFAULT_BAUD, ['UDP 14550'], '', true)).toBeNull();
  });

  it("the automatic name is the port's leaf", () => {
    expect(autoSerialName('Holybro radio (ttyUSB0)')).toBe('Holybro radio (ttyUSB0)');
    expect(autoSerialName(''), 'a port with no display name is suggested as plain Serial, as SerialSettings does').toBe('Serial');
    expect(portFor(LinkType.Tcp, '14550')).toBe('5760');
    expect(portFor(LinkType.Udp, '14551')).toBe('14551');
    expect(linkFormError(LinkType.Udp, '', '', '14551'), 'a blank UDP port means the configured listen port').toBeNull();
  });

  it('an empty port list explains itself rather than blaming the operator', () => {
    expect(serialFormError('', DEFAULT_BAUD, [], '', false)).toBe('Nothing is plugged in. Connect a radio over USB and it will appear here.');
  });
});

describe('LinkEditRulesTest', () => {
  const row = (connected = false, editing: LinkEditing = LinkEditing.PortOnly) => LinkRow({ index: 0, name: 'UDP 14550', statusLine: '', connected, heard: false, lastError: '', editing });

  it('a link carrying a vehicle is not edited underneath itself, but one still retrying is', () => {
    expect(linkIsEditable(LinkRow({ index: 0, name: 'UDP 14550', statusLine: '', connected: true, heard: true, lastError: '', editing: LinkEditing.PortOnly }))).toBe(false);
    expect(linkIsEditable(row(true))).toBe(true);
  });

  it('a WebSocket link edits its address, refuses one that is not ws://, and writes only that', () => {
    const socket = LinkRow({ index: 3, name: '127.0.0.1:9', statusLine: 'Reconnecting', connected: true, heard: false, lastError: '', editing: linkEditingFrom('url'), url: 'ws://127.0.0.1:9/mavlink' });
    expect(linkIsEditable(socket)).toBe(true);
    const edit = { ...linkEdit(socket), host: 'ws://127.0.0.1:5760' };
    expect(linkEdit(socket).host).toBe('ws://127.0.0.1:9/mavlink');
    expect(editError(LinkEditing.Url, { ...edit, host: 'http://127.0.0.1' }, '14550')).toBe('A MAVLink WebSocket address starts with ws:// or wss://.');
    expect(editError(LinkEditing.Url, edit, '14550')).toBeNull();
    expect(editWrites(LinkEditing.Url, '127.0.0.1:5760', edit.host, 0, '', 0, true, false)).toEqual([
      ['name', '127.0.0.1:5760'],
      ['autoConnect', true],
      ['highLatency', false],
      ['url', 'ws://127.0.0.1:5760'],
    ]);
  });

  it('a link that keeps failing waits apart from the links that hear a vehicle', () => {
    const retrying = LinkRow({ index: 0, name: 'ws', statusLine: 'Reconnecting', connected: true, heard: false, lastError: 'No MAVLink WebSocket answered', editing: LinkEditing.HostAndPort });
    const flying = LinkRow({ index: 1, name: 'udp', statusLine: 'Connected', connected: true, heard: true, lastError: '', editing: LinkEditing.PortOnly });
    const saved = LinkRow({ index: 2, name: 'tcp', statusLine: '', connected: false, heard: false, lastError: '', editing: LinkEditing.HostAndPort });
    expect(linkSections([retrying, flying, saved], []).map((it) => [it.title, it.rows.map((each) => each.name)])).toEqual([
      ['Connected', ['udp']],
      [LINK_WAITING, ['ws']],
      ['Saved', ['tcp']],
    ]);
  });

  it('a disconnected udp link is editable', () => {
    expect(linkIsEditable(row())).toBe(true);
  });

  it('a kind the core has no form for is not editable', () => {
    expect(linkIsEditable(row(false, LinkEditing.None))).toBe(false);
  });

  it('each link kind shows its own icon', () => {
    expect(linkIcon(LinkType.Udp)).toBe('ic_wifi');
    expect(linkIcon(LinkType.Serial)).toBe('ic_usb');
    expect(linkIcon(LinkType.Bluetooth)).toBe('ic_bluetooth');
    expect(linkIcon(LinkType.Tcp)).toBe('ic_lan');
    expect(linkIcon(LinkType.AircastCloud)).toBe('ic_cloud');
    expect(linkIcon(LinkType.LogReplay)).toBe('ic_history');
    expect(linkIcon(LinkType.Mock)).toBe('ic_science');
    expect(linkIcon(LinkType.Other)).toBe('ic_link');
    const typed = linkRows(json('{"configured":[{"index":0,"type":"tcp"}]}'));
    expect(typed).toHaveLength(1);
    expect(typed[0]!.type).toBe(LinkType.Tcp);
  });

  it("udp writes its own port property and not tcp's", () => {
    const writes = editWrites(LinkEditing.PortOnly, 'n', '', 14551, '', 0, false, false);
    expect(writes).toEqual([
      ['name', 'n'],
      ['autoConnect', false],
      ['highLatency', false],
      ['localPort', 14551],
    ]);
  });

  it('tcp writes host and port', () => {
    const writes = editWrites(LinkEditing.HostAndPort, 'n', '1.2.3.4', 5760, '', 0, true, false);
    expect(writes).toEqual([
      ['name', 'n'],
      ['autoConnect', true],
      ['highLatency', false],
      ['host', '1.2.3.4'],
      ['port', 5760],
    ]);
  });

  it('serial writes the port name, baud and the advanced framing', () => {
    const writes = editWrites(LinkEditing.Serial, 'n', '', 0, '/dev/ttyUSB0', 57600, false, true, SerialFraming({ dataBits: 7, stopBits: 2, parity: 2, flowControl: 1 }));
    expect(writes).toEqual([
      ['name', 'n'],
      ['autoConnect', false],
      ['highLatency', true],
      ['portName', '/dev/ttyUSB0'],
      ['baud', 57600],
      ['dataBits', 7],
      ['stopBits', 2],
      ['parity', 2],
      ['flowControl', 1],
    ]);
  });

  it('an unknown kind still renames and writes nothing else', () => {
    expect(editWrites(LinkEditing.None, 'n', 'h', 1, 'p', 2, false, false)).toEqual([
      ['name', 'n'],
      ['autoConnect', false],
      ['highLatency', false],
    ]);
  });

  it("a new link's flags are written to its row", () => {
    expect(linkFlagWrites(3, true, false)).toEqual([
      ['links.linkConfigurations.3.autoConnect', true],
      ['links.linkConfigurations.3.highLatency', false],
    ]);
  });

  it('a row reads its connect-on-start and high latency flags', () => {
    const rows = linkRows(json('{"configured":[{"index":0,"name":"n","autoConnect":true,"highLatency":true}]}'));
    expect([rows[0]!.autoConnect, rows[0]!.highLatency]).toEqual([true, true]);
  });

  it('a new serial link gets its framing written before it connects, as SerialSettings does on add', () => {
    expect(framingWrites(2, SerialFraming({ dataBits: 7, stopBits: 2, parity: 2, flowControl: 1 }))).toEqual([
      ['links.linkConfigurations.2.dataBits', 7],
      ['links.linkConfigurations.2.stopBits', 2],
      ['links.linkConfigurations.2.parity', 2],
      ['links.linkConfigurations.2.flowControl', 1],
    ]);
  });

  it('a saved bluetooth link is edited by picking another device, as BluetoothSettings does', () => {
    expect(linkIsEditable(LinkRow({ index: 0, name: 'HC-05', statusLine: '', connected: false, heard: false, lastError: '', editing: LinkEditing.Device }))).toBe(true);
    expect(editWrites(LinkEditing.Device, 'HC-05', '', 0, '', 0, false, false), 'the device is set by address through setDeviceByAddress, not written as a field').toEqual([
      ['name', 'HC-05'],
      ['autoConnect', false],
      ['highLatency', false],
    ]);
  });

  it('a saved log replay link is edited by choosing another log, as LogReplaySettings does', () => {
    expect(linkIsEditable(LinkRow({ index: 0, name: 'Replay', statusLine: '', connected: false, heard: false, lastError: '', editing: LinkEditing.LogFile }))).toBe(true);
    expect(editWrites(LinkEditing.LogFile, 'Replay', '', 0, '', 0, false, false, SerialFraming(), '/data/r.tlog')).toEqual([
      ['name', 'Replay'],
      ['autoConnect', false],
      ['highLatency', false],
      ['filename', '/data/r.tlog'],
    ]);
  });
});

describe('UdpServerFormTest', () => {
  it('a bare address sends to the listening port like UDPConfiguration addHost', () => {
    expect(udpServer('10.0.0.2', '14550')).toBe('10.0.0.2:14550');
    expect(udpServer(' 10.0.0.2:14555 ', '14550')).toBe('10.0.0.2:14555');
  });

  it('an address the configuration would refuse is not listed', () => {
    expect(udpServer('a:b:c', '14550')).toBeNull();
    expect(udpServer(':14550', '14550')).toBeNull();
    expect(udpServer('10.0.0.2:70000', '14550')).toBeNull();
  });

  it('a server already listed is not added twice', () => {
    const once = withServer([], '10.0.0.2', '14550');
    expect(withServer(once, '10.0.0.2:14550', '14550')).toEqual(['10.0.0.2:14550']);
    expect(withServer(once, 'bad:host:name', '14550')).toEqual(once);
  });

  it('a server still in the field is added when the link is saved, like UdpSettings saveSettings', () => {
    const typed = LinkDraft({ type: LinkType.Udp, servers: ['10.0.0.2:14550'], server: 'sitl.aircast.one' });
    expect(withTypedServer(typed, '14550')).toEqual({ ...typed, servers: ['10.0.0.2:14550', 'sitl.aircast.one:14550'], server: '' });
    expect(withTypedServer(LinkDraft({ type: LinkType.Udp }), '14550').servers).toEqual([]);
  });

  it('a server still in the field that could not be added stops the link from being made', () => {
    expect(serverFormError('', '14550')).toBeNull();
    expect(serverFormError('sitl.aircast.one', '14550')).toBeNull();
    expect(serverFormError('sitl.aircast.one:99999', '14550')).toBe(SERVER_FORMAT);
    expect(draftError(LinkDraft({ type: LinkType.Udp, server: 'a:b:c' }), '14550', [], false)).toBe(SERVER_FORMAT);
    expect(draftError(LinkDraft({ type: LinkType.Udp, server: 'sitl.aircast.one' }), '14550', [], false)).toBeNull();
  });

  it('formErrorsPointAtTheFieldThatFailed', () => {
    expect(linkFormErrorField(LinkType.Udp, '', '70000', '14550')).toBe('port');
    expect(linkFormErrorField(LinkType.Tcp, '10.0.0.2', 'abc')).toBe('port');
    expect(linkFormErrorField(LinkType.Tcp, '', '5760')).toBe('host');
    expect(linkFormErrorField(LinkType.Udp, '', '', '14550')).toBeNull();
    expect(linkFormErrorField(LinkType.Tcp, '10.0.0.2', '5760')).toBeNull();
  });
});

describe('AutoLinksTest', () => {
  it('connected automatic links are listed, saved and idle ones are not', () => {
    const view = json(`{"links":[
            {"name":"UDP Link (AutoConnect)","displaySummary":"UDP port 14550","dynamic":true,"connected":true,"heardVehicle":false},
            {"name":"TCP 127.0.0.1:5771","displaySummary":"127.0.0.1:5771","dynamic":true,"connected":true,"heardVehicle":true},
            {"name":"SiK","displaySummary":"ttyUSB0","dynamic":false,"connected":true,"heardVehicle":true},
            {"name":"Old","displaySummary":"x","dynamic":true,"connected":false,"heardVehicle":false}]}`);

    const links = autoLinks(view);
    expect(links.map((it) => it.name)).toEqual(['UDP Link (AutoConnect)', 'TCP 127.0.0.1:5771']);
    expect(links.map(autoLinkStatus)).toEqual(['Listening', 'Vehicle']);
  });

  it('a link with no address reads without a stray separator, and a mock link reads as simulated', () => {
    expect(autoLinkSubtitle({ name: 'Serial', summary: '', heard: false, type: LinkType.Other, index: 0 })).toBe('automatic');
    expect(autoLinkSubtitle({ name: 'UDP', summary: 'UDP port 14550', heard: false, type: LinkType.Other, index: 0 })).toBe('UDP port 14550 · automatic');
    expect(autoLinkSubtitle({ name: 'PX4 MultiRotor MockLink', summary: '', heard: true, type: LinkType.Mock, index: 0 })).toBe('Simulated');
  });
});

describe('LinkRowTest', () => {
  const twoLinks = `
{"available": true, "class": "Links", "configured": [
  {"index": 0, "name": "Packet radio", "type": "udp", "typeLabel": "UDP",
   "statusLine": "Connected · UDP port 14550", "connected": true, "heardVehicle": true,
   "goneQuiet": false, "dynamic": false, "lastError": "", "editing": "portOnly",
   "summary": "UDP port 14550", "displaySummary": "UDP port 14550", "path": "links.linkConfigurations.0"},
  {"index": 1, "name": "UDP 14551", "type": "udp", "typeLabel": "UDP",
   "statusLine": "Not hearing the vehicle · UDP port 14551", "connected": true, "heardVehicle": true,
   "goneQuiet": true, "dynamic": false, "lastError": "", "editing": "portOnly",
   "summary": "UDP port 14551", "displaySummary": "UDP port 14551", "path": "links.linkConfigurations.1"}
]}
`;

  it('a link that has gone quiet is carried through, not collapsed into heard', () => {
    const quiet = linkRows(json(twoLinks)).filter((it) => it.name === 'UDP 14551');
    expect(quiet).toHaveLength(1);
    expect(quiet[0]?.heard, 'heardVehicle stays true once a packet has ever arrived').toBe(true);
    expect(quiet[0]?.goneQuiet, "so the row needs the core's separate answer to tell the operator").toBe(true);
    expect(quiet[0]?.statusLine).toBe('Not hearing the vehicle · UDP port 14551');
  });

  it('the link still carrying the vehicle is not marked quiet', () => {
    const live = linkRows(json(twoLinks)).filter((it) => it.name === 'Packet radio');
    expect(live).toHaveLength(1);
    expect(live[0]?.heard).toBe(true);
    expect(live[0]?.goneQuiet).toBe(false);
  });
});

describe('RemedyTest', () => {
  it('an address that nothing answers says retrying will not help', () => {
    expect(remedyText(REMEDY_EDIT_ADDRESS)).toBe('Nothing is listening at that address. Retrying will not help until it is changed.');
  });

  it('a retryable error adds no advice, because Connect already says it', () => {
    expect(remedyText('retry')).toBeNull();
    expect(remedyText('')).toBeNull();
    expect(remedyText('somethingTheCoreAddedLater')).toBeNull();
  });
});

describe('LinkDraftTest', () => {
  const plugged = [{ port: '/dev/ttyUSB0', label: 'Holybro radio (ttyUSB0)' }];

  it('a link that saved but could not connect counts as added, not as a failure', () => {
    expect(addOutcome(true, true)).toBe(AddOutcome.Connected);
    expect(addOutcome(false, true)).toBe(AddOutcome.SavedNotConnected);
    expect(addOutcome(false, false)).toBe(AddOutcome.Failed);
  });

  it('a link that was never saved says so without guessing at the name', () => {
    expect(addFailure(LinkType.Tcp)).toBe('Could not add that link.');
    expect(addFailure(LinkType.Mock)).toBe('Could not start the simulated vehicle.');
  });

  it('each kind of new link asks for what it cannot work without', () => {
    expect(draftError(LinkDraft({ type: LinkType.Tcp, port: '5760' }), '14550', [], false)).toBe('A TCP link needs the address of the device to call.');
    expect(draftError(LinkDraft({ type: LinkType.Tcp, host: '10.0.0.2', port: '5760' }), '14550', [], false)).toBeNull();
    expect(draftError(LinkDraft({ type: LinkType.Udp }), '14550', [], false)).toBeNull();
    expect(draftError(LinkDraft({ type: LinkType.Bluetooth }), '14550', [], false)).toBe('Pick a Bluetooth device.');
    expect(draftError(LinkDraft({ type: LinkType.LogReplay }), '14550', [], false)).toBe('Choose a log file to replay.');
    expect(draftError(LinkDraft({ type: LinkType.Serial }), '14550', [], true)).toBe('Pick the port the radio is plugged into.');
    expect(draftError(LinkDraft({ type: LinkType.AircastCloud }), '14550', [], false)).toBe('');
    expect(draftError(LinkDraft({ type: LinkType.Mock }), '14550', [], false)).toBeNull();
  });

  it('a single plugged-in radio is picked for you, and a choice survives another radio appearing', () => {
    expect(withPluggedPort(LinkDraft({ type: LinkType.Serial }), plugged).portName).toBe('/dev/ttyUSB0');
    expect(withPluggedPort(LinkDraft({ type: LinkType.Serial }), [...plugged, { port: '/dev/ttyACM0', label: 'Pixhawk' }]).portName).toBe('');
    expect(withPluggedPort(LinkDraft({ type: LinkType.Serial, portName: '/dev/ttyACM0' }), plugged).portName).toBe('/dev/ttyACM0');
  });

  it('a new link is named after what it points at', () => {
    expect(suggestedLinkName(LinkDraft({ type: LinkType.Udp }), [], '14550')).toBe('UDP 14550');
    expect(suggestedLinkName(LinkDraft({ type: LinkType.Tcp, host: '10.0.0.2' }), [], '14550')).toBe('TCP 10.0.0.2:5760');
    expect(suggestedLinkName(LinkDraft({ type: LinkType.Serial, portName: '/dev/ttyUSB0' }), plugged, '14550')).toBe('Holybro radio (ttyUSB0)');
    expect(suggestedLinkName(LinkDraft({ type: LinkType.LogReplay }), [], '14550')).toBe(REPLAY_LINK_NAME);
  });

  it("link kinds and edit forms round-trip the core's strings, and anything new reads as Other or None", () => {
    const known = LinkTypeEntries.filter((it) => it !== LinkType.Other);
    expect(known.map((it) => linkTypeFrom(it.id))).toEqual(known);
    expect(linkTypeFrom('satellite')).toBe(LinkType.Other);
    expect(linkTypeFrom('')).toBe(LinkType.Other);
    expect(linkEditingFrom('portOnly')).toBe(LinkEditing.PortOnly);
    expect(linkEditingFrom('')).toBe(LinkEditing.None);
  });
});

describe('LinkEditTest', () => {
  const row = ({ editing = LinkEditing.PortOnly as LinkEditing, autoConnect = false, servers = [] as readonly string[], framing = SerialFraming() } = {}) =>
    LinkRow({ index: 0, name: 'UDP 14550', statusLine: '', connected: false, heard: false, lastError: '', editing, port: 14550, autoConnect, servers, framing });

  it('edit opens Advanced when the link already uses something in it', () => {
    expect(editShowsAdvanced(row())).toBe(false);
    expect(editShowsAdvanced(row({ autoConnect: true }))).toBe(true);
    expect(editShowsAdvanced(row({ servers: ['10.0.0.9:14550'] }))).toBe(false);
    expect(editShowsAdvanced(row({ editing: LinkEditing.Serial, framing: SerialFraming({ dataBits: 7 }) }))).toBe(true);
  });

  it('edit checks only the field that link kind has', () => {
    expect(editError(LinkEditing.PortOnly, linkEdit(row()), '14550')).toBeNull();
    expect(editError(LinkEditing.PortOnly, { ...linkEdit(row()), port: '0' }, '14550')).toBe('Enter a port between 1 and 65535, or leave it blank for 14550');
    expect(editError(LinkEditing.HostAndPort, { ...linkEdit(row()), port: 'abc' }, '14550')).toBe('Port must be a number between 1 and 65535.');
    expect(editError(LinkEditing.LogFile, { ...linkEdit(row()), logFile: '' }, '14550')).toBe('Choose a log file.');
    expect(editError(LinkEditing.Serial, linkEdit(row()), '14550')).toBeNull();
  });

  it('a blank udp port saves as the configured listen port', () => {
    expect(editedPort(LinkEditing.PortOnly, '', '14551')).toBe(14551);
    expect(editedPort(LinkEditing.HostAndPort, '', '14551')).toBe(0);
  });

  it('a link whose address is wrong offers editing first and a retry second', () => {
    expect(primaryLinkAction(true, false)).toBe('Disconnect');
    expect(primaryLinkAction(false, true)).toBe('Try again');
    expect(primaryLinkAction(false, false)).toBe('Connect');
  });

  it('the settings sheet hides its tabs inside a page or aircraft setup', () => {
    expect(sheetDrilled(null, false)).toBe(false);
    expect(sheetDrilled('Connections', false)).toBe(true);
    expect(sheetDrilled(null, true)).toBe(true);
  });
});

describe('AddableLinkTypesTest', () => {
  const links = (ids: string) => json(`{"kind":"object","class":"Links","links":[],"linkTypeIds":${ids}}`);

  it('a build without serial support stops offering serial', () => {
    expect(
      addableLinkTypes(links('["udp","tcp"]'), false),
      'linkTypeTable() wraps serial in QGC_NO_SERIAL_LINK, and createSerialConfiguration returns false on such a build - the chip was hardcoded, so it offered a link type that could only ever fail',
    ).toEqual([LinkType.WebSocket]);
  });

  it('a type this head cannot create is not offered just because the core lists it', () => {
    expect(
      addableLinkTypes(links('["serial","udp","tcp","bluetooth"]'), false),
      'createAndConnectLink handles udp and tcp and returns false for anything else, and Bluetooth is only offered where the core has a Bluetooth host',
    ).toEqual([LinkType.WebSocket, LinkType.Serial]);
  });

  it('a page served by an Aircast device offers UDP and TCP, carried by the device', () => {
    const paired = json('{"kind":"object","class":"Links","links":[],"linkTypeIds":["serial","udp","tcp"],"throughDevice":true}');
    expect(addableLinkTypes(paired, false)).toEqual([LinkType.WebSocket, LinkType.Udp, LinkType.Tcp, LinkType.Serial]);
  });

  it('a core too old to serve the list keeps all three rather than offering none', () => {
    expect(addableLinkTypes(json('{"kind":"object"}'), false)).toEqual([LinkType.WebSocket, LinkType.Serial]);
    expect(addableLinkTypes(null, false)).toEqual([LinkType.WebSocket, LinkType.Serial]);
    expect(addableLinkTypes(links('[]'), false)).toEqual([LinkType.WebSocket, LinkType.Serial]);
    expect(CREATABLE_LINK_TYPES).toEqual([LinkType.Udp, LinkType.Tcp, LinkType.Serial]);
  });

  it('bluetooth is offered when the core has a bluetooth host', () => {
    const view = json(`{"kind":"object","linkTypeIds":["serial","udp","tcp","bluetooth"],
            "bluetooth":{"available":true,"scanning":false,"devices":[{"name":"HC-05","address":"98:D3:31:F6:12:34"}]}}`);
    expect(addableLinkTypes(view, false)).toEqual([LinkType.WebSocket, LinkType.Serial, LinkType.Bluetooth]);
    expect(bluetoothState(view).devices).toEqual([{ name: 'HC-05', address: '98:D3:31:F6:12:34' }]);
  });

  it('log replay is offered when the core lists it, after the live links', () => {
    expect(addableLinkTypes(links('["serial","udp","tcp","logReplay"]'), false)).toEqual([LinkType.WebSocket, LinkType.Serial, LinkType.LogReplay]);
  });

  it('a browser reaches the aircraft over a MAVLink WebSocket, since it cannot open UDP or TCP sockets', () => {
    expect(addableLinkTypes(links('["serial","udp","tcp"]'), false)[0]).toBe(LinkType.WebSocket);
    expect(webSocketUrlError('ws://192.168.1.10:5760')).toBeNull();
    expect(webSocketUrlError('wss://bridge.example/mavlink')).toBeNull();
    expect(webSocketUrlError('http://192.168.1.10:5760')).toBe('A MAVLink WebSocket address starts with ws:// or wss://.');
    expect(webSocketUrlError('192.168.1.10')).toBe('Enter an address such as ws://192.168.1.10:5760.');
  });

  it('the desktop app opens UDP and TCP sockets itself, so it offers them and no WebSocket the native core cannot open', () => {
    expect(addableLinkTypes(links('["serial","udp","tcp","logReplay"]'), true)).toEqual([LinkType.Udp, LinkType.Tcp, LinkType.Serial, LinkType.LogReplay]);
  });

  it('a cloud link with a mistyped device is fixed in place, not deleted and added again', () => {
    const view = parseJson('{"configured":[{"index":1,"name":"Aircast cloud","statusLine":"Not connected","editing":"cloud","type":"aircastCloud","apiBase":"https://api.aircast.one","deviceId":"zzz"}]}');
    const [cloud] = linkRows(view);
    expect(cloud === undefined ? false : linkIsEditable(cloud)).toBe(true);
    const edit = cloud === undefined ? null : linkEdit(cloud);
    expect(edit?.cloud).toEqual({ apiBase: 'https://api.aircast.one', deviceId: 'zzz' });
    expect(edit === null ? null : editError(LinkEditing.Cloud, { ...edit, cloud: { ...edit.cloud, deviceId: ' ' } }, '14550')).toBe('');
    expect(edit === null ? null : editError(LinkEditing.Cloud, { ...edit, cloud: { ...edit.cloud, deviceId: 'drone-1' } }, '14550')).toBeNull();
    expect(editWrites(LinkEditing.Cloud, 'Aircast cloud', '', 0, '', 0, true, false, SerialFraming(), '', { apiBase: ' https://api.aircast.one ', deviceId: ' drone-1 ' })).toEqual([
      ['name', 'Aircast cloud'],
      ['autoConnect', true],
      ['highLatency', false],
      ['apiBase', 'https://api.aircast.one'],
      ['deviceId', 'drone-1'],
    ]);
  });

  it('a WebSocket address typed without ws:// gets it, and a port past 65535 is named as the problem', () => {
    expect(webSocketAddress(' 192.168.1.10:5760 ')).toBe('ws://192.168.1.10:5760');
    expect(webSocketAddress('wss://bridge.example/mavlink')).toBe('wss://bridge.example/mavlink');
    expect(webSocketAddress('http://192.168.1.10:5760')).toBe('http://192.168.1.10:5760');
    expect(webSocketAddress('  ')).toBe('');
    expect(webSocketUrlError(webSocketAddress('192.168.1.10:5760'))).toBeNull();
    expect(webSocketUrlError('ws://10.0.0.1:99999')).toBe('Port must be a number between 1 and 65535.');
    expect(webSocketUrlError('ws://10.0.0.1:5760/mavlink')).toBeNull();
    const socket = LinkRow({ index: 3, name: 'bridge', statusLine: '', connected: false, heard: false, lastError: '', editing: linkEditingFrom('url'), url: 'ws://127.0.0.1:9' });
    expect(editError(LinkEditing.Url, { ...linkEdit(socket), host: '10.0.0.1:5760' }, '14550')).toBeNull();
    expect(editWrites(LinkEditing.Url, 'bridge', '10.0.0.1:5760', 0, '', 0, true, false)).toContainEqual(['url', 'ws://10.0.0.1:5760']);
  });

  it('the WebSocket form offers the Aircast cloud simulator as one tap, not as a value already typed in', () => {
    expect(SIMULATOR_WEBSOCKET_URL).toBe('wss://sitl.aircast.one/ws/mavlink');
    expect(webSocketUrlError(SIMULATOR_WEBSOCKET_URL)).toBeNull();
  });

  it('a saved WebSocket link is named after the host it reaches', () => {
    expect(webSocketLinkName('wss://sitl.aircast.one/ws/mavlink')).toBe('sitl.aircast.one');
    expect(webSocketLinkName(' ws://192.168.1.10:5760 ')).toBe('192.168.1.10:5760');
    expect(webSocketLinkName('nonsense')).toBe('WebSocket');
  });

  it("each type is named in full, as LinkManager's type strings are", () => {
    expect([LinkType.Udp, LinkType.Tcp, LinkType.Serial, LinkType.Bluetooth, LinkType.LogReplay, LinkType.AircastCloud].map(linkTypeLabel)).toEqual(['UDP', 'TCP', 'Serial', 'Bluetooth', 'Log replay', 'Aircast Cloud']);
  });

  it('add link lists pilot links first, puts a plugged-in USB radio on top, and keeps replay and simulation apart', () => {
    const offered = [LinkType.Udp, LinkType.Tcp, LinkType.Serial, LinkType.Bluetooth, LinkType.LogReplay, LinkType.Mock, LinkType.AircastCloud];
    expect(pilotLinkKinds(offered, false)).toEqual([LinkType.Udp, LinkType.Serial, LinkType.Bluetooth, LinkType.AircastCloud, LinkType.Tcp]);
    expect(pilotLinkKinds(offered, true)).toEqual([LinkType.Serial, LinkType.Udp, LinkType.Bluetooth, LinkType.AircastCloud, LinkType.Tcp]);
    expect(toolLinkKinds(offered)).toEqual([LinkType.LogReplay, LinkType.Mock]);
    expect(pilotLinkKinds([LinkType.Udp, LinkType.LogReplay], true)).toEqual([LinkType.Udp]);
  });

  it('a USB link is named as the MAVLink radio it opens, apart from the RC transmitter that flies on its sticks', () => {
    expect(linkKind(LinkType.Serial).detail).toBe('MAVLink radio on USB');
    expect(RC_TRANSMITTER.title).toBe('RC transmitter (EdgeTX script)');
    expect(RC_TRANSMITTER.detail.length).toBeLessThanOrEqual(32);
    expect(RC_TRANSMITTER.picked).toContain('Control > Radio on USB');
  });

  it('every link kind has a pilot title, an explanation, and a subtitle within the 32-character row budget', () => {
    expect([...PILOT_LINK_ORDER, ...TOOL_LINK_ORDER].map(linkKind).filter((kind) => !(kind.title.trim() !== '' && kind.about.trim() !== '' && kind.detail.length <= 32)).map((kind) => kind.type.id)).toEqual([]);
  });

  it('an address is taken as people paste it: with a port, a scheme, a path or IPv6 brackets', () => {
    expect(parsedAddress('sitl.aircast.one')).toEqual({ host: 'sitl.aircast.one', port: null });
    expect(parsedAddress('  sitl.aircast.one:5760 ')).toEqual({ host: 'sitl.aircast.one', port: '5760' });
    expect(parsedAddress('tcp://sitl.aircast.one:5760')).toEqual({ host: 'sitl.aircast.one', port: '5760' });
    expect(parsedAddress('tcp://192.168.1.10:5762/mavlink')).toEqual({ host: '192.168.1.10', port: '5762' });
    expect(parsedAddress('[::1]:5760')).toEqual({ host: '::1', port: '5760' });
    expect(parsedAddress('fe80::1')).toEqual({ host: 'fe80::1', port: null });
  });

  it('a pasted host:port fills the port field only for a TCP link, and keeps the typed port when the address has none', () => {
    const tcp = LinkDraft({ type: LinkType.Tcp, host: 'sitl.aircast.one:5770', port: '5760' });
    expect(withParsedAddress(tcp)).toMatchObject({ host: 'sitl.aircast.one', port: '5770' });
    expect(withParsedAddress({ ...tcp, host: 'sitl.aircast.one' })).toMatchObject({ host: 'sitl.aircast.one', port: '5760' });
    const udp = LinkDraft({ type: LinkType.Udp, host: 'x:1', port: '14550' });
    expect(withParsedAddress(udp)).toBe(udp);
  });

  it('a dial succeeds only when an aircraft is heard, fails at once on a link error, and splits a silent open link from no answer', () => {
    const row = (fields: Partial<LinkRow>) => LinkRow({ index: 3, name: 'sim', statusLine: '', connected: false, heard: false, lastError: '', ...fields });
    expect(dialState(null, false)).toBe('failed');
    expect(dialState(row({ connected: true, heard: true }), false)).toBe('heard');
    expect(dialState(row({ lastError: "Can't find 192.168.1.5 on this network." }), false)).toBe('failed');
    expect(dialState(row({ connected: true }), false)).toBe('pending');
    expect(dialState(row({ connected: true }), true)).toBe('silent');
    expect(dialState(row({}), false)).toBe('pending');
    expect(dialState(row({}), true)).toBe('failed');
  });

  it('a failed dial gives the core reason and remedy, or says what to check', () => {
    const failed = LinkRow({ index: 1, name: 'sim', statusLine: '', connected: false, heard: false, lastError: "Can't find 192.168.1.5 on this network.", errorRemedy: 'Check the address.' });
    expect(dialFailure('192.168.1.5', '5760', failed)).toBe("Can't find 192.168.1.5 on this network. Check the address.");
    expect(dialFailure('sitl.aircast.one', '5760', null)).toBe('No answer from sitl.aircast.one:5760. Check the address, and that the device is on and listening.');
  });

  it('a link added for an aircraft reconnects next launch by default; a simulator and a replay do not', () => {
    expect(rememberedByDefault(LinkType.Tcp)).toBe(true);
    expect(rememberedByDefault(LinkType.Udp)).toBe(true);
    expect(rememberedByDefault(LinkType.Serial)).toBe(true);
    expect(rememberedByDefault(LinkType.Mock)).toBe(false);
    expect(rememberedByDefault(LinkType.LogReplay)).toBe(false);
  });

  it('a typed name already in use is refused before anything is created', () => {
    const draft = LinkDraft({ type: LinkType.Tcp, host: 'sitl.aircast.one', port: '5760', name: 'Sim' });
    expect(draftError(draft, '14550', ['Sim'], false)).toBe('A link named "Sim" already exists.');
    expect(draftError({ ...draft, name: '' }, '14550', ['Sim'], false)).toBeNull();
  });
});
