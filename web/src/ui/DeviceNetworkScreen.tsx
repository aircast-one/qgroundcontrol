import { type ReactNode, useState } from 'react';
import { FilledTonalButton, IconButton, LinearProgressIndicator, OutlinedTextField, Switch, Text, TextButton } from '@/m3';
import { MaterialIcon } from '@/res/Icon';
import { md } from '@/theme/Colors';
import {
  ActionButton,
  type DeviceBadge,
  DeviceBadgeText,
  DeviceChart,
  DeviceDisclosure,
  DeviceFact,
  DeviceForm,
  DeviceNotice,
  DeviceSection,
  DeviceSelect,
  DeviceSwitchRow,
  DeviceText,
  type DeviceOption,
  RefusalText,
  levelColor,
  useDeviceAction,
} from '@/ui/DeviceKit';
import { type ShapedField, ShapedForm } from '@/ui/DeviceKitForms';
import {
  type DataCard,
  type DataGauge,
  type DeviceProblem,
  GOOGLE_STUN,
  type IPv4Settings,
  type IceServer,
  type InterfacesCard,
  type ModemCard,
  type NetLink,
  type RelayCard,
  type SavedProfile,
  type StarlinkCard,
  type StarlinkDemo,
  type TailscaleCard,
  type TailscaleControl,
  type UplinkCard,
  connectionFieldShown,
  emptyIceServer,
  iceRows,
  iceServerLines,
  manualAddressing,
  parseNetwork,
  uplinkIcon,
} from '@/ui/DeviceNetwork';
import type { DeviceScreenProps } from '@/ui/DeviceScreens';
import { DeviceLevel } from '@/ui/DeviceLevel';
import { useDeviceView } from '@/ui/DeviceView';
import { WifiJoin } from '@/ui/DeviceWifiWizard';
import { FootNote } from '@/ui/SetupRow';

export const NetworkScreen = ({ onOpen }: DeviceScreenProps) => {
  const view = useDeviceView('network', parseNetwork);
  if (view === null) return <FootNote text="Waiting for the device to answer…" />;
  return (
    <>
      <UplinkSection uplink={view.uplink} onOpen={onOpen} />
      {view.modem !== null ? <ModemSection modem={view.modem} /> : null}
      <StarlinkSection starlink={view.starlink} />
      <DeviceSection title="Wi-Fi" badge={view.wifi.status}>
        <Summary text={view.wifi.summary} />
        {view.wifi.joined !== null ? <DeviceNotice notice={view.wifi.joined} /> : null}
        <WifiJoin reopen={view.wifi.reopen} />
      </DeviceSection>
      <InterfacesSection interfaces={view.interfaces} />
      <FootNote text="The device routes through the best uplink that reaches the internet and moves off one that stops — this is where you say what it may use." />
      <DataSection data={view.data} />
      <FootNote text="Mobile data is metered; the budget here is what keeps a flight from spending a month of it — set it before a SIM goes in." />
      <TailscaleSection tailscale={view.tailscale} />
      <RelaySection relay={view.relay} />
      <FootNote text="Reaching this device from somewhere else — it is not needed to fly." />
    </>
  );
};

const Summary = ({ text }: { text: string }) =>
  text === '' ? null : (
    <div className="px-list pb-1">
      <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
        {text}
      </Text>
    </div>
  );

const Leveled = ({ badge, typography = 'bodyMedium' }: { badge: DeviceBadge; typography?: 'bodyMedium' | 'titleMedium' }) => (
  <Text typography={typography} color={badge.level === DeviceLevel.Neutral ? md('onSurfaceVariant') : levelColor(badge.level)}>
    {badge.text}
  </Text>
);

const ProblemNotice = ({ problem }: { problem: DeviceProblem }) => (
  <>
    <DeviceNotice notice={{ level: DeviceLevel.Blocked, text: problem.text }} />
    {problem.detail !== '' ? (
      <DeviceDisclosure title="Technical detail">
        <div className="px-list pb-2">
          <Text typography="bodySmall" monospace className="break-all">
            {problem.detail}
          </Text>
        </div>
      </DeviceDisclosure>
    ) : null}
  </>
);

const Actions = ({ children }: { children: ReactNode }) => <div className="flex flex-wrap items-start gap-2 px-list py-2">{children}</div>;

