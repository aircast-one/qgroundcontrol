/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "QGCBridgeCoreTest.h"
#include "QGCBridgeCore.h"
#include "MultiVehicleManager.h"

#include "Fact.h"
#include "SettingsManager.h"
#include "UnitsSettings.h"

#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtTest/QTest>

namespace
{

constexpr const char *kSpeedUnits = "settings.unitsSettings.speedUnits";
constexpr const char *kUnitsGroup = "settings.unitsSettings";
constexpr const char *kVehicleFactGroup = "vehicles.offlineEditingVehicle.vehicle";

QJsonObject parse(const QString &json)
{
    return QJsonDocument::fromJson(json.toUtf8()).object();
}

QJsonObject readObject(const QString &path)
{
    return parse(QGCBridgeCore::get(path));
}

QJsonObject writeValue(const QString &path, const QJsonValue &value)
{
    const QJsonObject wrapper { { QStringLiteral("value"), value } };
    return parse(QGCBridgeCore::set(path, QString::fromUtf8(QJsonDocument(wrapper).toJson(QJsonDocument::Compact))));
}

QJsonObject callMethod(const QString &path, const QJsonArray &args = QJsonArray())
{
    return parse(QGCBridgeCore::invoke(path, QString::fromUtf8(QJsonDocument(args).toJson(QJsonDocument::Compact))));
}

} // namespace

void QGCBridgeCoreTest::init()
{
    UnitTest::init();
}

void QGCBridgeCoreTest::cleanup()
{
    QGCBridgeCore::watch(QStringList());
    QGCBridgeCore::setEventHandler(nullptr);
    UnitTest::cleanup();
}

void QGCBridgeCoreTest::_readsScalarProperty()
{
    const QJsonObject json = readObject(QStringLiteral("vehicles.activeVehicleAvailable"));

    QCOMPARE(json.value(QStringLiteral("kind")).toString(), QStringLiteral("value"));
    QVERIFY(json.contains(QStringLiteral("value")));
    QCOMPARE(json.value(QStringLiteral("value")).toBool(), false);
}

void QGCBridgeCoreTest::_readsFactWithMetadata()
{
    const QJsonObject json = readObject(QString::fromLatin1(kSpeedUnits));

    QCOMPARE(json.value(QStringLiteral("kind")).toString(), QStringLiteral("fact"));
    QCOMPARE(json.value(QStringLiteral("name")).toString(), QStringLiteral("speedUnits"));
    QVERIFY(!json.value(QStringLiteral("enumStrings")).toArray().isEmpty());
    QVERIFY(json.contains(QStringLiteral("valueString")));
    QVERIFY(json.contains(QStringLiteral("typeIsBool")));
}

void QGCBridgeCoreTest::_readsFactsOfSettingsGroup()
{
    const QJsonObject json = readObject(QString::fromLatin1(kUnitsGroup));
    const QJsonArray facts = json.value(QStringLiteral("facts")).toArray();

    QCOMPARE(json.value(QStringLiteral("kind")).toString(), QStringLiteral("object"));
    QVERIFY(!facts.isEmpty());

    bool foundSpeedUnits = false;
    for (const QJsonValue &fact : facts) {
        if (fact.toObject().value(QStringLiteral("name")).toString() == QStringLiteral("speedUnits")) {
            foundSpeedUnits = true;
            break;
        }
    }
    QVERIFY(foundSpeedUnits);
}

void QGCBridgeCoreTest::_writesFactValue()
{
    Fact *const fact = SettingsManager::instance()->unitsSettings()->speedUnits();
    const QVariant original = fact->cookedValue();

    const QJsonObject result = writeValue(QString::fromLatin1(kSpeedUnits), 1);

    QCOMPARE(result.value(QStringLiteral("ok")).toBool(), true);
    QCOMPARE(fact->cookedValue().toInt(), 1);

    fact->setCookedValue(original);
}

void QGCBridgeCoreTest::_writesEnumIndex()
{
    Fact *const fact = SettingsManager::instance()->unitsSettings()->speedUnits();
    const QVariant original = fact->cookedValue();

    const QJsonObject result = writeValue(QString::fromLatin1(kSpeedUnits) + QStringLiteral(".enumIndex"), 2);

    QCOMPARE(result.value(QStringLiteral("ok")).toBool(), true);
    QCOMPARE(fact->enumIndex(), 2);

    fact->setCookedValue(original);
}

void QGCBridgeCoreTest::_resolvesListIndex()
{
    const QJsonObject beyondEnd = readObject(QStringLiteral("links.linkConfigurations.9999"));
    QCOMPARE(beyondEnd.value(QStringLiteral("kind")).toString(), QStringLiteral("value"));

    const QJsonObject list = readObject(QStringLiteral("links.linkConfigurations"));
    const QJsonArray elements = list.value(QStringLiteral("elements")).toArray();
    if (elements.isEmpty()) {
        QSKIP("No link configurations present to index positively");
    }

    const QJsonObject first = readObject(QStringLiteral("links.linkConfigurations.0"));
    QCOMPARE(first.value(QStringLiteral("kind")).toString(), QStringLiteral("object"));
    QCOMPARE(first.value(QStringLiteral("name")), elements.at(0).toObject().value(QStringLiteral("name")));
}

