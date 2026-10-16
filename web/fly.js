import { start, views, set, invoke, openMock, openSerial, openSocket, openLinks, closeLink } from './core.js';
import { createMap } from './map.js';

const POLL_MS = 200;
const TOAST_MS = 4000;
const CONFIRM_AT = 97;
const HEIGHT_KEY = 'aircast.takeoffHeight';
const DEFAULT_TAKEOFF_METERS = 10;
const EARTH_METERS = 6371000;
const PATHS = ['view.coreVehicle', 'view.flyState', 'view.guidedActions', 'view.battery', 'view.gps', 'view.flightModes', 'view.messages', 'vehicle.flightTime', 'view.guidedTakeoff'];
const ICONS = { arm: 'bolt', disarm: 'bolt', takeoff: 'flight_takeoff', pause: 'pause_circle', rtl: 'home', land: 'flight_land', emergencyStop: 'dangerous', forceArm: 'bolt', startMission: 'play_arrow', continueMission: 'play_arrow' };
const MORE = ['startMission', 'continueMission', 'forceArm', 'emergencyStop'];
const SLIDE_LABELS = { takeoff: 'Slide to take off', land: 'Slide to land', rtl: 'Slide to return', pause: 'Slide to pause', arm: 'Slide to arm', disarm: 'Slide to disarm', emergencyStop: 'Slide to stop motors' };

const $ = (selector) => document.querySelector(selector);
const latest = { views: {}, seenMessages: null, confirm: null };

const toast = (text, tone = '') => {
  const box = $('#toast');
  const token = crypto.randomUUID();
  Object.assign(box, { textContent: text, hidden: false });
  box.dataset.tone = tone;
  box.dataset.token = token;
  setTimeout(() => box.dataset.token === token && (box.hidden = true), TOAST_MS);
};

const refused = (reply) => reply && !reply.ok && toast(reply.reason || 'The vehicle did not accept that.', 'bad');

const meters = (value) => (Number.isFinite(value) ? `${value.toFixed(1)}m` : '–');
const speed = (value) => (Number.isFinite(value) ? `${value.toFixed(1)}m/s` : '–');
const clock = (seconds) => {
  const whole = Number.isFinite(seconds) && seconds > 0 ? Math.floor(seconds) : 0;
  return `${String(Math.floor(whole / 60)).padStart(2, '0')}'${String(whole % 60).padStart(2, '0')}"`;
};

const distance = (from, to) => {
  if (!from || !to) return NaN;
  const radians = (degrees) => (degrees * Math.PI) / 180;
  const dLat = radians(to.latitude - from.latitude);
  const dLon = radians(to.longitude - from.longitude);
  const a = Math.sin(dLat / 2) ** 2 + Math.cos(radians(from.latitude)) * Math.cos(radians(to.latitude)) * Math.sin(dLon / 2) ** 2;
  return 2 * EARTH_METERS * Math.asin(Math.sqrt(a));
};

const tone = (fly) => {
  if (!fly?.connected) return '';
  if (fly.fault || fly.contactLost) return 'bad';
  return fly.nominal || fly.flying ? 'good' : 'caution';
};

const batteryColor = (level) => ({ critical: 'var(--error)', low: 'var(--warning)', warning: 'var(--warning)' })[level] ?? 'var(--success)';

const offersOf = (view) => Object.fromEntries((view?.actions ?? []).map((action) => [action.id, action]));

const shown = (offer) => offer && offer.offer !== 'hidden';

const railFor = (offers, armed) => [armed ? offers.disarm : offers.arm, offers.takeoff, offers.pause, offers.rtl, offers.land].filter(shown);

const labelOf = (offer) => (offer.id === 'rtl' ? 'Return' : offer.title);

