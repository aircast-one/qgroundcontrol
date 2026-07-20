/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#pragma once

#include "UnitTest.h"

class DebugApiServerTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _missingAuthHeaderRejected();
    void _statusEndpoint();
    void _unknownPathReturns404();
    void _handlerErrorReturns400();
    void _motorTestRefusedWithoutActuatorGate();
};
