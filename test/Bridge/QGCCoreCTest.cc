#include "QGCCoreCTest.h"
#include "QGCMapUrlEngine.h"
#include <QtSql/QSqlError>
#include <QtSql/QSqlQuery>
#include <QtSql/QSqlDatabase>
#include "QGCMapEngine.h"

#include "MockLink.h"
#include "UDPLink.h"
#include "LinkManager.h"
#include "LogReplayLink.h"
#include "MultiVehicleManager.h"
#include "CoreLink.h"
#include "QGCBridgeC.h"
#include "QGCCoreC.h"
#include "MAVLinkLib.h"

#include <QtCore/QDir>
#include <QtCore/QElapsedTimer>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QScopeGuard>
#include <QtCore/QTemporaryFile>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtNetwork/QNetworkDatagram>
#include <QtNetwork/QUdpSocket>
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
    _disconnectMockLink();
    QTRY_VERIFY_WITH_TIMEOUT(!take(qgc_bridge_get("vehicles")).value(QStringLiteral("activeVehicleAvailable")).toBool(true), 5000);
    UnitTest::cleanup();
}

bool QGCCoreCTest::_unavailable(const char *path)
{
    const QJsonObject view = take(qgc_bridge_get(path));
    return view.contains(QStringLiteral("available")) && !view.value(QStringLiteral("available")).toBool(true);
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
    const QJsonArray declared = take(qgc_bridge_get("view.contract")).value(QStringLiteral("enumerations")).toObject().value(QStringLiteral("view.guidedActions.actions[].id")).toArray();
    QVERIFY(!declared.isEmpty());
    QCOMPARE(hidden.count(), declared.count());
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
    QVERIFY2(_unavailable("view.guidedAltitude"), "altitude reads available, or lacks the field, with no vehicle");

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
    QVERIFY2(_unavailable("view.guidedTakeoff"), "takeoff reads available, or lacks the field, with no vehicle");
    QVERIFY2(_unavailable("view.guidedSpeed"), "speed reads available, or lacks the field, with no vehicle");

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
    QVERIFY2(_unavailable("view.battery"), "battery reads available, or lacks the field, with no vehicle");
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
    QVERIFY(offline.value(QStringLiteral("armingBlocker")).isNull());

    _connectMockLink(MAV_AUTOPILOT_PX4);
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get("view.guidedActions")).value(QStringLiteral("connected")).toBool(false), 5000);
    const QJsonObject online = take(qgc_bridge_get("view.warnings"));
    QVERIFY(online.value(QStringLiteral("warnings")).isArray());
}

void QGCCoreCTest::_labelsAreHumanised()
{
    QCOMPARE(take(qgc_bridge_get("view.label(altitudeRelative)")).value(QStringLiteral("value")).toString(), QStringLiteral("Altitude Relative"));
    QCOMPARE(take(qgc_bridge_get("view.label(ADSBVehicleManager)")).value(QStringLiteral("value")).toString(), QStringLiteral("ADSB Vehicle Manager"));
    QCOMPARE(take(qgc_bridge_get("view.label")).value(QStringLiteral("value")).toString(), QString());
}

void QGCCoreCTest::_instrumentsResolveTheSelection()
{
    QVERIFY2(_unavailable("view.instruments"), "instruments read available, or lack the field, with no vehicle");
    _connectMockLink(MAV_AUTOPILOT_PX4);
    QTRY_COMPARE_WITH_TIMEOUT(take(qgc_bridge_get("view.instruments")).value(QStringLiteral("available")).toBool(false), true, 5000);
    const QJsonArray items = take(qgc_bridge_get("view.instruments")).value(QStringLiteral("items")).toArray();
    QCOMPARE(items.count(), 6);
    QCOMPARE(items.first().toObject().value(QStringLiteral("name")).toString(), QStringLiteral("altitudeRelative"));
    QVERIFY(!items.first().toObject().value(QStringLiteral("label")).toString().isEmpty());
    const QJsonArray chosen = take(qgc_bridge_get("view.instruments(gps/count,vehicle/heading)")).value(QStringLiteral("items")).toArray();
    QCOMPARE(chosen.count(), 2);
    QCOMPARE(chosen.first().toObject().value(QStringLiteral("id")).toString(), QStringLiteral("gps/count"));
}

void QGCCoreCTest::_vibrationBandsAreServed()
{
    const QJsonObject offline = take(qgc_bridge_get("view.vibration"));
    QCOMPARE(offline.value(QStringLiteral("class")).toString(), QStringLiteral("Vibration"));
    QCOMPARE(offline.value(QStringLiteral("warningLevel")).toDouble(), 30.0);
    QCOMPARE(offline.value(QStringLiteral("dangerLevel")).toDouble(), 60.0);
    QCOMPARE(offline.value(QStringLiteral("axes")).toArray().count(), 3);
    QCOMPARE(offline.value(QStringLiteral("clipCounts")).toArray().count(), 3);
}

void QGCCoreCTest::_sensorHealthIsOrdered()
{
    QVERIFY2(_unavailable("view.sensors"), "sensors read available, or lack the field, with no vehicle");
    QVERIFY(!take(qgc_bridge_get("view.sensors")).value(QStringLiteral("status")).toString().isEmpty());
    _connectMockLink(MAV_AUTOPILOT_PX4);
    QTRY_COMPARE_WITH_TIMEOUT(take(qgc_bridge_get("view.sensors")).value(QStringLiteral("available")).toBool(false), true, 5000);
    const QJsonArray listed = take(qgc_bridge_get("view.sensors")).value(QStringLiteral("sensors")).toArray();
    QVERIFY(!listed.isEmpty());
    QVERIFY(!listed.first().toObject().value(QStringLiteral("label")).toString().isEmpty());
}

void QGCCoreCTest::_controlsDescribeAFact()
{
    const QJsonObject units = take(qgc_bridge_get("view.control(settings.unitsSettings.verticalDistanceUnits)"));
    QCOMPARE(units.value(QStringLiteral("control")).toString(), QStringLiteral("choice"));
    QCOMPARE(units.value(QStringLiteral("options")).toArray().count(), 2);
    QVERIFY(!units.value(QStringLiteral("display")).toString().isEmpty());
    const QJsonObject muted = take(qgc_bridge_get("view.control(settings.appSettings.audioMuted)"));
    QCOMPARE(muted.value(QStringLiteral("control")).toString(), QStringLiteral("toggle"));
    const QJsonObject altitude = take(qgc_bridge_get("view.control(settings.appSettings.defaultMissionItemAltitude)"));
    QCOMPARE(altitude.value(QStringLiteral("control")).toString(), QStringLiteral("number"));
    QCOMPARE(take(qgc_bridge_get("view.control(settings.nope)")).value(QStringLiteral("kind")).toString(), QStringLiteral("null"));
    QCOMPARE(take(qgc_bridge_get("view.control")).value(QStringLiteral("kind")).toString(), QStringLiteral("null"));
}

void QGCCoreCTest::_linksAreListedAndTheFormValidates()
{
    const QJsonObject links = take(qgc_bridge_get("view.links"));
    QCOMPARE(links.value(QStringLiteral("available")).toBool(false), true);
    QVERIFY(!links.value(QStringLiteral("linkTypes")).toArray().isEmpty());
    QVERIFY(!links.value(QStringLiteral("baudRates")).toArray().isEmpty());
    QVERIFY(links.value(QStringLiteral("links")).isArray());
    const QJsonObject form = take(qgc_bridge_get("view.linkForm(tcp,,5760)"));
    QCOMPARE(form.value(QStringLiteral("valid")).toBool(true), false);
    const QJsonObject ok = take(qgc_bridge_get("view.linkForm(udp,,14550)"));
    QCOMPARE(ok.value(QStringLiteral("valid")).toBool(false), true);
    QCOMPARE(ok.value(QStringLiteral("name")).toString(), QStringLiteral("UDP 14550"));
}

void QGCCoreCTest::_mapScaleFollowsTheUnitSetting()
{
    const QJsonObject bar = take(qgc_bridge_get("view.mapScale(120)"));
    QCOMPARE(bar.value(QStringLiteral("available")).toBool(false), true);
    QVERIFY(bar.value(QStringLiteral("text")).toString() == QStringLiteral("100 m") || bar.value(QStringLiteral("text")).toString() == QStringLiteral("500 ft"));
    QCOMPARE(bar.value(QStringLiteral("imperial")).toBool(), bar.value(QStringLiteral("text")).toString().endsWith(QStringLiteral("ft")));
    QCOMPARE(take(qgc_bridge_get("view.mapScale")).value(QStringLiteral("available")).toBool(true), false);
}

void QGCCoreCTest::_terrainProfileReadsThePlan()
{
    const QJsonObject empty = take(qgc_bridge_get("view.terrainProfile"));
    QCOMPARE(empty.value(QStringLiteral("class")).toString(), QStringLiteral("TerrainProfile"));
    QCOMPARE(empty.value(QStringLiteral("usable")).toBool(true), false);
    QVERIFY(empty.value(QStringLiteral("points")).isArray());
    QVERIFY(!empty.value(QStringLiteral("distanceText")).toString().isEmpty());
}

void QGCCoreCTest::_missionKindsAndSeedsAreServed()
{
    QCOMPARE(take(qgc_bridge_get("view.missionKinds")).value(QStringLiteral("kinds")).toArray().count(), 7);
    QCOMPARE(take(qgc_bridge_get("view.missionKinds(Survey)")).value(QStringLiteral("geometryProperty")).toString(), QStringLiteral("surveyAreaPolygon"));
    const QJsonObject seed = take(qgc_bridge_get("view.missionSeed(survey,47.0,8.0)"));
    QCOMPARE(seed.value(QStringLiteral("points")).toArray().count(), 4);
    QCOMPARE(take(qgc_bridge_get("view.missionSeed(waypoint,47.0,8.0)")).value(QStringLiteral("kind")).toString(), QStringLiteral("null"));
}

void QGCCoreCTest::_calibrationIsListedWithoutAnApmVehicle()
{
    const QJsonObject view = take(qgc_bridge_get("view.calibration"));
    QCOMPARE(view.value(QStringLiteral("class")).toString(), QStringLiteral("Calibration"));
    QCOMPARE(view.value(QStringLiteral("connected")).toBool(true), false);
    QCOMPARE(view.value(QStringLiteral("routines")).toArray().count(), 5);
    QCOMPARE(view.value(QStringLiteral("sides")).toArray().count(), 6);
}

void QGCCoreCTest::_radioFollowsTheVehicle()
{
    const QJsonObject offline = take(qgc_bridge_get("view.radio"));
    QCOMPARE(offline.value(QStringLiteral("connected")).toBool(true), false);
    QCOMPARE(offline.value(QStringLiteral("summary")).toString(), QStringLiteral("No vehicle is connected."));
    _connectMockLink(MAV_AUTOPILOT_PX4);
    QTRY_COMPARE_WITH_TIMEOUT(take(qgc_bridge_get("view.radio")).value(QStringLiteral("connected")).toBool(false), true, 5000);
    const QJsonObject online = take(qgc_bridge_get("view.radio"));
    QCOMPARE(online.value(QStringLiteral("sticks")).toArray().count(), 4);
    QVERIFY(online.value(QStringLiteral("minimumChannels")).toInt() > 0);
}

void QGCCoreCTest::_logsFollowTheController()
{
    const QJsonObject offline = take(qgc_bridge_get("view.logs"));
    QCOMPARE(offline.value(QStringLiteral("class")).toString(), QStringLiteral("Logs"));
    QCOMPARE(offline.value(QStringLiteral("canRefresh")).toBool(true), false);
    QCOMPARE(offline.value(QStringLiteral("emptyText")).toString(), QStringLiteral("Connect a vehicle to list its logs."));
    _connectMockLink(MAV_AUTOPILOT_PX4);
    QTRY_COMPARE_WITH_TIMEOUT(take(qgc_bridge_get("view.logs")).value(QStringLiteral("connected")).toBool(false), true, 5000);
    QVERIFY(take(qgc_bridge_get("view.logs")).value(QStringLiteral("entries")).isArray());
}

void QGCCoreCTest::_inspectorListsMessages()
{
    _connectMockLink(MAV_AUTOPILOT_PX4);
    QTRY_VERIFY_WITH_TIMEOUT(!take(qgc_bridge_get("view.inspector")).value(QStringLiteral("messages")).toArray().isEmpty(), 5000);
    const QJsonObject view = take(qgc_bridge_get("view.inspector"));
    QCOMPARE(view.value(QStringLiteral("rateChoices")).toArray().count(), 15);
    const QJsonObject first = view.value(QStringLiteral("messages")).toArray().first().toObject();
    QVERIFY(!first.value(QStringLiteral("name")).toString().isEmpty());
    QVERIFY(first.value(QStringLiteral("path")).toString().startsWith(QStringLiteral("mavlinkInspector.activeSystem.messages.")));
}

void QGCCoreCTest::_flightModesFollowTheVehicle()
{
    QVERIFY2(_unavailable("view.flightModes"), "flight modes read available, or lack the field, with no vehicle");
    _connectMockLink(MAV_AUTOPILOT_PX4);
    QTRY_COMPARE_WITH_TIMEOUT(take(qgc_bridge_get("view.flightModes")).value(QStringLiteral("available")).toBool(false), true, 5000);
    const QJsonObject view = take(qgc_bridge_get("view.flightModes"));
    QVERIFY(!view.value(QStringLiteral("modes")).toArray().isEmpty());
    QVERIFY(!view.value(QStringLiteral("everyday")).toArray().isEmpty());
    const QString current = view.value(QStringLiteral("current")).toString();
    QVERIFY(std::any_of(view.value(QStringLiteral("modes")).toArray().begin(), view.value(QStringLiteral("modes")).toArray().end(), [&current](const QJsonValue &m) { return m.toObject().value(QStringLiteral("name")).toString() == current; }));
}

void QGCCoreCTest::_settingsPagesDecodeTheirControls()
{
    QCOMPARE(take(qgc_bridge_get("view.settings")).value(QStringLiteral("pages")).toArray().count(), 15);
    const QJsonObject general = take(qgc_bridge_get("view.settings(General)"));
    const QJsonArray sections = general.value(QStringLiteral("sections")).toArray();
    QCOMPARE(sections.count(), 3);
    const QJsonArray subsections = sections.first().toObject().value(QStringLiteral("subsections")).toArray();
    QVERIFY(!subsections.isEmpty());
    const QJsonArray controls = subsections.first().toObject().value(QStringLiteral("controls")).toArray();
    QVERIFY(!controls.isEmpty());
    QVERIFY(!controls.first().toObject().value(QStringLiteral("control")).toString().isEmpty());
    QVERIFY(controls.first().toObject().value(QStringLiteral("path")).toString().startsWith(QStringLiteral("settings.appSettings.")));
}