const UplinkSection = ({ uplink, onOpen }: { uplink: UplinkCard; onOpen: (href: string) => void }) => (
  <DeviceSection title="Primary uplink" badge={uplink.status}>
    <div className="flex items-start gap-3 px-list py-2">
      <MaterialIcon name={uplinkIcon(uplink.icon)} style={{ color: md('onSurfaceVariant') }} />
      <div className="flex min-w-0 flex-1 flex-col gap-1">
        <div role="heading" aria-level={3}>
          <Text typography="titleMedium" className="break-words">
            {uplink.title}
          </Text>
        </div>
        <Leveled badge={uplink.verdict} />
        {uplink.detail !== '' ? (
          <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
            {uplink.detail}
          </Text>
        ) : null}
        {uplink.addr !== '' || uplink.meta !== '' ? (
          <Text typography="bodySmall" color={md('onSurfaceVariant')} className="break-words">
            {[uplink.addr, uplink.meta].filter((it) => it !== '').join(' · ')}
          </Text>
        ) : null}
      </div>
      {uplink.rate !== '' ? (
        <div className="flex shrink-0 flex-col items-end">
          <Text typography="labelSmall" color={md('onSurfaceVariant')}>
            Sending now
          </Text>
          <Text typography="titleMedium" monospace>
            {uplink.rate}
          </Text>
        </div>
      ) : null}
    </div>
    {uplink.warning !== '' ? (
      <>
        <DeviceNotice notice={{ level: DeviceLevel.Warn, text: uplink.warning }} />
        <Actions>
          <TextButton onClick={() => onOpen('/streaming')}>Check the camera →</TextButton>
        </Actions>
      </>
    ) : null}
    {uplink.failover.length > 0 ? (
      <div role="list" aria-label="Backup uplinks" className="flex flex-col">
        {uplink.failover.map((row) => (
          <div key={row.label} role="listitem" className="flex flex-wrap items-baseline justify-between gap-x-4 px-list py-1.5">
            <Text typography="bodyMedium">{row.label}</Text>
            <Leveled badge={row.status} />
          </div>
        ))}
      </div>
    ) : null}
    <DeviceText text={uplink.switchText} />
    <DeviceText text={uplink.backup} />
  </DeviceSection>
);

const ModemSection = ({ modem }: { modem: ModemCard }) => (
  <DeviceSection title="Cellular modem" badge={modem.status}>
    {modem.notices.map((notice) => (
      <DeviceNotice key={notice.text} notice={notice} />
    ))}
    <div className="flex flex-col px-list py-2">
      <Text typography="bodyLarge">{modem.operator}</Text>
      <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
        {modem.detail}
      </Text>
    </div>
    {modem.signal !== null ? <DeviceChart chart={modem.signal} /> : null}
    {modem.facts.map((fact) => (
      <DeviceFact key={fact.label} label={fact.label} value={fact.value} />
    ))}
    <DeviceDisclosure title="Text messages" summary={modem.messages.summary}>
      {modem.messages.items.length === 0 ? (
        <DeviceText text="Messages the operator sends to this SIM — package expiry, top-ups, portal codes — show up here." />
      ) : (
        <div role="list" aria-label="Text messages" className="flex flex-col">
          {modem.messages.items.map((message) => (
            <div key={`${message.when}|${message.text}`} role="listitem" className="flex flex-col px-list py-2">
              <div className="flex items-baseline justify-between gap-3">
                <Text typography="bodyLarge">{message.from}</Text>
                <Text typography="bodySmall" color={md('onSurfaceVariant')}>
                  {message.when}
                </Text>
              </div>
              <Text typography="bodyMedium" color={md('onSurfaceVariant')} className="break-words whitespace-pre-line">
                {message.text}
              </Text>
            </div>
          ))}
        </div>
      )}
    </DeviceDisclosure>
    {modem.sim !== null ? (
      <DeviceDisclosure title="SIM & APN" summary={modem.sim.summary}>
        <DeviceForm
          key={`${modem.sim.apn}|${modem.sim.metered}`}
          method="PUT"
          path="/api/network/cellular"
          submit="Save"
          success="Cellular settings applied."
          note="Re-dials the modem on save."
          confirm={{ title: 'Re-dial the modem?', text: "Saving re-dials the modem — the cellular link drops briefly. If you're connected over cellular you may lose access until it reconnects. Continue?", confirm: 'Save' }}
          fields={[
            { name: 'apn', label: 'APN', value: modem.sim.apn, placeholder: modem.sim.placeholder },
            { name: 'pin', label: 'SIM PIN', value: '', keyboard: 'password', hint: 'Leave blank to keep the current PIN.' },
            { name: 'metered', label: 'Data usage', value: modem.sim.metered, options: modem.sim.options },
          ]}
        />
      </DeviceDisclosure>
    ) : null}
  </DeviceSection>
);

