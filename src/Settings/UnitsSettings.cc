/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "UnitsSettings.h"

#include <algorithm>

#include <QtQml/QQmlEngine>

DECLARE_SETTINGSFACT(UnitsSettings, customUnits)

DECLARE_SETTINGGROUP(Units, "Units")
{
    qmlRegisterUncreatableType<UnitsSettings>("QGroundControl.SettingsManager", 1, 0, "UnitsSettings", "Reference only");
}

void UnitsSettings::_watchUnitSystemFacts()
{
    if (_unitSystemFactsWatched) {
        return;
    }
    _unitSystemFactsWatched = true;

    const QList<Fact*> systemFacts = {
        horizontalDistanceUnits(), verticalDistanceUnits(), areaUnits(),
        speedUnits(), temperatureUnits(), customUnits()
    };
    for (Fact *const fact : systemFacts) {
        (void) connect(fact, &Fact::rawValueChanged, this, &UnitsSettings::unitSystemChanged);
    }
}

DECLARE_SETTINGSFACT_NO_FUNC(UnitsSettings, horizontalDistanceUnits)
{
    if (!_horizontalDistanceUnitsFact) {
        // Distance/Area/Speed units settings can't be loaded from json since it creates an infinite loop of meta data loading.
        QStringList     enumStrings;
        QVariantList    enumValues;
        enumStrings << UnitsSettings::tr("Feet") << UnitsSettings::tr("Meters");
        enumValues << QVariant::fromValue(static_cast<uint32_t>(HorizontalDistanceUnitsFeet))
                   << QVariant::fromValue(static_cast<uint32_t>(HorizontalDistanceUnitsMeters));
        FactMetaData* metaData = new FactMetaData(FactMetaData::valueTypeUint32, this);
        metaData->setName(horizontalDistanceUnitsName);
        metaData->setShortDescription(UnitsSettings::tr("Horizontal Distance"));
        metaData->setEnumInfo(enumStrings, enumValues);

        HorizontalDistanceUnits defaultHorizontalDistanceUnit = HorizontalDistanceUnitsMeters;
        switch(QLocale::system().measurementSystem()) {
            case QLocale::MetricSystem: {
                defaultHorizontalDistanceUnit = HorizontalDistanceUnitsMeters;
            } break;
            case QLocale::ImperialUSSystem:
            case QLocale::ImperialUKSystem:
                defaultHorizontalDistanceUnit = HorizontalDistanceUnitsFeet;
                break;
        }
        metaData->setRawDefaultValue(defaultHorizontalDistanceUnit);
        metaData->setQGCRebootRequired(true);
        _horizontalDistanceUnitsFact = new SettingsFact(_settingsGroup, metaData, this);
    }
    return _horizontalDistanceUnitsFact;
}

DECLARE_SETTINGSFACT_NO_FUNC(UnitsSettings, verticalDistanceUnits)
{
    if (!_verticalDistanceUnitsFact) {
        // Distance/Area/Speed units settings can't be loaded from json since it creates an infinite loop of meta data loading.
        QStringList     enumStrings;
        QVariantList    enumValues;
        enumStrings << UnitsSettings::tr("Feet") << UnitsSettings::tr("Meters");
        enumValues << QVariant::fromValue(static_cast<uint32_t>(VerticalDistanceUnitsFeet))
                   << QVariant::fromValue(static_cast<uint32_t>(VerticalDistanceUnitsMeters));
        FactMetaData* metaData = new FactMetaData(FactMetaData::valueTypeUint32, this);
        metaData->setName(verticalDistanceUnitsName);
        metaData->setShortDescription(UnitsSettings::tr("Vertical Distance"));
        metaData->setEnumInfo(enumStrings, enumValues);
        VerticalDistanceUnits defaultVerticalAltitudeUnit = VerticalDistanceUnitsMeters;
        switch(QLocale::system().measurementSystem()) {
            case QLocale::MetricSystem: {
                defaultVerticalAltitudeUnit = VerticalDistanceUnitsMeters;
            } break;
            case QLocale::ImperialUSSystem:
            case QLocale::ImperialUKSystem:
                defaultVerticalAltitudeUnit = VerticalDistanceUnitsFeet;
                break;
        }
        metaData->setRawDefaultValue(defaultVerticalAltitudeUnit);
        metaData->setQGCRebootRequired(true);
        _verticalDistanceUnitsFact = new SettingsFact(_settingsGroup, metaData, this);
    }
    return _verticalDistanceUnitsFact;
}