void QGCBridgeCoreTest::_resolvesAccessorCall()
{
    const QJsonObject json = readObject(QString::fromLatin1(kVehicleFactGroup) + QStringLiteral(".getFact(heading)"));

    QCOMPARE(json.value(QStringLiteral("kind")).toString(), QStringLiteral("fact"));
    QCOMPARE(json.value(QStringLiteral("name")).toString(), QStringLiteral("heading"));

    const QJsonObject missing = readObject(QString::fromLatin1(kVehicleFactGroup) + QStringLiteral(".getFact(no_such_fact)"));
    QVERIFY(missing.value(QStringLiteral("kind")).toString() != QStringLiteral("fact"));
    QVERIFY(missing.value(QStringLiteral("value")).isNull());
}

void QGCBridgeCoreTest::_writesThroughAccessorCall()
{
    const QString path = QString::fromLatin1(kVehicleFactGroup) + QStringLiteral(".getFact(heading)");

    const QJsonObject result = writeValue(path, 42);
    QCOMPARE(result.value(QStringLiteral("ok")).toBool(), true);
    QCOMPARE(readObject(path).value(QStringLiteral("value")).toDouble(), 42.0);
}

void QGCBridgeCoreTest::_invokeReturnsValue()
{
    const QJsonObject result = callMethod(QString::fromLatin1(kVehicleFactGroup) + QStringLiteral(".factExists"),
                                          QJsonArray { QStringLiteral("heading") });

    QCOMPARE(result.value(QStringLiteral("ok")).toBool(), true);
    QCOMPARE(result.value(QStringLiteral("result")).toBool(), true);
}

void QGCBridgeCoreTest::_invokeConvertsArguments()
{
    const QString path = QString::fromLatin1(kVehicleFactGroup) + QStringLiteral(".factExists");

    const QJsonObject missing = callMethod(path, QJsonArray { QStringLiteral("no_such_fact_name") });
    QCOMPARE(missing.value(QStringLiteral("ok")).toBool(), true);
    QCOMPARE(missing.value(QStringLiteral("result")).toBool(), false);

    const QJsonObject wrongArity = callMethod(path);
    QCOMPARE(wrongArity.value(QStringLiteral("ok")).toBool(), false);

    const QJsonObject numericArg = callMethod(path, QJsonArray { 7 });
    QCOMPARE(numericArg.value(QStringLiteral("ok")).toBool(), true);
    QCOMPARE(numericArg.value(QStringLiteral("result")).toBool(), false);
}

void QGCBridgeCoreTest::_invokeReturnsAFactObject()
{
    const QJsonObject result = callMethod(
        QStringLiteral("vehicles.offlineEditingVehicle.vehicle.getFact"),
        QJsonArray { QStringLiteral("heading") });
    QVERIFY(result.value(QStringLiteral("ok")).toBool());

    const QJsonObject fact = result.value(QStringLiteral("result")).toObject();
    QCOMPARE(fact.value(QStringLiteral("kind")).toString(), QStringLiteral("fact"));
    QCOMPARE(fact.value(QStringLiteral("name")).toString(), QStringLiteral("heading"));
}

void QGCBridgeCoreTest::_invokeRejectsUnresolvableObjectReference()
{
    const QJsonObject result = callMethod(
        QStringLiteral("vehicles.offlineEditingVehicle.vehicle.factExists"),
        QJsonArray { QStringLiteral("@no.such.object") });
    QVERIFY(!result.value(QStringLiteral("ok")).toBool());
}

void QGCBridgeCoreTest::_invokeRejectsTooManyArguments()
{
    const QJsonObject result = callMethod(
        QStringLiteral("vehicles.offlineEditingVehicle.vehicle.getFact"),
        QJsonArray { 1, 2, 3, 4, 5 });
    QVERIFY(!result.value(QStringLiteral("ok")).toBool());
}

void QGCBridgeCoreTest::_accessorCallNeedsAQObjectReturn()
{
    const QJsonObject json = readObject(
        QStringLiteral("vehicles.offlineEditingVehicle.vehicle.factExists(heading).anything"));
    QCOMPARE(json.value(QStringLiteral("kind")).toString(), QStringLiteral("value"));
    QVERIFY(json.value(QStringLiteral("value")).isNull());
}

void QGCBridgeCoreTest::_vehicleRootIsNullWithoutAnActiveVehicle()
{
    QVERIFY(!MultiVehicleManager::instance()->activeVehicle());
    QCOMPARE(readObject(QStringLiteral("vehicle.armed")).value(QStringLiteral("kind")).toString(),
             QStringLiteral("null"));
}

