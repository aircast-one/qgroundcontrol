#pragma once

#include "VehicleTest.h"

class VehicleArmVideoTest : public VehicleTest
{
    Q_OBJECT

private slots:
    void _armingTurnsVideoBackOn();
    void _aVehicleArmedBeforeItIsActiveTurnsVideoOn();
    void _anotherVehicleArmingLeavesVideoAlone();
};