void QGCCoreCTest::_surveyStatsNeedAnItem()
{
    QCOMPARE(take(qgc_bridge_get("view.surveyStats")).value(QStringLiteral("kind")).toString(), QStringLiteral("null"));
    const QJsonObject missing = take(qgc_bridge_get("view.surveyStats(7)"));
    QCOMPARE(missing.value(QStringLiteral("class")).toString(), QStringLiteral("SurveyStats"));
    QCOMPARE(missing.value(QStringLiteral("available")).toBool(true), false);
    QCOMPARE(missing.value(QStringLiteral("shotsText")).toString(), QStringLiteral("\u2014"));
}

void QGCCoreCTest::_fencesAndPolygonsAreServed()
{
    const QJsonObject fences = take(qgc_bridge_get("view.fences"));
    QCOMPARE(fences.value(QStringLiteral("class")).toString(), QStringLiteral("Fences"));
    QVERIFY(fences.value(QStringLiteral("polygons")).isArray());
    QVERIFY(fences.value(QStringLiteral("rallyPoints")).isArray());
    QCOMPARE(take(qgc_bridge_get("view.polygon")).value(QStringLiteral("kind")).toString(), QStringLiteral("null"));
    QCOMPARE(take(qgc_bridge_get("view.polygon(plan.geoFenceController.polygons.99)")).value(QStringLiteral("kind")).toString(), QStringLiteral("null"));
}

void QGCCoreCTest::_setupOverviewFollowsTheVehicle()
{
    const QJsonObject offline = take(qgc_bridge_get("view.setup"));
    QCOMPARE(offline.value(QStringLiteral("headline")).toString(), QStringLiteral("No vehicle connected"));
    QCOMPARE(offline.value(QStringLiteral("groups")).toArray().count(), 3);
    QCOMPARE(take(qgc_bridge_get("view.setup(Safety)")).value(QStringLiteral("class")).toString(), QStringLiteral("SetupPage"));
    _connectMockLink(MAV_AUTOPILOT_PX4);
    QTRY_COMPARE_WITH_TIMEOUT(take(qgc_bridge_get("view.setup")).value(QStringLiteral("connected")).toBool(false), true, 5000);
    const QJsonObject online = take(qgc_bridge_get("view.setup"));
    QCOMPARE(online.value(QStringLiteral("firmware")).toString(), QStringLiteral("px4"));
    QVERIFY(!online.value(QStringLiteral("headline")).toString().isEmpty());
}

void QGCCoreCTest::_setupPageServesApmParameters()
{
    _connectMockLink(MAV_AUTOPILOT_ARDUPILOTMEGA);
    QTRY_COMPARE_WITH_TIMEOUT(take(qgc_bridge_get("view.setup")).value(QStringLiteral("connected")).toBool(false), true, 5000);
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get("vehicle.parameterManager.parametersReady")).value(QStringLiteral("value")).toBool(false), 20000);
    const QJsonObject raw = take(qgc_bridge_get("vehicle.parameterManager.getParameter(-1,RTL_ALT)"));
    QCOMPARE(raw.value(QStringLiteral("name")).toString(), QStringLiteral("RTL_ALT"));
    const QJsonObject page = take(qgc_bridge_get("view.setup(Safety)"));
    QCOMPARE(page.value(QStringLiteral("firmware")).toString(), QStringLiteral("apm"));
    QCOMPARE(page.value(QStringLiteral("available")).toBool(false), true);
    const QJsonArray sections = page.value(QStringLiteral("sections")).toArray();
    QVERIFY(!sections.isEmpty());
    for (const QJsonValue &section : sections) {
        for (const QJsonValue &control : section.toObject().value(QStringLiteral("controls")).toArray()) {
            const QJsonObject c = control.toObject();
            QVERIFY2(!c.value(QStringLiteral("name")).toString().isEmpty(), qPrintable(QStringLiteral("blank control at %1").arg(c.value(QStringLiteral("path")).toString())));
            QVERIFY(!c.value(QStringLiteral("label")).toString().isEmpty());
        }
    }
    const QJsonObject control = take(qgc_bridge_get("view.control(vehicle.parameterManager.getParameter(-1,RTL_ALT))"));
    QCOMPARE(control.value(QStringLiteral("name")).toString(), QStringLiteral("RTL_ALT"));
}

void QGCCoreCTest::_coreUdpLinkFramesAPeer()
{
    QUdpSocket peer;
    QVERIFY(peer.bind(QHostAddress::LocalHost, 0));
    const QString config = QStringLiteral("{\"kind\":\"udp\",\"name\":\"Probe\",\"port\":0,\"hosts\":[{\"host\":\"127.0.0.1\",\"port\":%1}]}").arg(peer.localPort());
    const QJsonObject opened = take(qgc_core_link_open(config.toUtf8().constData()));
    QVERIFY2(opened.value(QStringLiteral("ok")).toBool(false), qPrintable(QString::fromUtf8(QJsonDocument(opened).toJson(QJsonDocument::Compact))));
    const uint32_t id = static_cast<uint32_t>(opened.value(QStringLiteral("id")).toInt());

    mavlink_message_t message{};
    mavlink_msg_heartbeat_pack(1, 1, &message, MAV_TYPE_QUADROTOR, MAV_AUTOPILOT_PX4, 0, 0, MAV_STATE_ACTIVE);
    uint8_t frame[MAVLINK_MAX_PACKET_LEN]{};
    const uint16_t len = mavlink_msg_to_send_buffer(frame, &message);
    QVERIFY(qgc_core_link_write(id, frame, len));
    QTRY_VERIFY_WITH_TIMEOUT(peer.hasPendingDatagrams(), 2000);
    const QNetworkDatagram datagram = peer.receiveDatagram();
    QCOMPARE(datagram.data().size(), static_cast<qsizetype>(len));
    QVERIFY(peer.writeDatagram(datagram.data(), datagram.senderAddress(), datagram.senderPort()) == len);

    const auto framesIn = [id]() {
        const QJsonArray links = take(qgc_bridge_get("view.transports")).value(QStringLiteral("links")).toArray();
        for (const QJsonValue &link : links) {
            if (link.toObject().value(QStringLiteral("id")).toInt() == static_cast<int>(id)) {
                return link.toObject().value(QStringLiteral("framesIn")).toInt();
            }
        }
        return -1;
    };
    QTRY_COMPARE_WITH_TIMEOUT(framesIn(), 1, 3000);
    const auto linkById = [id]() {
        const QJsonArray links = take(qgc_bridge_get("view.transports")).value(QStringLiteral("links")).toArray();
        for (const QJsonValue &link : links) {
            if (link.toObject().value(QStringLiteral("id")).toInt() == static_cast<int>(id)) {
                return link.toObject();
            }
        }
        return QJsonObject();
    };
    QCOMPARE(linkById().value(QStringLiteral("owner")).toString(), QStringLiteral("core"));
    QVERIFY2(linkById().value(QStringLiteral("bytesOut")).toInt() >= static_cast<int>(len), "the peer's bytes and the core's own connect requests both count as sent");
    QVERIFY(qgc_core_link_close(id, "test done"));
    QVERIFY(!qgc_core_link_write(id, frame, len));
}

