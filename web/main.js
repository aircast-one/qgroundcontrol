const worker = new Worker('worker.js', { type: 'module' });
const pending = new Map();
const transports = new Map();
const AUTOPILOTS = { 3: 'ArduPilot', 12: 'PX4' };

const $ = (selector) => document.querySelector(selector);
const say = (text) => { $('#status').textContent = text; };

const call = (name, ...args) => new Promise((resolve) => {
  const id = crypto.randomUUID();
  pending.set(id, resolve);
  worker.postMessage({ id, call: name, args });
});

const settle = ({ id, result }) => {
  pending.get(id)?.(result);
  pending.delete(id);
};

const fixed = (value, digits, unit) => (typeof value === 'number' ? `${value.toFixed(digits)}${unit}` : '—');

const rows = (vehicle) => [
  ['Vehicle', `${vehicle.id} · ${AUTOPILOTS[vehicle.autopilot] ?? `autopilot ${vehicle.autopilot}`}`],
  ['Mode', vehicle.flightMode ?? '—'],
  ['Armed', vehicle.armed ? 'yes' : 'no'],
  ['Altitude', fixed(vehicle.altitudeRelative, 1, ' m')],
  ['Position', vehicle.coordinate ? `${fixed(vehicle.coordinate.latitude, 6, '')}, ${fixed(vehicle.coordinate.longitude, 6, '')}` : '—'],
  ['Heading', fixed(vehicle.attitude?.heading, 0, '°')],
  ['Ground speed', fixed(vehicle.groundSpeed, 1, ' m/s')],
  ['Battery', vehicle.batteries?.[0] ? `${vehicle.batteries[0].percentRemaining}% · ${fixed(vehicle.batteries[0].voltage, 2, ' V')}` : '—'],
  ['Parameters', vehicle.parameters ? `${vehicle.parameters.count}${vehicle.parameters.ready ? '' : ' (loading)'}` : '—'],
];

const cell = (tag, text) => Object.assign(document.createElement(tag), { textContent: text });

const refresh = async () => {
  const view = JSON.parse(await call('get', 'view.coreVehicle'));
  $('#raw').textContent = JSON.stringify(view, null, 2);
  if (view.vehicle) {
    $('#vehicle').replaceChildren(...rows(view.vehicle).flatMap(([name, value]) => [cell('dt', name), cell('dd', value)]));
  }
};

const opened = (answer) => {
  const { ok, id, reason } = JSON.parse(answer);
  say(ok ? `Link ${id} open` : `The link did not open: ${reason}`);
  return ok ? id : null;
};

const closed = (id, reason) => {
  transports.delete(id);
  say(reason);
  return call('hostLinkClosed', id, reason);
};

const openMock = async (firmwareType) => opened(await call('linkOpen', JSON.stringify({ kind: 'mock', name: `Mock ${AUTOPILOTS[firmwareType]}`, firmwareType, vehicleType: 2 })));

const openSerial = async () => {
  const port = await navigator.serial.requestPort();
  await port.open({ baudRate: Number($('#baud').value) });
  const { usbVendorId, usbProductId } = port.getInfo();
  const id = opened(await call('hostLinkOpen', 'serial', `Serial ${usbVendorId?.toString(16) ?? '?'}:${usbProductId?.toString(16) ?? '?'}`));
  if (id === null) return port.close();
  const writer = port.writable.getWriter();
  transports.set(id, (bytes) => writer.write(bytes));
  const reader = port.readable.getReader();
  const read = async () => {
    const { value, done } = await reader.read().catch(() => ({ done: true }));
    return done ? closed(id, 'The serial port closed') : (call('hostLinkBytes', id, value), read());
  };
  return read();
};

const openSocket = async (url) => {
  const socket = new WebSocket(url);
  socket.binaryType = 'arraybuffer';
  await new Promise((resolve, reject) => {
    socket.onopen = resolve;
    socket.onerror = () => reject(new Error(`No WebSocket at ${url}`));
  });
  const id = opened(await call('hostLinkOpen', 'websocket', url));
  if (id === null) return socket.close();
  transports.set(id, (bytes) => socket.send(bytes));
  socket.onmessage = ({ data }) => call('hostLinkBytes', id, new Uint8Array(data));
  socket.onclose = () => closed(id, 'The WebSocket closed');
  return id;
};

const command = async (action) => {
  const { vehicleId } = JSON.parse(await call('get', 'view.coreGuided'));
  const { ok, reason } = JSON.parse(await call('guided', JSON.stringify({ ...action, vehicle: vehicleId })));
  $('#answer').textContent = ok ? '' : reason;
};

const failed = (error) => say(error.message);

worker.onmessage = ({ data }) => {
  if (data.ready) {
    say('Core ready. Connect a vehicle.');
    setInterval(refresh, 250);
  } else if (data.write) {
    transports.get(data.write.id)?.(data.write.bytes);
  } else {
    settle(data);
  }
};

document.querySelectorAll('[data-mock]').forEach((button) => button.addEventListener('click', () => openMock(Number(button.dataset.mock)).catch(failed)));
document.querySelectorAll('[data-guided]').forEach((button) => button.addEventListener('click', () => command(JSON.parse(button.dataset.guided)).catch(failed)));
$('#serial').addEventListener('click', () => openSerial().catch(failed));
$('#serial').disabled = !('serial' in navigator);
$('#socket').addEventListener('submit', (event) => {
  event.preventDefault();
  openSocket($('#url').value).catch(failed);
});
