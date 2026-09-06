/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#pragma once

#include "UnitTest.h"

class FlyViewToolBarEditTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _hiddenToolComesBackAsAGhostInEditMode();
    void _narrowBarKeepsEveryToolClearOfTheButtons();
    void _narrowBarDropsTheIndicatorRow();
    void _editModeSwallowsTapsOnTheStatusPill();
};