void QGCCoreCTest::_coreBackedLinkBringsUpAVehicle()
{
#ifdef QGC_RUST_CORE
    qputenv("QGC_CORE_LINKS", "1");
    QUdpSocket peer;
    QVERIFY(peer.bind(QHostAddress::LocalHost, 0));
    UDPConfiguration *const udp = new UDPConfiguration(QStringLiteral("Core UDP"));
    udp->setLocalPort(0);
    udp->addHost(QStringLiteral("127.0.0.1"), peer.localPort());
    udp->setDynamic(true);
    {
        const QMetaObject *const meta = udp->metaObject();
        QMetaMethod addHost;
        for (int index = meta->methodOffset(); index < meta->methodCount(); index++) {
            const QMetaMethod candidate = meta->method(index);
            if (candidate.name() == QByteArrayLiteral("addHost") && candidate.parameterCount() == 2) {
                addHost = candidate;
            }
        }
        QVERIFY(addHost.isValid());
        QCOMPARE(addHost.parameterTypeName(1), QByteArray(addHost.parameterMetaType(1).name()));

        QVariant host = QStringLiteral("127.0.0.2");
        QVariant port = 14557;
        QVERIFY(port.convert(addHost.parameterMetaType(1)));
        QVERIFY2(addHost.invoke(udp, Qt::DirectConnection,
                                QGenericArgument(addHost.parameterMetaType(0).name(), host.constData()),
                                QGenericArgument(addHost.parameterMetaType(1).name(), port.constData())),
                 "a Qt typedef parameter is normalised by moc to the same spelling the metatype uses, so the bridge's canonical name is accepted; a cstdint name is not normalised and fails earlier, at the metatype conversion");
        QVERIFY(udp->hostList().contains(QStringLiteral("127.0.0.2:14557")));
        udp->removeHost(QStringLiteral("127.0.0.2"), 14557);
    }

    SharedLinkConfigurationPtr config = LinkManager::instance()->addConfiguration(udp);
    const auto tearDown = qScopeGuard([&config]() {
        if (config->link()) {
            config->link()->disconnect();
        }
        LinkManager::instance()->removeConfiguration(config.get());
        qunsetenv("QGC_CORE_LINKS");
    });
    QVERIFY(LinkManager::instance()->createConnectedLink(config));
    QVERIFY(config->link());
    QVERIFY(qobject_cast<CoreLink *>(config->link()));

    const auto coreLocalPort = []() {
        const QJsonArray links = take(qgc_bridge_get("view.transports")).value(QStringLiteral("links")).toArray();
        for (const QJsonValue &link : links) {
            if (link.toObject().value(QStringLiteral("name")).toString() == QStringLiteral("Core UDP") && link.toObject().value(QStringLiteral("owner")).toString() == QStringLiteral("core")) {
                return link.toObject().value(QStringLiteral("localPort")).toInt(0);
            }
        }
        return 0;
    };
    QTRY_VERIFY_WITH_TIMEOUT(coreLocalPort() > 0, 3000);
    const int port = coreLocalPort();

    mavlink_message_t message{};
    mavlink_msg_heartbeat_pack(7, 1, &message, MAV_TYPE_QUADROTOR, MAV_AUTOPILOT_PX4, MAV_MODE_FLAG_CUSTOM_MODE_ENABLED, 0, MAV_STATE_STANDBY);
    uint8_t frame[MAVLINK_MAX_PACKET_LEN]{};
    const uint16_t len = mavlink_msg_to_send_buffer(frame, &message);
    const auto vehicleUp = []() { return take(qgc_bridge_get("vehicles.activeVehicleAvailable")).value(QStringLiteral("value")).toBool(false); };
    for (int attempt = 0; attempt < 30 && !vehicleUp(); ++attempt) {
        peer.writeDatagram(reinterpret_cast<const char *>(frame), len, QHostAddress::LocalHost, static_cast<quint16>(port));
        QTest::qWait(100);
    }
    QVERIFY2(vehicleUp(), "no vehicle appeared over the core-backed link");
    QTRY_VERIFY_WITH_TIMEOUT(peer.hasPendingDatagrams(), 3000);
    const QJsonObject core = take(qgc_bridge_get("view.coreVehicle(7)"));
    QCOMPARE(core.value(QStringLiteral("available")).toBool(false), true);
    QVERIFY(core.value(QStringLiteral("vehicle")).toObject().value(QStringLiteral("heartbeats")).toInt() > 0);
    QCOMPARE(core.value(QStringLiteral("vehicle")).toObject().value(QStringLiteral("id")).toInt(), 7);
    QVERIFY(core.value(QStringLiteral("vehicleIds")).toArray().contains(7));

    config->link()->disconnect();
    QTRY_VERIFY_WITH_TIMEOUT(!vehicleUp(), 10000);
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_coreGuidedTakeoffReachesThePeer()
{
#ifdef QGC_RUST_CORE
    qputenv("QGC_CORE_LINKS", "1");
    QUdpSocket peer;
    QVERIFY(peer.bind(QHostAddress::LocalHost, 0));
    UDPConfiguration *const udp = new UDPConfiguration(QStringLiteral("Core Guided UDP"));
    udp->setLocalPort(0);
    udp->addHost(QStringLiteral("127.0.0.1"), peer.localPort());
    udp->setDynamic(true);
    SharedLinkConfigurationPtr config = LinkManager::instance()->addConfiguration(udp);
    const auto tearDown = qScopeGuard([&config]() {
        if (config->link()) {
            config->link()->disconnect();
        }
        LinkManager::instance()->removeConfiguration(config.get());
        qunsetenv("QGC_CORE_LINKS");
    });
    QVERIFY(LinkManager::instance()->createConnectedLink(config));
    QVERIFY(qobject_cast<CoreLink *>(config->link()));

    const auto coreLocalPort = []() {
        const QJsonArray links = take(qgc_bridge_get("view.transports")).value(QStringLiteral("links")).toArray();
        for (const QJsonValue &link : links) {
            if (link.toObject().value(QStringLiteral("name")).toString() == QStringLiteral("Core Guided UDP") && link.toObject().value(QStringLiteral("owner")).toString() == QStringLiteral("core")) {
                return link.toObject().value(QStringLiteral("localPort")).toInt(0);
            }
        }
        return 0;
    };
    QTRY_VERIFY_WITH_TIMEOUT(coreLocalPort() > 0, 3000);
    const quint16 port = static_cast<quint16>(coreLocalPort());

    const auto send = [&peer, port](const mavlink_message_t &message) {
        uint8_t frame[MAVLINK_MAX_PACKET_LEN]{};
        const uint16_t len = mavlink_msg_to_send_buffer(frame, &message);
        peer.writeDatagram(reinterpret_cast<const char *>(frame), len, QHostAddress::LocalHost, port);
    };
    mavlink_message_t heartbeat{};
    mavlink_msg_heartbeat_pack(9, 1, &heartbeat, MAV_TYPE_QUADROTOR, MAV_AUTOPILOT_PX4, MAV_MODE_FLAG_CUSTOM_MODE_ENABLED, 0, MAV_STATE_STANDBY);
    mavlink_message_t position{};
    mavlink_msg_global_position_int_pack(9, 1, &position, 0, 474000000, 85000000, 500000, 0, 0, 0, 0, UINT16_MAX);
    const auto coreSeesVehicle = []() { return take(qgc_bridge_get("view.coreVehicle(9)")).value(QStringLiteral("available")).toBool(false); };
    for (int attempt = 0; attempt < 30 && !coreSeesVehicle(); ++attempt) {
        send(heartbeat);
        send(position);
        QTest::qWait(100);
    }
    QVERIFY2(coreSeesVehicle(), "the core did not see the vehicle");
    send(position);
    QTest::qWait(100);

    const QJsonObject started = take(qgc_core_guided("{\"vehicle\":9,\"action\":\"takeoff\",\"altitude\":15}"));
    QVERIFY2(started.value(QStringLiteral("ok")).toBool(false), qPrintable(started.value(QStringLiteral("reason")).toString()));

    mavlink_message_t parsing{};
    mavlink_status_t parsingStatus{};
    mavlink_message_t received{};
    mavlink_status_t status{};
    bool takeoffSeen = false;
    QElapsedTimer waitingForTakeoff;
    waitingForTakeoff.start();
    while (!takeoffSeen && waitingForTakeoff.elapsed() < 4000) {
        if (!peer.hasPendingDatagrams()) {
            QTest::qWait(20);
            continue;
        }
        const QByteArray datagram = peer.receiveDatagram().data();
        for (const char byte : datagram) {
            if (mavlink_frame_char_buffer(&parsing, &parsingStatus, static_cast<uint8_t>(byte), &received, &status) == MAVLINK_FRAMING_OK && received.msgid == MAVLINK_MSG_ID_COMMAND_LONG) {
                mavlink_command_long_t command{};
                mavlink_msg_command_long_decode(&received, &command);
                if (command.command == MAV_CMD_NAV_TAKEOFF) {
                    QCOMPARE(command.target_system, 9);
                    QCOMPARE(received.sysid, 255);
                    QCOMPARE(received.compid, MAV_COMP_ID_MISSIONPLANNER);
                    QCOMPARE(command.param7, 515.0f);
                    takeoffSeen = true;
                }
            }
        }
    }
    QVERIFY2(takeoffSeen, "no NAV_TAKEOFF reached the peer");
    const QJsonObject guided = take(qgc_bridge_get("view.coreGuided(9)"));
    QCOMPARE(guided.value(QStringLiteral("guided")).toObject().value(QStringLiteral("state")).toString(), QStringLiteral("done"));

    const QJsonObject refused = take(qgc_core_guided("{\"vehicle\":42,\"action\":\"land\"}"));
    QCOMPARE(refused.value(QStringLiteral("ok")).toBool(true), false);
    QVERIFY(!refused.value(QStringLiteral("reason")).toString().isEmpty());

    const QJsonObject calNoVehicle = take(qgc_core_calibrate("{\"vehicle\":42,\"action\":\"start\",\"type\":\"gyro\"}"));
    QCOMPARE(calNoVehicle.value(QStringLiteral("ok")).toBool(true), false);
    QVERIFY(!calNoVehicle.value(QStringLiteral("reason")).toString().isEmpty());
    const QJsonObject calNoType = take(qgc_core_calibrate("{\"vehicle\":9,\"action\":\"start\"}"));
    QCOMPARE(calNoType.value(QStringLiteral("ok")).toBool(true), false);
    const QJsonObject calStarted = take(qgc_core_calibrate("{\"vehicle\":9,\"action\":\"start\",\"type\":\"gyro\"}"));
    QVERIFY2(calStarted.value(QStringLiteral("ok")).toBool(false), qPrintable(calStarted.value(QStringLiteral("reason")).toString()));
    bool gyroSeen = false;
    QElapsedTimer waitingForGyro;
    waitingForGyro.start();
    while (!gyroSeen && waitingForGyro.elapsed() < 4000) {
        if (!peer.hasPendingDatagrams()) {
            QTest::qWait(20);
            continue;
        }
        const QByteArray datagram = peer.receiveDatagram().data();
        for (const char byte : datagram) {
            if (mavlink_frame_char_buffer(&parsing, &parsingStatus, static_cast<uint8_t>(byte), &received, &status) == MAVLINK_FRAMING_OK && received.msgid == MAVLINK_MSG_ID_COMMAND_LONG) {
                mavlink_command_long_t command{};
                mavlink_msg_command_long_decode(&received, &command);
                if (command.command == MAV_CMD_PREFLIGHT_CALIBRATION) {
                    QCOMPARE(command.target_system, 9);
                    QCOMPARE(command.param1, 1.0f);
                    gyroSeen = true;
                }
            }
        }
    }
    QVERIFY2(gyroSeen, "no PREFLIGHT_CALIBRATION reached the peer");
    const QJsonObject calRunning = take(qgc_bridge_get("view.coreCalibration(9)")).value(QStringLiteral("calibration")).toObject();
    QCOMPARE(calRunning.value(QStringLiteral("running")).toString(), QStringLiteral("gyro"));
    QVERIFY(take(qgc_core_calibrate("{\"vehicle\":9,\"action\":\"cancel\"}")).value(QStringLiteral("ok")).toBool(false));

    config->link()->disconnect();
    QTRY_VERIFY_WITH_TIMEOUT(!coreSeesVehicle(), 10000);
    QTRY_VERIFY_WITH_TIMEOUT(!MultiVehicleManager::instance()->activeVehicle(), 10000);
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_detectionsFollowTheRtspUrl()
{
#ifdef QGC_RUST_CORE
    const QString before = take(qgc_bridge_get("settings.videoSettings.rtspUrl.rawValue")).value(QStringLiteral("value")).toString();
    const auto restore = qScopeGuard([before]() {
        take(qgc_bridge_set("settings.videoSettings.rtspUrl", "{\"value\":\"\"}"));
        take(qgc_bridge_get("view.detections"));
        take(qgc_bridge_set("settings.videoSettings.rtspUrl", QJsonDocument(QJsonObject{{QStringLiteral("value"), before}}).toJson(QJsonDocument::Compact).constData()));
    });
    QVERIFY(take(qgc_bridge_set("settings.videoSettings.rtspUrl", "{\"value\":\"rtsp://127.0.0.1:8554/front/whep\"}")).value(QStringLiteral("ok")).toBool(false));
    const QJsonObject detections = take(qgc_bridge_get("view.detections"));
    QCOMPARE(detections.value(QStringLiteral("class")).toString(), QStringLiteral("Detections"));
    QCOMPARE(detections.value(QStringLiteral("available")).toBool(false), true);
    QCOMPARE(detections.value(QStringLiteral("host")).toString(), QStringLiteral("127.0.0.1"));
    QCOMPARE(detections.value(QStringLiteral("camera")).toString(), QStringLiteral("front"));
    QCOMPARE(detections.value(QStringLiteral("stale")).toBool(false), true);
    QVERIFY(detections.value(QStringLiteral("boxes")).toArray().isEmpty());
    QVERIFY(take(qgc_bridge_set("settings.videoSettings.rtspUrl", "{\"value\":\"\"}")).value(QStringLiteral("ok")).toBool(false));
    QCOMPARE(take(qgc_bridge_get("view.detections")).value(QStringLiteral("available")).toBool(true), false);
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_coreConnectSequenceReachesParameters()
{
#ifdef QGC_RUST_CORE
    qputenv("QGC_CORE_LINKS", "1");
    QUdpSocket peer;
    QVERIFY(peer.bind(QHostAddress::LocalHost, 0));
    UDPConfiguration *const udp = new UDPConfiguration(QStringLiteral("Core Connect UDP"));
    udp->setLocalPort(0);
    udp->addHost(QStringLiteral("127.0.0.1"), peer.localPort());
    udp->setDynamic(true);
    SharedLinkConfigurationPtr config = LinkManager::instance()->addConfiguration(udp);
    const auto tearDown = qScopeGuard([&config]() {
        if (config->link()) {
            config->link()->disconnect();
        }
        LinkManager::instance()->removeConfiguration(config.get());
        qunsetenv("QGC_CORE_LINKS");
    });
    QVERIFY(LinkManager::instance()->createConnectedLink(config));
    QVERIFY(qobject_cast<CoreLink *>(config->link()));
    const auto coreLocalPort = []() {
        const QJsonArray links = take(qgc_bridge_get("view.transports")).value(QStringLiteral("links")).toArray();
        for (const QJsonValue &link : links) {
            if (link.toObject().value(QStringLiteral("name")).toString() == QStringLiteral("Core Connect UDP") && link.toObject().value(QStringLiteral("owner")).toString() == QStringLiteral("core")) {
                return link.toObject().value(QStringLiteral("localPort")).toInt(0);
            }
        }
        return 0;
    };
    QTRY_VERIFY_WITH_TIMEOUT(coreLocalPort() > 0, 3000);
    const quint16 port = static_cast<quint16>(coreLocalPort());
    const auto send = [&peer, port](const mavlink_message_t &message) {
        uint8_t frame[MAVLINK_MAX_PACKET_LEN]{};
        const uint16_t len = mavlink_msg_to_send_buffer(frame, &message);
        peer.writeDatagram(reinterpret_cast<const char *>(frame), len, QHostAddress::LocalHost, port);
    };
    QList<mavlink_message_t> seen;
    const auto expectRequest = [&peer, &seen](int messageId, uint32_t requested) {
        const auto matches = [messageId, requested](const mavlink_message_t &message) {
            if (message.msgid != static_cast<uint32_t>(messageId)) {
                return false;
            }
            if (messageId == MAVLINK_MSG_ID_MISSION_REQUEST_LIST) {
                mavlink_mission_request_list_t list{};
                mavlink_msg_mission_request_list_decode(&message, &list);
                return list.mission_type == requested;
            }
            if (messageId != MAVLINK_MSG_ID_COMMAND_LONG) {
                return true;
            }
            mavlink_command_long_t command{};
            mavlink_msg_command_long_decode(&message, &command);
            return command.command == MAV_CMD_REQUEST_MESSAGE && static_cast<uint32_t>(command.param1) == requested;
        };
        mavlink_message_t parsing{};
        mavlink_status_t parsingStatus{};
        QElapsedTimer waited;
        waited.start();
        while (waited.elapsed() < 4000) {
            const int found = static_cast<int>(std::distance(seen.cbegin(), std::find_if(seen.cbegin(), seen.cend(), matches)));
            if (found < seen.count()) {
                seen.remove(0, found + 1);
                return true;
            }
            if (!peer.hasPendingDatagrams()) {
                QTest::qWait(20);
                continue;
            }
            const QByteArray datagram = peer.receiveDatagram().data();
            for (const char byte : datagram) {
                mavlink_message_t received{};
                mavlink_status_t status{};
                if (mavlink_frame_char_buffer(&parsing, &parsingStatus, static_cast<uint8_t>(byte), &received, &status) == MAVLINK_FRAMING_OK) {
                    seen.append(received);
                }
            }
        }
        return false;
    };

    mavlink_get_channel_status(MAVLINK_COMM_0)->flags &= ~MAVLINK_STATUS_FLAG_OUT_MAVLINK1;
    mavlink_message_t heartbeat{};
    mavlink_msg_heartbeat_pack(11, 1, &heartbeat, MAV_TYPE_QUADROTOR, MAV_AUTOPILOT_ARDUPILOTMEGA, MAV_MODE_FLAG_CUSTOM_MODE_ENABLED, 0, MAV_STATE_STANDBY);
    const auto coreSeesVehicle = []() { return take(qgc_bridge_get("view.coreVehicle(11)")).value(QStringLiteral("available")).toBool(false); };
    for (int attempt = 0; attempt < 30 && !coreSeesVehicle(); ++attempt) {
        send(heartbeat);
        QTest::qWait(100);
    }
    QVERIFY2(coreSeesVehicle(), "the core did not see the vehicle");
    QVERIFY2(expectRequest(MAVLINK_MSG_ID_COMMAND_LONG, MAVLINK_MSG_ID_AUTOPILOT_VERSION), "no autopilot version request reached the peer");

    mavlink_message_t version{};
    const uint8_t custom[8]{};
    const uint8_t uid2[18]{};
    const uint64_t capabilities = MAV_PROTOCOL_CAPABILITY_COMMAND_INT | MAV_PROTOCOL_CAPABILITY_MISSION_INT | MAV_PROTOCOL_CAPABILITY_MISSION_FENCE | MAV_PROTOCOL_CAPABILITY_MISSION_RALLY;
    mavlink_msg_autopilot_version_pack(11, 1, &version, capabilities, 0x04050600, 0, 0, 0, custom, custom, custom, 0, 0, 0, uid2);
    send(version);
    QVERIFY2(expectRequest(MAVLINK_MSG_ID_COMMAND_LONG, MAVLINK_MSG_ID_AVAILABLE_MODES), "no standard modes request reached the peer");
    mavlink_message_t ack{};
    mavlink_msg_command_ack_pack(11, 1, &ack, MAV_CMD_REQUEST_MESSAGE, MAV_RESULT_UNSUPPORTED, 0, 0, 255, MAV_COMP_ID_MISSIONPLANNER);
    send(ack);
    QVERIFY2(expectRequest(MAVLINK_MSG_ID_COMMAND_LONG, MAVLINK_MSG_ID_COMPONENT_METADATA), "no component metadata request reached the peer");
    send(ack);
    QVERIFY2(expectRequest(MAVLINK_MSG_ID_PARAM_REQUEST_LIST, 0), "no parameter list request reached the peer");
    mavlink_message_t value{};
    mavlink_msg_param_value_pack(11, 1, &value, "RTL_ALT", 1500.0f, MAV_PARAM_TYPE_REAL32, 2, 0);
    send(value);
    mavlink_msg_param_value_pack(11, 1, &value, "WPNAV_SPEED", 250.0f, MAV_PARAM_TYPE_REAL32, 2, 1);
    send(value);
    QVERIFY2(expectRequest(MAVLINK_MSG_ID_MISSION_REQUEST_LIST, MAV_MISSION_TYPE_MISSION), "no mission request list reached the peer");
    mavlink_message_t count{};
    mavlink_msg_mission_count_pack(11, 1, &count, 255, MAV_COMP_ID_MISSIONPLANNER, 0, MAV_MISSION_TYPE_MISSION, 0);
    send(count);
    QVERIFY2(expectRequest(MAVLINK_MSG_ID_MISSION_ACK, 0), "the empty mission was not acknowledged");
    const auto coreState = []() {
        const QJsonObject vehicle = take(qgc_bridge_get("view.coreVehicle(11)")).value(QStringLiteral("vehicle")).toObject();
        const QJsonObject plans = take(qgc_bridge_get("view.coreMission(11)")).value(QStringLiteral("plans")).toObject();
        const QJsonArray errors = take(qgc_bridge_get("view.coreGuided(11)")).value(QStringLiteral("guided")).toObject().value(QStringLiteral("errors")).toArray();
        return QStringLiteral("proto %1 caps %2 step %3 complete %4 plans %5 errors %6").arg(vehicle.value(QStringLiteral("maxProtoVersion")).toInt(-1)).arg(vehicle.value(QStringLiteral("capabilities")).toInt(-1)).arg(vehicle.value(QStringLiteral("connectStep")).toString()).arg(vehicle.value(QStringLiteral("initialConnectComplete")).toBool()).arg(QString::fromUtf8(QJsonDocument(plans).toJson(QJsonDocument::Compact))).arg(QString::fromUtf8(QJsonDocument(errors).toJson(QJsonDocument::Compact)));
    };
    const auto leftovers = [&seen]() {
        QStringList ids;
        for (const mavlink_message_t &message : std::as_const(seen)) {
            ids << QString::number(message.msgid) + QStringLiteral("/") + QString::number(message.len);
        }
        return ids.join(QLatin1Char(' '));
    };
    QVERIFY2(expectRequest(MAVLINK_MSG_ID_MISSION_REQUEST_LIST, MAV_MISSION_TYPE_FENCE), qPrintable(QStringLiteral("no fence request list reached the peer; heartbeat magic %1, %2, leftovers %3").arg(heartbeat.magic).arg(coreState()).arg(leftovers())));
    mavlink_msg_mission_count_pack(11, 1, &count, 255, MAV_COMP_ID_MISSIONPLANNER, 0, MAV_MISSION_TYPE_FENCE, 0);
    send(count);
    QVERIFY2(expectRequest(MAVLINK_MSG_ID_MISSION_REQUEST_LIST, MAV_MISSION_TYPE_RALLY), "no rally request list reached the peer");
    mavlink_msg_mission_count_pack(11, 1, &count, 255, MAV_COMP_ID_MISSIONPLANNER, 0, MAV_MISSION_TYPE_RALLY, 0);
    send(count);
    const auto connected = []() { return take(qgc_bridge_get("view.coreVehicle(11)")).value(QStringLiteral("vehicle")).toObject().value(QStringLiteral("initialConnectComplete")).toBool(false); };
    QTRY_VERIFY_WITH_TIMEOUT(connected(), 3000);
    const QJsonObject vehicle = take(qgc_bridge_get("view.coreVehicle(11)")).value(QStringLiteral("vehicle")).toObject();
    QCOMPARE(vehicle.value(QStringLiteral("capabilities")).toInt(), static_cast<int>(capabilities));
    QCOMPARE(vehicle.value(QStringLiteral("firmware")).toObject().value(QStringLiteral("version")).toString(), QStringLiteral("4.5.6 (0)"));
    QCOMPARE(vehicle.value(QStringLiteral("parameters")).toObject().value(QStringLiteral("count")).toInt(), 2);
    const QJsonObject parameter = take(qgc_bridge_get("view.coreParameter(11, RTL_ALT)"));
    QVERIFY2(parameter.value(QStringLiteral("available")).toBool(false), qPrintable(QString::fromUtf8(QJsonDocument(parameter).toJson(QJsonDocument::Compact))));
    QCOMPARE(parameter.value(QStringLiteral("value")).toDouble(), 1500.0);
    QCOMPARE(take(qgc_bridge_get("view.coreParameters(11)")).value(QStringLiteral("count")).toInt(), 2);

    const QJsonObject written = take(qgc_core_parameter("{\"vehicle\":11,\"name\":\"RTL_ALT\",\"value\":2000}"));
    QVERIFY2(written.value(QStringLiteral("ok")).toBool(false), qPrintable(written.value(QStringLiteral("reason")).toString()));
    QVERIFY2(expectRequest(MAVLINK_MSG_ID_PARAM_SET, 0), "no PARAM_SET reached the peer");
    mavlink_msg_param_value_pack(11, 1, &value, "RTL_ALT", 2000.0f, MAV_PARAM_TYPE_REAL32, 2, 0);
    send(value);
    const auto rtlAltitude = []() { return take(qgc_bridge_get("view.coreParameter(11, RTL_ALT)")).value(QStringLiteral("value")).toDouble(); };
    QTRY_COMPARE_WITH_TIMEOUT(rtlAltitude(), 2000.0, 3000);
    QCOMPARE(take(qgc_core_parameter("{\"vehicle\":11,\"name\":\"UNKNOWN\",\"value\":1}")).value(QStringLiteral("ok")).toBool(true), false);
    QCOMPARE(take(qgc_core_parameter("{\"vehicle\":11,\"name\":\"RTL_ALT\",\"value\":1e40}")).value(QStringLiteral("ok")).toBool(true), false);

    const QJsonObject writing = take(qgc_core_mission("{\"vehicle\":11,\"action\":\"write\",\"items\":[{\"frame\":0,\"command\":16,\"params\":[0,0,0,0,47.0,8.0,0]},{\"frame\":3,\"command\":16,\"params\":[0,0,0,0,47.2,8.2,60]}]}"));
    QVERIFY2(writing.value(QStringLiteral("ok")).toBool(false), qPrintable(writing.value(QStringLiteral("reason")).toString()));
    QVERIFY2(expectRequest(MAVLINK_MSG_ID_MISSION_COUNT, 0), "no mission count reached the peer");
    mavlink_message_t request{};
    mavlink_msg_mission_request_int_pack(11, 1, &request, 255, MAV_COMP_ID_MISSIONPLANNER, 0, MAV_MISSION_TYPE_MISSION);
    send(request);
    QVERIFY2(expectRequest(MAVLINK_MSG_ID_MISSION_ITEM_INT, 0), "the requested item did not reach the peer");
    mavlink_message_t accepted{};
    mavlink_msg_mission_ack_pack(11, 1, &accepted, 255, MAV_COMP_ID_MISSIONPLANNER, MAV_MISSION_ACCEPTED, MAV_MISSION_TYPE_MISSION, 0);
    send(accepted);
    const auto missionCount = []() { return take(qgc_bridge_get("view.coreMission(11)")).value(QStringLiteral("plans")).toObject().value(QStringLiteral("mission")).toObject().value(QStringLiteral("count")).toInt(-1); };
    QTRY_COMPARE_WITH_TIMEOUT(missionCount(), 1, 3000);

    config->link()->disconnect();
    QTRY_VERIFY_WITH_TIMEOUT(!coreSeesVehicle(), 10000);
    QTRY_VERIFY_WITH_TIMEOUT(!MultiVehicleManager::instance()->activeVehicle(), 10000);
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_replayedLogAgreesBetweenTheModels()
{
#ifdef QGC_RUST_CORE
    const QString sample = QFileInfo(QString::fromUtf8(__FILE__)).dir().filePath(QStringLiteral("../../mav.tlog"));
    QFile file(sample);
    QVERIFY(file.open(QIODevice::ReadOnly));
    const QByteArray bytes = file.readAll();

    const QJsonObject opened = take(qgc_core_host_link_open("logReplay", "tlog replay"));
    QVERIFY(opened.value(QStringLiteral("ok")).toBool(false));
    const uint32_t id = static_cast<uint32_t>(opened.value(QStringLiteral("id")).toInt());
    int logSystemId = -1;
    qsizetype at = 0;
    while (at + 8 < bytes.size()) {
        const qsizetype start = at + 8;
        const uint8_t stx = static_cast<uint8_t>(bytes[start]);
        qsizetype length = 0;
        if (stx == 0xFD && start + 2 < bytes.size()) {
            length = 12 + static_cast<uint8_t>(bytes[start + 1]) + ((static_cast<uint8_t>(bytes[start + 2]) & 1) ? 13 : 0);
        } else if (stx == 0xFE && start + 1 < bytes.size()) {
            length = 8 + static_cast<uint8_t>(bytes[start + 1]);
        } else {
            at++;
            continue;
        }
        if (start + length > bytes.size()) {
            break;
        }
        if (logSystemId < 0) {
            logSystemId = static_cast<uint8_t>(bytes[start + (stx == 0xFD ? 5 : 3)]);
        }
        qgc_core_host_link_bytes(id, reinterpret_cast<const uint8_t *>(bytes.constData() + start), static_cast<size_t>(length));
        at = start + length;
    }
    QVERIFY(logSystemId > 0);
    QJsonObject core;
    for (const QJsonValue &candidate : take(qgc_bridge_get("view.coreVehicle")).value(QStringLiteral("vehicleIds")).toArray()) {
        const QJsonObject vehicle = take(qgc_bridge_get(QStringLiteral("view.coreVehicle(%1)").arg(candidate.toInt()).toUtf8().constData())).value(QStringLiteral("vehicle")).toObject();
        if (vehicle.value(QStringLiteral("messagesReceived")).toInt() > core.value(QStringLiteral("messagesReceived")).toInt(0)) {
            core = vehicle;
        }
    }
    (void) qgc_core_host_link_closed(id, "replay complete");
    QVERIFY2(core.value(QStringLiteral("heartbeats")).toInt() > 0, qPrintable(QStringLiteral("first frame system %1, no core vehicle with heartbeats").arg(logSystemId)));
    const QJsonObject lingering = take(qgc_bridge_get(QStringLiteral("view.coreVehicle(%1)").arg(core.value(QStringLiteral("id")).toInt()).toUtf8().constData()));
    QVERIFY2(!lingering.value(QStringLiteral("available")).toBool(true), qPrintable(QStringLiteral("the core vehicle should leave with its link %1 but is still on link %2 among %3").arg(id).arg(lingering.value(QStringLiteral("vehicle")).toObject().value(QStringLiteral("link")).toInt()).arg(QString::fromUtf8(QJsonDocument(lingering.value(QStringLiteral("vehicleIds")).toArray()).toJson(QJsonDocument::Compact)))));

    const auto vehicleUp = []() { return take(qgc_bridge_get("vehicles.activeVehicleAvailable")).value(QStringLiteral("value")).toBool(false); };
    QTRY_VERIFY_WITH_TIMEOUT(!vehicleUp() && !MultiVehicleManager::instance()->activeVehicle(), 10000);
    LogReplayLink *const link = LinkManager::instance()->startLogReplay(sample);
    QVERIFY(link);
    QStringList errors;
    (void) connect(link, &LinkInterface::communicationError, this, [&errors](const QString &title, const QString &error) { errors.append(title + ": " + error); });
    const auto tearDown = qScopeGuard([link]() { link->disconnect(); });
    QTRY_VERIFY2_WITH_TIMEOUT(link->isConnected() || !errors.isEmpty(), qPrintable(errors.join("; ")), 10000);
    QVERIFY2(errors.isEmpty(), qPrintable(errors.join("; ")));
    link->setPlaybackSpeed(20.0);
    QTRY_VERIFY2_WITH_TIMEOUT(vehicleUp(), qPrintable(errors.join("; ")), 10000);
    QTRY_VERIFY_WITH_TIMEOUT(!link->isPlaying(), 60000);
    QTest::qWait(500);

    const auto factValue = [](const char *path) { return take(qgc_bridge_get(path)).value(QStringLiteral("value")).toDouble(); };
    const int qtMessages = take(qgc_bridge_get("vehicle.messagesReceived")).value(QStringLiteral("value")).toInt();
    const int coreMessages = core.value(QStringLiteral("messagesReceived")).toInt();
    QVERIFY2(qAbs(qtMessages - coreMessages) <= 2, qPrintable(QStringLiteral("messages qt %1 core %2").arg(qtMessages).arg(coreMessages)));
    const QJsonObject gps = core.value(QStringLiteral("gps")).toObject();
    QVERIFY(qAbs(factValue("vehicle.gps.lat") - gps.value(QStringLiteral("latitude")).toDouble()) < 1e-6);
    QVERIFY(qAbs(factValue("vehicle.gps.lon") - gps.value(QStringLiteral("longitude")).toDouble()) < 1e-6);
    const auto close = [](double a, double b, double tolerance, const char *what) {
        return qAbs(a - b) < tolerance ? QString() : QStringLiteral("%1 qt %2 core %3").arg(QString::fromUtf8(what)).arg(a).arg(b);
    };
    const QString altitude = close(factValue("vehicle.altitudeRelative"), core.value(QStringLiteral("altitudeRelative")).toDouble(), 0.01, "altitudeRelative");
    QVERIFY2(altitude.isEmpty(), qPrintable(altitude));
    const QString heading = close(factValue("vehicle.heading"), core.value(QStringLiteral("attitude")).toObject().value(QStringLiteral("heading")).toDouble(), 0.5, "heading");
    QVERIFY2(heading.isEmpty(), qPrintable(heading));
    const QString speed = close(factValue("vehicle.groundSpeed"), core.value(QStringLiteral("groundSpeed")).toDouble(), 0.01, "groundSpeed");
    QVERIFY2(speed.isEmpty(), qPrintable(speed));
    const QJsonObject qtCoordinate = take(qgc_bridge_get("vehicle.coordinate"));
    const QJsonObject coreCoordinate = core.value(QStringLiteral("coordinate")).toObject();
    const QString latitude = close(qtCoordinate.value(QStringLiteral("latitude")).toDouble(), coreCoordinate.value(QStringLiteral("latitude")).toDouble(), 1e-6, "coordinate.latitude");
    QVERIFY2(latitude.isEmpty(), qPrintable(latitude));
    QCOMPARE(take(qgc_bridge_get("vehicle.armed")).value(QStringLiteral("value")).toBool(!core.value(QStringLiteral("armed")).toBool()), core.value(QStringLiteral("armed")).toBool());
    const QString qtMode = take(qgc_bridge_get("vehicle.flightMode")).value(QStringLiteral("value")).toString();
    const QString modeContext = QStringLiteral("qt mode %1 (type %2 firmware %3 id %4) core mode %5 (type %6 autopilot %7 id %8 base %9 custom %10)")
                                    .arg(qtMode)
                                    .arg(take(qgc_bridge_get("vehicle.vehicleTypeString")).value(QStringLiteral("value")).toString())
                                    .arg(take(qgc_bridge_get("vehicle.firmwareTypeString")).value(QStringLiteral("value")).toString())
                                    .arg(factValue("vehicle.id"))
                                    .arg(core.value(QStringLiteral("flightMode")).toString())
                                    .arg(core.value(QStringLiteral("vehicleType")).toInt())
                                    .arg(core.value(QStringLiteral("autopilot")).toInt())
                                    .arg(core.value(QStringLiteral("id")).toInt())
                                    .arg(core.value(QStringLiteral("baseMode")).toInt())
                                    .arg(core.value(QStringLiteral("customMode")).toInt());
    QVERIFY2(qtMode == core.value(QStringLiteral("flightMode")).toString(), qPrintable(modeContext));
    const QJsonArray coreBatteries = core.value(QStringLiteral("batteries")).toArray();
    const QJsonArray qtBatteries = take(qgc_bridge_get("vehicle.batteries")).value(QStringLiteral("elements")).toArray();
    QCOMPARE(coreBatteries.count(), qtBatteries.count());
    if (!coreBatteries.isEmpty()) {
        const QString voltage = close(factValue("vehicle.batteries.0.voltage"), coreBatteries.first().toObject().value(QStringLiteral("voltage")).toDouble(), 0.01, "battery voltage");
        QVERIFY2(voltage.isEmpty(), qPrintable(voltage));
        const QString percent = close(factValue("vehicle.batteries.0.percentRemaining"), coreBatteries.first().toObject().value(QStringLiteral("percentRemaining")).toDouble(), 0.5, "battery percent");
        QVERIFY2(percent.isEmpty(), qPrintable(percent));
    }
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_videoAndCameraAreServed()
{
    const QJsonObject video = take(qgc_bridge_get("view.video"));
    QCOMPARE(video.value(QStringLiteral("class")).toString(), QStringLiteral("Video"));
    QVERIFY(!video.value(QStringLiteral("summary")).toString().isEmpty());
    QVERIFY(video.value(QStringLiteral("cameras")).isArray());
    const QJsonObject camera = take(qgc_bridge_get("view.camera"));
    QCOMPARE(camera.value(QStringLiteral("present")).toBool(true), false);
    QCOMPARE(camera.value(QStringLiteral("shotsText")).toString(), QStringLiteral("00000"));

    const QJsonObject enabled = take(qgc_bridge_invoke("settings.videoSettings.sourceEnabled", "[0]"));
    QVERIFY2(enabled.value(QStringLiteral("ok")).toBool(false), "the core asks the settings object whether a slot is enabled instead of comparing a translated status string");
    QVERIFY2(enabled.value(QStringLiteral("result")).isBool(), "sourceEnabled answers a bool; its value follows this machine's settings and is not asserted here");
    const QJsonObject configured = take(qgc_bridge_invoke("settings.videoSettings.sourceConfigured", "[0]"));
    QVERIFY2(configured.value(QStringLiteral("ok")).toBool(false), "sourceConfigured is reachable from the bridge");
    QVERIFY(configured.value(QStringLiteral("result")).isBool());
}

namespace
{

QJsonValue shapeOf(const QJsonValue &value)
{
    if (value.isObject()) {
        const QJsonObject object = value.toObject();
        QJsonObject described;
        for (auto it = object.begin(); it != object.end(); ++it) {
            described.insert(it.key(), shapeOf(it.value()));
        }
        return described;
    }
    if (value.isArray()) {
        const QJsonArray array = value.toArray();
        return QJsonArray { array.isEmpty() ? QJsonValue(QStringLiteral("empty")) : shapeOf(array.first()) };
    }
    return value.isNull() ? QStringLiteral("null") : value.isBool() ? QStringLiteral("bool") : value.isDouble() ? QStringLiteral("number") : QStringLiteral("string");
}

QJsonValue mergeShapes(const QJsonValue &a, const QJsonValue &b)
{
    if (a.isObject() && b.isObject()) {
        const QJsonObject left = a.toObject();
        const QJsonObject right = b.toObject();
        QJsonObject merged;
        for (auto it = left.begin(); it != left.end(); ++it) {
            merged.insert(it.key(), right.contains(it.key()) ? mergeShapes(it.value(), right.value(it.key())) : it.value());
        }
        for (auto it = right.begin(); it != right.end(); ++it) {
            if (!left.contains(it.key())) {
                merged.insert(it.key(), it.value());
            }
        }
        return merged;
    }
    if (a.isArray() && b.isArray()) {
        const QJsonValue first = a.toArray().first();
        const QJsonValue second = b.toArray().first();
        return QJsonArray { first.toString() == QStringLiteral("empty") ? second : second.toString() == QStringLiteral("empty") ? first : mergeShapes(first, second) };
    }
    if (a == b) {
        return a;
    }
    QStringList types = (a.toString() + QStringLiteral("|") + b.toString()).split(QLatin1Char('|'));
    types.removeDuplicates();
    types.sort();
    return types.join(QStringLiteral("|"));
}

constexpr int kMockStatusTextCount = 9;

const char *const kViewPaths[] = {
    "view.messages", "view.plan", "view.guidedActions", "view.guidedAltitude", "view.guidedAltitude(30)",
    "view.guidedTakeoff", "view.guidedTakeoff(10)", "view.guidedSpeed", "view.guidedSpeed(3)", "view.battery",
    "view.preflight", "view.warnings", "view.label(altitudeRelative)", "view.instruments", "view.vibration",
    "view.sensors", "view.control(settings.appSettings.audioMuted)", "view.links", "view.linkForm(udp,,14550)",
    "view.mapScale(120)", "view.terrainProfile", "view.missionKinds", "view.missionSeed(survey,47,8)",
    "view.calibration", "view.radio", "view.logs", "view.inspector", "view.flightModes", "view.settings",
    "view.settings(General)", "view.surveyStats(0)", "view.fences", "view.polygon", "view.setup",
    "view.setup(Safety)", "view.video", "view.camera", "view.detections", "view.coreCalibration", "view.flyState", "view.track",
    "view.altitudeModes", "view.altitudeModes(item,4)",
};

} // namespace

void QGCCoreCTest::_viewShapesMatchTheRecordedContract()
{
    QJsonObject offline;
    for (const char *path : kViewPaths) {
        offline.insert(QString::fromUtf8(path), shapeOf(take(qgc_bridge_get(path))));
    }

    _connectMockLink(MAV_AUTOPILOT_PX4);
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get("view.guidedActions")).value(QStringLiteral("connected")).toBool(false), 5000);
    _mockLink->sendStatusTextMessages();
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get("view.messages")).value(QStringLiteral("count")).toInt() >= kMockStatusTextCount, 5000);

    QJsonObject recorded;
    for (const char *path : kViewPaths) {
        const QString key = QString::fromUtf8(path);
        recorded.insert(key, mergeShapes(offline.value(key), shapeOf(take(qgc_bridge_get(path)))));
    }
    (void) take(qgc_bridge_invoke("plan.start", "[]"));
    const auto corner = [](double latitude, double longitude) { return QJsonObject { { QStringLiteral("latitude"), latitude }, { QStringLiteral("longitude"), longitude }, { QStringLiteral("altitude"), 0.0 } }; };
    const QByteArray box = QJsonDocument(QJsonArray { corner(47.398, 8.545), corner(47.396, 8.548) }).toJson(QJsonDocument::Compact);
    (void) take(qgc_bridge_invoke("plan.rallyPointController.addPoint", QJsonDocument(QJsonArray { corner(47.397, 8.546) }).toJson(QJsonDocument::Compact).constData()));
    (void) take(qgc_bridge_invoke("plan.geoFenceController.addInclusionPolygon", box.constData()));
    (void) take(qgc_bridge_invoke("plan.geoFenceController.addInclusionCircle", box.constData()));
    (void) take(qgc_bridge_invoke("plan.missionController.insertSimpleMissionItem", QJsonDocument(QJsonArray { corner(47.397, 8.546), 1, true }).toJson(QJsonDocument::Compact).constData()));
    (void) take(qgc_bridge_invoke("plan.missionController.insertSimpleMissionItem", QJsonDocument(QJsonArray { corner(47.3975, 8.5465), 2, true }).toJson(QJsonDocument::Compact).constData()));
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get("view.fences")).value(QStringLiteral("rallyPoints")).toArray().count() == 1, 5000);
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get("view.fences")).value(QStringLiteral("circles")).toArray().count() == 1, 5000);
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get("view.fences")).value(QStringLiteral("polygons")).toArray().count() == 1, 5000);
    (void) take(qgc_bridge_invoke("links.createAndConnectLink", QJsonDocument(QJsonArray { QStringLiteral("tcp"), QStringLiteral("Recorder TCP"), QStringLiteral("127.0.0.1"), 1 }).toJson(QJsonDocument::Compact).constData()));
    const auto recorderIndex = []() {
        const QJsonArray configured = take(qgc_bridge_get("view.links")).value(QStringLiteral("configured")).toArray();
        for (const QJsonValue &link : configured) {
            if (link.toObject().value(QStringLiteral("name")).toString() == QStringLiteral("Recorder TCP")) {
                return link.toObject().value(QStringLiteral("index")).toInt(-1);
            }
        }
        return -1;
    };
    QTRY_VERIFY_WITH_TIMEOUT(recorderIndex() >= 0, 5000);
    for (const char *path : kViewPaths) {
        const QString key = QString::fromUtf8(path);
        recorded.insert(key, mergeShapes(recorded.value(key), shapeOf(take(qgc_bridge_get(path)))));
    }
    (void) take(qgc_bridge_invoke("plan.removeAll", "[]"));
    const int recorder = recorderIndex();
    if (recorder >= 0) {
        (void) take(qgc_bridge_invoke("links.removeConfiguration", QJsonDocument(QJsonArray { QStringLiteral("@links.linkConfigurations.%1").arg(recorder) }).toJson(QJsonDocument::Compact).constData()));
    }
    recorded.insert(QStringLiteral("view.contract"), take(qgc_bridge_get("view.contract")));
    const QByteArray current = QJsonDocument(recorded).toJson(QJsonDocument::Indented);

    const QString fixture = QFileInfo(QString::fromUtf8(__FILE__)).dir().filePath(QStringLiteral("fixtures/view-shapes.json"));
    if (qEnvironmentVariableIsSet("QGC_RECORD_VIEW_CONTRACT")) {
        QFile out(fixture);
        QVERIFY(out.open(QIODevice::WriteOnly | QIODevice::Truncate));
        out.write(current);
        return;
    }

    QFile in(fixture);
    QVERIFY2(in.open(QIODevice::ReadOnly), "no recorded view contract; run with QGC_RECORD_VIEW_CONTRACT=1 once");
    const QJsonObject expected = QJsonDocument::fromJson(in.readAll()).object();
    QStringList keys;
    for (const char *path : kViewPaths) {
        keys.append(QString::fromUtf8(path));
    }
    keys.append(QStringLiteral("view.contract"));
    for (const QString &key : keys) {
        const QByteArray was = QJsonDocument(expected.value(key).toObject()).toJson(QJsonDocument::Compact);
        const QByteArray now = QJsonDocument(recorded.value(key).toObject()).toJson(QJsonDocument::Compact);
        QVERIFY2(was == now, qPrintable(QStringLiteral("%1 changed shape\n was: %2\n now: %3").arg(key, QString::fromUtf8(was), QString::fromUtf8(now))));
    }
}

