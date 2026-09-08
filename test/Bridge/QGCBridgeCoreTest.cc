/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "QGCBridgeCoreTest.h"

#include "QGCMapCircle.h"
#include "QGCBridgeCore.h"
#include "MultiVehicleManager.h"

#include "Fact.h"
#include "SettingsManager.h"
#include "UnitsSettings.h"

#include <QtCore/QDir>
#include <QtCore/QElapsedTimer>
#include <QtCore/QFile>
#include <QtCore/QStandardPaths>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QThread>

#include <atomic>
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

void QGCBridgeCoreTest::_radioCalRootIsNullWithoutAnActiveVehicle()
{
    QVERIFY(!MultiVehicleManager::instance()->activeVehicle());
    QCOMPARE(readObject(QStringLiteral("radioCal.channelCount")).value(QStringLiteral("kind")).toString(),
             QStringLiteral("null"));
    QCOMPARE(readObject(QStringLiteral("radioCal")).value(QStringLiteral("kind")).toString(),
             QStringLiteral("null"));
}

void QGCBridgeCoreTest::_sensorsCalRootIsNullWithoutAnApmVehicle()
{
    QVERIFY(!MultiVehicleManager::instance()->activeVehicle());
    QCOMPARE(readObject(QStringLiteral("sensorsCal.calibrationInProgress")).value(QStringLiteral("kind")).toString(),
             QStringLiteral("null"));
    QCOMPARE(readObject(QStringLiteral("sensorsCal")).value(QStringLiteral("kind")).toString(),
             QStringLiteral("null"));
}

void QGCBridgeCoreTest::_indexesAVariantListOfObjects()
{
    const QString pluginPath = QStringLiteral("vehicles.offlineEditingVehicle.autopilotPlugin");
    if (readObject(pluginPath).value(QStringLiteral("kind")).toString() == QStringLiteral("null")) {
        QSKIP("no autopilot plugin on the offline editing vehicle");
    }

    const QString listPath = pluginPath + QStringLiteral(".vehicleComponents");
    const QJsonArray components = readObject(listPath).value(QStringLiteral("value")).toArray();
    if (components.isEmpty()) {
        QSKIP("the offline editing vehicle exposes no vehicle components");
    }

    const QJsonObject first = readObject(listPath + QStringLiteral(".0"));
    QCOMPARE(first.value(QStringLiteral("kind")).toString(), QStringLiteral("object"));

    const QJsonObject name = readObject(listPath + QStringLiteral(".0.name"));
    QCOMPARE(name.value(QStringLiteral("kind")).toString(), QStringLiteral("value"));
    QVERIFY(!name.value(QStringLiteral("value")).toString().isEmpty());

    const QJsonObject past = readObject(listPath + QStringLiteral(".9999.name"));
    QVERIFY(past.value(QStringLiteral("value")).isNull());
}

