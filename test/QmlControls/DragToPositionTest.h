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
    void _resizeHandleGrowsUpAndPinsBottom();
};