namespace
{

QJsonObject coordinateJson(double latitude, double longitude)
{
    return QJsonObject { { QStringLiteral("latitude"), latitude }, { QStringLiteral("longitude"), longitude }, { QStringLiteral("altitude"), 0.0 } };
}

QJsonArray polygonSquare()
{
    return QJsonArray { coordinateJson(47.3960, 8.5440), coordinateJson(47.3960, 8.5480), coordinateJson(47.3990, 8.5480), coordinateJson(47.3990, 8.5440) };
}

QJsonArray polygonTriangle()
{
    return QJsonArray { coordinateJson(47.3960, 8.5440), coordinateJson(47.3960, 8.5500), coordinateJson(47.3995, 8.5470) };
}

QJsonArray polygonConcave()
{
    return QJsonArray {
        coordinateJson(47.3960, 8.5440), coordinateJson(47.3960, 8.5500), coordinateJson(47.3980, 8.5500),
        coordinateJson(47.3975, 8.5470), coordinateJson(47.3995, 8.5470), coordinateJson(47.3995, 8.5440),
    };
}

struct CameraCase {
    const char *name;
    double      focalLength;
    double      sensorWidth;
    double      sensorHeight;
    double      imageWidth;
    double      imageHeight;
    bool        landscape;
    double      frontalOverlap;
    double      sideOverlap;
    double      distanceToSurface;
};

struct CorridorCase {
    const char *name;
    QJsonArray  polyline;
    double      width;
    double      spacing;
    double      turnAround;
    int         entryRotations;
};

QJsonArray polylineStraight()
{
    return QJsonArray { coordinateJson(47.3960, 8.5440), coordinateJson(47.3990, 8.5440) };
}

QJsonArray polylineBent()
{
    return QJsonArray { coordinateJson(47.3960, 8.5440), coordinateJson(47.3975, 8.5470), coordinateJson(47.3995, 8.5470) };
}

struct SurveyCase {
    const char *name;
    QJsonArray  polygon;
    double      gridAngle;
    double      footprintSide;
    double      footprintFrontal;
    double      turnAround;
    bool        refly;
    bool        alternate;
    int         entryRotations;
};

QString roundedCoordinates(const QJsonArray &points)
{
    QStringList text;
    for (const QJsonValue &point : points) {
        const QJsonObject at = point.toObject();
        text.append(QStringLiteral("%1,%2")
                        .arg(at.value(QStringLiteral("latitude")).toDouble(), 0, 'f', 7)
                        .arg(at.value(QStringLiteral("longitude")).toDouble(), 0, 'f', 7));
    }
    return text.join(QStringLiteral(" "));
}

} // namespace

