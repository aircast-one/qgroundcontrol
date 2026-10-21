#pragma once

#include <QtQmlIntegration/QtQmlIntegration>

#include "SettingsGroup.h"

class FlyViewSettings : public SettingsGroup
{
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("")
public:
    FlyViewSettings(QObject* parent = nullptr);

    DEFINE_SETTING_NAME_GROUP()

    DEFINE_SETTINGFACT(guidedMinimumAltitude)
    DEFINE_SETTINGFACT(guidedMaximumAltitude)
    DEFINE_SETTINGFACT(showLogReplayStatusBar)
    DEFINE_SETTINGFACT(showAdditionalIndicatorsCompass)
    DEFINE_SETTINGFACT(lockNoseUpCompass)
    DEFINE_SETTINGFACT(forwardFlightGoToLocationLoiterRad)
    DEFINE_SETTINGFACT(goToLocationRequiresConfirmInGuided)
    DEFINE_SETTINGFACT(keepMapCenteredOnVehicle)
    DEFINE_SETTINGFACT(showSimpleCameraControl)
    DEFINE_SETTINGFACT(showPhotoVideoControl)
    DEFINE_SETTINGFACT(showObstacleDistanceOverlay)
    DEFINE_SETTINGFACT(updateHomePosition)
    DEFINE_SETTINGFACT(instrumentQmlFile2)
    DEFINE_SETTINGFACT(requestControlAllowTakeover)
    DEFINE_SETTINGFACT(requestControlTimeout)
    DEFINE_SETTINGFACT(gimbalTiltChannel)
    DEFINE_SETTINGFACT(gimbalPanChannel)
    DEFINE_SETTINGFACT(cameraZoomChannel)
    DEFINE_SETTINGFACT(cameraLightChannel)
    DEFINE_SETTINGFACT(cameraRecordChannel)
    DEFINE_SETTINGFACT(rcControls)
    DEFINE_SETTINGFACT(enableAutomaticMissionPopups)
    DEFINE_SETTINGFACT(payloadBays)
    DEFINE_SETTINGFACT(payloadRelease)
    DEFINE_SETTINGFACT(payloadBay1Output)
    DEFINE_SETTINGFACT(payloadBay1ReleasePwm)
    DEFINE_SETTINGFACT(payloadBay1HoldPwm)
    DEFINE_SETTINGFACT(payloadBay2Output)
    DEFINE_SETTINGFACT(payloadBay2ReleasePwm)
    DEFINE_SETTINGFACT(payloadBay2HoldPwm)
    DEFINE_SETTINGFACT(payloadBay3Output)
    DEFINE_SETTINGFACT(payloadBay3ReleasePwm)
    DEFINE_SETTINGFACT(payloadBay3HoldPwm)
    DEFINE_SETTINGFACT(payloadBay4Output)
    DEFINE_SETTINGFACT(payloadBay4ReleasePwm)
    DEFINE_SETTINGFACT(payloadBay4HoldPwm)
    DEFINE_SETTINGFACT(payloadBay5Output)
    DEFINE_SETTINGFACT(payloadBay5ReleasePwm)
    DEFINE_SETTINGFACT(payloadBay5HoldPwm)
    DEFINE_SETTINGFACT(payloadBay6Output)
    DEFINE_SETTINGFACT(payloadBay6ReleasePwm)
    DEFINE_SETTINGFACT(payloadBay6HoldPwm)
};
