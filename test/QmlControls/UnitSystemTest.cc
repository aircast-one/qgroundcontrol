#include "UnitSystemTest.h"
#include "SettingsManager.h"
#include "UnitsSettings.h"

#include <QtTest/QTest>

namespace {

UnitsSettings *units()
{
    return SettingsManager::instance()->unitsSettings();
}

QList<Fact*> systemFacts()
{
    return { units()->horizontalDistanceUnits(), units()->verticalDistanceUnits(), units()->areaUnits(),
             units()->speedUnits(), units()->temperatureUnits(), units()->customUnits() };
}

}

void UnitSystemTest::init()
{
    const QList<Fact*> facts = systemFacts();
    _savedUnits.clear();
    for (Fact *const fact : facts) {
        _savedUnits.append(fact->rawValue());
    }
}

void UnitSystemTest::cleanup()
{
    const QList<Fact*> facts = systemFacts();
    for (int i = 0; i < facts.count(); ++i) {
        facts.at(i)->setRawValue(_savedUnits.at(i));
    }
}

void UnitSystemTest::_localeDefaultsResolveToAKnownUnitSystem()
{
    QVERIFY(units()->unitSystem() != UnitsSettings::UnitSystemCustom);
}

void UnitSystemTest::_settingAUnitSystemAppliesEveryUnit()
{
    units()->setUnitSystem(UnitsSettings::UnitSystemMetric);
    QCOMPARE(units()->unitSystem(), static_cast<int>(UnitsSettings::UnitSystemMetric));
    QCOMPARE(units()->horizontalDistanceUnits()->rawValue().toUInt(),
             static_cast<uint>(UnitsSettings::HorizontalDistanceUnitsMeters));
    QCOMPARE(units()->temperatureUnits()->rawValue().toUInt(),
             static_cast<uint>(UnitsSettings::TemperatureUnitsCelsius));

    units()->setUnitSystem(UnitsSettings::UnitSystemImperial);
    QCOMPARE(units()->unitSystem(), static_cast<int>(UnitsSettings::UnitSystemImperial));
    QCOMPARE(units()->areaUnits()->rawValue().toUInt(),
             static_cast<uint>(UnitsSettings::AreaUnitsSquareMiles));
    QCOMPARE(units()->speedUnits()->rawValue().toUInt(),
             static_cast<uint>(UnitsSettings::SpeedUnitsMilesPerHour));
}

void UnitSystemTest::_oneUnitOffPresetReadsAsCustom()
{
    units()->setUnitSystem(UnitsSettings::UnitSystemMetric);
    units()->areaUnits()->setRawValue(UnitsSettings::AreaUnitsAcres);

    QCOMPARE(units()->unitSystem(), static_cast<int>(UnitsSettings::UnitSystemCustom));
}

void UnitSystemTest::_customSurvivesValuesThatMatchAPreset()
{
    units()->setUnitSystem(UnitsSettings::UnitSystemMetric);
    units()->setUnitSystem(UnitsSettings::UnitSystemCustom);

    QCOMPARE(units()->unitSystem(), static_cast<int>(UnitsSettings::UnitSystemCustom));
    QCOMPARE(units()->horizontalDistanceUnits()->rawValue().toUInt(),
             static_cast<uint>(UnitsSettings::HorizontalDistanceUnitsMeters));
}

void UnitSystemTest::_outOfRangeSystemChangesNothing()
{
    units()->setUnitSystem(UnitsSettings::UnitSystemImperial);
    const QVariant before = units()->areaUnits()->rawValue();

    units()->setUnitSystem(99);

    QCOMPARE(units()->areaUnits()->rawValue(), before);
}
