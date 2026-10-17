import { useState } from 'react';
import { settingControl, useQgcPath, useQgcString } from '@/bridge/QgcState';
import { optBoolean } from '@/bridge/Json';
import { Text } from '@/m3';
import { AircastSheet } from '@/map/AircastSheet';
import { aircast } from '@/theme/Colors';

const SUPPORT_LINKS_VIEW = 'view.links';
const SUPPORT_HOST_SETTING = settingControl('settings.mavlinkSettings.forwardMavlinkAPMSupportHostName');

export const SupportForwardingCell = () => {
  const links = useQgcPath(SUPPORT_LINKS_VIEW);
  return optBoolean(links, 'supportForwarding') ? <SupportForwardingNotice /> : null;
};

const SupportForwardingNotice = () => {
  const host = useQgcString(SUPPORT_HOST_SETTING);
  const [open, setOpen] = useState(false);
  return (
    <>
      <button type="button" onClick={() => setOpen(true)} className="text-label-medium" style={{ color: aircast('success') }}>
        Support
      </button>
      {open ? (
        <AircastSheet onDismissRequest={() => setOpen(false)}>
          <div className="flex w-full flex-col gap-2 px-sheet pb-6">
            <Text typography="bodyMedium">Mavlink traffic is being forwarded to a support server</Text>
            <Text typography="bodyMedium" className="whitespace-pre-wrap">{`Server name:  ${host}`}</Text>
          </div>
        </AircastSheet>
      ) : null}
    </>
  );
};
