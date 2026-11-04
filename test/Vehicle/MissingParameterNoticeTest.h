#pragma once

#include "BaseClasses/VehicleTest.h"

class MissingParameterNoticeTest : public VehicleTest
{
    Q_OBJECT

public:
    explicit MissingParameterNoticeTest(QObject* parent = nullptr) : VehicleTest(parent) { setWaitForParameters(true); }

private slots:
    void _missingParameterIsAVehicleWarningNotADialog();
    void _theSameGroundStationWarningIsLoggedOnce();
};
