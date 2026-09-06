/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#pragma once

#include "UnitTest.h"

class UDPConfigurationTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _newConfigurationListensOnTheStandardMavlinkPort();
    void _newConfigurationFollowsTheConfiguredListenPort();
    void _hostIsParsedFromAnAddressPortString();
    void _hostWithoutPortFallsBackToTheLocalPort();
    void _copyFromCarriesPortAndHosts();
    void _typedServerAddressIsCommittedOnSave();
    void _typedPortIsCommittedOnSave();
    void _outOfRangePortFallsBackToTheDefault();
};
