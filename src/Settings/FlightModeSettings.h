#pragma once

#include <QtQmlIntegration/QtQmlIntegration>

#include "SettingsGroup.h"

class FlightModeSettings : public SettingsGroup
{
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("")
public:
    FlightModeSettings(QObject* parent = nullptr);

    DEFINE_SETTING_NAME_GROUP()
    DEFINE_SETTINGFACT(px4HiddenFlightModesMultiRotor)
    DEFINE_SETTINGFACT(px4HiddenFlightModesFixedWing)
    DEFINE_SETTINGFACT(px4HiddenFlightModesVTOL)
    DEFINE_SETTINGFACT(px4HiddenFlightModesRoverBoat)
    DEFINE_SETTINGFACT(px4HiddenFlightModesSub)
    DEFINE_SETTINGFACT(px4HiddenFlightModesAirship)
    DEFINE_SETTINGFACT(apmHiddenFlightModesMultiRotor)
    DEFINE_SETTINGFACT(apmHiddenFlightModesFixedWing)
    DEFINE_SETTINGFACT(apmHiddenFlightModesVTOL)
    DEFINE_SETTINGFACT(apmHiddenFlightModesRoverBoat)
    DEFINE_SETTINGFACT(apmHiddenFlightModesSub)
    DEFINE_SETTINGFACT(apmHiddenFlightModesAirship)
    DEFINE_SETTINGFACT(px4PinnedFlightModesMultiRotor)
    DEFINE_SETTINGFACT(px4PinnedFlightModesFixedWing)
    DEFINE_SETTINGFACT(px4PinnedFlightModesVTOL)
    DEFINE_SETTINGFACT(px4PinnedFlightModesRoverBoat)
    DEFINE_SETTINGFACT(px4PinnedFlightModesSub)
    DEFINE_SETTINGFACT(px4PinnedFlightModesAirship)
    DEFINE_SETTINGFACT(apmPinnedFlightModesMultiRotor)
    DEFINE_SETTINGFACT(apmPinnedFlightModesFixedWing)
    DEFINE_SETTINGFACT(apmPinnedFlightModesVTOL)
    DEFINE_SETTINGFACT(apmPinnedFlightModesRoverBoat)
    DEFINE_SETTINGFACT(apmPinnedFlightModesSub)
    DEFINE_SETTINGFACT(apmPinnedFlightModesAirship)
    DEFINE_SETTINGFACT(requireModeChangeConfirmation)
};
