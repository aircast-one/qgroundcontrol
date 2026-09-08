#include "QGCCoreCTest.h"

#include "MockLink.h"
#include "QGCBridgeC.h"

#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtTest/QTest>

#include <algorithm>

namespace
{

QStringList paths;
QStringList payloads;
QString readDuringEvent;

QJsonObject take(char *owned)
{
    const QJsonObject object = QJsonDocument::fromJson(QByteArray(owned)).object();
    qgc_bridge_free(owned);
    return object;
}

void onEvent(const char *path, const char *json)
{
    paths.append(QString::fromUtf8(path));
    payloads.append(QString::fromUtf8(json));
    if (readDuringEvent.isEmpty() && paths.last() == QStringLiteral("view.messages")) {
        char *const armed = qgc_bridge_get("vehicle.armed");
        readDuringEvent = QString::fromUtf8(armed);
        qgc_bridge_free(armed);
    }
}

int latestViewCount()
{
    const qsizetype index = paths.lastIndexOf(QStringLiteral("view.messages"));
    return index < 0 ? -1 : QJsonDocument::fromJson(payloads.at(index).toUtf8()).object().value(QStringLiteral("count")).toInt(-1);
}

} // namespace

void QGCCoreCTest::init()
{
    UnitTest::init();
    paths.clear();
    payloads.clear();
    readDuringEvent.clear();
}

void QGCCoreCTest::cleanup()
{
    qgc_bridge_watch("");
    qgc_bridge_watch_client("fly", "");
    qgc_bridge_watch_client("plan", "");
    qgc_bridge_set_event_handler(nullptr);
    _disconnectMockLink();
    UnitTest::cleanup();
}

void QGCCoreCTest::_viewMessagesReachTheHeadThroughTheRustCore()
{
    qgc_bridge_set_event_handler(onEvent);
    qgc_bridge_watch("view.messages,vehicles.activeVehicleAvailable");

    _connectMockLink(MAV_AUTOPILOT_PX4);
    _mockLink->sendStatusTextMessages();

    QTRY_VERIFY_WITH_TIMEOUT(latestViewCount() > 0, 5000);
    QVERIFY2(paths.contains(QStringLiteral("vehicles.activeVehicleAvailable")), "a directly watched Qt path stopped passing through");
    QVERIFY2(!readDuringEvent.isEmpty(), "a bridge read from inside the event handler never returned");

    const QJsonObject view = take(qgc_bridge_get("view.messages"));
    const QJsonObject raw = take(qgc_bridge_get("vehicle.formattedMessages"));
    QCOMPARE(view.value(QStringLiteral("class")).toString(), QStringLiteral("VehicleMessages"));
    QCOMPARE(view.value(QStringLiteral("count")).toInt(), raw.value(QStringLiteral("value")).toString().count(QStringLiteral("</font><br/>")));
    const QJsonObject first = view.value(QStringLiteral("items")).toArray().first().toObject();
    QVERIFY(!first.value(QStringLiteral("text")).toString().isEmpty());
    QVERIFY(!first.value(QStringLiteral("text")).toString().contains(QLatin1Char('<')));
}

void QGCCoreCTest::_viewPathsAreReadOnlyAtTheCAbi()
{
    QCOMPARE(take(qgc_bridge_set("view.messages", "{\"value\":1}")).value(QStringLiteral("ok")).toBool(true), false);
    QCOMPARE(take(qgc_bridge_invoke("view.messages", "[]")).value(QStringLiteral("ok")).toBool(true), false);
    QCOMPARE(take(qgc_bridge_get("view.nothing")).value(QStringLiteral("kind")).toString(), QStringLiteral("null"));
    QCOMPARE(take(qgc_bridge_get("settings.unitsSettings")).value(QStringLiteral("kind")).toString(), QStringLiteral("object"));
}

void QGCCoreCTest::_viewFieldsProjectAndNameTheUnknown()
{
    const QJsonObject projected = take(qgc_bridge_get_fields("view.messages", "count,bogus"));
    QVERIFY(projected.contains(QStringLiteral("count")));
    QVERIFY(!projected.contains(QStringLiteral("items")));
    QCOMPARE(projected.value(QStringLiteral("unknownFields")).toArray().first().toString(), QStringLiteral("bogus"));
}

void QGCCoreCTest::_clientsWatchIndependently()
{
    qgc_bridge_set_event_handler(onEvent);
    qgc_bridge_watch_client("fly", "settings.unitsSettings.speedUnits");
    qgc_bridge_watch_client("plan", "vehicles.activeVehicleAvailable");
    QTRY_VERIFY_WITH_TIMEOUT(paths.contains(QStringLiteral("settings.unitsSettings.speedUnits")) && paths.contains(QStringLiteral("vehicles.activeVehicleAvailable")), 3000);

    qgc_bridge_watch_client("plan", "");
    QTest::qWait(500);
    paths.clear();
    _connectMockLink(MAV_AUTOPILOT_PX4);
    QTest::qWait(1000);
    QVERIFY2(!paths.contains(QStringLiteral("vehicles.activeVehicleAvailable")), "a client that unregistered kept receiving events");
}