void QGCBridgeCoreTest::_writeSaysWhyItFailed()
{
    const QJsonObject unknownPath = parse(QGCBridgeCore::set(
        QStringLiteral("settings.nope.nothing"), QStringLiteral("{\"value\":1}")));
    QVERIFY(!unknownPath.value(QStringLiteral("ok")).toBool());
    const QString missing = unknownPath.value(QStringLiteral("reason")).toString();
    QVERIFY2(missing.contains(QStringLiteral("no property")), qPrintable(missing));
    QVERIFY2(missing.contains(QStringLiteral("nothing")), qPrintable(missing));

    const QJsonObject readOnly = parse(QGCBridgeCore::set(
        QStringLiteral("vehicles.activeVehicleAvailable"), QStringLiteral("{\"value\":true}")));
    QVERIFY(!readOnly.value(QStringLiteral("ok")).toBool());
    const QString reason = readOnly.value(QStringLiteral("reason")).toString();
    QVERIFY2(reason.contains(QStringLiteral("WRITE")) || reason.contains(QStringLiteral("no property")),
             qPrintable(reason));
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

void QGCBridgeCoreTest::_watchReplacesItsPreviousPaths()
{
    QStringList paths;
    QGCBridgeCore::setEventHandler([&paths](const QString &path, const QString &) {
        paths.append(path);
    });

    QGCBridgeCore::watch(QStringList { QString::fromLatin1(kSpeedUnits) });
    QTRY_VERIFY_WITH_TIMEOUT(paths.contains(QString::fromLatin1(kSpeedUnits)), 3000);

    QGCBridgeCore::watch(QStringList { QStringLiteral("vehicles.activeVehicleAvailable") });
    QTRY_VERIFY_WITH_TIMEOUT(
        paths.contains(QStringLiteral("vehicles.activeVehicleAvailable")), 3000);

    paths.clear();

    Fact *const fact = SettingsManager::instance()->unitsSettings()->speedUnits();
    const QVariant original = fact->cookedValue();
    fact->setCookedValue(original.toInt() == 0 ? 1 : 0);
    QTest::qWait(1000);
    fact->setCookedValue(original);
    QTest::qWait(1000);

    QVERIFY2(!paths.contains(QString::fromLatin1(kSpeedUnits)),
             "a replaced path kept reporting after the watch list changed");
}

void QGCBridgeCoreTest::_watchStopsWhenGivenNoPaths()
{
    QStringList paths;
    QGCBridgeCore::setEventHandler([&paths](const QString &path, const QString &) {
        paths.append(path);
    });

    QGCBridgeCore::watch(QStringList { QString::fromLatin1(kSpeedUnits) });
    QTRY_VERIFY_WITH_TIMEOUT(!paths.isEmpty(), 3000);

    QGCBridgeCore::watch(QStringList());
    QTest::qWait(500);
    paths.clear();

    Fact *const fact = SettingsManager::instance()->unitsSettings()->speedUnits();
    const QVariant original = fact->cookedValue();
    fact->setCookedValue(original.toInt() == 0 ? 1 : 0);
    QTest::qWait(1000);
    fact->setCookedValue(original);
    QTest::qWait(1000);

    QVERIFY2(paths.isEmpty(), "an emptied watch list kept reporting");
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

void QGCBridgeCoreTest::_invokesFactValidateAndReturnsQGCsOwnError()
{
    const QString path = QStringLiteral("settings.appSettings.batteryPercentRemainingAnnounce.validate");

    const QJsonObject inRange = QJsonDocument::fromJson(
        QGCBridgeCore::invoke(path, QStringLiteral("[\"50\", false]")).toUtf8()).object();
    QVERIFY2(inRange.value(QStringLiteral("ok")).toBool(), "validate was not reachable through the bridge");
    QCOMPARE(inRange.value(QStringLiteral("result")).toString(), QString());

    const QJsonObject tooHigh = QJsonDocument::fromJson(
        QGCBridgeCore::invoke(path, QStringLiteral("[\"9999\", false]")).toUtf8()).object();
    QVERIFY2(tooHigh.value(QStringLiteral("ok")).toBool(), "validate was not reachable through the bridge");
    QVERIFY2(!tooHigh.value(QStringLiteral("result")).toString().isEmpty(),
             "a value above the fact maximum was reported as valid");

    const QJsonObject notANumber = QJsonDocument::fromJson(
        QGCBridgeCore::invoke(path, QStringLiteral("[\"abc\", false]")).toUtf8()).object();
    QVERIFY2(!notANumber.value(QStringLiteral("result")).toString().isEmpty(),
             "a non-numeric value was reported as valid");
}

void QGCBridgeCoreTest::_factCarriesWhatQGCKnowsAboutItsRange()
{
    const QJsonObject group = QJsonDocument::fromJson(
        QGCBridgeCore::get(QStringLiteral("settings.appSettings")).toUtf8()).object();
    const QJsonArray facts = group.value(QStringLiteral("facts")).toArray();
    QVERIFY2(!facts.isEmpty(), "app settings reported no facts");

    QJsonObject announce;
    for (const QJsonValue &value : facts) {
        if (value.toObject().value(QStringLiteral("name")).toString()
            == QStringLiteral("batteryPercentRemainingAnnounce")) {
            announce = value.toObject();
            break;
        }
    }
    QVERIFY2(!announce.isEmpty(), "batteryPercentRemainingAnnounce was not reported");

    for (const QString &key : { QStringLiteral("minString"), QStringLiteral("maxString"),
                                QStringLiteral("minIsDefaultForType"),
                                QStringLiteral("maxIsDefaultForType"),
                                QStringLiteral("defaultValueString"),
                                QStringLiteral("vehicleRebootRequired"),
                                QStringLiteral("qgcRebootRequired") }) {
        QVERIFY2(announce.contains(key), qPrintable(QStringLiteral("fact is missing %1").arg(key)));
    }

    QCOMPARE(announce.value(QStringLiteral("minIsDefaultForType")).toBool(), true);
    QCOMPARE(announce.value(QStringLiteral("maxIsDefaultForType")).toBool(), false);
    QCOMPARE(announce.value(QStringLiteral("maxString")).toString(), QStringLiteral("100"));
}

void QGCBridgeCoreTest::_watchFromAnotherThreadDoesNotBlockTheCaller()
{
    QStringList paths;
    QGCBridgeCore::setEventHandler([&paths](const QString &path, const QString &) {
        paths.append(path);
    });
    QGCBridgeCore::watch(QStringList());

    QElapsedTimer elapsed;
    std::atomic_bool finished { false };
    qint64 callMSecs = -1;

    QThread *const worker = QThread::create([&elapsed, &finished, &callMSecs]() {
        elapsed.start();
        QGCBridgeCore::watch(QStringList { QString::fromLatin1(kSpeedUnits) });
        callMSecs = elapsed.elapsed();
        finished = true;
    });

    worker->start();
    QTRY_VERIFY_WITH_TIMEOUT(finished.load(), 3000);
    worker->wait();
    delete worker;

    QVERIFY2(callMSecs >= 0, "the worker never recorded a duration");
    QVERIFY2(callMSecs < 100, qPrintable(QStringLiteral(
        "watch blocked the calling thread for %1ms; it posts and must not wait").arg(callMSecs)));

    QTRY_VERIFY_WITH_TIMEOUT(!paths.isEmpty(), 3000);
    QCOMPARE(paths.first(), QString::fromLatin1(kSpeedUnits));

    QGCBridgeCore::watch(QStringList());
}

void QGCBridgeCoreTest::_writesAnObjectPropertyFromAnAtPath()
{
    const QString target = QStringLiteral("vehicles.activeVehicle");

    const QJsonObject plain = parse(QGCBridgeCore::set(
        target, QStringLiteral("{\"value\": 7}")));
    QCOMPARE(plain.value(QStringLiteral("ok")).toBool(), false);
    QVERIFY2(plain.value(QStringLiteral("reason")).toString().contains(QStringLiteral("@path")),
             qPrintable(plain.value(QStringLiteral("reason")).toString()));

    const QJsonObject missing = parse(QGCBridgeCore::set(
        target, QStringLiteral("{\"value\": \"@no.such.thing\"}")));
    QCOMPARE(missing.value(QStringLiteral("ok")).toBool(), false);
    QVERIFY2(missing.value(QStringLiteral("reason")).toString().contains(QStringLiteral("does not resolve")),
             qPrintable(missing.value(QStringLiteral("reason")).toString()));

    const QJsonObject wrongType = parse(QGCBridgeCore::set(
        target, QStringLiteral("{\"value\": \"@settings.unitsSettings\"}")));
    QCOMPARE(wrongType.value(QStringLiteral("ok")).toBool(), false);
    QVERIFY2(wrongType.value(QStringLiteral("reason")).toString().contains(QStringLiteral("not a")),
             qPrintable(wrongType.value(QStringLiteral("reason")).toString()));
}

// resolve() follows the last segment into whatever the property holds, so a write to a
// property that already points at an object used to arrive with no property name and be
// refused with a bare ok:false. unitsSettings is the reachable case: a QObject* that is
// never null. activeVehicle has the same shape but reads null in this fixture, which is
// exactly why the older test above passes either way.
void QGCBridgeCoreTest::_findsThePropertyEvenWhenItAlreadyHoldsAnObject()
{
    const QJsonObject held = parse(QGCBridgeCore::set(
        QStringLiteral("settings.unitsSettings"), QStringLiteral("{\"value\": \"@settings.appSettings\"}")));

    QCOMPARE(held.value(QStringLiteral("ok")).toBool(), false);

    const QString reason = held.value(QStringLiteral("reason")).toString();
    QVERIFY2(!reason.isEmpty(), "a refused write must say why");
    QVERIFY2(reason.contains(QStringLiteral("unitsSettings")), qPrintable(reason));
    QVERIFY2(reason.contains(QStringLiteral("WRITE")), qPrintable(reason));
}

// A Q_ENUM is state a native head needs like any other. Read as null, or an invoke that
// fails outright, loses it silently rather than erroring — the failure shape this bridge
// keeps producing. The numeric value is what a caller wants; the printed name is a trap.
void QGCBridgeCoreTest::_readsEnumsAsNumbers()
{
    const QJsonObject precheck = parse(QGCBridgeCore::invoke(
        QStringLiteral("plan.missionController.sendToVehiclePreCheck"), QStringLiteral("[]")));
    QVERIFY2(precheck.value(QStringLiteral("ok")).toBool(),
             "an enum-returning invokable must not fail");
    QVERIFY2(precheck.value(QStringLiteral("result")).isDouble(),
             qPrintable(QJsonDocument(precheck).toJson(QJsonDocument::Compact)));

    // readyForSaveState is a Q_ENUM property, which fromVariant renders as null.
    const QJsonObject items = parse(QGCBridgeCore::get(
        QStringLiteral("plan.missionController.visualItems")));
    const QJsonArray elements = items.value(QStringLiteral("elements")).toArray();
    QVERIFY(!elements.isEmpty());
    QVERIFY2(elements.first().toObject().value(QStringLiteral("readyForSaveState")).isDouble(),
             qPrintable(QJsonDocument(elements.first().toObject()).toJson(QJsonDocument::Compact)));
}

// Facts cross the bridge cooked while invokables and plain doubles stay metric, so a
// head that puts the two beside each other mixes units silently. QGC converts for
// display through QmlUnitsConversion; without it a native head either carries its own
// factors or shows metres whatever the operator selected.
void QGCBridgeCoreTest::_convertsUnitsForNativeHeads()
{
    const QJsonObject roundTrip = parse(QGCBridgeCore::invoke(
        QStringLiteral("units.appSettingsVerticalDistanceUnitsToMeters"),
        QStringLiteral("[100]")));
    QVERIFY2(roundTrip.value(QStringLiteral("ok")).toBool(),
             qPrintable(QJsonDocument(roundTrip).toJson(QJsonDocument::Compact)));
    QVERIFY(roundTrip.value(QStringLiteral("result")).isDouble());

    const QJsonObject units = parse(QGCBridgeCore::get(QStringLiteral("units")));
    QVERIFY2(!units.value(QStringLiteral("appSettingsVerticalDistanceUnitsString"))
                  .toString().isEmpty(),
             qPrintable(QJsonDocument(units).toJson(QJsonDocument::Compact)));
}

void QGCBridgeCoreTest::_anObjectSaysWhatClassItIs()
{
    const QJsonObject simple = parse(QGCBridgeCore::invoke(
        QStringLiteral("plan.missionController.insertSimpleMissionItem"),
        QStringLiteral("[{\"latitude\": 47.397, \"longitude\": 8.545, \"altitude\": 50}, -1, false]")));
    QVERIFY2(simple.value(QStringLiteral("ok")).toBool(),
             qPrintable(QJsonDocument(simple).toJson(QJsonDocument::Compact)));

    const QJsonObject corridor = parse(QGCBridgeCore::invoke(
        QStringLiteral("plan.missionController.insertComplexMissionItem"),
        QStringLiteral("[\"Corridor Scan\", {\"latitude\": 47.397, \"longitude\": 8.545}, -1, false]")));
    QVERIFY2(corridor.value(QStringLiteral("ok")).toBool(),
             qPrintable(QJsonDocument(corridor).toJson(QJsonDocument::Compact)));

    const QJsonArray elements = parse(QGCBridgeCore::get(
        QStringLiteral("plan.missionController.visualItems")))
        .value(QStringLiteral("elements")).toArray();
    QVERIFY(elements.size() >= 3);

    const QStringList classes = [&elements]() {
        QStringList found;
        for (const QJsonValue &element : elements) {
            found.append(element.toObject().value(QStringLiteral("class")).toString());
        }
        return found;
    }();

    QVERIFY2(classes.contains(QStringLiteral("SimpleMissionItem")), qPrintable(classes.join(QChar(','))));
    QVERIFY2(classes.contains(QStringLiteral("CorridorScanComplexItem")), qPrintable(classes.join(QChar(','))));
    QVERIFY2(!classes.contains(QString()), qPrintable(classes.join(QChar(','))));

    const QJsonObject corridorItem = [&elements]() {
        for (const QJsonValue &element : elements) {
            if (element.toObject().value(QStringLiteral("class")).toString()
                == QStringLiteral("CorridorScanComplexItem")) {
                return element.toObject();
            }
        }
        return QJsonObject();
    }();
    QCOMPARE(corridorItem.value(QStringLiteral("isSimpleItem")).toBool(), false);
    QCOMPARE(corridorItem.value(QStringLiteral("isSurveyItem")).toBool(), false);
    QVERIFY(!corridorItem.contains(QStringLiteral("surveyAreaPolygon")));

    (void) QGCBridgeCore::invoke(QStringLiteral("plan.removeAll"), QStringLiteral("[]"));
}

// A coordinate read directly carries "valid"; nested in an object read it did not, so a
// caller checking that key could not tell a good coordinate from a missing field. The
// macOS head read nil there and would have hidden Orbit from a vehicle with a valid home.
void QGCBridgeCoreTest::_aNestedCoordinateSaysItIsValid()
{
    // Insert a waypoint so there is definitely a valid coordinate to read back. Asserting
    // against whatever the fixture happened to hold passed without running the check at
    // all — the settings item has no home coordinate with no vehicle.
    const QJsonObject inserted = parse(QGCBridgeCore::invoke(
        QStringLiteral("plan.missionController.insertSimpleMissionItem"),
        QStringLiteral("[{\"latitude\": 47.397, \"longitude\": 8.545, \"altitude\": 50}, -1, false]")));
    QVERIFY2(inserted.value(QStringLiteral("ok")).toBool(),
             qPrintable(QJsonDocument(inserted).toJson(QJsonDocument::Compact)));

    const QJsonObject plan = parse(QGCBridgeCore::get(
        QStringLiteral("plan.missionController.visualItems")));
    const QJsonArray elements = plan.value(QStringLiteral("elements")).toArray();
    QVERIFY(elements.size() >= 2);

    const QJsonValue nested = elements.last().toObject().value(QStringLiteral("coordinate"));
    QVERIFY2(nested.isObject(),
             qPrintable(QJsonDocument(elements.last().toObject()).toJson(QJsonDocument::Compact)));
    QVERIFY2(nested.toObject().contains(QStringLiteral("valid")),
             qPrintable(QJsonDocument(nested.toObject()).toJson(QJsonDocument::Compact)));
    QCOMPARE(nested.toObject().value(QStringLiteral("valid")).toBool(), true);
    QCOMPARE(nested.toObject().value(QStringLiteral("latitude")).toDouble(), 47.397);

    const QJsonObject position = parse(QGCBridgeCore::get(QStringLiteral("positionManager")));
    QVERIFY2(position.contains(QStringLiteral("gcsHeading")),
             qPrintable(QJsonDocument(position).toJson(QJsonDocument::Compact)));

    // The plan controller is shared across this suite, so put it back as it was found.
    (void) QGCBridgeCore::invoke(QStringLiteral("plan.removeAll"), QStringLiteral("[]"));
}

// A fact nested in an object read is identified by the property it came from; its name is
// a label. The macOS head reported the orbit circle's radius arriving with an empty name,
// which would make a head that matches on name silently find nothing.
void QGCBridgeCoreTest::_aNestedFactKeepsItsName()
{
    const QJsonObject added = parse(QGCBridgeCore::invoke(
        QStringLiteral("plan.geoFenceController.addInclusionCircle"),
        QStringLiteral("[{\"latitude\": 47.397, \"longitude\": 8.545, \"altitude\": 0},"
                       " {\"latitude\": 47.398, \"longitude\": 8.546, \"altitude\": 0}]")));
    QVERIFY2(added.value(QStringLiteral("ok")).toBool(),
             qPrintable(QJsonDocument(added).toJson(QJsonDocument::Compact)));

    const QJsonObject circles = parse(QGCBridgeCore::get(
        QStringLiteral("plan.geoFenceController.circles")));
    const QJsonArray elements = circles.value(QStringLiteral("elements")).toArray();
    QVERIFY(!elements.isEmpty());

    const QJsonArray facts = elements.first().toObject().value(QStringLiteral("facts")).toArray();
    QVERIFY2(!facts.isEmpty(),
             qPrintable(QJsonDocument(elements.first().toObject()).toJson(QJsonDocument::Compact)));

    bool sawRadius = false;
    for (const QJsonValue &entry : facts) {
        const QJsonObject described = entry.toObject();
        QVERIFY2(!described.value(QStringLiteral("property")).toString().isEmpty(),
                 qPrintable(QJsonDocument(described).toJson(QJsonDocument::Compact)));
        if (described.value(QStringLiteral("property")).toString() == QStringLiteral("radius")) {
            sawRadius = true;
            QVERIFY2(!described.value(QStringLiteral("name")).toString().isEmpty(),
                     qPrintable(QJsonDocument(described).toJson(QJsonDocument::Compact)));
        }
    }
    QVERIFY2(sawRadius, "the circle exposes no radius fact to check");

    // The fence circle above is built by a constructor that names its fact. Vehicle's
    // orbit circle is default-constructed, and that constructor did not — so the check
    // above passed while the reported case stayed broken.
    QGCMapCircle defaultConstructed;
    QVERIFY2(!defaultConstructed.radius()->name().isEmpty(),
             "a default-constructed circle hands out an unnamed radius fact");

    (void) QGCBridgeCore::invoke(QStringLiteral("plan.removeAll"), QStringLiteral("[]"));
}

void QGCBridgeCoreTest::_planStartsWithItsSettingsItemAndNoVehicle()
{
    const QJsonObject items = parse(QGCBridgeCore::get(
        QStringLiteral("plan.missionController.visualItems")));
    const QJsonArray elements = items.value(QStringLiteral("elements")).toArray();

    QVERIFY2(elements.size() == 1, qPrintable(QStringLiteral(
        "visualItems reported %1 elements with no vehicle; MissionController::_init "
        "adds exactly one settings item to a started controller")
        .arg(elements.size())));
}

void QGCBridgeCoreTest::_planSaveReportsWhetherItWrote()
{
    const QString writable = QDir(QStandardPaths::writableLocation(
        QStandardPaths::TempLocation)).filePath(QStringLiteral("qgcbridge-plan"));
    QFile::remove(writable + QStringLiteral(".plan"));

    const QJsonObject wrote = parse(QGCBridgeCore::invoke(
        QStringLiteral("plan.saveToFile"),
        QStringLiteral("[\"%1\"]").arg(writable)));
    QCOMPARE(wrote.value(QStringLiteral("result")).toBool(), true);

    QVERIFY2(QFile::exists(writable + QStringLiteral(".plan")),
             "saveToFile appends its own extension when the name carries none");

    const QJsonObject current = parse(QGCBridgeCore::get(QStringLiteral("plan.currentPlanFile")));
    QCOMPARE(current.value(QStringLiteral("value")).toString(), writable + QStringLiteral(".plan"));

    const QJsonObject refused = parse(QGCBridgeCore::invoke(
        QStringLiteral("plan.saveToFile"),
        QStringLiteral("[\"/no/such/directory/plan.plan\"]")));
    QCOMPARE(refused.value(QStringLiteral("result")).toBool(), false);

    QFile::remove(writable + QStringLiteral(".plan"));
}

void QGCBridgeCoreTest::_aRefusedLoadLeavesTheExistingPlanAlone()
{
    const auto itemCount = []() {
        return parse(QGCBridgeCore::get(QStringLiteral("plan.missionController.visualItems")))
            .value(QStringLiteral("elements")).toArray().size();
    };

    QGCBridgeCore::invoke(QStringLiteral("plan.missionController.insertSimpleMissionItem"),
                          QStringLiteral("[{\"latitude\":47.4,\"longitude\":8.5,\"altitude\":30}, 1]"));
    const int planned = itemCount();
    QVERIFY2(planned > 1, qPrintable(QStringLiteral(
        "expected a planned item on top of the settings item, saw %1").arg(planned)));

    const QString rubbish = QDir(QStandardPaths::writableLocation(
        QStandardPaths::TempLocation)).filePath(QStringLiteral("not-a-plan.plan"));
    QFile file(rubbish);
    QVERIFY(file.open(QIODevice::WriteOnly | QIODevice::Text));
    file.write("{\"fileType\":\"NotAPlan\",\"version\":99}");
    file.close();

    const QJsonObject refused = parse(QGCBridgeCore::invoke(
        QStringLiteral("plan.loadFromFile"), QStringLiteral("[\"%1\"]").arg(rubbish)));
    QCOMPARE(refused.value(QStringLiteral("result")).toBool(), false);

    QCOMPARE(itemCount(), planned);

    QFile::remove(rubbish);
}