const StarlinkSection = ({ starlink }: { starlink: StarlinkCard }) => (
  <DeviceSection title="Starlink" badge={starlink.status}>
    <Summary text={starlink.summary} />
    {starlink.demo !== null && starlink.demo.first ? <DemoDish demo={starlink.demo} /> : null}
    {starlink.notices.map((notice) => (
      <DeviceNotice key={notice.text} notice={notice} />
    ))}
    {starlink.pollError !== '' ? (
      <div className="px-list pb-2">
        <Text typography="bodySmall" monospace className="break-all">
          {starlink.pollError}
        </Text>
      </div>
    ) : null}
    {starlink.facts.map((fact) => (
      <DeviceFact key={fact.label} label={fact.label} value={fact.value} />
    ))}
    {starlink.firmware !== '' ? <DeviceFact label="Firmware" value={starlink.firmware} /> : null}
    <DeviceDisclosure title="Dish address" initiallyOpen={starlink.address.open}>
      <DeviceText text="The network address of the Starlink dish. Leave the default unless the dish is reached at a different address on this device's network. Port 9200 is assumed if you leave it off." />
      {starlink.address.defaultFirst ? <UseDefaultAddress fallback={starlink.address.fallback} /> : null}
      <DeviceForm
        key={starlink.address.value}
        method="PUT"
        path="/api/starlink/config"
        submit="Save"
        success="Dish address saved."
        fields={[{ name: 'addr', label: 'Dish address', value: starlink.address.value, placeholder: '192.168.100.1:9200', keyboard: 'uri' }]}
      />
      {!starlink.address.defaultFirst ? <UseDefaultAddress fallback={starlink.address.fallback} /> : null}
    </DeviceDisclosure>
    {starlink.demo !== null && !starlink.demo.first ? <DemoDish demo={starlink.demo} /> : null}
  </DeviceSection>
);

const UseDefaultAddress = ({ fallback }: { fallback: string }) =>
  fallback === '' ? null : (
    <Actions>
      <ActionButton label={`Use default (${fallback})`} request={{ method: 'PUT', path: '/api/starlink/config', body: { addr: fallback } }} success="Dish address back to the default." />
    </Actions>
  );

const DemoDish = ({ demo }: { demo: StarlinkDemo }) => {
  const action = useDeviceAction();
  return (
    <div role="group" aria-label="Demo dish">
      <DeviceSwitchRow
        title="Demo dish"
        detail={demo.note}
        checked={demo.on}
        on={{ method: 'POST', path: '/api/starlink/demo', body: { condition: 'healthy' } }}
        off={{ method: 'POST', path: '/api/starlink/demo', body: { condition: 'off' } }}
        success={demo.on ? 'Demo dish off.' : 'Demo dish on — simulated, not a real dish.'}
      />
      {demo.on ? (
        <div className="px-list py-2">
          <DeviceSelect
            label="Condition"
            value={demo.condition}
            options={demo.conditions}
            enabled={!action.busy}
            onPick={(condition) => void action.run({ method: 'POST', path: '/api/starlink/demo', body: { condition } }, 'Demo dish condition changed.')}
          />
          <RefusalText refusal={action.refusal} />
        </div>
      ) : null}
    </div>
  );
};

const ipv4Fields = (methods: readonly DeviceOption[], ipv4: IPv4Settings): readonly ShapedField[] => [
  { name: 'method', label: 'IPv4 method', value: ipv4.method || (methods[0]?.value ?? 'auto'), options: methods },
  { name: 'metric', label: 'Route metric (priority)', value: ipv4.metric, placeholder: ipv4.metricPlaceholder, keyboard: 'number' },
  { name: 'address', label: 'Address (CIDR)', value: ipv4.address, placeholder: '192.168.8.2/24', keyboard: 'uri', shownWhen: (values) => manualAddressing(values.method ?? '') },
  { name: 'gateway', label: 'Gateway', value: ipv4.gateway, placeholder: 'optional', keyboard: 'uri', shownWhen: (values) => manualAddressing(values.method ?? '') },
  { name: 'dns', label: 'DNS', value: ipv4.dns, placeholder: '8.8.8.8, 1.1.1.1', keyboard: 'uri' },
];