void QGCCoreCTest::_planViewFollowsTheVehicle()
{
    const QJsonObject offline = take(qgc_bridge_get("view.plan"));
    QCOMPARE(offline.value(QStringLiteral("class")).toString(), QStringLiteral("PlanStatus"));
    QCOMPARE(offline.value(QStringLiteral("readiness")).toObject().value(QStringLiteral("ready")).toBool(), true);
    QCOMPARE(offline.value(QStringLiteral("upload")).toObject().value(QStringLiteral("state")).toInt(-1), 1);
    QCOMPARE(offline.value(QStringLiteral("sync")).toObject().value(QStringLiteral("state")).toString(), QStringLiteral("offline"));
    QCOMPARE(offline.value(QStringLiteral("status")).toString(), QStringLiteral("New plan"));

    qgc_bridge_set_event_handler(onEvent);
    qgc_bridge_watch("view.plan");
    _connectMockLink(MAV_AUTOPILOT_PX4);
    QTRY_VERIFY_WITH_TIMEOUT(paths.contains(QStringLiteral("view.plan")), 5000);
    QTRY_COMPARE_WITH_TIMEOUT(take(qgc_bridge_get("view.plan")).value(QStringLiteral("sync")).toObject().value(QStringLiteral("state")).toString(), QStringLiteral("ready"), 5000);
    const QJsonObject online = take(qgc_bridge_get("view.plan"));
    QVERIFY2(online.value(QStringLiteral("upload")).toObject().value(QStringLiteral("state")).toInt(-1) != 1, "a connected vehicle still reads as absent");
    QCOMPARE(online.value(QStringLiteral("actions")).toObject().value(QStringLiteral("clearMission")).toBool(), true);
}

void QGCCoreCTest::_guidedActionsFollowTheVehicle()
{
    const QJsonObject none = take(qgc_bridge_get("view.guidedActions"));
    QCOMPARE(none.value(QStringLiteral("class")).toString(), QStringLiteral("GuidedActions"));
    QCOMPARE(none.value(QStringLiteral("connected")).toBool(true), false);
    const QJsonArray hidden = none.value(QStringLiteral("actions")).toArray();
    QCOMPARE(hidden.count(), 14);
    QVERIFY(std::all_of(hidden.begin(), hidden.end(), [](const QJsonValue &a) { return a.toObject().value(QStringLiteral("offer")).toString() == QStringLiteral("hidden"); }));

    _connectMockLink(MAV_AUTOPILOT_PX4);
    QTRY_COMPARE_WITH_TIMEOUT(take(qgc_bridge_get("view.guidedActions")).value(QStringLiteral("connected")).toBool(false), true, 5000);
    const QJsonObject online = take(qgc_bridge_get("view.guidedActions"));
    const QJsonArray actions = online.value(QStringLiteral("actions")).toArray();
    const auto offer = [&actions](const QString &id) {
        const auto it = std::find_if(actions.begin(), actions.end(), [&id](const QJsonValue &a) { return a.toObject().value(QStringLiteral("id")).toString() == id; });
        return it == actions.end() ? QString() : it->toObject().value(QStringLiteral("offer")).toString();
    };
    QCOMPARE(offer(QStringLiteral("arm")), QStringLiteral("ready"));
    QCOMPARE(offer(QStringLiteral("takeoff")), QStringLiteral("ready"));
    QCOMPARE(offer(QStringLiteral("rtl")), QStringLiteral("hidden"));
    QCOMPARE(offer(QStringLiteral("emergencyStop")), QStringLiteral("hidden"));
}

void QGCCoreCTest::_guidedAltitudeTakesATarget()
{
    QCOMPARE(take(qgc_bridge_get("view.guidedAltitude")).value(QStringLiteral("available")).toBool(true), false);

    _connectMockLink(MAV_AUTOPILOT_PX4);
    QTRY_COMPARE_WITH_TIMEOUT(take(qgc_bridge_get("view.guidedAltitude")).value(QStringLiteral("available")).toBool(false), true, 5000);
    const QJsonObject range = take(qgc_bridge_get("view.guidedAltitude"));
    QVERIFY(!range.value(QStringLiteral("unit")).toString().isEmpty());
    const double current = range.value(QStringLiteral("current")).toDouble();
    QVERIFY(range.value(QStringLiteral("minimum")).toDouble() <= current && current <= range.value(QStringLiteral("maximum")).toDouble());

    const QJsonObject climb = take(qgc_bridge_get(QStringLiteral("view.guidedAltitude(%1)").arg(current + 10).toUtf8().constData()));
    QCOMPARE(climb.value(QStringLiteral("sends")).toBool(false), true);
    QVERIFY(climb.value(QStringLiteral("sentence")).toString().startsWith(QStringLiteral("The aircraft will climb")));
    const QJsonObject same = take(qgc_bridge_get(QStringLiteral("view.guidedAltitude(%1)").arg(current).toUtf8().constData()));
    QCOMPARE(same.value(QStringLiteral("sends")).toBool(true), false);
    QVERIFY(same.value(QStringLiteral("sentence")).toString().contains(QStringLiteral("will not move")));
}

