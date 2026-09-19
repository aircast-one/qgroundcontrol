#pragma once

#include "UnitTest.h"

class TCPLinkErrorTest : public UnitTest
{
    Q_OBJECT

protected:
    void init() override;

private slots:
    void _refusedConnectionAsksForTheAddress();
    void _missingAddressIsNamed();
    void _newFailureClearsTheOldOne();
};