void QGCCoreCTest::_serialConfigurationsCanBeCreatedByPath()
{
#ifdef QGC_RUST_CORE
    const QJsonObject refusedName = take(qgc_bridge_invoke("links.createSerialConfiguration", "[\"\",\"/dev/nonexistent\",57600]"));
    QVERIFY2(refusedName.value(QStringLiteral("ok")).toBool(false), qPrintable(refusedName.value(QStringLiteral("reason")).toString()));
    QCOMPARE(refusedName.value(QStringLiteral("result")).toBool(true), false);

    const QJsonObject refusedBaud = take(qgc_bridge_invoke("links.createSerialConfiguration", "[\"Serial Test\",\"/dev/nonexistent\",0]"));
    QCOMPARE(refusedBaud.value(QStringLiteral("result")).toBool(true), false);

    QVERIFY2(take(qgc_bridge_invoke("links.commitLinkConfigurations", "[]")).value(QStringLiteral("ok")).toBool(false),
             "editing a registered configuration is property writes that stay in memory; without a reachable commit a head can change a link, watch it take effect, and lose it at restart");

    const QJsonObject missing = take(qgc_bridge_invoke("links.createSerialConfigurationTypo", "[\"Serial Test\",\"/dev/nonexistent\",57600]"));
    QCOMPARE(missing.value(QStringLiteral("ok")).toBool(true), false);
    QVERIFY2(!missing.contains(QStringLiteral("result")),
             "a method the bridge cannot find answers without a result, which is how the refusals above prove this one is reachable rather than absent");

    QVERIFY2(!refusedBaud.value(QStringLiteral("reason")).toString().contains(QStringLiteral("does not resolve")),
             "createSerialConfiguration must be reachable through the bridge; the four step flow it replaces passes a configuration pointer no head can hold");
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_surveyTransectsMatchTheRecordedOracle()
{
#ifdef QGC_RUST_CORE
    const QList<SurveyCase> cases = {
        { "square-0deg",            polygonSquare(),   0.0,   60.0, 40.0,  0.0, false , false, 0 },
        { "square-30deg",           polygonSquare(),  30.0,   60.0, 40.0,  0.0, false , false, 0 },
        { "square-90deg",           polygonSquare(),  90.0,   60.0, 40.0,  0.0, false , false, 0 },
        { "square-neg45deg",        polygonSquare(), -45.0,   60.0, 40.0,  0.0, false , false, 0 },
        { "square-turnaround",      polygonSquare(),   0.0,   60.0, 40.0, 30.0, false , false, 0 },
        { "square-tight-spacing",   polygonSquare(),   0.0,   25.0, 20.0,  0.0, false , false, 0 },
        { "triangle-0deg",          polygonTriangle(), 0.0,   60.0, 40.0,  0.0, false , false, 0 },
        { "triangle-45deg",         polygonTriangle(),45.0,   60.0, 40.0,  0.0, false , false, 0 },
        { "concave-0deg",           polygonConcave(),  0.0,   60.0, 40.0,  0.0, false , false, 0 },
        { "concave-0deg-refly",     polygonConcave(),  0.0,   60.0, 40.0,  0.0, true  , false, 0 },
        { "concave-60deg",          polygonConcave(), 60.0,   60.0, 40.0,  0.0, false , false, 0 },
        { "concave-60deg-refly",    polygonConcave(), 60.0,   60.0, 40.0,  0.0, true  , false, 0 },
        { "square-entry-1",         polygonSquare(),   0.0,   60.0, 40.0,  0.0, false, false, 1 },
        { "square-entry-2",         polygonSquare(),   0.0,   60.0, 40.0,  0.0, false, false, 2 },
        { "square-entry-3",         polygonSquare(),   0.0,   60.0, 40.0,  0.0, false, false, 3 },
        { "square-alternate",       polygonSquare(),   0.0,   60.0, 40.0,  0.0, false, true,  0 },
        { "concave-alternate",      polygonConcave(),  0.0,   60.0, 40.0,  0.0, false, true,  0 },
    };

    (void) take(qgc_bridge_invoke("plan.start", "[]"));
    const auto restore = []() { (void) take(qgc_bridge_invoke("plan.removeAll", "[]")); };
    const auto leaveNoPlanBehind = qScopeGuard(restore);
    restore();

    const QString item = QStringLiteral("plan.missionController.visualItems.1");
    const auto compact = [](const QJsonArray &array) { return QJsonDocument(array).toJson(QJsonDocument::Compact); };
    const auto setFact = [&](const QString &path, const QJsonValue &value) {
        const QByteArray body = QJsonDocument(QJsonObject { { QStringLiteral("value"), value } }).toJson(QJsonDocument::Compact);
        QVERIFY2(take(qgc_bridge_set(path.toUtf8().constData(), body.constData())).value(QStringLiteral("ok")).toBool(false), qPrintable(path));
        const double wanted = value.isBool() ? (value.toBool() ? 1.0 : 0.0) : value.toDouble();
        const QJsonValue back = take(qgc_bridge_get((path + QStringLiteral(".rawValue")).toUtf8().constData())).value(QStringLiteral("value"));
        const double readBack = back.isBool() ? (back.toBool() ? 1.0 : 0.0) : back.toDouble();
        QVERIFY2(qFuzzyCompare(readBack + 1.0, wanted + 1.0), qPrintable(QStringLiteral("%1 was set to %2 and reads back %3, so this case is not the case it is named after").arg(path).arg(wanted).arg(readBack)));
    };

    QJsonObject recorded;
    for (const SurveyCase &survey : cases) {
        restore();
        (void) take(qgc_bridge_invoke("plan.missionController.insertComplexMissionItem",
                                      compact(QJsonArray { QStringLiteral("Survey"), coordinateJson(47.3975, 8.5460), -1 }).constData()));
        QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get(item.toUtf8().constData())).value(QStringLiteral("kind")).toString() == QStringLiteral("object"), 5000);

        (void) take(qgc_bridge_invoke((item + QStringLiteral(".surveyAreaPolygon.clear")).toUtf8().constData(), "[]"));
        for (const QJsonValue &vertex : survey.polygon) {
            (void) take(qgc_bridge_invoke((item + QStringLiteral(".surveyAreaPolygon.appendVertex")).toUtf8().constData(),
                                          compact(QJsonArray { vertex }).constData()));
        }

        setFact(item + QStringLiteral(".cameraCalc.adjustedFootprintSide"), QJsonValue(survey.footprintSide));
        setFact(item + QStringLiteral(".cameraCalc.adjustedFootprintFrontal"), QJsonValue(survey.footprintFrontal));
        setFact(item + QStringLiteral(".turnAroundDistance"), QJsonValue(survey.turnAround));
        setFact(item + QStringLiteral(".refly90Degrees"), QJsonValue(survey.refly));
        setFact(item + QStringLiteral(".flyAlternateTransects"), QJsonValue(survey.alternate));
        for (int rotation = 0; rotation < survey.entryRotations; rotation++) {
            (void) take(qgc_bridge_invoke((item + QStringLiteral(".rotateEntryPoint")).toUtf8().constData(), "[]"));
        }
        const int entryPoint = take(qgc_bridge_get((item + QStringLiteral(".entryPoint")).toUtf8().constData())).value(QStringLiteral("value")).toInt(-1);
        QCOMPARE(entryPoint, survey.entryRotations);
        setFact(item + QStringLiteral(".gridAngle"), QJsonValue(survey.gridAngle));

        const auto points = [&]() { return take(qgc_bridge_get((item + QStringLiteral(".visualTransectPoints")).toUtf8().constData())).value(QStringLiteral("value")).toArray(); };
        QTRY_VERIFY_WITH_TIMEOUT(points().count() > 0, 5000);
        recorded.insert(QString::fromUtf8(survey.name), QJsonObject {
            { QStringLiteral("kind"), QStringLiteral("survey") },
            { QStringLiteral("polygon"), survey.polygon },
            { QStringLiteral("gridAngle"), survey.gridAngle },
            { QStringLiteral("gridSpacing"), survey.footprintSide },
            { QStringLiteral("frontalSpacing"), survey.footprintFrontal },
            { QStringLiteral("turnAround"), survey.turnAround },
            { QStringLiteral("refly"), survey.refly },
            { QStringLiteral("alternate"), survey.alternate },
            { QStringLiteral("entryPoint"), survey.entryRotations },
            { QStringLiteral("transects"), roundedCoordinates(points()) },
        });
    }
    restore();

    const QList<CorridorCase> corridors = {
        { "corridor-straight",       polylineStraight(), 120.0, 60.0,  0.0, 0 },
        { "corridor-straight-wide",  polylineStraight(), 300.0, 60.0,  0.0, 0 },
        { "corridor-straight-turn",  polylineStraight(), 120.0, 60.0, 30.0, 0 },
        { "corridor-bent",           polylineBent(),     120.0, 60.0,  0.0, 0 },
        { "corridor-bent-entry-1",   polylineBent(),     120.0, 60.0,  0.0, 1 },
        { "corridor-bent-entry-2",   polylineBent(),     120.0, 60.0,  0.0, 2 },
        { "corridor-bent-entry-3",   polylineBent(),     120.0, 60.0,  0.0, 3 },
        { "corridor-single",         polylineBent(),      40.0, 60.0,  0.0, 0 },
    };

    for (const CorridorCase &corridor : corridors) {
        restore();
        (void) take(qgc_bridge_invoke("plan.missionController.insertComplexMissionItem",
                                      compact(QJsonArray { QStringLiteral("Corridor Scan"), coordinateJson(47.3975, 8.5460), -1 }).constData()));
        QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get(item.toUtf8().constData())).value(QStringLiteral("kind")).toString() == QStringLiteral("object"), 5000);

        (void) take(qgc_bridge_invoke((item + QStringLiteral(".corridorPolyline.clear")).toUtf8().constData(), "[]"));
        for (const QJsonValue &vertex : corridor.polyline) {
            (void) take(qgc_bridge_invoke((item + QStringLiteral(".corridorPolyline.appendVertex")).toUtf8().constData(), compact(QJsonArray { vertex }).constData()));
        }

        setFact(item + QStringLiteral(".cameraCalc.adjustedFootprintSide"), QJsonValue(corridor.spacing));
        setFact(item + QStringLiteral(".corridorWidth"), QJsonValue(corridor.width));
        setFact(item + QStringLiteral(".turnAroundDistance"), QJsonValue(corridor.turnAround));
        for (int rotation = 0; rotation < corridor.entryRotations; rotation++) {
            (void) take(qgc_bridge_invoke((item + QStringLiteral(".rotateEntryPoint")).toUtf8().constData(), "[]"));
        }

        const auto points = [&]() { return take(qgc_bridge_get((item + QStringLiteral(".visualTransectPoints")).toUtf8().constData())).value(QStringLiteral("value")).toArray(); };
        QTRY_VERIFY_WITH_TIMEOUT(points().count() > 0, 5000);
        recorded.insert(QString::fromUtf8(corridor.name), QJsonObject {
            { QStringLiteral("polyline"), corridor.polyline },
            { QStringLiteral("corridorWidth"), corridor.width },
            { QStringLiteral("gridSpacing"), corridor.spacing },
            { QStringLiteral("turnAround"), corridor.turnAround },
            { QStringLiteral("entryPoint"), corridor.entryRotations },
        });
        recorded[QString::fromUtf8(corridor.name)] = QJsonObject {
            { QStringLiteral("kind"), QStringLiteral("corridor") },
            { QStringLiteral("polyline"), corridor.polyline },
            { QStringLiteral("corridorWidth"), corridor.width },
            { QStringLiteral("gridSpacing"), corridor.spacing },
            { QStringLiteral("turnAround"), corridor.turnAround },
            { QStringLiteral("entryPoint"), corridor.entryRotations },
            { QStringLiteral("transects"), roundedCoordinates(points()) },
        };
    }
    restore();

    const QList<CameraCase> cameras = {
        { "camera-landscape",       8.6,  13.2,  8.8, 5472.0, 3648.0, true,  70.0, 70.0, 50.0 },
        { "camera-portrait",        8.6,  13.2,  8.8, 5472.0, 3648.0, false, 70.0, 70.0, 50.0 },
        { "camera-low-overlap",     8.6,  13.2,  8.8, 5472.0, 3648.0, true,  20.0, 10.0, 50.0 },
        { "camera-high-altitude",   8.6,  13.2,  8.8, 5472.0, 3648.0, true,  70.0, 70.0, 120.0 },
        { "camera-long-lens",      24.0,  36.0, 24.0, 6000.0, 4000.0, true,  60.0, 60.0, 80.0 },
    };

    for (const CameraCase &camera : cameras) {
        restore();
        (void) take(qgc_bridge_invoke("plan.missionController.insertComplexMissionItem",
                                      compact(QJsonArray { QStringLiteral("Survey"), coordinateJson(47.3975, 8.5460), -1 }).constData()));
        QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get(item.toUtf8().constData())).value(QStringLiteral("kind")).toString() == QStringLiteral("object"), 5000);

        const QString calc = item + QStringLiteral(".cameraCalc");
        const QString custom = take(qgc_bridge_get((calc + QStringLiteral(".xlatCustomCameraName")).toUtf8().constData())).value(QStringLiteral("value")).toString();
        QVERIFY(!custom.isEmpty());
        const QByteArray named = QJsonDocument(QJsonObject { { QStringLiteral("value"), custom } }).toJson(QJsonDocument::Compact);
        QVERIFY2(take(qgc_bridge_set((calc + QStringLiteral(".cameraBrand")).toUtf8().constData(), named.constData())).value(QStringLiteral("ok")).toBool(false), "the custom camera is what unlocks the calculation");
        QCOMPARE(take(qgc_bridge_get((calc + QStringLiteral(".isManualCamera")).toUtf8().constData())).value(QStringLiteral("value")).toBool(true), false);

        setFact(calc + QStringLiteral(".valueSetIsDistance"), QJsonValue(true));
        setFact(calc + QStringLiteral(".focalLength"), QJsonValue(camera.focalLength));
        setFact(calc + QStringLiteral(".sensorWidth"), QJsonValue(camera.sensorWidth));
        setFact(calc + QStringLiteral(".sensorHeight"), QJsonValue(camera.sensorHeight));
        setFact(calc + QStringLiteral(".imageWidth"), QJsonValue(camera.imageWidth));
        setFact(calc + QStringLiteral(".imageHeight"), QJsonValue(camera.imageHeight));
        setFact(calc + QStringLiteral(".landscape"), QJsonValue(camera.landscape));
        setFact(calc + QStringLiteral(".frontalOverlap"), QJsonValue(camera.frontalOverlap));
        setFact(calc + QStringLiteral(".sideOverlap"), QJsonValue(camera.sideOverlap));
        setFact(calc + QStringLiteral(".distanceToSurface"), QJsonValue(camera.distanceToSurface));

        const auto readFact = [&](const QString &name) {
            return take(qgc_bridge_get((calc + QStringLiteral(".") + name + QStringLiteral(".rawValue")).toUtf8().constData())).value(QStringLiteral("value")).toDouble();
        };
        recorded[QString::fromUtf8(camera.name)] = QJsonObject {
            { QStringLiteral("kind"), QStringLiteral("camera") },
            { QStringLiteral("focalLength"), camera.focalLength },
            { QStringLiteral("sensorWidth"), camera.sensorWidth },
            { QStringLiteral("sensorHeight"), camera.sensorHeight },
            { QStringLiteral("imageWidth"), camera.imageWidth },
            { QStringLiteral("imageHeight"), camera.imageHeight },
            { QStringLiteral("landscape"), camera.landscape },
            { QStringLiteral("frontalOverlap"), camera.frontalOverlap },
            { QStringLiteral("sideOverlap"), camera.sideOverlap },
            { QStringLiteral("distanceToSurface"), camera.distanceToSurface },
            { QStringLiteral("imageDensity"), QString::number(readFact(QStringLiteral("imageDensity")), 'f', 7) },
            { QStringLiteral("adjustedFootprintSide"), QString::number(readFact(QStringLiteral("adjustedFootprintSide")), 'f', 7) },
            { QStringLiteral("adjustedFootprintFrontal"), QString::number(readFact(QStringLiteral("adjustedFootprintFrontal")), 'f', 7) },
        };
    }
    restore();

    const QList<QPair<QString, QPair<QJsonArray, double>>> offsets = {
        { QStringLiteral("offset-square-plus"),   { polygonSquare(),   40.0 } },
        { QStringLiteral("offset-square-minus"),  { polygonSquare(),  -40.0 } },
        { QStringLiteral("offset-triangle-plus"), { polygonTriangle(), 60.0 } },
        { QStringLiteral("offset-concave-plus"),  { polygonConcave(),  25.0 } },
        { QStringLiteral("offset-concave-minus"), { polygonConcave(), -25.0 } },
    };

    for (const auto &offset : offsets) {
        restore();
        (void) take(qgc_bridge_invoke("plan.missionController.insertComplexMissionItem",
                                      compact(QJsonArray { QStringLiteral("Survey"), coordinateJson(47.3975, 8.5460), -1 }).constData()));
        QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get(item.toUtf8().constData())).value(QStringLiteral("kind")).toString() == QStringLiteral("object"), 5000);

        const QString area = item + QStringLiteral(".surveyAreaPolygon");
        (void) take(qgc_bridge_invoke((area + QStringLiteral(".clear")).toUtf8().constData(), "[]"));
        for (const QJsonValue &vertex : offset.second.first) {
            (void) take(qgc_bridge_invoke((area + QStringLiteral(".appendVertex")).toUtf8().constData(), compact(QJsonArray { vertex }).constData()));
        }
        (void) take(qgc_bridge_invoke((area + QStringLiteral(".offset")).toUtf8().constData(), compact(QJsonArray { offset.second.second }).constData()));

        const QJsonArray moved = take(qgc_bridge_get((area + QStringLiteral(".path")).toUtf8().constData())).value(QStringLiteral("value")).toArray();
        QCOMPARE(moved.count(), offset.second.first.count());
        recorded[offset.first] = QJsonObject {
            { QStringLiteral("kind"), QStringLiteral("offset") },
            { QStringLiteral("polygon"), offset.second.first },
            { QStringLiteral("offset"), offset.second.second },
            { QStringLiteral("moved"), roundedCoordinates(moved) },
        };
    }
    restore();

    const QList<QPair<QString, SurveyCase>> plans = {
        { QStringLiteral("items-square-0deg"),       cases[0] },
        { QStringLiteral("items-square-turnaround"), cases[4] },
        { QStringLiteral("items-concave-0deg"),      cases[8] },
    };

    for (const auto &planned : plans) {
        const SurveyCase &survey = planned.second;
        restore();
        (void) take(qgc_bridge_invoke("plan.missionController.insertComplexMissionItem",
                                      compact(QJsonArray { QStringLiteral("Survey"), coordinateJson(47.3975, 8.5460), -1 }).constData()));
        QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get(item.toUtf8().constData())).value(QStringLiteral("kind")).toString() == QStringLiteral("object"), 5000);
        (void) take(qgc_bridge_invoke((item + QStringLiteral(".surveyAreaPolygon.clear")).toUtf8().constData(), "[]"));
        for (const QJsonValue &vertex : survey.polygon) {
            (void) take(qgc_bridge_invoke((item + QStringLiteral(".surveyAreaPolygon.appendVertex")).toUtf8().constData(), compact(QJsonArray { vertex }).constData()));
        }
        setFact(item + QStringLiteral(".cameraCalc.adjustedFootprintSide"), QJsonValue(survey.footprintSide));
        setFact(item + QStringLiteral(".cameraCalc.adjustedFootprintFrontal"), QJsonValue(survey.footprintFrontal));
        setFact(item + QStringLiteral(".cameraCalc.distanceToSurface"), QJsonValue(60.0));
        setFact(item + QStringLiteral(".turnAroundDistance"), QJsonValue(survey.turnAround));
        setFact(item + QStringLiteral(".gridAngle"), QJsonValue(survey.gridAngle));

        const QString planFile = QDir::temp().filePath(QStringLiteral("qgc-core-survey-%1.plan").arg(QCoreApplication::applicationPid()));
        QFile::remove(planFile);
        QVERIFY2(take(qgc_bridge_invoke("plan.saveToFile", compact(QJsonArray { planFile }).constData())).value(QStringLiteral("result")).toBool(false), qPrintable(planFile));
        QFile saved(planFile);
        QVERIFY(saved.open(QIODevice::ReadOnly));
        const QJsonObject plan = QJsonDocument::fromJson(saved.readAll()).object();
        saved.close();
        QFile::remove(planFile);

        const QJsonArray visual = plan.value(QStringLiteral("mission")).toObject().value(QStringLiteral("items")).toArray();
        QJsonArray built;
        for (const QJsonValue &entry : visual) {
            const QJsonObject complex = entry.toObject().value(QStringLiteral("TransectStyleComplexItem")).toObject();
            for (const QJsonValue &generated : complex.value(QStringLiteral("Items")).toArray()) {
                const QJsonObject mission = generated.toObject();
                built.append(QStringLiteral("%1 %2 %3")
                                 .arg(mission.value(QStringLiteral("command")).toInt())
                                 .arg(mission.value(QStringLiteral("frame")).toInt())
                                 .arg(QJsonDocument(mission.value(QStringLiteral("params")).toArray()).toJson(QJsonDocument::Compact).constData()));
            }
        }
        QVERIFY2(!built.isEmpty(), qPrintable(QStringLiteral("%1 generated no mission items").arg(planned.first)));
        recorded[planned.first] = QJsonObject {
            { QStringLiteral("kind"), QStringLiteral("items") },
            { QStringLiteral("polygon"), survey.polygon },
            { QStringLiteral("gridAngle"), survey.gridAngle },
            { QStringLiteral("gridSpacing"), survey.footprintSide },
            { QStringLiteral("turnAround"), survey.turnAround },
            { QStringLiteral("distanceToSurface"), 60.0 },
            { QStringLiteral("items"), built },
        };
    }
    restore();

    const QList<QPair<QString, CorridorCase>> corridorPlans = {
        { QStringLiteral("corridor-items-straight"), corridors[0] },
        { QStringLiteral("corridor-items-bent"),     corridors[3] },
        { QStringLiteral("corridor-items-turn"),     corridors[2] },
    };

    for (const auto &planned : corridorPlans) {
        const CorridorCase &corridor = planned.second;
        restore();
        (void) take(qgc_bridge_invoke("plan.missionController.insertComplexMissionItem",
                                      compact(QJsonArray { QStringLiteral("Corridor Scan"), coordinateJson(47.3975, 8.5460), -1 }).constData()));
        QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get(item.toUtf8().constData())).value(QStringLiteral("kind")).toString() == QStringLiteral("object"), 5000);
        (void) take(qgc_bridge_invoke((item + QStringLiteral(".corridorPolyline.clear")).toUtf8().constData(), "[]"));
        for (const QJsonValue &vertex : corridor.polyline) {
            (void) take(qgc_bridge_invoke((item + QStringLiteral(".corridorPolyline.appendVertex")).toUtf8().constData(), compact(QJsonArray { vertex }).constData()));
        }
        setFact(item + QStringLiteral(".cameraCalc.adjustedFootprintSide"), QJsonValue(corridor.spacing));
        setFact(item + QStringLiteral(".cameraCalc.adjustedFootprintFrontal"), QJsonValue(40.0));
        setFact(item + QStringLiteral(".cameraCalc.distanceToSurface"), QJsonValue(60.0));
        setFact(item + QStringLiteral(".corridorWidth"), QJsonValue(corridor.width));
        setFact(item + QStringLiteral(".turnAroundDistance"), QJsonValue(corridor.turnAround));

        const QString planFile = QDir::temp().filePath(QStringLiteral("qgc-core-corridor-%1.plan").arg(QCoreApplication::applicationPid()));
        QFile::remove(planFile);
        QVERIFY(take(qgc_bridge_invoke("plan.saveToFile", compact(QJsonArray { planFile }).constData())).value(QStringLiteral("result")).toBool(false));
        QFile saved(planFile);
        QVERIFY(saved.open(QIODevice::ReadOnly));
        const QJsonObject plan = QJsonDocument::fromJson(saved.readAll()).object();
        saved.close();
        QFile::remove(planFile);

        QJsonArray built;
        for (const QJsonValue &entry : plan.value(QStringLiteral("mission")).toObject().value(QStringLiteral("items")).toArray()) {
            const QJsonObject complex = entry.toObject().value(QStringLiteral("TransectStyleComplexItem")).toObject();
            for (const QJsonValue &generated : complex.value(QStringLiteral("Items")).toArray()) {
                const QJsonObject mission = generated.toObject();
                built.append(QStringLiteral("%1 %2 %3")
                                 .arg(mission.value(QStringLiteral("command")).toInt())
                                 .arg(mission.value(QStringLiteral("frame")).toInt())
                                 .arg(QJsonDocument(mission.value(QStringLiteral("params")).toArray()).toJson(QJsonDocument::Compact).constData()));
            }
        }
        QVERIFY2(!built.isEmpty(), qPrintable(planned.first));
        recorded[planned.first] = QJsonObject {
            { QStringLiteral("kind"), QStringLiteral("corridorItems") },
            { QStringLiteral("polyline"), corridor.polyline },
            { QStringLiteral("corridorWidth"), corridor.width },
            { QStringLiteral("gridSpacing"), corridor.spacing },
            { QStringLiteral("turnAround"), corridor.turnAround },
            { QStringLiteral("distanceToSurface"), 60.0 },
            { QStringLiteral("triggerDistance"), 40.0 },
            { QStringLiteral("items"), built },
        };
    }
    restore();

    const QString fixture = QFileInfo(QString::fromUtf8(__FILE__)).dir().filePath(QStringLiteral("fixtures/survey-transects.json"));
    const QByteArray current = QJsonDocument(recorded).toJson(QJsonDocument::Indented);
    if (qEnvironmentVariableIsSet("QGC_RECORD_VIEW_CONTRACT")) {
        QFile out(fixture);
        QVERIFY(out.open(QIODevice::WriteOnly | QIODevice::Truncate));
        out.write(current);
        return;
    }

    QFile in(fixture);
    QVERIFY2(in.open(QIODevice::ReadOnly), "no recorded survey oracle; run with QGC_RECORD_VIEW_CONTRACT=1 once");
    const QJsonObject expected = QJsonDocument::fromJson(in.readAll()).object();
    QStringList names;
    for (const SurveyCase &survey : cases) {
        names.append(QString::fromUtf8(survey.name));
    }
    for (const CorridorCase &corridor : corridors) {
        names.append(QString::fromUtf8(corridor.name));
    }
    for (const CameraCase &camera : cameras) {
        names.append(QString::fromUtf8(camera.name));
    }
    for (const auto &offset : offsets) {
        names.append(offset.first);
    }
    for (const auto &planned : plans) {
        names.append(planned.first);
    }
    for (const auto &planned : corridorPlans) {
        names.append(planned.first);
    }
    for (const QString &key : names) {
        QVERIFY2(expected.contains(key), qPrintable(key));
        const QByteArray was = QJsonDocument(expected.value(key).toObject()).toJson(QJsonDocument::Compact);
        const QByteArray now = QJsonDocument(recorded.value(key).toObject()).toJson(QJsonDocument::Compact);
        QVERIFY2(was == now, qPrintable(QStringLiteral("%1 changed\n was: %2\n now: %3").arg(key, QString::fromUtf8(was), QString::fromUtf8(now))));
    }
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_tlogSummaryDecodesTheSampleLog()
{
    const QString sample = QFileInfo(QString::fromUtf8(__FILE__)).dir().filePath(QStringLiteral("../../mav.tlog"));
    const QJsonObject summary = take(qgc_bridge_get(QStringLiteral("view.tlog(%1)").arg(sample).toUtf8().constData()));
    QCOMPARE(summary.value(QStringLiteral("readable")).toBool(false), true);
    QVERIFY(summary.value(QStringLiteral("frames")).toInt() > 1000);
    QVERIFY(summary.value(QStringLiteral("byName")).toObject().contains(QStringLiteral("HEARTBEAT")));

    QFile file(sample);
    QVERIFY(file.open(QIODevice::ReadOnly));
    const QByteArray bytes = file.readAll();
    const uint8_t channel = MAVLINK_COMM_NUM_BUFFERS - 1;
    mavlink_reset_channel_status(channel);
    int cFrames = 0;
    int cHeartbeats = 0;
    int cDropped = 0;
    qsizetype at = 0;
    while (at + 8 < bytes.size()) {
        at += 8;
        bool found = false;
        while (!found && at < bytes.size()) {
            mavlink_message_t message{};
            mavlink_status_t status{};
            found = mavlink_parse_char(channel, static_cast<uint8_t>(bytes[at++]), &message, &status) == MAVLINK_FRAMING_OK;
            cDropped += status.packet_rx_drop_count;
            if (found) {
                cFrames++;
                cHeartbeats += (message.msgid == MAVLINK_MSG_ID_HEARTBEAT);
            }
        }
    }
    QCOMPARE(summary.value(QStringLiteral("frames")).toInt(), cFrames);
    QCOMPARE(summary.value(QStringLiteral("byName")).toObject().value(QStringLiteral("HEARTBEAT")).toInt(), cHeartbeats);
    QCOMPARE(summary.value(QStringLiteral("undecodable")).toInt(), cDropped);
    QCOMPARE(take(qgc_bridge_get("view.tlog(/nonexistent.tlog)")).value(QStringLiteral("readable")).toBool(true), false);
    QCOMPARE(take(qgc_bridge_get("view.tlog")).value(QStringLiteral("kind")).toString(), QStringLiteral("null"));
}

