#pragma once

#include "BaseClasses/VehicleTest.h"

class APMSensorsComponentControllerTest : public VehicleTestAPM
{
    Q_OBJECT

private slots:
    void _setupNeededIsFalseBeforeParametersLoad();
};
