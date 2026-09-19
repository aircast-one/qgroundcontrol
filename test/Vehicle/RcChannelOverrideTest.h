/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#pragma once

#include "BaseClasses/VehicleTestManualConnect.h"

class RcChannelOverrideTest : public VehicleTestManualConnect
{
    Q_OBJECT

protected:
    void init() override;

private slots:
    void _overrideHoldsOneChannelAndReleasesTheRest();
    void _clearHandsTheChannelBackToTheTransmitter();
    void _releaseIsRepeatedThenStops();
    void _outOfRangeChannelIsRejected();
    void _pwmIsClampedToTheRcRange();
    void _releasingWithNothingHeldSendsNothing();
    void _grabbingAgainDuringReleaseCancelsIt();
};
