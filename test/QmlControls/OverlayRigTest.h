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
    void _pushedItemParksOnTheNeighbourGutter();
    void _displacedItemStaysAgainstWhatPushedIt();
    void _itemSpringsBackWhenTheObstructionLeaves();
    void _itemFollowsAMovedHomeWhenTheObstructionLeaves();
    void _crowdedItemsAllFindSeparatePlaces();
    void _droppedItemNeverSnapsBackToWhereItCameFrom();
    void _dropAlignsWithANeighboursEdge();
    void _droppedItemTakesItsSlotAndTheNeighbourYields();
    void _gapReadoutMarksTheGutterBesideANeighbour();
    void _overlapIsAnsweredOnTheChangeThatCausedIt();
    void _disturbedLayoutComesToRest();
    void _sweepingObstacleDoesNotFlingItemsAcrossTheWindow();
    void _furnitureBoltedToAMovableDoesNotFeedItself();
    void _anEdgeCarriesWhatItHitsInItsOwnDirection();
    void _releasedItemStaysWhereItWasDropped();
    void cleanupTestCase();
};
