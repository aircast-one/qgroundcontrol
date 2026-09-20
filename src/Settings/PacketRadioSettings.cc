#include "PacketRadioSettings.h"


DECLARE_SETTINGGROUP(PacketRadio, "PacketRadio")
{
#ifndef QGC_HEADLESS_CORE
    qmlRegisterUncreatableType<PacketRadioSettings>("QGroundControl.SettingsManager", 1, 0, "PacketRadioSettings", "Reference only");
#endif
}

DECLARE_SETTINGSFACT(PacketRadioSettings, enabled)
DECLARE_SETTINGSFACT(PacketRadioSettings, channel)
DECLARE_SETTINGSFACT(PacketRadioSettings, channelWidth)
DECLARE_SETTINGSFACT(PacketRadioSettings, keyFile)
DECLARE_SETTINGSFACT(PacketRadioSettings, deviceName)
DECLARE_SETTINGSFACT(PacketRadioSettings, alinkEnabled)
DECLARE_SETTINGSFACT(PacketRadioSettings, alinkTxPower)
