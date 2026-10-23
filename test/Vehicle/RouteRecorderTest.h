#pragma once

#include "UnitTest.h"

class RouteRecorderTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _straightLineKeepsOnlyTheEnds();
    void _cornerAndClimbAreKept();
    void _savedRouteLoadsBackWithAbsoluteAltitudes();
};