DECLARE_SETTINGSFACT_NO_FUNC(UnitsSettings, areaUnits)
{
    if (!_areaUnitsFact) {
        // Distance/Area/Speed units settings can't be loaded from json since it creates an infinite loop of meta data loading.
        QStringList     enumStrings;
        QVariantList    enumValues;
        enumStrings << UnitsSettings::tr("Square Feet") << UnitsSettings::tr("Square Meters") << UnitsSettings::tr("Square Kilometers") << UnitsSettings::tr("Hectares") << UnitsSettings::tr("Acres") << UnitsSettings::tr("Square Miles");
        enumValues <<
            QVariant::fromValue(static_cast<uint32_t>(AreaUnitsSquareFeet)) <<
            QVariant::fromValue(static_cast<uint32_t>(AreaUnitsSquareMeters)) <<
            QVariant::fromValue(static_cast<uint32_t>(AreaUnitsSquareKilometers)) <<
            QVariant::fromValue(static_cast<uint32_t>(AreaUnitsHectares)) <<
            QVariant::fromValue(static_cast<uint32_t>(AreaUnitsAcres)) <<
            QVariant::fromValue(static_cast<uint32_t>(AreaUnitsSquareMiles));
        FactMetaData* metaData = new FactMetaData(FactMetaData::valueTypeUint32, this);
        metaData->setName(areaUnitsName);
        metaData->setShortDescription(UnitsSettings::tr("Area"));
        metaData->setEnumInfo(enumStrings, enumValues);

        AreaUnits defaultAreaUnit = AreaUnitsSquareMeters;
        switch(QLocale::system().measurementSystem()) {
            case QLocale::MetricSystem: {
                defaultAreaUnit = AreaUnitsSquareMeters;
            } break;
            case QLocale::ImperialUSSystem:
            case QLocale::ImperialUKSystem:
                defaultAreaUnit = AreaUnitsSquareMiles;
                break;
        }
        metaData->setRawDefaultValue(defaultAreaUnit);
        metaData->setQGCRebootRequired(true);
        _areaUnitsFact = new SettingsFact(_settingsGroup, metaData, this);
    }
    return _areaUnitsFact;
}

DECLARE_SETTINGSFACT_NO_FUNC(UnitsSettings, speedUnits)
{
    if (!_speedUnitsFact) {
        // Distance/Area/Speed units settings can't be loaded from json since it creates an infinite loop of meta data loading.
        QStringList     enumStrings;
        QVariantList    enumValues;
        enumStrings << UnitsSettings::tr("Feet per Second") << UnitsSettings::tr("Meters per Second") << UnitsSettings::tr("Miles per Hour") << UnitsSettings::tr("Kilometers per Hour") << UnitsSettings::tr("Knots");
        enumValues <<
            QVariant::fromValue(static_cast<uint32_t>(SpeedUnitsFeetPerSecond)) <<
            QVariant::fromValue(static_cast<uint32_t>(SpeedUnitsMetersPerSecond)) <<
            QVariant::fromValue(static_cast<uint32_t>(SpeedUnitsMilesPerHour)) <<
            QVariant::fromValue(static_cast<uint32_t>(SpeedUnitsKilometersPerHour)) <<
            QVariant::fromValue(static_cast<uint32_t>(SpeedUnitsKnots));
        FactMetaData* metaData = new FactMetaData(FactMetaData::valueTypeUint32, this);
        metaData->setName(speedUnitsName);
        metaData->setShortDescription(UnitsSettings::tr("Speed"));
        metaData->setEnumInfo(enumStrings, enumValues);

        SpeedUnits defaultSpeedUnit = SpeedUnitsMetersPerSecond;
        switch(QLocale::system().measurementSystem()) {
            case QLocale::MetricSystem: {
                defaultSpeedUnit = SpeedUnitsMetersPerSecond;
            } break;
            case QLocale::ImperialUSSystem:
            case QLocale::ImperialUKSystem:
                defaultSpeedUnit = SpeedUnitsMilesPerHour;
                break;
        }
        metaData->setRawDefaultValue(defaultSpeedUnit);
        metaData->setQGCRebootRequired(true);
        _speedUnitsFact = new SettingsFact(_settingsGroup, metaData, this);
    }
    return _speedUnitsFact;
}

DECLARE_SETTINGSFACT_NO_FUNC(UnitsSettings, temperatureUnits)
{
    if (!_temperatureUnitsFact) {
        // Units settings can't be loaded from json since it creates an infinite loop of meta data loading.
        QStringList     enumStrings;
        QVariantList    enumValues;
        enumStrings << UnitsSettings::tr("Celsius") << UnitsSettings::tr("Fahrenheit");
        enumValues << QVariant::fromValue(static_cast<uint32_t>(TemperatureUnitsCelsius)) << QVariant::fromValue(static_cast<uint32_t>(TemperatureUnitsFarenheit));
        FactMetaData* metaData = new FactMetaData(FactMetaData::valueTypeUint32, this);
        metaData->setName(temperatureUnitsName);
        metaData->setShortDescription(UnitsSettings::tr("Temperature"));
        metaData->setEnumInfo(enumStrings, enumValues);

        TemperatureUnits defaultTemperatureUnit = TemperatureUnitsCelsius;
        switch(QLocale::system().measurementSystem()) {
            case QLocale::MetricSystem: {
                defaultTemperatureUnit = TemperatureUnitsCelsius;
            } break;
            case QLocale::ImperialUSSystem:
            case QLocale::ImperialUKSystem:
                defaultTemperatureUnit = TemperatureUnitsFarenheit;
                break;
        }
        metaData->setRawDefaultValue(defaultTemperatureUnit);
        metaData->setQGCRebootRequired(true);
        _temperatureUnitsFact = new SettingsFact(_settingsGroup, metaData, this);
    }
    return _temperatureUnitsFact;
}

