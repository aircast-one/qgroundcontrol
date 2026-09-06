#pragma once

#include "UnitTest.h"

class PlatformThemeTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _everyThemeSuppliesValidTonesInBothAppearances();
    void _inkReadsOnEverySurfaceInEveryTheme();
    void _paletteRolesDeriveFromTheHostTones();
};
