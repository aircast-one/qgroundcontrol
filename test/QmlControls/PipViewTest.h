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

class PipViewTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _dragRepositionsAndPersists();
    void _dragOffscreenClampsCommittedPosition();
    void _dragBackToDefaultSnapsAndResets();
    void _clickSwapsWithoutDrag();
    void _gripResizesAndPersists();
    void _resizeClampsToViewportFraction();
    void _hoverRevealHoldsSteadyOverTheGrip();
    void _gripResizeOutsideEditModeDoesNotSwap();
    void _forcingItem1FullOverridesTheSavedArrangement();
    void _forcingItem1FullDoesNotDisturbTheSavedArrangement();
    void _swapIsRefusedWhileItem1IsForcedFull();
};