const ipv4Key = (ipv4: IPv4Settings): string => [ipv4.method, ipv4.metric, ipv4.address, ipv4.gateway, ipv4.dns].join('|');

const RenameForm = ({ uuid, name }: { uuid: string; name: string }) => (
  <DeviceForm
    key={name}
    method="PUT"
    path={`/api/network/connections/${uuid}/name`}
    submit="Rename"
    success="Connection renamed."
    fields={[{ name: 'name', label: 'Connection name', value: name, placeholder: 'e.g. Wired connection 2' }]}
  />
);

const InterfacesSection = ({ interfaces }: { interfaces: InterfacesCard }) => {
  const listed = interfaces.links.length + interfaces.saved.length + interfaces.other.length > 0;
  return (
    <DeviceSection title="Interfaces & priority" badge={interfaces.status}>
      <Summary text={interfaces.summary} />
      {interfaces.problem !== null ? <ProblemNotice problem={interfaces.problem} /> : null}
      {interfaces.problem === null && !listed ? <DeviceNotice notice={{ level: DeviceLevel.Neutral, text: 'No connections.' }} /> : null}
      {interfaces.links.map((link) => (
        <LinkRow key={link.editor?.uuid || link.name} link={link} methods={interfaces.methods} />
      ))}
      {interfaces.saved.map((profile) => (
        <SavedRow key={profile.uuid} profile={profile} methods={interfaces.methods} />
      ))}
      {interfaces.problem === null && interfaces.links.length + interfaces.saved.length > 0 ? (
        <DeviceText text="The primary uplink carries internet traffic; a backup takes over if it drops. Open a connection to edit it — changes to a running connection apply immediately; inactive ones are saved profiles that aren't running." />
      ) : null}
      {interfaces.other.length > 0 ? (
        <div role="list" aria-label="Other interfaces">
          <div className="px-list pt-3">
            <Text typography="labelMedium" color={md('onSurfaceVariant')}>
              Other interfaces
            </Text>
          </div>
          {interfaces.other.map((usage) => (
            <div key={usage.label} role="listitem" className="flex flex-col px-list py-2">
              <Text typography="bodyLarge">{usage.label}</Text>
              <Text typography="bodySmall" color={md('onSurfaceVariant')}>
                {`${usage.month} · all-time ${usage.total}`}
              </Text>
            </div>
          ))}
        </div>
      ) : null}
      <DeviceDisclosure title="Add a connection" summary="Create a new NetworkManager profile — ethernet, Wi-Fi or cellular.">
        <div role="group" aria-label="Add a connection">
          <ShapedForm
            method="POST"
            path="/api/network/connections"
            submit="Create"
            success="Connection created."
            note="Created inactive unless autoconnect is on — it won't disturb the current uplink."
            fields={[
              { name: 'type', label: 'Type', value: interfaces.add.types[0]?.value ?? 'ethernet', options: interfaces.add.types },
              { name: 'name', label: 'Name', value: '', placeholder: 'camera-net' },
              { name: 'interface', label: 'Interface', value: '', options: [{ value: '', label: 'Any interface' }, ...interfaces.add.ports] },
              { name: 'autoconnect', label: 'Autoconnect', value: interfaces.add.autoconnect[0]?.value ?? 'false', options: interfaces.add.autoconnect },
              { name: 'ssid', label: 'SSID', value: '', placeholder: 'network name', shownWhen: (values) => connectionFieldShown(values.type ?? '', 'ssid') },
              { name: 'psk', label: 'Passphrase', value: '', keyboard: 'password', hint: 'Leave blank for an open network.', shownWhen: (values) => connectionFieldShown(values.type ?? '', 'psk') },
              { name: 'apn', label: 'APN', value: '', placeholder: 'internet', shownWhen: (values) => connectionFieldShown(values.type ?? '', 'apn') },
              { name: 'pin', label: 'SIM PIN', value: '', placeholder: 'optional', keyboard: 'password', shownWhen: (values) => connectionFieldShown(values.type ?? '', 'pin') },
              ...ipv4Fields(interfaces.methods, { method: '', metric: '', metricPlaceholder: 'automatic', address: '', gateway: '', dns: '' }),
            ]}
          />
        </div>
      </DeviceDisclosure>
    </DeviceSection>
  );
};