const button = (offer, onClick, icon = ICONS[offer.id] ?? 'radio_button_unchecked') => {
  const node = document.createElement('button');
  node.innerHTML = `<span class="icon">${icon}</span><span></span>`;
  node.lastChild.textContent = labelOf(offer);
  node.disabled = offer.offer !== 'ready';
  node.title = offer.offer === 'ready' ? offer.prompt : offer.reason || offer.prompt;
  node.dataset.warning = String(offer.id === 'disarm');
  node.addEventListener('click', onClick);
  return node;
};

const takeoffRange = () => {
  const view = latest.views['view.guidedTakeoff'];
  const minimum = view?.minimumMeters ?? view?.minimum ?? 1;
  const maximum = view?.maximum ?? 120;
  const saved = Number(localStorageGet(HEIGHT_KEY));
  const wanted = Number.isFinite(saved) && saved > 0 ? saved : DEFAULT_TAKEOFF_METERS;
  return { minimum, maximum, value: Math.min(maximum, Math.max(minimum, wanted)) };
};

const localStorageGet = (key) => {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
};

const localStorageSet = (key, value) => {
  try {
    localStorage.setItem(key, value);
  } catch {
    return;
  }
};

const run = (offer) => {
  const commands = {
    arm: () => set('vehicle.armed', true),
    disarm: () => set('vehicle.armed', false),
    takeoff: (height) => (offer.carriesValue === false ? invoke('vehicle.guidedModeTakeoff') : invoke('vehicle.guidedModeTakeoff', height)),
    pause: () => invoke('vehicle.guidedModeChangeAltitude', 0, true),
    rtl: () => invoke('vehicle.guidedModeRTL', false),
    land: () => invoke('vehicle.guidedModeLand'),
    emergencyStop: () => invoke('vehicle.emergencyStop'),
    forceArm: () => invoke('vehicle.forceArm'),
    startMission: () => invoke('vehicle.startMission'),
    continueMission: () => invoke('vehicle.startMission'),
  };
  return commands[offer.id] ?? null;
};

const showHeight = (height) => {
  $('#height-value').value = `${height.value.toFixed(0)} m`;
  $('#height').dataset.value = String(height.value);
};

const openConfirm = (offer) => {
  const command = run(offer);
  if (!command) return;
  const height = offer.id === 'takeoff' && offer.carriesValue !== false ? takeoffRange() : null;
  latest.confirm = { offer, command, height };
  $('#confirm-title').textContent = labelOf(offer);
  $('#confirm-prompt').textContent = offer.prompt;
  $('#confirm-icon').textContent = ICONS[offer.id] ?? 'help';
  $('#slide-label').textContent = SLIDE_LABELS[offer.id] ?? `Slide to ${labelOf(offer).toLowerCase()}`;
  $('#slide-input').setAttribute('aria-label', $('#slide-label').textContent);
  $('#confirm').dataset.destructive = String(Boolean(offer.destructive) || offer.id === 'emergencyStop' || offer.id === 'disarm');
  $('#height').hidden = !height;
  if (height) showHeight(height);
  $('#slide-input').value = 0;
  $('#confirm').hidden = false;
  $('#more').hidden = true;
  $('#slide-input').focus();
};

const closeConfirm = () => {
  latest.confirm = null;
  $('#confirm').hidden = true;
};

const step = (direction) => {
  const height = latest.confirm?.height;
  if (!height) return;
  const value = Math.min(height.maximum, Math.max(height.minimum, Math.round(height.value + direction)));
  latest.confirm = { ...latest.confirm, height: { ...height, value } };
  showHeight(latest.confirm.height);
};

const confirmed = async () => {
  const pending = latest.confirm;
  closeConfirm();
  if (!pending) return;
  if (pending.height) localStorageSet(HEIGHT_KEY, String(pending.height.value));
  refused(await pending.command(pending.height?.value));
};

