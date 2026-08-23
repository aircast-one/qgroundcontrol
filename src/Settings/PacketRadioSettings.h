#pragma once

#include "SettingsGroup.h"

class PacketRadioSettings : public SettingsGroup
{
    Q_OBJECT
public:
    PacketRadioSettings(QObject* parent = nullptr);
    DEFINE_SETTING_NAME_GROUP()

    DEFINE_SETTINGFACT(enabled)
    DEFINE_SETTINGFACT(channel)
    DEFINE_SETTINGFACT(channelWidth)
    DEFINE_SETTINGFACT(keyFile)
    DEFINE_SETTINGFACT(deviceName)
    DEFINE_SETTINGFACT(alinkEnabled)
    DEFINE_SETTINGFACT(alinkTxPower)
};