void QGCCoreCTest::_planFileAgreesWithTheCppLoader()
{
    const QString fixture = QFileInfo(QString::fromUtf8(__FILE__)).dir().filePath(QStringLiteral("../MissionManager/SectionTest.plan"));
    const QJsonObject read = take(qgc_bridge_get(QStringLiteral("view.planFile(%1)").arg(fixture).toUtf8().constData()));
    QCOMPARE(read.value(QStringLiteral("valid")).toBool(false), true);
    const int rustCount = read.value(QStringLiteral("itemCount")).toInt();
    QVERIFY(rustCount > 0);

    (void) take(qgc_bridge_invoke("plan.start", "[]"));
    const QJsonObject loaded = take(qgc_bridge_invoke("plan.loadFromFile", QJsonDocument(QJsonArray { fixture }).toJson(QJsonDocument::Compact).constData()));
    QVERIFY2(loaded.value(QStringLiteral("ok")).toBool(false) && loaded.value(QStringLiteral("result")).toBool(false), qPrintable(QStringLiteral("the C++ loader refused the fixture: %1").arg(QString::fromUtf8(QJsonDocument(loaded).toJson(QJsonDocument::Compact)))));
    const auto lastSequence = []() {
        const QJsonArray elements = take(qgc_bridge_get_fields("plan.missionController.visualItems", "lastSequenceNumber")).value(QStringLiteral("elements")).toArray();
        return elements.isEmpty() ? -1 : elements.last().toObject().value(QStringLiteral("lastSequenceNumber")).toInt(-1);
    };
    QTRY_COMPARE_WITH_TIMEOUT(lastSequence(), rustCount, 5000);
    const QJsonObject home = take(qgc_bridge_get("plan.missionController.plannedHomePosition"));
    QVERIFY(qAbs(home.value(QStringLiteral("latitude")).toDouble() - read.value(QStringLiteral("home")).toObject().value(QStringLiteral("latitude")).toDouble()) < 1e-6);
    (void) take(qgc_bridge_invoke("plan.removeAll", "[]"));
    QCOMPARE(take(qgc_bridge_get("view.planFile(/nonexistent.plan)")).value(QStringLiteral("readable")).toBool(true), false);
}

