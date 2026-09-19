#pragma once

#include "UnitTest.h"

class LinkStateTest : public UnitTest
{
    Q_OBJECT

protected:
    void init() override;

private slots:
    void _connectingNameFollowsTheLink();
    void _stallFiresWhileNothingAnswersAndClearsOnDisconnect();
    void _newConnectClearsTheFailure();
};
