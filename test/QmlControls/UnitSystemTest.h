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

#include <QtCore/QVariantList>

class UnitSystemTest : public UnitTest
{
    Q_OBJECT

private slots:
    void init();
    void cleanup();

    void _localeDefaultsResolveToAKnownUnitSystem();
    void _settingAUnitSystemAppliesEveryUnit();
    void _oneUnitOffPresetReadsAsCustom();
    void _customSurvivesValuesThatMatchAPreset();
    void _outOfRangeSystemChangesNothing();

private:
    QVariantList _savedUnits;
};
