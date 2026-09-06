#pragma once

#include "UnitTest.h"

class LinkStateTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _connectingNameFollowsTheLink();
    void _stallFiresWhileNothingAnswersAndClearsOnDisconnect();
    void _newConnectClearsTheFailure();
};
