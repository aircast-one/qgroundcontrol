#pragma once

#include "UnitTest.h"

class FlightMapTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _trackpadOrMagicMouseScrollPans();
    void _momentumKeepsPanningAfterTheStopSignal();
    void _scrollWhileAButtonIsHeldIsIgnored();
    void _touchpadWithoutPixelDeltaStillPans();
    void _mouseWheelZoomsAboutTheCursor();
    void _modifierScrollZooms();
    void _pinchFollowsTheFingersWhilePanning();
    void _rightClickSignalsApartFromLeftClick();
    void _mouseDragFlicksWithInertia();
};