const renderRail = (offers, armed) => {
  const entries = railFor(offers, armed);
  const extras = MORE.map((id) => offers[id]).filter(shown);
  const signature = JSON.stringify([entries, extras].map((list) => list.map((offer) => [offer.id, offer.offer, offer.title, offer.reason])));
  if ($('#rail').dataset.signature === signature) return;
  $('#rail').dataset.signature = signature;
  const more = extras.length ? [button({ id: 'more', title: 'More', offer: 'ready', prompt: 'More actions' }, (event) => {
    const menu = $('#more');
    menu.style.top = `${event.currentTarget.getBoundingClientRect().top}px`;
    menu.hidden = !menu.hidden;
  }, 'more_horiz')] : [];
  $('#rail').replaceChildren(...entries.map((offer) => button(offer, () => openConfirm(offer))), ...more);
  $('#more').replaceChildren(...extras.map((offer) => {
    const node = document.createElement('button');
    node.innerHTML = `<span class="icon">${ICONS[offer.id] ?? 'bolt'}</span><span></span><span class="summary"></span>`;
    node.children[1].textContent = offer.title;
    node.children[2].textContent = offer.offer === 'ready' ? offer.prompt : offer.reason;
    node.disabled = offer.offer !== 'ready';
    node.dataset.destructive = String(offer.id === 'emergencyStop');
    node.setAttribute('role', 'menuitem');
    node.addEventListener('click', () => openConfirm(offer));
    return node;
  }));
};

const renderModes = (modes) => {
  const listed = (modes?.everyday ?? []).filter((mode) => !mode.hidden);
  const signature = JSON.stringify(listed.map((mode) => [mode.name, mode.current]));
  $('#mode').disabled = !modes?.available;
  if ($('#modes').dataset.signature === signature) return;
  $('#modes').dataset.signature = signature;
  $('#modes').replaceChildren(...listed.map((mode) => {
    const node = document.createElement('button');
    node.setAttribute('role', 'menuitemradio');
    node.setAttribute('aria-checked', String(mode.current));
    node.innerHTML = `<span class="icon">${mode.current ? 'check' : ''}</span><span></span><span class="summary"></span>`;
    node.children[1].textContent = mode.name;
    node.children[2].textContent = mode.summary;
    node.addEventListener('click', async () => {
      $('#modes').hidden = true;
      $('#mode').setAttribute('aria-expanded', 'false');
      if (!latest.views['view.flightModes']?.canSet) return toast(latest.views['view.flightModes']?.cannotSetNotice || 'The flight mode cannot be changed now.', 'bad');
      return refused(await set('vehicle.flightMode', mode.name));
    });
    return node;
  }));
};

const renderLinks = () => {
  $('#links').replaceChildren(...openLinks().map((link) => {
    const row = document.createElement('div');
    row.className = 'link';
    row.innerHTML = '<span class="icon">link</span><span class="name"></span><button>Disconnect</button>';
    row.querySelector('.name').textContent = link.name;
    row.querySelector('button').addEventListener('click', async () => {
      await closeLink(link.id);
      renderLinks();
    });
    return row;
  }));
};

const announceMessages = (messages) => {
  const items = messages?.items ?? [];
  if (latest.seenMessages !== null && items.length > latest.seenMessages) {
    const newest = items.at(-1);
    const text = newest?.text ?? newest?.message ?? '';
    if (text) toast(text, (newest?.severity ?? 6) <= 3 ? 'bad' : '');
  }
  latest.seenMessages = items.length;
};