const linkSummary = (link: NetLink): string => [link.kind, link.role?.text ?? (link.local ? 'local' : ''), link.where, link.usage].filter((it) => it !== '').join(' · ');

const LinkRow = ({ link, methods }: { link: NetLink; methods: readonly DeviceOption[] }) =>
  link.editor === null ? (
    <div className="flex flex-col px-list py-2">
      <Text typography="bodyLarge">{link.name}</Text>
      <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
        {linkSummary(link)}
      </Text>
    </div>
  ) : (
    <DeviceDisclosure title={link.name} summary={linkSummary(link)}>
      <div role="group" aria-label={link.name} className="flex flex-col">
        <ShapedForm
          key={ipv4Key(link.editor.ipv4)}
          method="PUT"
          path={`/api/network/interfaces/${link.editor.iface}`}
          submit="Save"
          success={`${link.editor.iface} updated.`}
          note="Applies immediately."
          confirm={link.editor.confirm === '' ? null : { title: `Reconfigure ${link.editor.iface}?`, text: link.editor.confirm, confirm: 'Continue' }}
          fields={ipv4Fields(methods, link.editor.ipv4)}
        />
        {link.editor.uuid !== '' ? (
          <>
            <RenameForm uuid={link.editor.uuid} name={link.name} />
            <DeviceText text="Bring this connection down, or remove the saved profile." />
            <Actions>
              <ActionButton
                label="Deactivate"
                request={{ method: 'POST', path: `/api/network/connections/${link.editor.uuid}/deactivate` }}
                success={`${link.name} deactivated.`}
                confirm={{ title: `Deactivate ${link.name}?`, text: link.editor.deactivateConfirm, confirm: 'Deactivate' }}
              />
              <ActionButton
                label="Delete"
                tonal={false}
                request={{ method: 'DELETE', path: `/api/network/connections/${link.editor.uuid}` }}
                success={`${link.name} deleted.`}
                confirm={{ title: `Delete ${link.name}?`, text: link.editor.deleteConfirm, confirm: 'Delete' }}
              />
            </Actions>
          </>
        ) : null}
      </div>
    </DeviceDisclosure>
  );

const SavedRow = ({ profile, methods }: { profile: SavedProfile; methods: readonly DeviceOption[] }) => (
  <DeviceDisclosure title={profile.name} summary={`${profile.kind} · inactive`}>
    <div role="group" aria-label={profile.name} className="flex flex-col">
      <RenameForm uuid={profile.uuid} name={profile.name} />
      <ShapedForm
        key={ipv4Key(profile.ipv4)}
        method="PUT"
        path={`/api/network/connections/${profile.uuid}/ipv4`}
        submit="Save"
        success={`${profile.name} updated.`}
        note="Saved to the profile (not activated)."
        fields={ipv4Fields(methods, profile.ipv4)}
      />
      <Actions>
        <ActionButton
          label="Activate"
          request={{ method: 'POST', path: `/api/network/connections/${profile.uuid}/activate` }}
          success={`${profile.name} activated.`}
          confirm={{ title: `Activate ${profile.name}?`, text: profile.activateConfirm, confirm: 'Activate' }}
        />
        <ActionButton
          label="Delete"
          tonal={false}
          request={{ method: 'DELETE', path: `/api/network/connections/${profile.uuid}` }}
          success={`${profile.name} deleted.`}
          confirm={{ title: `Delete ${profile.name}?`, text: profile.deleteConfirm, confirm: 'Delete' }}
        />
      </Actions>
    </div>
  </DeviceDisclosure>
);

