#pragma once

#include "UnitTest.h"

class LinkDuplicateConnectTest : public UnitTest
{
    Q_OBJECT

protected:
    void init() override;

private slots:
    void _connectingTwiceReusesTheSameLink();
    void _oneDisconnectClosesTheConfiguration();
    void _disconnectingADynamicLinkRemovesItsConfiguration();
    void _createAndConnectLinkRefusesADuplicateName();
    void _createAndConnectLinkConnectsAndRegisters();
};
