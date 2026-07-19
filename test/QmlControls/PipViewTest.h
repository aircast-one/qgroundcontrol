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
    void _resizeFromCornerPersists();
};
