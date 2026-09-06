#pragma once

#include "UnitTest.h"

class TCPLinkErrorTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _refusedConnectionAsksForTheAddress();
    void _missingAddressIsNamed();
    void _newFailureClearsTheOldOne();
};
