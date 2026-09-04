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

class DragToPositionTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _dragRepositionsAndPersists();
    void _dragOffscreenClampsCommittedPosition();
    void _dragBackToDefaultSnapsAndResets();
    void _clickStillReachesChild();
    void _comboBoxInPanelOpensPopup();
};
