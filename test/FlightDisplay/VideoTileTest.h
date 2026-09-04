#pragma once

#include "UnitTest.h"

class VideoTileTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _tuckPersistsAcrossReload();
    void _extraCameraTileAttachedToPip();
    void _gridPersistsAcrossReload();
    void _focusLayoutOverflowsIntoMore();
    void _statusPillRegistersAsAnObstacleOwnedByThePip();
};
