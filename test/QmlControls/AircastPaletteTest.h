#pragma once

#include "UnitTest.h"

class AircastPaletteTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _paletteRolesAreTheTokenRoles();
    void _textReadsOnEverySurfaceInBothThemes();
};
