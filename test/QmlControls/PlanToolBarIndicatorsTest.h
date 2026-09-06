#pragma once

#include "UnitTest.h"

class PlanToolBarIndicatorsTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _narrowRowKeepsEveryControlInside();
    void _wideRowShowsTheFigures();
};
