#pragma once

#include "UnitTest.h"

class ColoredSvgImageProviderTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _tintsAnSvg();
    void _rasterStillDraws();
    void _missingResourceDrawsNothing();
};