const render = (all) => {
  latest.views = all;
  const vehicle = all['view.coreVehicle']?.vehicle;
  const fly = all['view.flyState'];
  const battery = all['view.battery'];
  const gps = all['view.gps'];
  const connected = Boolean(vehicle && fly?.connected);
  document.body.dataset.connected = String(connected);
  document.body.dataset.tone = tone(fly);
  $('#mode-name').textContent = connected ? fly.mode : 'No vehicle';
  $('#state').textContent = connected ? fly.stateText || fly.state : 'Disconnected';
  const percent = battery?.headline?.percent;
  const color = batteryColor(battery?.headline?.level);
  document.body.style.setProperty('--battery', color);
  $('#battery-arc').style.strokeDasharray = `${Number.isFinite(percent) ? percent : 0} 100`;
  $('#battery-text').textContent = Number.isFinite(percent) ? String(Math.round(percent)) : '–';
  $('#return-fill').style.width = `${Number.isFinite(percent) ? percent : 0}%`;
  const pack = battery?.indicatorPacks?.[0];
  $('#current').textContent = Number.isFinite(pack?.current) ? `${pack.current.toFixed(1)}A` : '–';
  $('#flight-time').textContent = clock(all['vehicle.flightTime']?.value);
  $('#satellites').textContent = gps?.available ? String(gps.satellites ?? 0) : '–';
  $('.gps').dataset.lock = gps?.lock >= 3 ? 'good' : 'poor';
  $('#hs').textContent = speed(vehicle?.groundSpeed);
  $('#vs').textContent = speed(vehicle?.climbRate);
  $('#h').textContent = meters(vehicle?.altitudeRelative);
  $('#d').textContent = meters(distance(vehicle?.home, vehicle?.coordinate));
  $('#rose').style.transform = `rotate(${-(vehicle?.attitude?.heading ?? 0)}deg)`;
  renderRail(connected ? offersOf(all['view.guidedActions']) : {}, Boolean(fly?.armed));
  renderModes(all['view.flightModes']);
  announceMessages(all['view.messages']);
  map.show(vehicle);
  if (!connected && $('#connect').hidden) {
    renderLinks();
    $('#connect').hidden = false;
  }
  if (latest.confirm && !shown(offersOf(all['view.guidedActions'])[latest.confirm.offer.id])) closeConfirm();
};

const poll = async () => {
  render(await views(PATHS).catch(() => latest.views));
  setTimeout(poll, POLL_MS);
};

const connected = (reply) => {
  if (reply?.ok) {
    $('#connect').hidden = true;
    toast('Link open. Waiting for the vehicle…');
  } else {
    refused(reply);
  }
};

const failed = (error) => error?.name !== 'NotFoundError' && toast(error.message, 'bad');

const map = createMap($('#map'), (following) => { document.body.dataset.follow = String(following); });

document.querySelectorAll('[data-mock]').forEach((node) => node.addEventListener('click', () => openMock(Number(node.dataset.mock), node.textContent.trim().replace(/^deployed_code/, '')).then(connected).catch(failed)));
$('#serial').disabled = !('serial' in navigator);
$('#serial').title = 'serial' in navigator ? 'Pick a USB radio or flight controller' : 'This browser has no Web Serial: use Chrome or Edge';
$('#serial').addEventListener('click', () => openSerial(Number($('#baud').value)).then(connected).catch(failed));
$('#socket').addEventListener('submit', (event) => {
  event.preventDefault();
  openSocket($('#url').value).then(connected).catch(failed);
});
$('#menu').addEventListener('click', () => {
  renderLinks();
  $('#connect').hidden = !$('#connect').hidden;
});
$('#connect-close').addEventListener('click', () => { $('#connect').hidden = true; });
$('#mode').addEventListener('click', () => {
  const open = $('#modes').hidden;
  $('#modes').hidden = !open;
  $('#mode').setAttribute('aria-expanded', String(open));
});
$('#recenter').addEventListener('click', () => map.recenter());
$('#confirm-cancel').addEventListener('click', closeConfirm);
document.querySelectorAll('[data-step]').forEach((node) => node.addEventListener('click', () => step(Number(node.dataset.step))));
$('#slide-input').addEventListener('change', (event) => {
  if (Number(event.target.value) >= CONFIRM_AT) {
    confirmed();
  } else {
    event.target.value = 0;
  }
});
document.addEventListener('keydown', (event) => {
  if (event.key !== 'Escape') return;
  closeConfirm();
  $('#modes').hidden = true;
  $('#more').hidden = true;
});
document.addEventListener('click', (event) => {
  if (!event.target.closest('#modes, #mode')) $('#modes').hidden = true;
  if (!event.target.closest('#more, #rail')) $('#more').hidden = true;
});

await start();
poll();
