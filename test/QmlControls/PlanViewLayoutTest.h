#pragma once

#include "UnitTest.h"

class PlanViewLayoutTest : public UnitTest
{
    Q_OBJECT

protected:
    void init() override;

private slots:
    void _narrowWindowStacksTheInspectorUnderTheDock();
    void _narrowWindowGivesTheTerrainProfileItsOwnBand();
    void _wideWindowKeepsTheInspectorOnTheRight();
    void _rotatingBackRestoresTheSideInspector();
};
