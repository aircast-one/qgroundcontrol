#pragma once

#include "UnitTest.h"

class VideoTileTest : public UnitTest
{
    Q_OBJECT

protected:
    void init() override;

private slots:
    void _tuckPersistsAcrossReload();
    void _extraCameraTileAttachedToPip();
    void _gridPersistsAcrossReload();
    void _onlyThePipCameraGetsATile();
    void _statusPillRegistersAsAnObstacleOwnedByThePip();
};