DECLARE_SETTINGSFACT_NO_FUNC(UnitsSettings, weightUnits)
{
    if (!_weightUnitsFact) {
        // Units settings can't be loaded from json since it creates an infinite loop of meta data loading.
        QStringList     enumStrings;
        QVariantList    enumValues;
        enumStrings << UnitsSettings::tr("Grams") << UnitsSettings::tr("Kilograms") << UnitsSettings::tr("Ounces") << UnitsSettings::tr("Pounds");
        enumValues
            << QVariant::fromValue(static_cast<uint32_t>(WeightUnitsGrams))
            << QVariant::fromValue(static_cast<uint32_t>(WeightUnitsKg))
            << QVariant::fromValue(static_cast<uint32_t>(WeightUnitsOz))
            << QVariant::fromValue(static_cast<uint32_t>(WeightUnitsLbs));
        FactMetaData* metaData = new FactMetaData(FactMetaData::valueTypeUint32, this);
        metaData->setName(weightUnitsName);
        metaData->setShortDescription(UnitsSettings::tr("Weight"));
        metaData->setEnumInfo(enumStrings, enumValues);
        WeightUnits defaultWeightUnit = WeightUnitsGrams;
        switch(QLocale::system().measurementSystem()) {
            case QLocale::MetricSystem:
            case QLocale::ImperialUKSystem: {
                defaultWeightUnit = WeightUnitsGrams;
            } break;
            case QLocale::ImperialUSSystem:
                defaultWeightUnit = WeightUnitsOz;
                break;
        }
        metaData->setRawDefaultValue(defaultWeightUnit);
        metaData->setQGCRebootRequired(true);
        _weightUnitsFact = new SettingsFact(_settingsGroup, metaData, this);
    }
    return _weightUnitsFact;
}

namespace {
    struct UnitSystemPreset {
        UnitsSettings::HorizontalDistanceUnits  horizontal;
        UnitsSettings::VerticalDistanceUnits    vertical;
        UnitsSettings::AreaUnits                area;
        UnitsSettings::SpeedUnits               speed;
        UnitsSettings::TemperatureUnits         temperature;
    };

    constexpr UnitSystemPreset kUnitSystemPresets[] = {
        { UnitsSettings::HorizontalDistanceUnitsMeters, UnitsSettings::VerticalDistanceUnitsMeters,
          UnitsSettings::AreaUnitsSquareMeters, UnitsSettings::SpeedUnitsMetersPerSecond,
          UnitsSettings::TemperatureUnitsCelsius },
        { UnitsSettings::HorizontalDistanceUnitsFeet, UnitsSettings::VerticalDistanceUnitsFeet,
          UnitsSettings::AreaUnitsSquareMiles, UnitsSettings::SpeedUnitsMilesPerHour,
          UnitsSettings::TemperatureUnitsFarenheit }
    };
}

int UnitsSettings::unitSystem()
{
    _watchUnitSystemFacts();

    if (customUnits()->rawValue().toBool()) {
        return UnitSystemCustom;
    }

    const auto matches = [this](const UnitSystemPreset &preset) {
        return (horizontalDistanceUnits()->rawValue().toUInt() == preset.horizontal) &&
               (verticalDistanceUnits()->rawValue().toUInt() == preset.vertical) &&
               (areaUnits()->rawValue().toUInt() == preset.area) &&
               (speedUnits()->rawValue().toUInt() == preset.speed) &&
               (temperatureUnits()->rawValue().toUInt() == preset.temperature);
    };

    const auto *const found = std::find_if(std::begin(kUnitSystemPresets), std::end(kUnitSystemPresets), matches);
    if (found == std::end(kUnitSystemPresets)) {
        return UnitSystemCustom;
    }
    return static_cast<int>(std::distance(std::begin(kUnitSystemPresets), found));
}

void UnitsSettings::setUnitSystem(int system)
{
    _watchUnitSystemFacts();

    customUnits()->setRawValue(system == UnitSystemCustom);

    if ((system < 0) || (system >= static_cast<int>(std::size(kUnitSystemPresets)))) {
        return;
    }

    const UnitSystemPreset &preset = kUnitSystemPresets[system];
    horizontalDistanceUnits()->setRawValue(preset.horizontal);
    verticalDistanceUnits()->setRawValue(preset.vertical);
    areaUnits()->setRawValue(preset.area);
    speedUnits()->setRawValue(preset.speed);
    temperatureUnits()->setRawValue(preset.temperature);
}
