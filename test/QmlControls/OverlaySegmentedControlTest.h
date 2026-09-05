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

class OverlaySegmentedControlTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _tapReportsTheSegmentTapped();
    void _tapLeavesTheOwnersBindingIntact();
    void _disabledSegmentReportsNothing();
};
