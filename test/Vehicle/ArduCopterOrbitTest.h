#pragma once

#include "UnitTest.h"

class ArduCopterOrbitTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _clockwiseNorthOfCenterMovesEast();
    void _counterClockwiseEastOfCenterMovesNorth();
};
