#pragma once

#include "UnitTest.h"

class TrackpadTouchFilterTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _testDropsTrackpadTouches();
    void _testKeepsTouchscreenTouches();
    void _testKeepsTrackpadScroll();
};