void QGCCoreCTest::_planWrittenFromWaypointsLoadsInCpp()
{
    const QString fixture = QFileInfo(QString::fromUtf8(__FILE__)).dir().filePath(QStringLiteral("../MissionManager/MissionPlanner.waypoints"));
    const QJsonObject converted = take(qgc_bridge_get(QStringLiteral("view.planFromWaypoints(%1)").arg(fixture).toUtf8().constData()));
    QCOMPARE(converted.value(QStringLiteral("valid")).toBool(false), true);
    const int rustCount = converted.value(QStringLiteral("itemCount")).toInt();
    QVERIFY(rustCount > 0);
    QTemporaryFile written(QDir::tempPath() + QStringLiteral("/core-written-XXXXXX.plan"));
    QVERIFY(written.open());
    written.write(converted.value(QStringLiteral("plan")).toString().toUtf8());
    written.close();

    (void) take(qgc_bridge_invoke("plan.start", "[]"));
    const QJsonObject loaded = take(qgc_bridge_invoke("plan.loadFromFile", QJsonDocument(QJsonArray { written.fileName() }).toJson(QJsonDocument::Compact).constData()));
    QVERIFY2(loaded.value(QStringLiteral("ok")).toBool(false) && loaded.value(QStringLiteral("result")).toBool(false), qPrintable(QStringLiteral("the C++ loader refused the written plan: %1").arg(QString::fromUtf8(QJsonDocument(loaded).toJson(QJsonDocument::Compact)))));
    const auto lastSequence = []() {
        const QJsonArray elements = take(qgc_bridge_get_fields("plan.missionController.visualItems", "lastSequenceNumber")).value(QStringLiteral("elements")).toArray();
        return elements.isEmpty() ? -1 : elements.last().toObject().value(QStringLiteral("lastSequenceNumber")).toInt(-1);
    };
    QTRY_COMPARE_WITH_TIMEOUT(lastSequence(), rustCount, 5000);
    const QJsonObject home = take(qgc_bridge_get("plan.missionController.plannedHomePosition"));
    const QJsonObject read = take(qgc_bridge_get(QStringLiteral("view.waypointsFile(%1)").arg(fixture).toUtf8().constData()));
    QVERIFY(qAbs(home.value(QStringLiteral("latitude")).toDouble() - read.value(QStringLiteral("home")).toObject().value(QStringLiteral("latitude")).toDouble()) < 1e-6);
    (void) take(qgc_bridge_invoke("plan.removeAll", "[]"));
    QCOMPARE(take(qgc_bridge_get("view.planFromWaypoints(/nonexistent.waypoints)")).value(QStringLiteral("readable")).toBool(true), false);
}

