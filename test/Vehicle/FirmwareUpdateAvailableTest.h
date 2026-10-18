#pragma once

#include "BaseClasses/VehicleTestManualConnect.h"

class FirmwareUpdateAvailableTest : public VehicleTestManualConnect
{
    Q_OBJECT

private slots:
    void _theVersionStartsEmptySoNothingClaimsAnUpdate();
    void _settingTheVersionAnnouncesItOnce();
    void _settingTheSameVersionAgainAnnouncesNothing();
    void _clearingTheVersionAnnouncesIt();
};
