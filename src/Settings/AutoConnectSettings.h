#pragma once

#include <QtQmlIntegration/QtQmlIntegration>

#include "SettingsGroup.h"

class AutoConnectSettings : public SettingsGroup
{
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("")
public:
    AutoConnectSettings(QObject* parent = nullptr);

    enum NmeaSource {
        NmeaSourceDisabled = 0,
        NmeaSourceUdp,
        NmeaSourceSerial,
    };
    Q_ENUM(NmeaSource)

    DEFINE_SETTING_NAME_GROUP()

    DEFINE_SETTINGFACT(autoConnectUDP)
    DEFINE_SETTINGFACT(autoConnectPixhawk)
    DEFINE_SETTINGFACT(autoConnectSiKRadio)
    DEFINE_SETTINGFACT(autoConnectRTKGPS)
    DEFINE_SETTINGFACT(autoConnectLibrePilot)
    DEFINE_SETTINGFACT(nmeaSource)
    DEFINE_SETTINGFACT(autoConnectNmeaPort)
    DEFINE_SETTINGFACT(autoConnectNmeaBaud)
    DEFINE_SETTINGFACT(udpListenPort)
    DEFINE_SETTINGFACT(udpTargetHostIP)
    DEFINE_SETTINGFACT(udpTargetHostPort)
    DEFINE_SETTINGFACT(nmeaUdpPort)
    DEFINE_SETTINGFACT(usbRadioPort)
    DEFINE_SETTINGFACT(usbRadioStandby)
    DEFINE_SETTINGFACT(usbRadioPitchChannel)
    DEFINE_SETTINGFACT(usbRadioYawChannel)
    DEFINE_SETTINGFACT(usbRadioZoomChannel)
    DEFINE_SETTINGFACT(usbRadioPictureChannel)
    DEFINE_SETTINGFACT(usbRadioPaletteChannel)
    DEFINE_SETTINGFACT(usbRadioDropChannel)
    DEFINE_SETTINGFACT(usbRadioArmChannel)
    DEFINE_SETTINGFACT(usbRadioBayChannel)
};
