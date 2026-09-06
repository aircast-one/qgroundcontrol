#pragma once

#include "UnitTest.h"

class ScreenToolsTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _desktopKeepsTheDesktopTiers();
    void _mobileFloorsTheSmallTierAndTheTouchSize();
    void _systemFontScaleMultipliesTheBase();
};