const DataSection = ({ data }: { data: DataCard }) => (
  <DeviceSection title="Cellular data" badge={data.status}>
    <Summary text={data.summary} />
    {data.gauge !== null ? <Gauge gauge={data.gauge} /> : <DeviceNotice notice={{ level: DeviceLevel.Neutral, text: 'No usage recorded yet — counters accumulate as data flows.' }} />}
    <DeviceDisclosure title="Data caps & warnings" summary={data.budget.summary}>
      <DeviceForm
        key={`${data.budget.monthlyCapGB}|${data.budget.flightCapMB}|${data.budget.warnPercent}`}
        method="PUT"
        path="/api/usage/budget"
        submit="Save"
        success="Data caps saved."
        note="Warnings alert the ground station; caps never cut the link."
        fields={[
          { name: 'monthlyCapGB', label: 'Monthly plan (GB)', value: data.budget.monthlyCapGB, placeholder: 'e.g. 10 — blank for none', keyboard: 'decimal' },
          { name: 'flightCapMB', label: 'Per-flight budget (MB)', value: data.budget.flightCapMB, placeholder: 'e.g. 800 — blank for none', keyboard: 'decimal' },
          { name: 'warnPercent', label: 'Warn at (%)', value: data.budget.warnPercent, placeholder: '80', keyboard: 'number' },
        ]}
      />
    </DeviceDisclosure>
  </DeviceSection>
);

const Gauge = ({ gauge }: { gauge: DataGauge }) => (
  <div role="group" aria-label="Cellular · LTE" className="mx-list my-2 flex flex-col gap-3 rounded-shape-md bg-surface-container-high p-4">
    <div className="flex flex-wrap items-center justify-between gap-2">
      <Text typography="labelMedium" color={md('onSurfaceVariant')}>
        Cellular · LTE
      </Text>
      <div className="flex flex-wrap items-center gap-3">
        {gauge.badges.map((badge) => (
          <DeviceBadgeText key={badge.text} badge={badge} />
        ))}
      </div>
    </div>
    <div className="flex flex-wrap items-baseline gap-2">
      <Text typography="headlineSmall">{gauge.hero}</Text>
      <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
        {gauge.heroCaption}
      </Text>
    </div>
    <div className="flex flex-col gap-1.5">
      <LinearProgressIndicator progress={gauge.uplinkPercent / 100} label="Uplink share" className="h-2" />
      <div className="flex flex-wrap justify-between gap-2">
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          {gauge.uplink}
        </Text>
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          {gauge.downlink}
        </Text>
      </div>
    </div>
    {gauge.meters.map((meter) => (
      <div key={meter.label} role="group" aria-label={meter.label} className="flex flex-col gap-1">
        <div className="flex flex-wrap items-baseline justify-between gap-2">
          <Text typography="bodySmall" color={md('onSurfaceVariant')}>
            {meter.label}
          </Text>
          <Text typography="bodySmall" monospace color={meter.level === DeviceLevel.Neutral ? md('onSurface') : levelColor(meter.level)}>
            {meter.used}
          </Text>
        </div>
        <LinearProgressIndicator progress={meter.percent / 100} color={meter.level === DeviceLevel.Neutral ? md('primary') : levelColor(meter.level)} label={meter.label} className="h-2" />
        <Text typography="bodySmall" color={meter.level === DeviceLevel.Neutral ? md('onSurfaceVariant') : levelColor(meter.level)}>
          {meter.remaining}
        </Text>
        {meter.note !== '' ? (
          <Text typography="bodySmall" color={md('onSurfaceVariant')}>
            {meter.note}
          </Text>
        ) : null}
      </div>
    ))}
    <div className="grid grid-cols-3 gap-2">
      {gauge.stats.map((stat) => (
        <div key={stat.label} className="flex min-w-0 flex-col">
          <Text typography="labelSmall" color={md('onSurfaceVariant')}>
            {stat.label}
          </Text>
          <Text typography="titleSmall" monospace>
            {stat.value}
          </Text>
          {stat.note !== '' ? (
            <Text typography="bodySmall" color={md('onSurfaceVariant')}>
              {stat.note}
            </Text>
          ) : null}
        </div>
      ))}
    </div>
  </div>
);