void QGCCoreCTest::_missionFileAgreesWithTheCppLoader()
{
    const QString fixture = QFileInfo(QString::fromUtf8(__FILE__)).dir().filePath(QStringLiteral("../MissionManager/OldFileFormat.mission"));
    const QJsonObject read = take(qgc_bridge_get(QStringLiteral("view.missionFile(%1)").arg(fixture).toUtf8().constData()));
    QCOMPARE(read.value(QStringLiteral("valid")).toBool(false), true);
    const int rustCount = read.value(QStringLiteral("itemCount")).toInt();
    QVERIFY(rustCount > 0);
    const auto lastSequence = []() {
        const QJsonArray elements = take(qgc_bridge_get_fields("plan.missionController.visualItems", "lastSequenceNumber")).value(QStringLiteral("elements")).toArray();
        return elements.isEmpty() ? -1 : elements.last().toObject().value(QStringLiteral("lastSequenceNumber")).toInt(-1);
    };
    const auto homeLatitude = []() { return take(qgc_bridge_get("plan.missionController.plannedHomePosition")).value(QStringLiteral("latitude")).toDouble(); };
    const double rustHome = read.value(QStringLiteral("home")).toObject().value(QStringLiteral("latitude")).toDouble();

    (void) take(qgc_bridge_invoke("plan.start", "[]"));
    const QJsonObject loaded = take(qgc_bridge_invoke("plan.loadFromFile", QJsonDocument(QJsonArray { fixture }).toJson(QJsonDocument::Compact).constData()));
    QVERIFY2(loaded.value(QStringLiteral("result")).toBool(false), "the C++ loader refused the legacy mission fixture");
    QTRY_COMPARE_WITH_TIMEOUT(lastSequence(), rustCount, 5000);
    QVERIFY(qAbs(homeLatitude() - rustHome) < 1e-6);
    (void) take(qgc_bridge_invoke("plan.removeAll", "[]"));

    QTemporaryFile written(QDir::tempPath() + QStringLiteral("/core-mission-XXXXXX.plan"));
    QVERIFY(written.open());
    written.write(read.value(QStringLiteral("plan")).toString().toUtf8());
    written.close();
    const QJsonObject reloaded = take(qgc_bridge_invoke("plan.loadFromFile", QJsonDocument(QJsonArray { written.fileName() }).toJson(QJsonDocument::Compact).constData()));
    QVERIFY2(reloaded.value(QStringLiteral("ok")).toBool(false) && reloaded.value(QStringLiteral("result")).toBool(false), qPrintable(QStringLiteral("the C++ loader refused the rewritten mission: %1").arg(QString::fromUtf8(QJsonDocument(reloaded).toJson(QJsonDocument::Compact)))));
    QTRY_COMPARE_WITH_TIMEOUT(lastSequence(), rustCount, 5000);
    QVERIFY(qAbs(homeLatitude() - rustHome) < 1e-6);
    (void) take(qgc_bridge_invoke("plan.removeAll", "[]"));
    QCOMPARE(take(qgc_bridge_get("view.missionFile(/nonexistent.mission)")).value(QStringLiteral("readable")).toBool(true), false);
}

void QGCCoreCTest::_waypointsFileAgreesWithTheCppLoader()
{
    const QString fixture = QFileInfo(QString::fromUtf8(__FILE__)).dir().filePath(QStringLiteral("../MissionManager/MissionPlanner.waypoints"));
    const QJsonObject read = take(qgc_bridge_get(QStringLiteral("view.waypointsFile(%1)").arg(fixture).toUtf8().constData()));
    QCOMPARE(read.value(QStringLiteral("valid")).toBool(false), true);
    QCOMPARE(read.value(QStringLiteral("homeInFile")).toBool(false), true);
    const int rustCount = read.value(QStringLiteral("itemCount")).toInt();
    QVERIFY(rustCount > 0);

    (void) take(qgc_bridge_invoke("plan.start", "[]"));
    const QJsonObject loaded = take(qgc_bridge_invoke("plan.loadFromFile", QJsonDocument(QJsonArray { fixture }).toJson(QJsonDocument::Compact).constData()));
    QVERIFY2(loaded.value(QStringLiteral("result")).toBool(false), "the C++ text loader refused the fixture");
    const auto lastSequence = []() {
        const QJsonArray elements = take(qgc_bridge_get_fields("plan.missionController.visualItems", "lastSequenceNumber")).value(QStringLiteral("elements")).toArray();
        return elements.isEmpty() ? -1 : elements.last().toObject().value(QStringLiteral("lastSequenceNumber")).toInt(-1);
    };
    QTRY_COMPARE_WITH_TIMEOUT(lastSequence(), rustCount, 5000);
    const QJsonObject home = take(qgc_bridge_get("plan.missionController.plannedHomePosition"));
    QVERIFY(qAbs(home.value(QStringLiteral("latitude")).toDouble() - read.value(QStringLiteral("home")).toObject().value(QStringLiteral("latitude")).toDouble()) < 1e-6);
    (void) take(qgc_bridge_invoke("plan.removeAll", "[]"));
}

void QGCCoreCTest::_kmlFilesFollowTheMapPolygonTest()
{
    const QDir fixtures = QFileInfo(QString::fromUtf8(__FILE__)).dir();
    const auto read = [&fixtures](const char *name) { return take(qgc_bridge_get(QStringLiteral("view.kmlFile(%1)").arg(fixtures.filePath(QStringLiteral("../MissionManager/") + QString::fromUtf8(name))).toUtf8().constData())); };
    const QJsonObject good = read("PolygonGood.kml");
    QCOMPARE(good.value(QStringLiteral("valid")).toBool(false), true);
    QCOMPARE(good.value(QStringLiteral("shape")).toString(), QStringLiteral("polygon"));
    QVERIFY(good.value(QStringLiteral("count")).toInt() >= 4);
    QCOMPARE(read("PolygonBadXml.kml").value(QStringLiteral("valid")).toBool(true), false);
    QCOMPARE(read("PolygonMissingNode.kml").value(QStringLiteral("valid")).toBool(true), false);
    QCOMPARE(read("PolygonBadCoordinatesNode.kml").value(QStringLiteral("valid")).toBool(true), false);
}

void QGCCoreCTest::_shapeFilesFollowShapeTest()
{
    const QDir fixtures = QFileInfo(QString::fromUtf8(__FILE__)).dir();
    const auto read = [&fixtures](const char *name) { return take(qgc_bridge_get(QStringLiteral("view.shapeFile(%1)").arg(fixtures.filePath(QStringLiteral("../Utilities/Shape/") + QString::fromUtf8(name))).toUtf8().constData())); };
    const QJsonObject polygon = read("polygon.shp");
    QCOMPARE(polygon.value(QStringLiteral("valid")).toBool(false), true);
    QCOMPARE(polygon.value(QStringLiteral("shape")).toString(), QStringLiteral("polygon"));
    QVERIFY(polygon.value(QStringLiteral("count")).toInt() >= 3);
    const QJsonObject line = read("pline.shp");
    QCOMPARE(line.value(QStringLiteral("shape")).toString(), QStringLiteral("polyline"));
    QCOMPARE(read("polygon.kml").value(QStringLiteral("valid")).toBool(true), false);
}

void QGCCoreCTest::_geoConversionsMatchGeoTest()
{
    const QJsonObject ned = take(qgc_bridge_get("view.geoToNed(47.364869,8.594398,0,47.3764,8.5481,0)"));
    QVERIFY(qAbs(ned.value(QStringLiteral("x")).toDouble() - -1282.58731618) < 0.00001);
    QVERIFY(qAbs(ned.value(QStringLiteral("y")).toDouble() - 3490.85591324) < 0.00001);
    const QJsonObject utm = take(qgc_bridge_get("view.geoToUtm(47.3764,8.5481)"));
    QCOMPARE(utm.value(QStringLiteral("zone")).toInt(), 32);
    QVERIFY(qAbs(utm.value(QStringLiteral("easting")).toDouble() - 465886.092246) < 0.01);
    const QJsonObject back = take(qgc_bridge_get("view.utmToGeo(465886.092246,5247092.44892,32)"));
    QVERIFY(qAbs(back.value(QStringLiteral("latitude")).toDouble() - 47.3764) < 0.00001);
    QCOMPARE(take(qgc_bridge_get("view.nedToGeo(1,2)")).value(QStringLiteral("kind")).toString(), QStringLiteral("null"));
}

void QGCCoreCTest::_terrainTileNeedsAFile()
{
    QCOMPARE(take(qgc_bridge_get("view.terrainTile")).value(QStringLiteral("kind")).toString(), QStringLiteral("null"));
    QCOMPARE(take(qgc_bridge_get("view.terrainTile(/nonexistent.tile)")).value(QStringLiteral("readable")).toBool(true), false);
}

void QGCCoreCTest::_mapProvidersMatchTheRecordedHashes()
{
    QJsonObject providers;
    for (const QString &name : UrlFactory::getProviderTypes()) {
        providers[name] = UrlFactory::hashFromProviderType(name);
    }
    QVERIFY2(!providers.isEmpty(), "the url factory knows no map providers");

    QJsonObject samples;
    const QList<QList<int>> tiles = { { 0, 0, 0 }, { 1, 2, 3 }, { 8523, 5606, 14 }, { 2147483647, 99999999, 22 } };
    for (const QList<int> &tile : tiles) {
        for (const QString &name : UrlFactory::getProviderTypes()) {
            samples[QStringLiteral("%1 %2/%3/%4").arg(name).arg(tile[0]).arg(tile[1]).arg(tile[2])] =
                UrlFactory::getTileHash(name, tile[0], tile[1], tile[2]);
        }
    }

    const QJsonObject recorded { { QStringLiteral("providers"), providers }, { QStringLiteral("tileHashes"), samples } };
    const QString fixture = QFileInfo(QString::fromUtf8(__FILE__)).dir().filePath(QStringLiteral("fixtures/tile-providers.json"));
    if (qEnvironmentVariableIsSet("QGC_RECORD_VIEW_CONTRACT")) {
        QFile out(fixture);
        QVERIFY2(out.open(QIODevice::WriteOnly | QIODevice::Text), qPrintable(fixture));
        out.write(QJsonDocument(recorded).toJson(QJsonDocument::Indented));
        out.close();
        QSKIP("recorded the map provider hashes");
    }

    QFile in(fixture);
    QVERIFY2(in.open(QIODevice::ReadOnly), "no recorded provider hashes; run with QGC_RECORD_VIEW_CONTRACT=1 once");
    const QJsonObject expected = QJsonDocument::fromJson(in.readAll()).object();
    in.close();

    const QJsonObject expectedProviders = expected.value(QStringLiteral("providers")).toObject();
    QStringList missing;
    for (const QString &name : providers.keys()) {
        if (!expectedProviders.contains(name)) {
            missing.append(name);
        }
    }
    QVERIFY2(missing.isEmpty(), qPrintable(QStringLiteral("map providers with no recorded hash, so the Rust cache cannot key their tiles: %1").arg(missing.join(QStringLiteral(", ")))));
    QCOMPARE(QJsonDocument(providers).toJson(QJsonDocument::Compact), QJsonDocument(expectedProviders).toJson(QJsonDocument::Compact));
    QCOMPARE(QJsonDocument(samples).toJson(QJsonDocument::Compact), QJsonDocument(expected.value(QStringLiteral("tileHashes")).toObject()).toJson(QJsonDocument::Compact));
}

void QGCCoreCTest::_theTileCacheSchemaMatchesTheRecordedOne()
{
    const QString databasePath = QDir::temp().filePath(QStringLiteral("qgc-core-tilecache-%1.db").arg(QCoreApplication::applicationPid()));
    QFile::remove(databasePath);
    QGCMapEngine::instance()->init(databasePath);

    QTRY_VERIFY_WITH_TIMEOUT(QFileInfo::exists(databasePath) && QFileInfo(databasePath).size() > 0, 10000);

    QJsonObject schema;
    {
        QSqlDatabase database = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), QStringLiteral("tileCacheSchemaProbe"));
        database.setDatabaseName(databasePath);
        QVERIFY2(database.open(), qPrintable(database.lastError().text()));
        QSqlQuery query(database);
        QVERIFY(query.exec(QStringLiteral("SELECT type, name, sql FROM sqlite_master WHERE sql IS NOT NULL ORDER BY name")));
        while (query.next()) {
            schema[query.value(1).toString()] = QStringLiteral("%1 %2").arg(query.value(0).toString(), query.value(2).toString());
        }
        QSqlQuery sets(database);
        QVERIFY(sets.exec(QStringLiteral("SELECT name, defaultSet FROM TileSets ORDER BY name")));
        QJsonObject recordedSets;
        while (sets.next()) {
            recordedSets[sets.value(0).toString()] = sets.value(1).toInt();
        }
        schema[QStringLiteral("_tileSets")] = QJsonDocument(recordedSets).toJson(QJsonDocument::Compact).constData();
        database.close();
    }
    QSqlDatabase::removeDatabase(QStringLiteral("tileCacheSchemaProbe"));
    QFile::remove(databasePath);

    QVERIFY2(schema.contains(QStringLiteral("Tiles")), "the map engine did not create the tile cache");

    const QString fixture = QFileInfo(QString::fromUtf8(__FILE__)).dir().filePath(QStringLiteral("fixtures/tile-cache-schema.json"));
    if (qEnvironmentVariableIsSet("QGC_RECORD_VIEW_CONTRACT")) {
        QFile out(fixture);
        QVERIFY2(out.open(QIODevice::WriteOnly | QIODevice::Text), qPrintable(fixture));
        out.write(QJsonDocument(schema).toJson(QJsonDocument::Indented));
        out.close();
        QSKIP("recorded the tile cache schema");
    }

    QFile in(fixture);
    QVERIFY2(in.open(QIODevice::ReadOnly), "no recorded tile cache schema; run with QGC_RECORD_VIEW_CONTRACT=1 once");
    const QJsonObject expected = QJsonDocument::fromJson(in.readAll()).object();
    in.close();
    QCOMPARE(QJsonDocument(schema).toJson(QJsonDocument::Compact), QJsonDocument(expected).toJson(QJsonDocument::Compact));
}