void QGCBridgeCoreTest::_setRejectsAPayloadWithoutAValue()
{
    Fact *const fact = SettingsManager::instance()->unitsSettings()->speedUnits();
    const QVariant before = fact->cookedValue();

    QJsonObject json = parse(QGCBridgeCore::set(
        QString::fromLatin1(kSpeedUnits), QStringLiteral("{\"notValue\":1}")));
    QVERIFY(!json.value(QStringLiteral("ok")).toBool());
    QCOMPARE(fact->cookedValue(), before);

    json = parse(QGCBridgeCore::set(QString::fromLatin1(kSpeedUnits), QStringLiteral("not json")));
    QVERIFY(!json.value(QStringLiteral("ok")).toBool());
    QCOMPARE(fact->cookedValue(), before);
}

void QGCBridgeCoreTest::_resolvesMavlinkInspectorRoot()
{
    const QJsonObject systems = readObject(QStringLiteral("mavlinkInspector.systems"));
    QCOMPARE(systems.value(QStringLiteral("kind")).toString(), QStringLiteral("object"));
    QVERIFY(systems.contains(QStringLiteral("elements")));
    QCOMPARE(systems.value(QStringLiteral("count")).toInt(),
             systems.value(QStringLiteral("elements")).toArray().count());

    const QJsonObject names = readObject(QStringLiteral("mavlinkInspector.systemNames"));
    QCOMPARE(names.value(QStringLiteral("kind")).toString(), QStringLiteral("value"));
    QVERIFY(names.value(QStringLiteral("value")).isArray());
}

void QGCBridgeCoreTest::_resolvesMavlinkConsoleRoot()
{
    const QJsonObject lines = readObject(QStringLiteral("mavlinkConsole.lines"));
    QCOMPARE(lines.value(QStringLiteral("kind")).toString(), QStringLiteral("value"));
    QVERIFY(lines.value(QStringLiteral("value")).isArray());

    const QJsonObject sent = callMethod(
        QStringLiteral("mavlinkConsole.sendCommand"),
        QJsonArray { QStringLiteral("help") });
    QVERIFY(sent.value(QStringLiteral("ok")).toBool());

    const QJsonObject recalled = callMethod(
        QStringLiteral("mavlinkConsole.historyUp"),
        QJsonArray { QString() });
    QVERIFY(recalled.value(QStringLiteral("ok")).toBool());
    QCOMPARE(recalled.value(QStringLiteral("result")).toString(), QStringLiteral("help"));
}

void QGCBridgeCoreTest::_resolvesLogDownloadRoot()
{
    const QJsonObject root = readObject(QStringLiteral("logDownload"));
    QCOMPARE(root.value(QStringLiteral("kind")).toString(), QStringLiteral("object"));
    QVERIFY(root.contains(QStringLiteral("requestingList")));
    QVERIFY(root.contains(QStringLiteral("downloadingLogs")));
    QVERIFY(root.value(QStringLiteral("children")).toArray().contains(QStringLiteral("model")));

    const QJsonObject model = readObject(QStringLiteral("logDownload.model"));
    QCOMPARE(model.value(QStringLiteral("kind")).toString(), QStringLiteral("object"));
    QVERIFY(model.contains(QStringLiteral("elements")));
    QCOMPARE(model.value(QStringLiteral("count")).toInt(), model.value(QStringLiteral("elements")).toArray().count());
}

void QGCBridgeCoreTest::_rejectsUnknownPaths()
{
    QCOMPARE(readObject(QStringLiteral("nosuchroot.thing")).value(QStringLiteral("kind")).toString(),
             QStringLiteral("null"));
    QCOMPARE(readObject(QString()).value(QStringLiteral("kind")).toString(), QStringLiteral("null"));
    QCOMPARE(writeValue(QStringLiteral("nosuchroot.thing"), 1).value(QStringLiteral("ok")).toBool(), false);
    QCOMPARE(callMethod(QStringLiteral("nosuchroot.method")).value(QStringLiteral("ok")).toBool(), false);
    QCOMPARE(writeValue(QString::fromLatin1(kUnitsGroup), 1).value(QStringLiteral("ok")).toBool(), false);
}

void QGCBridgeCoreTest::_watchEmitsOnChange()
{
    QStringList paths;
    QStringList payloads;

    QGCBridgeCore::setEventHandler([&paths, &payloads](const QString &path, const QString &json) {
        paths.append(path);
        payloads.append(json);
    });

    QGCBridgeCore::watch(QStringList { QString::fromLatin1(kSpeedUnits) });
    QTRY_VERIFY_WITH_TIMEOUT(!paths.isEmpty(), 3000);

    const int initialCount = paths.count();
    QCOMPARE(paths.first(), QString::fromLatin1(kSpeedUnits));
    QVERIFY(parse(payloads.first()).value(QStringLiteral("kind")).toString() == QStringLiteral("fact"));

    Fact *const fact = SettingsManager::instance()->unitsSettings()->speedUnits();
    const QVariant original = fact->cookedValue();
    fact->setCookedValue(original.toInt() == 0 ? 1 : 0);

    QTRY_VERIFY_WITH_TIMEOUT(paths.count() > initialCount, 3000);

    fact->setCookedValue(original);
}