export const TailscaleSection = ({ tailscale }: { tailscale: TailscaleCard }) => (
  <DeviceSection title="Tailscale" badge={tailscale.status}>
    <Summary text={tailscale.summary} />
    {tailscale.problem !== null ? <ProblemNotice problem={tailscale.problem} /> : null}
    {tailscale.banner !== null ? (
      <div className="mx-list my-2 flex items-start gap-3 rounded-shape-md bg-surface-container-high px-4 py-3">
        <div className="flex min-w-0 flex-1 flex-col">
          <Text typography="titleSmall">{tailscale.banner.title}</Text>
          <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
            {tailscale.banner.subtitle}
          </Text>
        </div>
        <DeviceBadgeText badge={tailscale.banner.status} />
      </div>
    ) : null}
    {tailscale.facts.map((fact) => (
      <DeviceFact key={fact.label} label={fact.label} value={fact.value} />
    ))}
    {tailscale.addresses.length > 0 ? <DeviceFact label="This device's addresses" value={tailscale.addresses.join(', ')} /> : null}
    {tailscale.signIn !== null ? (
      <div className="flex flex-col gap-2 px-list py-2">
        <div>
          <a
            href={tailscale.signIn.href}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex h-10 items-center gap-2 rounded-full bg-secondary-container px-6 text-label-large text-on-secondary-container hover:brightness-110"
          >
            {tailscale.signIn.label}
            <MaterialIcon name="open_in_new" size={18} />
          </a>
        </div>
        <Text typography="bodySmall" monospace color={md('onSurfaceVariant')} className="break-all">
          {tailscale.signIn.href}
        </Text>
      </div>
    ) : null}
    {tailscale.connect !== '' || tailscale.cancel || tailscale.logOut !== '' ? (
      <Actions>
        {tailscale.connect !== '' ? <ActionButton label={tailscale.connect} request={{ method: 'POST', path: '/api/tailscale/connection' }} /> : null}
        {tailscale.cancel ? <ActionButton label="Cancel sign-in" tonal={false} request={{ method: 'POST', path: '/api/tailscale/connection/cancel' }} success="Sign-in cancelled." /> : null}
        {tailscale.logOut !== '' ? (
          <ActionButton
            label="Log out"
            request={{ method: 'DELETE', path: '/api/tailscale/connection' }}
            success="Logged out of Tailscale."
            confirm={{ title: 'Log out of the tailnet?', text: tailscale.logOut, confirm: 'Log out' }}
          />
        ) : null}
      </Actions>
    ) : null}
    <DeviceText text={tailscale.hint} />
    {tailscale.control !== null ? <ControlServer control={tailscale.control} /> : null}
  </DeviceSection>
);

const ControlServer = ({ control }: { control: TailscaleControl }) => (
  <DeviceDisclosure title={control.title} initiallyOpen={control.open}>
    <DeviceForm
      key={`${control.server}|${control.authKeySet}`}
      method="PUT"
      path="/api/tailscale/control"
      submit="Save"
      success="Control server saved."
      fields={[
        { name: 'controlServer', label: 'Control server URL', value: control.server, placeholder: 'https://headscale.example.com', keyboard: 'uri' },
        { name: 'authKey', label: 'Auth key — device joins automatically, no sign-in', value: '', placeholder: control.authKeyPlaceholder, keyboard: 'password' },
      ]}
    />
    {control.authKeySet ? (
      <Actions>
        <ActionButton
          label="Remove auth key"
          tonal={false}
          request={{ method: 'DELETE', path: '/api/tailscale/control/authkey' }}
          success="Auth key removed."
          confirm={{ title: 'Remove the auth key?', text: 'Remove the saved auth key? Switching will then need an interactive sign-in over the LAN.', confirm: 'Remove' }}
        />
      </Actions>
    ) : null}
    {control.switchError !== '' ? <DeviceNotice notice={{ level: DeviceLevel.Blocked, text: `Last switch failed (rolled back): ${control.switchError}` }} /> : null}
    <DeviceText text={control.pending} />
    {control.switchTo !== null || control.logOutToSwitch !== null ? (
      <Actions>
        {control.switchTo !== null ? (
          <ActionButton
            label={control.switchTo.label}
            request={{ method: 'POST', path: '/api/tailscale/switch' }}
            success="Switching… if it fails it rolls back automatically."
            confirm={{ title: `${control.switchTo.label}?`, text: control.switchTo.confirm, confirm: 'Switch' }}
          />
        ) : null}
        {control.logOutToSwitch !== null ? (
          <ActionButton
            label={control.logOutToSwitch.label}
            request={{ method: 'DELETE', path: '/api/tailscale/connection' }}
            success="Logged out — now connect to finish switching."
            confirm={{ title: 'Log out to switch?', text: control.logOutToSwitch.confirm, confirm: 'Log out' }}
          />
        ) : null}
      </Actions>
    ) : null}
    <DeviceText text={control.hint} />
  </DeviceDisclosure>
);

