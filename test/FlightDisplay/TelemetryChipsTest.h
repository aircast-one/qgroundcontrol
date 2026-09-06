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

class TelemetryChipsTestMainWindow : public QObject
{
    Q_OBJECT

public:
    Q_INVOKABLE void registerWindowDragExclusion(QObject*) {}
};

class TelemetryChipsTest : public UnitTest
{
    Q_OBJECT

private slots:
    void _chipPerValueIsRendered();
    void _rowsKeepTheirPitchOnACoarseSlotGrid();
    void _duplicateFactChipsKeepIndependentPositions();
    void _deleteColumnRemovesTargetedChip();
    void _addedColumnPicksAnUnusedFact();
    void _resetLayoutNeedsTwoTaps();
    void _leavingEditModeDisarmsReset();
    void _labelStyleSurvivesReload();
};
