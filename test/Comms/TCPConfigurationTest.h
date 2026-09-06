#pragma once

#include "UnitTest.h"

class TCPConfigurationTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _hostnameIsKeptVerbatim();
    void _copyFromCarriesHostAndPort();
};
