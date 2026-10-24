#pragma once

#include "UnitTest.h"

class RouteRecorderTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _climbAndDescentAtTheEndsAreTrimmed();
    void _hoveringRecordsNoRoute();
    void _savedRouteLoadsBackWithAbsoluteAltitudes();
    void _renameKeepsTheExtensionAndRefusesToOverwrite();
};