const RELAY_SWITCH = 'Use my relay instead of Tailscale';

const RelaySection = ({ relay }: { relay: RelayCard }) => (
  <DeviceSection title="Relay server (TURN)" badge={relay.status}>
    <Summary text={relay.summary} />
    {relay.configured ? (
      <DeviceSwitchRow
        title={RELAY_SWITCH}
        checked={relay.on}
        on={{ method: 'PUT', path: '/api/stream/webrtc-mode', body: { relay: 'true' } }}
        off={{ method: 'PUT', path: '/api/stream/webrtc-mode', body: { relay: 'false' } }}
        success={relay.on ? 'Remote video goes over Tailscale again.' : 'Remote video now goes through your relay.'}
      />
    ) : (
      <div className="flex items-center gap-4 px-list py-3">
        <div className="flex min-w-0 flex-1 flex-col">
          <Text typography="bodyLarge">{RELAY_SWITCH}</Text>
          <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
            {relay.reason}
          </Text>
        </div>
        <Switch checked={false} enabled={false} label={RELAY_SWITCH} onCheckedChange={null} />
      </div>
    )}
    <DeviceNotice notice={relay.notice} />
    <IceEditor key={JSON.stringify(relay.servers)} servers={relay.servers} />
    <DeviceDisclosure title="Technical details">
      <DeviceText text="Off (default) advertises the tailscale 100.x and LAN host candidates. On stops advertising the tailscale host candidate and routes WebRTC media through the configured STUN/TURN ICE servers over UDP, so symmetric-CGNAT sessions on cellular relay over UDP instead of falling back to Tailscale DERP (TCP). Same-LAN viewers still use the direct LAN candidate." />
    </DeviceDisclosure>
  </DeviceSection>
);

const IceEditor = ({ servers }: { servers: readonly IceServer[] }) => {
  const [rows, setRows] = useState(() => iceRows(servers));
  const action = useDeviceAction();
  const change = (index: number, next: IceServer) => setRows(rows.map((row, at) => (at === index ? next : row)));
  const save = async () => {
    const servers = await iceServerLines(rows);
    return servers === null ? false : action.run({ method: 'PUT', path: '/api/stream/webrtc-ice-servers', body: { servers } }, 'Servers saved.');
  };
  return (
    <div className="flex flex-col gap-3 px-list py-2">
      <Text typography="bodyLarge">Relay (TURN / STUN) servers</Text>
      {rows.map((row, index) => (
        <div key={index} role="group" aria-label="Relay server" className="flex flex-wrap items-end gap-2 rounded-shape-md bg-surface-container-high p-3">
          <OutlinedTextField
            label="Server URL"
            ariaLabel="Server URL"
            value={row.url}
            placeholder="turn:turn.example.com:443?transport=udp"
            keyboardType="uri"
            autoCorrect={false}
            className="min-w-56 flex-[2]"
            onValueChange={(url) => change(index, { ...row, url })}
          />
          <OutlinedTextField label="Username" ariaLabel="Username" value={row.username} placeholder="username" autoCorrect={false} className="min-w-28 flex-1" onValueChange={(username) => change(index, { ...row, username })} />
          <OutlinedTextField
            label="Password"
            ariaLabel="Password"
            value={row.password}
            placeholder="password"
            keyboardType="password"
            className="min-w-28 flex-1"
            onValueChange={(password) => change(index, { ...row, password })}
          />
          <IconButton label="Remove server" onClick={() => setRows(rows.filter((_, at) => at !== index))}>
            <MaterialIcon name="close" />
          </IconButton>
        </div>
      ))}
      <div className="flex flex-wrap gap-2">
        <TextButton onClick={() => setRows([...rows, emptyIceServer])}>+ Add server</TextButton>
        <TextButton onClick={() => setRows([...rows, { ...emptyIceServer, url: GOOGLE_STUN }])}>+ Add Google STUN</TextButton>
      </div>
      <RefusalText refusal={action.refusal} />
      <div>
        <FilledTonalButton enabled={!action.busy} onClick={() => void save()}>
          Save servers
        </FilledTonalButton>
      </div>
      <Text typography="bodySmall" color={md('onSurfaceVariant')}>
        A TURN server relays video when a direct connection isn't possible — use one on UDP, ideally port 443. STUN-only lines need no username or password.
      </Text>
    </div>
  );
};