void QGCCoreCTest::_takeoffAndSpeedRangesFollowTheVehicle()
{
    QCOMPARE(take(qgc_bridge_get("view.guidedTakeoff")).value(QStringLiteral("available")).toBool(true), false);
    QCOMPARE(take(qgc_bridge_get("view.guidedSpeed")).value(QStringLiteral("available")).toBool(true), false);

    _connectMockLink(MAV_AUTOPILOT_PX4);
    QTRY_COMPARE_WITH_TIMEOUT(take(qgc_bridge_get("view.guidedTakeoff")).value(QStringLiteral("available")).toBool(false), true, 5000);
    const QJsonObject takeoff = take(qgc_bridge_get("view.guidedTakeoff(5)"));
    QVERIFY(takeoff.value(QStringLiteral("minimumMeters")).toDouble() > 0);
    QVERIFY(takeoff.value(QStringLiteral("sentence")).toString().startsWith(QStringLiteral("The aircraft will take off")));

    QTRY_COMPARE_WITH_TIMEOUT(take(qgc_bridge_get("view.guidedSpeed")).value(QStringLiteral("available")).toBool(false), true, 5000);
    const QJsonObject speed = take(qgc_bridge_get("view.guidedSpeed(3)"));
    QCOMPARE(speed.value(QStringLiteral("label")).toString(), QStringLiteral("Ground speed"));
    QCOMPARE(speed.value(QStringLiteral("command")).toString(), QStringLiteral("guidedModeChangeGroundSpeedMetersSecond"));
    QVERIFY(speed.value(QStringLiteral("targetMetersSecond")).toDouble() > 0);
    QVERIFY(!speed.value(QStringLiteral("unit")).toString().isEmpty());
}

void QGCCoreCTest::_batteryAndPreflightFollowTheVehicle()
{
    QCOMPARE(take(qgc_bridge_get("view.battery")).value(QStringLiteral("available")).toBool(true), false);
    const QJsonObject offline = take(qgc_bridge_get("view.preflight"));
    QCOMPARE(offline.value(QStringLiteral("groups")).toArray().count(), 3);
    QVERIFY(offline.value(QStringLiteral("blocked")).toArray().contains(QJsonValue(QStringLiteral("GPS"))));

    _connectMockLink(MAV_AUTOPILOT_PX4);
    QTRY_COMPARE_WITH_TIMEOUT(take(qgc_bridge_get("view.battery")).value(QStringLiteral("available")).toBool(false), true, 5000);
    const QJsonObject battery = take(qgc_bridge_get("view.battery"));
    QVERIFY(!battery.value(QStringLiteral("level")).toString().isEmpty());
    QVERIFY(battery.value(QStringLiteral("packs")).toArray().count() >= 1);

    const QJsonObject online = take(qgc_bridge_get("view.preflight"));
    QCOMPARE(online.value(QStringLiteral("airframe")).toString(), QStringLiteral("Multirotor"));
    const QJsonArray first = online.value(QStringLiteral("groups")).toArray().first().toObject().value(QStringLiteral("checks")).toArray();
    QCOMPARE(first.count(), 5);
    QCOMPARE(first.at(1).toObject().value(QStringLiteral("name")).toString(), QStringLiteral("Battery"));
    QVERIFY(!first.at(1).toObject().value(QStringLiteral("reason")).toString().contains(QStringLiteral("No vehicle")));
}

void QGCCoreCTest::_warningsFollowTheVehicle()
{
    const QJsonObject offline = take(qgc_bridge_get("view.warnings"));
    QCOMPARE(offline.value(QStringLiteral("class")).toString(), QStringLiteral("VehicleWarnings"));
    QCOMPARE(offline.value(QStringLiteral("showing")).toBool(true), false);
    QVERIFY(offline.value(QStringLiteral("armingBlocker")).isNull());

    _connectMockLink(MAV_AUTOPILOT_PX4);
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get("view.guidedActions")).value(QStringLiteral("connected")).toBool(false), 5000);
    const QJsonObject online = take(qgc_bridge_get("view.warnings"));
    QVERIFY(online.value(QStringLiteral("warnings")).isArray());
    QCOMPARE(online.value(QStringLiteral("showing")).toBool(), !online.value(QStringLiteral("warnings")).toArray().isEmpty());
}
