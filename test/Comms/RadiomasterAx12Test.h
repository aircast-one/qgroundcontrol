#pragma once

#include "UnitTest.h"

class RadiomasterAx12Test : public UnitTest
{
    Q_OBJECT

private slots:
    void _isRemoteMatchesManufacturerIgnoringCase();
    void _defaultsToBuiltInSerialOnlyOnAx12WithElrsPort();
    void _ensureElrsLinkCreatesAutoConnectLinkOnce();
    void _ensureElrsLinkReusesExistingTtyS1Link();
};
