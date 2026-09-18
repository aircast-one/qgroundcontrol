#pragma once

#include "UnitTest.h"

class VideoTileTest : public UnitTest
{
    Q_OBJECT

protected:
    void init() override;

private slots:
    void _collapsePersistsAcrossReload();
};
