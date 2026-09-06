#pragma once

#include "UnitTest.h"

class PlanViewLayoutTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _narrowWindowStacksTheInspectorUnderTheDock();
    void _wideWindowKeepsTheInspectorOnTheRight();
    void _rotatingBackRestoresTheSideInspector();
};
