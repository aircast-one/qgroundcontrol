#pragma once

#include "UnitTest.h"

class AppUpdateOfferTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _theLatestStableReleaseOffersTheInstallerForThisMachine_test();
    void _onlyANewerStableVersionIsAnOffer_test();
};
