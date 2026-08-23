#pragma once

#include "UnitTest.h"

class PacketRadioTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _videoSettingsRoundTripTest();
    void _keyPathTest();
    void _selectAdapterTest();
};
