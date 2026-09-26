#pragma once

#include "UnitTest.h"

class AircastCloudLinkTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _carriesMAVLinkBothWaysWithTheAccountToken();
    void _withoutSigningInItAsksToSignIn();
};
