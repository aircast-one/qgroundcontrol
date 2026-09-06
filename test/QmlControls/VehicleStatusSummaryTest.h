/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#pragma once

#include "UnitTest.h"

class VehicleStatusSummaryTest : public UnitTest
{
    Q_OBJECT

public:
    VehicleStatusSummaryTest() = default;

private slots:
    void _faultsAndDisabledSensorsAreCountedApart();
    void _healthChecksDecideTheStatusWhenSupported();
    void _noVehicleDataReadsAsNominal();
};
