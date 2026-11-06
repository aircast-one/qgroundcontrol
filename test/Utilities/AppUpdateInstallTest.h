#pragma once

#include "UnitTest.h"

class AppUpdateInstallTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _theChecksumListingGivesTheSha256_test();
    void _eachPlatformInstallsItsOwnWay_test();
    void _aBundleIsReplacedOnlyWhereTheAppCanWrite_test();
};
