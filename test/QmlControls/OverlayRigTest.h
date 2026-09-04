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

class OverlayRigTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _hitTestFindsRegisteredItems();
    void _hitTestMissesEmptySpace();
    void _resizeLeavesArrangementAlone();
    void _eachWindowSizeKeepsItsOwnArrangement();
    void _unseenSizeStartsFromTheNearestArrangement();
    void _resolveLeavesNothingOverlapping();
    void _lightItemYieldsToHeavyOne();
    void _registerMovableDoesNotDuplicate();
    void _ownedStaticPushesOthersButNotOwner();
    void _staticThatMovesPushesMovablesOutOfTheWay();
    void cleanupTestCase();
};
