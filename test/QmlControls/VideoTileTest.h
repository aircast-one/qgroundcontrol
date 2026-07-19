#pragma once

#include "UnitTest.h"

class VideoTileTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _collapsePersistsAcrossReload();
};
