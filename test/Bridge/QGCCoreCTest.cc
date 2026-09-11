#include "QGCCoreCTest.h"
#include "QGCApplication.h"
#include "QGCMapUrlEngine.h"
#include "Vehicle.h"
#include "MissionItem.h"
#include "MissionManager.h"
#include <QtSql/QSqlError>
#include <QtSql/QSqlQuery>
#include <QtSql/QSqlDatabase>
#include "QGCMapEngine.h"
#include "QGCMapTasks.h"
#include "QGCCacheTile.h"
#include "QGeoFileTileCacheQGC.h"

#include "MockLink.h"
#include "UDPLink.h"
#include "LinkManager.h"
#include "LogReplayLink.h"
#include "MultiVehicleManager.h"
#include "ParameterManager.h"
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

#include <QtPositioning/QGeoCoordinate>

#include <algorithm>
#include <iterator>

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
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get("vehicle.parameterManager.parametersReady")).value(QStringLiteral("value")).toBool(false), 90000);
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


void flatten(const QJsonValue &value, const QString &path, QJsonObject &into)
{
    if (value.isObject()) {
        const QJsonObject object = value.toObject();
        for (auto it = object.begin(); it != object.end(); ++it) {
            flatten(it.value(), path + QStringLiteral(".") + it.key(), into);
        }
        return;
    }
    if (value.isArray()) {
        const QJsonArray array = value.toArray();
        into.insert(path + QStringLiteral(".#"), array.count());
        for (int index = 0; index < array.count(); index++) {
            flatten(array.at(index), QStringLiteral("%1.%2").arg(path).arg(index), into);
        }
        return;
    }
    into.insert(path, value);
}

QJsonObject snapshotOfEveryView(const char *const *paths, int count)
{
    QJsonObject flat;
    for (int index = 0; index < count; index++) {
        flatten(take(qgc_bridge_get(paths[index])), QString::fromUtf8(paths[index]), flat);
    }
    return flat;
}

QStringList fieldsAlwaysNull(const QList<QJsonObject> &states)
{
    QStringList null;
    if (states.isEmpty()) {
        return null;
    }
    for (auto it = states.first().begin(); it != states.first().end(); ++it) {
        const bool everywhere = std::all_of(states.cbegin(), states.cend(), [&it](const QJsonObject &state) {
            return state.value(it.key()).isNull();
        });
        if (everywhere && it.value().isNull()) {
            null.append(it.key());
        }
    }
    null.sort();
    return null;
}

QStringList fieldsThatNeverVaried(const QList<QJsonObject> &states)
{
    QStringList unchanged;
    if (states.isEmpty()) {
        return unchanged;
    }
    const QJsonObject &first = states.first();
    for (auto it = first.begin(); it != first.end(); ++it) {
        const bool moved = std::any_of(states.cbegin() + 1, states.cend(), [&it](const QJsonObject &state) {
            return !state.contains(it.key()) || state.value(it.key()) != it.value();
        });
        if (!moved) {
            unchanged.append(it.key());
        }
    }
    unchanged.sort();
    return unchanged;
}


// QGCCacheWorker::run opens the database only when it has none, so QGCMapEngine::init chooses the
// file once per process and every later call changes a path nothing reads again. Both tests that
// need a cache therefore share one, and neither pretends it can put the engine back afterwards.
QString sharedTileCache()
{
    static const QString path = QDir::temp().filePath(QStringLiteral("qgc-core-tilecache-%1.db").arg(QCoreApplication::applicationPid()));
    static bool started = false;
    if (!started) {
        started = true;
        QFile::remove(path);
        // The name carries this process's pid, so removing it only ever removed this run's own
        // file and every previous run's stayed. The Mac session found 270 of them, 22 MB. Anything
        // older than an hour cannot belong to a suite that is still running.
        const QDateTime stale = QDateTime::currentDateTime().addSecs(-3600);
        for (const QFileInfo &left : QDir::temp().entryInfoList({ QStringLiteral("qgc-core-tilecache-*.db") }, QDir::Files)) {
            if (left.lastModified() < stale) {
                QFile::remove(left.absoluteFilePath());
            }
        }
        QGCMapEngine::instance()->init(path);
    }
    return path;
}

bool tileCacheIsReady(const QString &path)
{
    if (!QFileInfo::exists(path)) {
        return false;
    }
    // removeDatabase while a QSqlDatabase copy or a query on it is still in scope leaks the
    // connection - Qt says so on stderr and then carries on. This is called from a QTRY loop, so
    // a leak here is dozens of open handles on one SQLite file, which is a plausible way for the
    // Rust reader's own open to start failing.
    const QString name = QStringLiteral("tileCacheReady");
    bool ready = false;
    {
        QSqlDatabase probe = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), name);
        probe.setDatabaseName(path);
        if (probe.open()) {
            QSqlQuery sets(probe);
            ready = sets.exec(QStringLiteral("SELECT setID FROM TileSets WHERE defaultSet = 1")) && sets.next();
        }
        probe.close();
    }
    QSqlDatabase::removeDatabase(name);
    return ready;
}

QJsonValue mergeShapes(const QJsonValue &a, const QJsonValue &b);

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
        if (array.isEmpty()) {
            return QJsonArray { QJsonValue(QStringLiteral("empty")) };
        }
        // Every element, not the first one. A list whose elements differ - a mission kind with a
        // complex name beside one without - would otherwise be recorded as whichever the first
        // happened to be, and a field that stopped ever being a string would look unchanged.
        QJsonValue element = shapeOf(array.first());
        for (const QJsonValue &entry : array) {
            element = mergeShapes(element, shapeOf(entry));
        }
        return QJsonArray { element };
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
    "view.preflight", "view.warnings", "view.modeSlots", "view.missionSummary", "view.missionItems", "view.vehicles", "view.label(altitudeRelative)", "view.instruments", "view.vibration",
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
    // A field that reads the same with no vehicle, with one connected, and with a plan on it is a
    // field that may not be answering the question its name asks. Six defects this week were that.
    QList<QJsonObject> states { snapshotOfEveryView(kViewPaths, int(std::size(kViewPaths))) };

    _connectMockLink(MAV_AUTOPILOT_PX4);
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get("view.guidedActions")).value(QStringLiteral("connected")).toBool(false), 5000);
    _mockLink->sendStatusTextMessages();
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get("view.messages")).value(QStringLiteral("count")).toInt() >= kMockStatusTextCount, 5000);

    states.append(snapshotOfEveryView(kViewPaths, int(std::size(kViewPaths))));

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

    // The controller recomputes its totals after an insert returns, so a snapshot taken straight
    // afterwards records the values from before the edit. Recording those as if they were this
    // state's answer makes them look like fields that never vary, which is what this list is for.
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_core_get("view.missionSummary")).value(QStringLiteral("distanceMetres")).toDouble(0.0) > 0.0, 10000);
    states.append(snapshotOfEveryView(kViewPaths, int(std::size(kViewPaths))));
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
    recorded.insert(QStringLiteral("_neverVaried"), QJsonArray::fromStringList(fieldsThatNeverVaried(states)));
    QStringList observed;
    for (const QJsonObject &state : states) {
        for (auto it = state.begin(); it != state.end(); ++it) {
            observed.append(it.key());
        }
    }
    observed.removeDuplicates();
    observed.sort();
    recorded.insert(QStringLiteral("_observed"), QJsonArray::fromStringList(observed));
    recorded.insert(QStringLiteral("_alwaysNull"), QJsonArray::fromStringList(fieldsAlwaysNull(states)));
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

    // Coarse on purpose: this run drives few states, so most of the recorded list is fields nothing
    // here moves rather than fields nothing can. What it catches is the list growing - a field that
    // used to vary and now does not, which is what all six of this week's defects looked like.
    QSet<QString> wereConstant;
    for (const QJsonValue &field : expected.value(QStringLiteral("_neverVaried")).toArray()) {
        wereConstant.insert(field.toString());
    }
    // A field the recording never saw has no history, so it cannot have stopped varying. Without
    // this, adding a field that is constant in these states reads as a regression in it.
    QSet<QString> seenBefore;
    for (const QJsonValue &field : expected.value(QStringLiteral("_observed")).toArray()) {
        seenBefore.insert(field.toString());
    }
    QStringList stopped;
    for (const QJsonValue &field : recorded.value(QStringLiteral("_neverVaried")).toArray()) {
        if (!wereConstant.contains(field.toString()) && seenBefore.contains(field.toString())) {
            stopped.append(field.toString());
        }
    }
    QSet<QString> allowedNull;
    for (const QJsonValue &field : expected.value(QStringLiteral("_alwaysNull")).toArray()) {
        allowedNull.insert(field.toString());
    }
    QStringList neverAnswered;
    for (const QJsonValue &field : recorded.value(QStringLiteral("_alwaysNull")).toArray()) {
        if (!allowedNull.contains(field.toString())) {
            neverAnswered.append(field.toString());
        }
    }
    QVERIFY2(neverAnswered.isEmpty(),
             qPrintable(QStringLiteral("these answered null with no vehicle, with one connected and with a plan on it. A field that is never anything "
                                       "is usually a field read under a name its producer does not use, which is how two of this week's defects got in: %1")
                            .arg(neverAnswered.mid(0, 12).join(QStringLiteral(", ")))));

    QVERIFY2(stopped.isEmpty(),
             qPrintable(QStringLiteral("these read the same with no vehicle, with one connected and with a plan on it, and they did not before. "
                                       "A value that never varies is not answering the question its name asks: %1").arg(stopped.mid(0, 12).join(QStringLiteral(", ")))));
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
    QCOMPARE(QJsonDocument(providers).toJson(QJsonDocument::Compact), QJsonDocument(expectedProviders).toJson(QJsonDocument::Compact));
    QCOMPARE(QJsonDocument(samples).toJson(QJsonDocument::Compact), QJsonDocument(expected.value(QStringLiteral("tileHashes")).toObject()).toJson(QJsonDocument::Compact));
}

void QGCCoreCTest::_theTileCacheSchemaMatchesTheRecordedOne()
{
    const QString databasePath = sharedTileCache();
    QTRY_VERIFY_WITH_TIMEOUT(tileCacheIsReady(databasePath), 20000);

    QJsonObject schema;
    {
        QSqlDatabase database = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), QStringLiteral("tileCacheSchemaProbe"));
        database.setDatabaseName(databasePath);

        // The worker creates the tables one at a time on its own thread, so a database file that
        // exists is not yet a database with a schema in it. Wait for the last thing it writes.
        QJsonObject recordedSets;
        QElapsedTimer waitingForTheWorker;
        waitingForTheWorker.start();
        while (recordedSets.isEmpty() && waitingForTheWorker.elapsed() < 10000) {
            if (!QFileInfo::exists(databasePath) || !database.open()) {
                QTest::qWait(50);
                continue;
            }
            QSqlQuery sets(database);
            if (sets.exec(QStringLiteral("SELECT name, defaultSet FROM TileSets ORDER BY name"))) {
                while (sets.next()) {
                    recordedSets[sets.value(0).toString()] = sets.value(1).toInt();
                }
            }
            if (recordedSets.isEmpty()) {
                database.close();
                QTest::qWait(50);
            }
        }
        QVERIFY2(!recordedSets.isEmpty(), "the map engine never finished creating the tile cache");

        QSqlQuery query(database);
        QVERIFY(query.exec(QStringLiteral("SELECT type, name, sql FROM sqlite_master WHERE sql IS NOT NULL ORDER BY name")));
        while (query.next()) {
            schema[query.value(1).toString()] = QStringLiteral("%1 %2").arg(query.value(0).toString(), query.value(2).toString());
        }
        schema[QStringLiteral("_tileSets")] = QJsonDocument(recordedSets).toJson(QJsonDocument::Compact).constData();
        database.close();
    }
    QSqlDatabase::removeDatabase(QStringLiteral("tileCacheSchemaProbe"));

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

void QGCCoreCTest::_polygonGeometryMatchesTheRecordedOracle()
{
#ifdef QGC_RUST_CORE
    const QList<QPair<QString, QJsonArray>> shapes = {
        { QStringLiteral("square"),   polygonSquare()   },
        { QStringLiteral("triangle"), polygonTriangle() },
        { QStringLiteral("concave"),  polygonConcave()  },
    };
    const QList<QPair<double, double>> probes = {
        { 47.3975, 8.5460 }, { 47.3960, 8.5440 }, { 47.3990, 8.5480 }, { 47.3985, 8.5490 },
        { 47.3970, 8.5485 }, { 47.3999, 8.5460 }, { 47.3950, 8.5460 }, { 47.3980, 8.5475 },
        { 47.3960, 8.5460 },
        { 47.3990, 8.5460 },
        { 47.3975, 8.5440 },
        { 47.3975, 8.5480 },
        { 47.3960, 8.5500 },
        { 47.3980, 8.5470 },
        { 47.3995, 8.5470 },
        { 47.3975, 8.5470 },
    };

    (void) take(qgc_bridge_invoke("plan.start", "[]"));
    const auto restore = []() { (void) take(qgc_bridge_invoke("plan.removeAll", "[]")); };
    const auto leaveNoPlanBehind = qScopeGuard(restore);
    restore();

    const auto compact = [](const QJsonArray &array) { return QJsonDocument(array).toJson(QJsonDocument::Compact); };
    const QString item = QStringLiteral("plan.missionController.visualItems.1");
    const QString area = item + QStringLiteral(".surveyAreaPolygon");

    const auto lay = [&](const QJsonArray &vertices) {
        restore();
        (void) take(qgc_bridge_invoke("plan.missionController.insertComplexMissionItem",
                                      compact(QJsonArray { QStringLiteral("Survey"), coordinateJson(47.3975, 8.5460), -1 }).constData()));
        QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get(item.toUtf8().constData())).value(QStringLiteral("kind")).toString() == QStringLiteral("object"), 5000);
        (void) take(qgc_bridge_invoke((area + QStringLiteral(".clear")).toUtf8().constData(), "[]"));
        for (const QJsonValue &vertex : vertices) {
            (void) take(qgc_bridge_invoke((area + QStringLiteral(".appendVertex")).toUtf8().constData(), compact(QJsonArray { vertex }).constData()));
        }
    };
    const auto path = [&]() {
        return take(qgc_bridge_get((area + QStringLiteral(".path")).toUtf8().constData())).value(QStringLiteral("value")).toArray();
    };

    QJsonObject recorded;
    for (const auto &shape : shapes) {
        lay(shape.second);
        QCOMPARE(path().count(), shape.second.count());

        QJsonArray contains;
        for (const auto &probe : probes) {
            const QJsonObject answer = take(qgc_bridge_invoke((area + QStringLiteral(".containsCoordinate")).toUtf8().constData(),
                                                              compact(QJsonArray { coordinateJson(probe.first, probe.second) }).constData()));
            QVERIFY2(answer.value(QStringLiteral("ok")).toBool(false), qPrintable(answer.value(QStringLiteral("reason")).toString()));
            contains.append(answer.value(QStringLiteral("result")).toBool());
        }

        QJsonObject splits;
        for (int vertex = 0; vertex < shape.second.count(); vertex++) {
            lay(shape.second);
            (void) take(qgc_bridge_invoke((area + QStringLiteral(".splitPolygonSegment")).toUtf8().constData(), compact(QJsonArray { vertex }).constData()));
            splits[QString::number(vertex)] = roundedCoordinates(path());
        }

        QJsonArray reversed;
        for (int vertex = shape.second.count() - 1; vertex >= 0; vertex--) {
            reversed.append(shape.second.at(vertex));
        }
        lay(reversed);
        (void) take(qgc_bridge_invoke((area + QStringLiteral(".verifyClockwiseWinding")).toUtf8().constData(), "[]"));
        const QString wound = roundedCoordinates(path());

        lay(shape.second);
        (void) take(qgc_bridge_invoke((area + QStringLiteral(".verifyClockwiseWinding")).toUtf8().constData(), "[]"));

        recorded[shape.first] = QJsonObject {
            { QStringLiteral("polygon"), shape.second },
            { QStringLiteral("area"), QString::number(take(qgc_bridge_get((area + QStringLiteral(".area")).toUtf8().constData())).value(QStringLiteral("value")).toDouble(), 'f', 4) },
            { QStringLiteral("contains"), contains },
            { QStringLiteral("splits"), splits },
            { QStringLiteral("reversedThenWound"), wound },
            { QStringLiteral("wound"), roundedCoordinates(path()) },
        };
    }
    restore();

    const QString fixture = QFileInfo(QString::fromUtf8(__FILE__)).dir().filePath(QStringLiteral("fixtures/polygon-geometry.json"));
    if (qEnvironmentVariableIsSet("QGC_RECORD_VIEW_CONTRACT")) {
        QFile out(fixture);
        QVERIFY2(out.open(QIODevice::WriteOnly | QIODevice::Text), qPrintable(fixture));
        out.write(QJsonDocument(recorded).toJson(QJsonDocument::Indented));
        out.close();
        QSKIP("recorded the polygon geometry oracle");
    }

    QFile in(fixture);
    QVERIFY2(in.open(QIODevice::ReadOnly), "no recorded polygon geometry; run with QGC_RECORD_VIEW_CONTRACT=1 once");
    const QJsonObject expected = QJsonDocument::fromJson(in.readAll()).object();
    in.close();
    QCOMPARE(QJsonDocument(recorded).toJson(QJsonDocument::Compact), QJsonDocument(expected).toJson(QJsonDocument::Compact));
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_structureScanFlightPathMatchesTheRecordedOracle()
{
#ifdef QGC_RUST_CORE
    const auto reversedOf = [](const QJsonArray &shape) {
        QJsonArray reversed;
        for (int vertex = shape.count() - 1; vertex >= 0; vertex--) {
            reversed.append(shape.at(vertex));
        }
        return reversed;
    };
    const QList<QPair<QString, QJsonArray>> shapes = {
        { QStringLiteral("square"),            polygonSquare()                },
        { QStringLiteral("square-reversed"),   reversedOf(polygonSquare())    },
        { QStringLiteral("triangle"),          polygonTriangle()              },
        { QStringLiteral("triangle-reversed"), reversedOf(polygonTriangle())  },
        { QStringLiteral("concave"),           polygonConcave()               },
        { QStringLiteral("concave-reversed"),  reversedOf(polygonConcave())   },
    };
    const QList<double> distances = { 25.0, 60.0, -25.0 };

    (void) take(qgc_bridge_invoke("plan.start", "[]"));
    const auto restore = []() { (void) take(qgc_bridge_invoke("plan.removeAll", "[]")); };
    const auto leaveNoPlanBehind = qScopeGuard(restore);
    restore();

    const auto compact = [](const QJsonArray &array) { return QJsonDocument(array).toJson(QJsonDocument::Compact); };
    const QString item = QStringLiteral("plan.missionController.visualItems.1");

    const auto setFact = [&](const QString &path, const QJsonValue &value) {
        const QByteArray body = QJsonDocument(QJsonObject { { QStringLiteral("value"), value } }).toJson(QJsonDocument::Compact);
        QVERIFY2(take(qgc_bridge_set(path.toUtf8().constData(), body.constData())).value(QStringLiteral("ok")).toBool(false), qPrintable(path));
        const QJsonValue back = take(qgc_bridge_get((path + QStringLiteral(".rawValue")).toUtf8().constData())).value(QStringLiteral("value"));
        QVERIFY2(qFuzzyCompare(back.toDouble() + 1.0, value.toDouble() + 1.0), qPrintable(QStringLiteral("%1 was set to %2 and reads back %3, so this case is not the case it is named after").arg(path).arg(value.toDouble()).arg(back.toDouble())));
    };

    QJsonObject recorded;
    for (const auto &shape : shapes) {
        for (const double distance : distances) {
            restore();
            (void) take(qgc_bridge_invoke("plan.missionController.insertComplexMissionItem",
                                          compact(QJsonArray { QStringLiteral("Structure Scan"), coordinateJson(47.3975, 8.5460), -1 }).constData()));
            QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get(item.toUtf8().constData())).value(QStringLiteral("kind")).toString() == QStringLiteral("object"), 5000);

            const QString structure = item + QStringLiteral(".structurePolygon");
            (void) take(qgc_bridge_invoke((structure + QStringLiteral(".clear")).toUtf8().constData(), "[]"));
            for (const QJsonValue &vertex : shape.second) {
                (void) take(qgc_bridge_invoke((structure + QStringLiteral(".appendVertex")).toUtf8().constData(), compact(QJsonArray { vertex }).constData()));
            }
            setFact(item + QStringLiteral(".cameraCalc.distanceToSurface"), QJsonValue(distance));

            const QJsonArray structurePath = take(qgc_bridge_get((structure + QStringLiteral(".path")).toUtf8().constData())).value(QStringLiteral("value")).toArray();
            const QString flight = item + QStringLiteral(".flightPolygon");
            const QJsonArray flightPath = take(qgc_bridge_get((flight + QStringLiteral(".path")).toUtf8().constData())).value(QStringLiteral("value")).toArray();

            recorded[QStringLiteral("%1 at %2").arg(shape.first).arg(distance)] = QJsonObject {
                { QStringLiteral("structure"), roundedCoordinates(structurePath) },
                { QStringLiteral("distanceToSurface"), distance },
                { QStringLiteral("flight"), roundedCoordinates(flightPath) },
                { QStringLiteral("structureArea"), QString::number(take(qgc_bridge_get((structure + QStringLiteral(".area")).toUtf8().constData())).value(QStringLiteral("value")).toDouble(), 'f', 4) },
                { QStringLiteral("flightArea"), QString::number(take(qgc_bridge_get((flight + QStringLiteral(".area")).toUtf8().constData())).value(QStringLiteral("value")).toDouble(), 'f', 4) },
            };
        }
    }
    restore();

    for (const QString &name : recorded.keys()) {
        const QJsonObject flown = recorded.value(name).toObject();
        const double structureArea = flown.value(QStringLiteral("structureArea")).toString().toDouble();
        const double flightArea = flown.value(QStringLiteral("flightArea")).toString().toDouble();
        const bool outward = flown.value(QStringLiteral("distanceToSurface")).toDouble() > 0.0;
        QVERIFY2(outward == (flightArea > structureArea),
                 qPrintable(QStringLiteral("%1: a scan distance of %2 put the flight path %3 the structure (%4 against %5). Which side the vehicle flies must not depend on the order the vertices were drawn in.")
                                .arg(name)
                                .arg(flown.value(QStringLiteral("distanceToSurface")).toDouble())
                                .arg(flightArea > structureArea ? QStringLiteral("outside") : QStringLiteral("inside"))
                                .arg(flightArea)
                                .arg(structureArea)));
    }

    const QString fixture = QFileInfo(QString::fromUtf8(__FILE__)).dir().filePath(QStringLiteral("fixtures/structure-scan.json"));
    if (qEnvironmentVariableIsSet("QGC_RECORD_VIEW_CONTRACT")) {
        QFile out(fixture);
        QVERIFY2(out.open(QIODevice::WriteOnly | QIODevice::Text), qPrintable(fixture));
        out.write(QJsonDocument(recorded).toJson(QJsonDocument::Indented));
        out.close();
        QSKIP("recorded the structure scan flight path");
    }

    QFile in(fixture);
    QVERIFY2(in.open(QIODevice::ReadOnly), "no recorded structure scan; run with QGC_RECORD_VIEW_CONTRACT=1 once");
    const QJsonObject expected = QJsonDocument::fromJson(in.readAll()).object();
    in.close();
    QCOMPARE(QJsonDocument(recorded).toJson(QJsonDocument::Compact), QJsonDocument(expected).toJson(QJsonDocument::Compact));
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_operatorNoticesReachAHeadWithNoQmlRoot()
{
#ifdef QGC_RUST_CORE
    const auto notices = []() {
        return take(qgc_bridge_get("host.notices")).value(QStringLiteral("value")).toArray();
    };
    const auto drain = [&]() {
        const QJsonArray waiting = notices();
        if (waiting.isEmpty()) {
            return;
        }
        const qint64 last = waiting.last().toObject().value(QStringLiteral("id")).toInteger();
        (void) take(qgc_bridge_invoke("host.acknowledgeThrough", QStringLiteral("[%1]").arg(last).toUtf8().constData()));
    };
    drain();
    QCOMPARE(notices().count(), 0);

    qgcApp()->showAppMessage(QStringLiteral("The vehicle refused the parameter write."), QStringLiteral("Parameters"));
    qgcApp()->showVehicleConfig();
    qgcApp()->showCriticalVehicleMessage(QStringLiteral("Compass calibration required"));

    const QJsonArray waiting = notices();
    QCOMPARE(waiting.count(), 3);
    QCOMPARE(waiting.at(0).toObject().value(QStringLiteral("kind")).toString(), QStringLiteral("message"));
    QCOMPARE(waiting.at(0).toObject().value(QStringLiteral("title")).toString(), QStringLiteral("Parameters"));
    QCOMPARE(waiting.at(0).toObject().value(QStringLiteral("text")).toString(), QStringLiteral("The vehicle refused the parameter write."));
    QVERIFY2(waiting.at(1).toObject().value(QStringLiteral("kind")).toString() == QStringLiteral("navigation"),
             "a token rather than an enum ordinal, so that inserting a kind in the middle of the list cannot silently change what a head already decodes");
    QVERIFY2(waiting.at(1).toObject().value(QStringLiteral("title")).toString() == QStringLiteral("setup"),
             "the jump to the setup tab has to arrive beside the message that explains it, or the app looks like it lost its place");
    QCOMPARE(waiting.at(2).toObject().value(QStringLiteral("kind")).toString(), QStringLiteral("vehicleError"));

    QVERIFY2(waiting.at(0).toObject().value(QStringLiteral("id")).toInteger() < waiting.at(1).toObject().value(QStringLiteral("id")).toInteger(),
             "ids rise with time, which is what lets a head acknowledge everything it has drawn in one call");

    (void) take(qgc_bridge_invoke("host.acknowledge", QStringLiteral("[%1]").arg(waiting.at(1).toObject().value(QStringLiteral("id")).toInteger()).toUtf8().constData()));
    QCOMPARE(notices().count(), 2);
    QVERIFY2(notices().at(0).toObject().value(QStringLiteral("id")).toInteger() == waiting.at(0).toObject().value(QStringLiteral("id")).toInteger(),
             "acknowledging one notice leaves the others, so a head that draws them one at a time loses none");

    const QJsonObject refused = take(qgc_bridge_invoke("host.acknowledge", "[999999]"));
    QCOMPARE(refused.value(QStringLiteral("result")).toBool(true), false);
    QCOMPARE(notices().count(), 2);

    drain();
    QCOMPARE(notices().count(), 0);
    QCOMPARE(take(qgc_bridge_get("host.count")).value(QStringLiteral("value")).toInt(), 0);

    // ParameterManager posts "Parameters are missing from firmware" once a second for as long as
    // the condition holds. Sixty of those are one piece of news, and while the queue keeps the
    // oldest eight and the newest, the duplicates in between displace every distinct message the
    // operator has not read yet - a vehicle error a minute old is what goes.
    const auto repeat = []() { qgcApp()->showAppMessage(QStringLiteral("Parameters are missing from firmware"), QStringLiteral("Parameters")); };
    repeat();
    const qint64 first = notices().last().toObject().value(QStringLiteral("id")).toInteger();
    const qint64 firstSeen = notices().last().toObject().value(QStringLiteral("at")).toInteger();
    repeat();
    repeat();
    QCOMPARE(notices().count(), 1);
    QCOMPARE(notices().last().toObject().value(QStringLiteral("repeated")).toInt(), 2);
    QVERIFY2(notices().last().toObject().value(QStringLiteral("id")).toInteger() == first,
             "a repeat is the same news, so it keeps the id a head has already drawn rather than arriving as a new banner");

    // The list is served oldestFirst by insertion, so at has to stay the first sighting or a
    // repeated notice in the middle carries a newer timestamp than the ones after it, and a head
    // that sorts by at draws them out of order. When it was last seen is a separate question.
    const QJsonObject folded = notices().last().toObject();
    QVERIFY2(folded.value(QStringLiteral("lastAt")).toInteger() >= folded.value(QStringLiteral("at")).toInteger(),
             "the last sighting cannot precede the first");
    QCOMPARE(folded.value(QStringLiteral("at")).toInteger(), firstSeen);

    qgcApp()->showCriticalVehicleMessage(QStringLiteral("Compass calibration required"));
    repeat();
    QCOMPARE(notices().count(), 3);
    QVERIFY2(notices().last().toObject().value(QStringLiteral("repeated")).toInt() == 0,
             "only a run of identical notices collapses; the same text after something else is news again");

    // A condition that is still ongoing and one that cleared and came back read the same to the
    // queue, and no rule about elapsed time can separate them. Acknowledgement can: once a head has
    // taken the notice, the operator has been told, so the next occurrence is news and arrives
    // under its own id. Collapsing only holds while the notice is still waiting to be read.
    const qint64 unread = notices().last().toObject().value(QStringLiteral("id")).toInteger();
    (void) take(qgc_bridge_invoke("host.acknowledge", QStringLiteral("[%1]").arg(unread).toUtf8().constData()));
    repeat();
    QCOMPARE(notices().count(), 3);
    QVERIFY2(notices().last().toObject().value(QStringLiteral("id")).toInteger() > unread,
             "a recurrence after the operator has been told is a new notice, not a repeat of one that is no longer on screen");
    QCOMPARE(notices().last().toObject().value(QStringLiteral("repeated")).toInt(), 0);

    drain();
    QCOMPARE(notices().count(), 0);

    // A head cannot see this channel work without something posting on it, and every real poster
    // needs a vehicle, an upload or a settings write. This is how a rig makes one arrive.
    QVERIFY2(take(qgc_bridge_invoke("host.postNotice", "[\"navigation\",\"setup\",\"\"]")).value(QStringLiteral("result")).toBool(false),
             "a rig has to be able to post a notice, or a head can only ever test its decoder");
    QCOMPARE(notices().count(), 1);
    QCOMPARE(notices().first().toObject().value(QStringLiteral("kind")).toString(), QStringLiteral("navigation"));

    // A head reads this several ways and every one has to carry the notices rather than their
    // count alone. A list whose length is right and whose elements are null looks like a queue
    // with something in it and reads as empty, which is the silence this channel exists to end.
    const auto carriesTheNotice = [](const QJsonArray &listed) {
        return listed.count() == 1 && listed.first().toObject().value(QStringLiteral("title")).toString() == QStringLiteral("setup");
    };
    QVERIFY2(carriesTheNotice(take(qgc_bridge_get("host.notices")).value(QStringLiteral("value")).toArray()), "reading the property directly lost the notice");
    QVERIFY2(carriesTheNotice(take(qgc_bridge_get("host")).value(QStringLiteral("notices")).toArray()), "reading the whole object lost the notice");
    QVERIFY2(carriesTheNotice(take(qgc_bridge_get_fields("host", "notices,count,dropped")).value(QStringLiteral("notices")).toArray()), "asking for named fields lost the notice");
    // The strong version of this check registers a QVariantMap to QGeoCoordinate converter, which
    // is what QtPositioning's QML plugin does in the running app and what made every map on the
    // wire serialise as null. It cannot live here: a converter is process-wide and permanent, and
    // registering one changed how unrelated suites in this binary read their own variants. What is
    // checkable here is that a map arrives as a map; the condition that broke it belongs to a
    // process that loads QML.

    QVERIFY2(take(qgc_bridge_get("host.notices.0")).value(QStringLiteral("found")).toBool(true) == false,
             "a list property is a leaf and cannot be walked into, and saying so is what tells a head to read the list rather than index it");
    QCOMPARE(take(qgc_bridge_invoke("host.postNotice", "[\"shouting\",\"x\",\"y\"]")).value(QStringLiteral("result")).toBool(true), false);
    QCOMPARE(notices().count(), 1);
    drain();

    const int droppedBefore = take(qgc_bridge_get("host.dropped")).value(QStringLiteral("value")).toInt();
    for (int index = 0; index < 70; index++) {
        qgcApp()->showAppMessage(QStringLiteral("message %1").arg(index));
    }
    QCOMPARE(notices().count(), 64);
    QVERIFY2(take(qgc_bridge_get("host.dropped")).value(QStringLiteral("value")).toInt() > droppedBefore,
             "a head that never drains must not grow the queue without bound, and it has to be able to see that it missed something");
    QVERIFY2(notices().first().toObject().value(QStringLiteral("text")).toString() == QStringLiteral("message 0"),
             "under a flood the first notice is the one that explains the rest, so it is not the one to lose");
    QCOMPARE(notices().at(7).toObject().value(QStringLiteral("text")).toString(), QStringLiteral("message 7"));
    QCOMPARE(notices().at(8).toObject().value(QStringLiteral("text")).toString(), QStringLiteral("message 14"));
    QCOMPARE(notices().last().toObject().value(QStringLiteral("text")).toString(), QStringLiteral("message 69"));
    drain();
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_everyNameTheCoreHandsAHeadToInterpolateResolves()
{
#ifdef QGC_RUST_CORE
    const QJsonArray kinds = take(qgc_core_get("view.missionKinds")).value(QStringLiteral("kinds")).toArray();
    QVERIFY2(!kinds.isEmpty(), "the core offers no mission kinds, so this test would pass by checking nothing");

    (void) take(qgc_bridge_invoke("plan.start", "[]"));
    const auto restore = []() { (void) take(qgc_bridge_invoke("plan.removeAll", "[]")); };
    const auto leaveNoPlanBehind = qScopeGuard(restore);

    const auto compact = [](const QJsonArray &array) { return QJsonDocument(array).toJson(QJsonDocument::Compact); };
    const QString item = QStringLiteral("plan.missionController.visualItems.1");

    QStringList unreachable;
    for (const QJsonValue &entry : kinds) {
        const QJsonObject kind = entry.toObject();
        const QString id = kind.value(QStringLiteral("id")).toString();
        const QString invokable = kind.value(QStringLiteral("invokable")).toString();
        restore();

        const QJsonArray arguments = kind.value(QStringLiteral("simple")).toBool()
            ? QJsonArray { coordinateJson(47.3975, 8.5460), -1 }
            : QJsonArray { kind.value(QStringLiteral("complexName")).toString(), coordinateJson(47.3975, 8.5460), -1 };
        const QJsonObject inserted = take(qgc_bridge_invoke((QStringLiteral("plan.missionController.") + invokable).toUtf8().constData(), compact(arguments).constData()));
        if (!inserted.contains(QStringLiteral("result"))) {
            unreachable.append(QStringLiteral("%1: missionController has no %2, so a head calling the name the core gave it calls nothing").arg(id, invokable));
            continue;
        }
        QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get(item.toUtf8().constData())).value(QStringLiteral("kind")).toString() == QStringLiteral("object"), 5000);

        const QJsonValue geometry = kind.value(QStringLiteral("geometryProperty"));
        if (geometry.isNull()) {
            continue;
        }
        const QString path = item + QStringLiteral(".") + geometry.toString();
        const QString resolved = take(qgc_bridge_get(path.toUtf8().constData())).value(QStringLiteral("kind")).toString();
        if (resolved != QStringLiteral("object")) {
            unreachable.append(QStringLiteral("%1: %2 answers %3, and a head interpolating that name reads null and draws its default").arg(id, path, resolved));
            continue;
        }
        const QString shape = take(qgc_bridge_get((path + QStringLiteral(".path")).toUtf8().constData())).value(QStringLiteral("kind")).toString();
        if (shape != QStringLiteral("value")) {
            unreachable.append(QStringLiteral("%1: %2.path answers %3, so the head has the object but not the vertices").arg(id, path, shape));
        }
    }
    restore();

    QVERIFY2(unreachable.isEmpty(), qPrintable(QStringLiteral("the core names things the bridge cannot resolve:\n%1").arg(unreachable.join(QStringLiteral("\n")))));
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_structureScanItemsMatchTheRecordedUpload()
{
#ifdef QGC_RUST_CORE
    _connectMockLink(MAV_AUTOPILOT_PX4);
    bool stillConnected = true;
    const auto disconnectWhenDone = qScopeGuard([this, &stillConnected]() {
        if (stillConnected) {
            _disconnectMockLink();
        }
    });
    Vehicle *const vehicle = MultiVehicleManager::instance()->activeVehicle();
    QVERIFY(vehicle);

    (void) take(qgc_bridge_invoke("plan.start", "[]"));
    const auto restore = []() { (void) take(qgc_bridge_invoke("plan.removeAll", "[]")); };
    const auto leaveNoPlanBehind = qScopeGuard(restore);

    const auto compact = [](const QJsonArray &array) { return QJsonDocument(array).toJson(QJsonDocument::Compact); };
    const QString item = QStringLiteral("plan.missionController.visualItems.1");

    struct ScanCase {
        const char *name;
        QJsonArray  structure;
        double      distanceToSurface;
        double      scanBottomAlt;
        double      structureHeight;
        double      entranceAlt;
        bool        startFromTop;
    };
    const QList<ScanCase> cases = {
        { "square-from-bottom", polygonSquare(),   30.0, 10.0, 50.0,  5.0, false },
        { "square-from-top",    polygonSquare(),   30.0, 10.0, 50.0,  5.0, true  },
        { "triangle-tall",      polygonTriangle(), 25.0,  0.0, 90.0, 12.0, false },
    };

    QJsonObject recorded;
    for (const ScanCase &scan : cases) {
        restore();
        (void) take(qgc_bridge_invoke("plan.missionController.insertComplexMissionItem",
                                      compact(QJsonArray { QStringLiteral("Structure Scan"), coordinateJson(47.3975, 8.5460), -1 }).constData()));
        QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get(item.toUtf8().constData())).value(QStringLiteral("kind")).toString() == QStringLiteral("object"), 5000);

        const QString structure = item + QStringLiteral(".structurePolygon");
        (void) take(qgc_bridge_invoke((structure + QStringLiteral(".clear")).toUtf8().constData(), "[]"));
        for (const QJsonValue &vertex : scan.structure) {
            (void) take(qgc_bridge_invoke((structure + QStringLiteral(".appendVertex")).toUtf8().constData(), compact(QJsonArray { vertex }).constData()));
        }

        const auto setFact = [&](const QString &path, double value) {
            const QByteArray body = QJsonDocument(QJsonObject { { QStringLiteral("value"), value } }).toJson(QJsonDocument::Compact);
            QVERIFY2(take(qgc_bridge_set(path.toUtf8().constData(), body.constData())).value(QStringLiteral("ok")).toBool(false), qPrintable(path));
        };
        setFact(item + QStringLiteral(".cameraCalc.distanceToSurface"), scan.distanceToSurface);
        setFact(item + QStringLiteral(".scanBottomAlt"), scan.scanBottomAlt);
        setFact(item + QStringLiteral(".structureHeight"), scan.structureHeight);
        setFact(item + QStringLiteral(".entranceAlt"), scan.entranceAlt);
        setFact(item + QStringLiteral(".startFromTop"), scan.startFromTop ? 1.0 : 0.0);

        const auto readFact = [&](const QString &name) {
            return take(qgc_bridge_get((item + QStringLiteral(".") + name + QStringLiteral(".rawValue")).toUtf8().constData())).value(QStringLiteral("value")).toDouble();
        };
        const QJsonArray flightPath = take(qgc_bridge_get((item + QStringLiteral(".flightPolygon.path")).toUtf8().constData())).value(QStringLiteral("value")).toArray();
        QVERIFY(flightPath.count() >= 3);

        QVERIFY2(take(qgc_bridge_invoke("plan.sendToVehicle", "[]")).value(QStringLiteral("ok")).toBool(false), "the plan could not be sent");
        QTRY_VERIFY_WITH_TIMEOUT(!vehicle->missionManager()->inProgress(), 20000);

        QJsonArray uploaded;
        for (const MissionItem *const sent : vehicle->missionManager()->missionItems()) {
            const QList<double> values = { sent->param1(), sent->param2(), sent->param3(), sent->param4(), sent->param5(), sent->param6(), sent->param7() };
            QJsonArray params;
            for (const double value : values) {
                params.append(qIsNaN(value) ? QJsonValue() : QJsonValue(QString::number(value, 'f', 7)));
            }
            uploaded.append(QStringLiteral("%1 %2 %3").arg(sent->command()).arg(sent->frame()).arg(QJsonDocument(params).toJson(QJsonDocument::Compact).constData()));
        }
        QVERIFY2(uploaded.count() > 4, "the vehicle received no structure scan");

        recorded[QString::fromUtf8(scan.name)] = QJsonObject {
            { QStringLiteral("flight"), roundedCoordinates(flightPath) },
            { QStringLiteral("adjustedFootprintSide"), QString::number(readFact(QStringLiteral("cameraCalc.adjustedFootprintSide")), 'f', 7) },
            { QStringLiteral("adjustedFootprintFrontal"), QString::number(readFact(QStringLiteral("cameraCalc.adjustedFootprintFrontal")), 'f', 7) },
            { QStringLiteral("layers"), readFact(QStringLiteral("layers")) },
            { QStringLiteral("scanBottomAlt"), scan.scanBottomAlt },
            { QStringLiteral("structureHeight"), scan.structureHeight },
            { QStringLiteral("entranceAlt"), scan.entranceAlt },
            { QStringLiteral("startFromTop"), scan.startFromTop },
            { QStringLiteral("gimbalPitch"), readFact(QStringLiteral("gimbalPitch")) },
            { QStringLiteral("items"), uploaded },
        };
    }
    restore();

    // The vehicle has nothing left to say, and a skip below would otherwise leave the disconnect to
    // run inside an already skipped test, where its wait for the vehicle to go never completes.
    _disconnectMockLink();
    stillConnected = false;

    const QString fixture = QFileInfo(QString::fromUtf8(__FILE__)).dir().filePath(QStringLiteral("fixtures/structure-scan-items.json"));
    if (qEnvironmentVariableIsSet("QGC_RECORD_VIEW_CONTRACT")) {
        QFile out(fixture);
        QVERIFY2(out.open(QIODevice::WriteOnly | QIODevice::Text), qPrintable(fixture));
        out.write(QJsonDocument(recorded).toJson(QJsonDocument::Indented));
        out.close();
        QSKIP("recorded the structure scan upload");
    }

    QFile in(fixture);
    QVERIFY2(in.open(QIODevice::ReadOnly), "no recorded structure scan upload; run with QGC_RECORD_VIEW_CONTRACT=1 once");
    const QJsonObject expected = QJsonDocument::fromJson(in.readAll()).object();
    in.close();
    QCOMPARE(QJsonDocument(recorded).toJson(QJsonDocument::Compact), QJsonDocument(expected).toJson(QJsonDocument::Compact));
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_aLargeSurveyMakesTheRoundTripUnchanged()
{
#ifdef QGC_RUST_CORE
    _connectMockLink(MAV_AUTOPILOT_PX4);
    const auto disconnectWhenDone = qScopeGuard([this]() { _disconnectMockLink(); });
    Vehicle *const vehicle = MultiVehicleManager::instance()->activeVehicle();
    QVERIFY(vehicle);

    (void) take(qgc_bridge_invoke("plan.start", "[]"));
    const auto restore = []() { (void) take(qgc_bridge_invoke("plan.removeAll", "[]")); };
    const auto leaveNoPlanBehind = qScopeGuard(restore);
    restore();

    const auto compact = [](const QJsonArray &array) { return QJsonDocument(array).toJson(QJsonDocument::Compact); };
    const QString item = QStringLiteral("plan.missionController.visualItems.1");
    (void) take(qgc_bridge_invoke("plan.missionController.insertComplexMissionItem",
                                  compact(QJsonArray { QStringLiteral("Survey"), coordinateJson(47.3975, 8.5460), -1 }).constData()));
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get(item.toUtf8().constData())).value(QStringLiteral("kind")).toString() == QStringLiteral("object"), 5000);

    (void) take(qgc_bridge_invoke((item + QStringLiteral(".surveyAreaPolygon.clear")).toUtf8().constData(), "[]"));
    for (const QJsonValue &vertex : polygonConcave()) {
        (void) take(qgc_bridge_invoke((item + QStringLiteral(".surveyAreaPolygon.appendVertex")).toUtf8().constData(), compact(QJsonArray { vertex }).constData()));
    }
    const auto setFact = [&](const QString &path, double value) {
        const QByteArray body = QJsonDocument(QJsonObject { { QStringLiteral("value"), value } }).toJson(QJsonDocument::Compact);
        QVERIFY2(take(qgc_bridge_set(path.toUtf8().constData(), body.constData())).value(QStringLiteral("ok")).toBool(false), qPrintable(path));
    };
    setFact(item + QStringLiteral(".cameraCalc.adjustedFootprintSide"), 6.0);
    setFact(item + QStringLiteral(".cameraCalc.adjustedFootprintFrontal"), 6.0);
    setFact(item + QStringLiteral(".cameraCalc.distanceToSurface"), 40.0);

    struct Sent {
        int          sequence;
        int          command;
        int          frame;
        QList<double> params;
        QString      spelled;
    };
    const auto spell = [](const QList<MissionItem *> &items) {
        QList<Sent> sent;
        for (const MissionItem *const entry : items) {
            const QList<double> values = { entry->param1(), entry->param2(), entry->param3(), entry->param4(), entry->param5(), entry->param6(), entry->param7() };
            QStringList params;
            for (const double value : values) {
                params.append(qIsNaN(value) ? QStringLiteral("nan") : QString::number(value, 'f', 7));
            }
            sent.append(Sent {
                entry->sequenceNumber(),
                static_cast<int>(entry->command()),
                static_cast<int>(entry->frame()),
                values,
                QStringLiteral("%1 %2 %3 %4").arg(entry->sequenceNumber()).arg(entry->command()).arg(entry->frame()).arg(params.join(QStringLiteral(","))),
            });
        }
        return sent;
    };

    // MISSION_ITEM_INT carries latitude and longitude as degrees times ten million, so a coordinate
    // comes back quantised and a not-a-number becomes a zero. Everything else has to survive exactly.
    const auto carriedIdentically = [](const Sent &sent, const Sent &read) {
        if (sent.sequence != read.sequence || sent.command != read.command || sent.frame != read.frame) {
            return false;
        }
        for (int index = 0; index < sent.params.count(); index++) {
            const double before = sent.params.at(index);
            const double after = read.params.at(index);
            const bool coordinate = index == 4 || index == 5;
            if (qIsNaN(before)) {
                if (!(qIsNaN(after) || (coordinate && after == 0.0))) {
                    return false;
                }
                continue;
            }
            if (qIsNaN(after)) {
                return false;
            }
            const double allowed = coordinate ? 1.5e-7 : 0.0;
            if (qAbs(before - after) > allowed) {
                return false;
            }
        }
        return true;
    };

    QVERIFY2(take(qgc_bridge_invoke("plan.sendToVehicle", "[]")).value(QStringLiteral("ok")).toBool(false), "the plan could not be sent");
    QTRY_VERIFY_WITH_TIMEOUT(!vehicle->missionManager()->inProgress(), 120000);
    const QList<Sent> uploaded = spell(vehicle->missionManager()->missionItems());
    QVERIFY2(uploaded.count() > 200, qPrintable(QStringLiteral("this survey is meant to be larger than a vehicle's usual mission and it came to %1 items").arg(uploaded.count())));

    restore();
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get("plan.missionController.visualItems.count")).value(QStringLiteral("value")).toInt(-1) <= 1, 5000);

    QVERIFY2(take(qgc_bridge_invoke("plan.loadFromVehicle", "[]")).value(QStringLiteral("ok")).toBool(false), "the plan could not be read back");
    QTRY_VERIFY_WITH_TIMEOUT(!vehicle->missionManager()->inProgress(), 120000);
    const QList<Sent> downloaded = spell(vehicle->missionManager()->missionItems());

    QCOMPARE(downloaded.count(), uploaded.count());
    QStringList differences;
    for (int index = 0; index < uploaded.count() && differences.count() < 5; index++) {
        if (!carriedIdentically(uploaded.at(index), downloaded.at(index))) {
            differences.append(QStringLiteral("item %1\n  sent: %2\n  read: %3").arg(index).arg(uploaded.at(index).spelled, downloaded.at(index).spelled));
        }
    }
    QVERIFY2(differences.isEmpty(), qPrintable(QStringLiteral("a survey of %1 items did not survive the round trip:\n%2").arg(uploaded.count()).arg(differences.join(QStringLiteral("\n")))));
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_theCoreRefusesAMissionItemThePlanHasDecidedAgainst()
{
#ifdef QGC_RUST_CORE
    (void) take(qgc_bridge_invoke("plan.start", "[]"));
    const auto restore = []() { (void) take(qgc_bridge_invoke("plan.removeAll", "[]")); };
    const auto leaveNoPlanBehind = qScopeGuard(restore);
    restore();

    const auto count = []() {
        return take(qgc_bridge_get("plan.missionController.visualItems.count")).value(QStringLiteral("value")).toInt(-1);
    };
    const auto settled = [&count]() {
        int seen = count();
        QElapsedTimer still;
        still.start();
        while (still.elapsed() < 1500) {
            QTest::qWait(150);
            const int now = count();
            if (now != seen) {
                seen = now;
                still.restart();
            }
        }
        return seen;
    };
    const auto insert = [](const char *args) {
        return take(qgc_core_invoke("mission.insert", args));
    };

    const int empty = settled();
    QVERIFY2(empty >= 0, "the plan did not answer how many items it holds");

    const QJsonObject refused = insert("[\"survey\", 47.3975, 8.5460, -1]");
    QCOMPARE(refused.value(QStringLiteral("ok")).toBool(true), false);
    QVERIFY2(!refused.value(QStringLiteral("reason")).toString().isEmpty(), "a refusal an operator will read has to say why");
    QCOMPARE(settled(), empty);

    QVERIFY2(insert("[\"takeoff\", 47.3975, 8.5460, -1]").value(QStringLiteral("ok")).toBool(false), "the takeoff was refused");
    const int withTakeoff = settled();
    QCOMPARE(withTakeoff, empty + 1);

    const QJsonObject launch = take(qgc_bridge_get("plan.missionController.visualItems.1.launchCoordinate"));
    QVERIFY2(launch.value(QStringLiteral("valid")).toBool(false),
             "a takeoff that does not know where the vehicle launches from cannot be flown, and the head should not have to write it");
    QVERIFY(qAbs(launch.value(QStringLiteral("latitude")).toDouble() - 47.3975) < 1e-6);

    QVERIFY2(insert("[\"takeoff\", 47.3975, 8.5460, -1]").value(QStringLiteral("ok")).toBool(true) == false,
             "the plan already takes off, and the core refusing this is the whole point of putting the gate behind the insert rather than beside it");
    QCOMPARE(settled(), withTakeoff);

    QVERIFY2(insert("[\"survey\", 47.3975, 8.5460, -1]").value(QStringLiteral("ok")).toBool(false), "the survey was refused after a takeoff, which the plan allows");
    const int withSurvey = settled();
    QCOMPARE(withSurvey, withTakeoff + 1);
    QCOMPARE(take(qgc_bridge_get(QStringLiteral("plan.missionController.visualItems.%1.isSurveyItem").arg(withSurvey - 1).toUtf8().constData())).value(QStringLiteral("value")).toBool(), true);

    const QJsonArray area = take(qgc_bridge_get(QStringLiteral("plan.missionController.visualItems.%1.surveyAreaPolygon.path").arg(withSurvey - 1).toUtf8().constData())).value(QStringLiteral("value")).toArray();
    QCOMPARE(area.count(), 4);
    QVERIFY2(qAbs(area.first().toObject().value(QStringLiteral("latitude")).toDouble() - 47.3975) < 0.01,
             "a survey with no area draws nothing and uploads nothing, so the action places one around where it was asked for");

    QVERIFY2(insert("[\"corridor\", 47.3975, 8.5460, -1]").value(QStringLiteral("ok")).toBool(false), "the corridor was refused");
    const int withCorridor = settled();
    QCOMPARE(withCorridor, withSurvey + 1);
    QCOMPARE(take(qgc_bridge_get(QStringLiteral("plan.missionController.visualItems.%1.corridorPolyline.path").arg(withCorridor - 1).toUtf8().constData())).value(QStringLiteral("value")).toArray().count(), 2);

    const QJsonObject keptSettings = take(qgc_core_invoke("mission.remove", "[0]"));
    QCOMPARE(keptSettings.value(QStringLiteral("ok")).toBool(true), false);
    QCOMPARE(settled(), withCorridor);

    const QJsonObject removed = take(qgc_core_invoke("mission.remove", QStringLiteral("[%1]").arg(withCorridor - 1).toUtf8().constData()));
    QVERIFY2(removed.value(QStringLiteral("ok")).toBool(false), qPrintable(removed.value(QStringLiteral("reason")).toString()));
    QCOMPARE(settled(), withCorridor - 1);
    QCOMPARE(removed.value(QStringLiteral("remaining")).toInt(-1), withCorridor - 1);

    const QJsonObject past = take(qgc_core_invoke("mission.remove", QStringLiteral("[%1]").arg(withCorridor + 5).toUtf8().constData()));
    QCOMPARE(past.value(QStringLiteral("ok")).toBool(true), false);
    QCOMPARE(settled(), withCorridor - 1);

    const QJsonObject unknown = insert("[\"Fixed Wing Landing Pattern\", 47.3975, 8.5460, -1]");
    QCOMPARE(unknown.value(QStringLiteral("ok")).toBool(true), false);
    QVERIFY2(unknown.value(QStringLiteral("unknown")).toString() == QStringLiteral("Fixed Wing Landing Pattern"),
             "QGC has item types this catalogue does not list, and a head holding one has to tell being unlisted apart from being turned down");
    QCOMPARE(settled(), withCorridor - 1);
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_theFlyViewControllerCountsTheMissionThePlanEditorCannot()
{
#ifdef QGC_RUST_CORE
    _connectMockLink(MAV_AUTOPILOT_PX4);
    bool stillConnected = true;
    const auto disconnectWhenDone = qScopeGuard([this, &stillConnected]() {
        if (stillConnected) {
            _disconnectMockLink();
        }
    });
    Vehicle *const vehicle = MultiVehicleManager::instance()->activeVehicle();
    QVERIFY(vehicle);

    (void) take(qgc_bridge_invoke("plan.start", "[]"));
    const auto restore = []() { (void) take(qgc_bridge_invoke("plan.removeAll", "[]")); };
    const auto leaveNoPlanBehind = qScopeGuard(restore);
    restore();

    const auto number = [](const char *path) {
        return take(qgc_bridge_get(path)).value(QStringLiteral("value")).toInt(-99);
    };

    QVERIFY2(take(qgc_core_invoke("mission.insert", "[\"takeoff\", 47.3975, 8.5460, -1]")).value(QStringLiteral("ok")).toBool(false), "the takeoff was refused");
    QVERIFY2(take(qgc_core_invoke("mission.insert", "[\"waypoint\", 47.3985, 8.5470, -1]")).value(QStringLiteral("ok")).toBool(false), "the waypoint was refused");
    QVERIFY2(take(qgc_bridge_invoke("plan.sendToVehicle", "[]")).value(QStringLiteral("ok")).toBool(false), "the plan could not be sent");
    QTRY_VERIFY_WITH_TIMEOUT(!vehicle->missionManager()->inProgress(), 30000);

    QTRY_VERIFY_WITH_TIMEOUT(number("planFly.missionController.missionItemCount") > 0, 30000);
    QVERIFY2(number("plan.missionController.missionItemCount") == 0,
             "the plan editor's controller answers zero however many items it holds, which is why reading it was giving the guided view a mission of no length");
    QVERIFY2(number("plan.missionController.currentMissionIndex") == -1,
             "and it answers minus one for where the vehicle is, so a paused mission never looked resumable");
    QVERIFY2(number("planFly.missionController.currentMissionIndex") >= 0,
             "the fly view's controller is the one that knows, and it is the one the guided view now reads");

    _disconnectMockLink();
    stillConnected = false;
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_everyRootTheCoreReadsFromIsRegistered()
{
#ifdef QGC_RUST_CORE
    _connectMockLink(MAV_AUTOPILOT_PX4);
    const auto disconnectWhenDone = qScopeGuard([this]() { _disconnectMockLink(); });

    const QStringList roots = {
        QStringLiteral("settings"),      QStringLiteral("vehicle"),          QStringLiteral("vehicles"),
        QStringLiteral("links"),         QStringLiteral("plan"),             QStringLiteral("planFly"),
        QStringLiteral("logDownload"),   QStringLiteral("video"),            QStringLiteral("geoTag"),
        QStringLiteral("positionManager"), QStringLiteral("units"),          QStringLiteral("missionCommandTree"),
        QStringLiteral("mavlinkConsole"), QStringLiteral("mavlinkInspector"), QStringLiteral("host"),
        QStringLiteral("corePlugin"),
    };

    QStringList unresolved;
    for (const QString &root : roots) {
        if (take(qgc_bridge_get(root.toUtf8().constData())).value(QStringLiteral("kind")).toString() == QStringLiteral("null")) {
            unresolved.append(root);
        }
    }
    QVERIFY2(unresolved.isEmpty(),
             qPrintable(QStringLiteral("the core reads from roots the bridge does not register, and an unregistered root answers null, "
                                       "so every value below it silently takes the core's default: %1").arg(unresolved.join(QStringLiteral(", ")))));

    QCOMPARE(take(qgc_bridge_get("corePlugin.options.showMissionAbsoluteAltitude")).value(QStringLiteral("value")).toBool(), true);
    QVERIFY2(take(qgc_bridge_get("notARoot")).value(QStringLiteral("kind")).toString() == QStringLiteral("null"),
             "a root that does not exist has to answer null, or the check above proves nothing");
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_setupSeesTheComponentsTheVehicleReports()
{
#ifdef QGC_RUST_CORE
    _connectMockLink(MAV_AUTOPILOT_PX4);
    const auto disconnectWhenDone = qScopeGuard([this]() { _disconnectMockLink(); });
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get("vehicle.parameterManager.parametersReady")).value(QStringLiteral("value")).toBool(false), 90000);

    const QJsonArray reported = take(qgc_bridge_get("vehicle.autopilotPlugin.vehicleComponents")).value(QStringLiteral("value")).toArray();
    QVERIFY2(!reported.isEmpty(), "this vehicle reports no setup components at all, so the test below would pass by checking nothing");

    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_core_get("view.setup")).value(QStringLiteral("components")).toArray().count() == reported.count(), 30000);

    const QJsonObject setup = take(qgc_core_get("view.setup"));
    QVERIFY2(!setup.value(QStringLiteral("headline")).toString().contains(QStringLiteral("no setup components")),
             "the vehicle reports components and the core has to see them; reading them under the wrong key made every vehicle look like it reported none");

    const QJsonArray named = setup.value(QStringLiteral("components")).toArray();
    QStringList missing;
    for (int index = 0; index < reported.count(); index++) {
        const QString expected = take(qgc_bridge_get(QStringLiteral("vehicle.autopilotPlugin.vehicleComponents.%1").arg(index).toUtf8().constData()))
                                     .value(QStringLiteral("name")).toString();
        const bool found = std::any_of(named.cbegin(), named.cend(), [&expected](const QJsonValue &component) {
            return component.toObject().value(QStringLiteral("name")).toString() == expected;
        });
        if (!found) {
            missing.append(expected);
        }
    }
    QVERIFY2(missing.isEmpty(), qPrintable(QStringLiteral("the setup view is missing components the vehicle reports: %1").arg(missing.join(QStringLiteral(", ")))));
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_listsRecordedAsEmptyAreCheckedAgainstAVehicle()
{
#ifdef QGC_RUST_CORE
    _connectMockLink(MAV_AUTOPILOT_PX4);
    const auto disconnectWhenDone = qScopeGuard([this]() { _disconnectMockLink(); });
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get("view.sensors")).value(QStringLiteral("available")).toBool(false), 10000);

    // A list recorded as empty is a question rather than a shape: it may be a list nothing filled,
    // or a read that never finds anything. These two are the ones a connected vehicle should fill.
    const QJsonArray channels = take(qgc_core_get("view.radio")).value(QStringLiteral("channels")).toArray();
    const QJsonArray raw = take(qgc_bridge_get("radioCal.rcValues")).value(QStringLiteral("value")).toArray();
    QVERIFY2(channels.count() == raw.count(),
             qPrintable(QStringLiteral("the vehicle reports %1 radio channels and the view carries %2").arg(raw.count()).arg(channels.count())));

    const QJsonObject sensors = take(qgc_core_get("view.sensors"));
    const QJsonArray all = sensors.value(QStringLiteral("sensors")).toArray();
    QVERIFY2(!all.isEmpty(), "the sensors view names no sensors at all for a connected vehicle, which is what a read that finds nothing looks like");
    const QJsonArray failing = sensors.value(QStringLiteral("failing")).toArray();
    int unhealthy = 0;
    for (const QJsonValue &sensor : all) {
        if (sensor.toObject().value(QStringLiteral("status")).toString() == QStringLiteral("unhealthy")) {
            unhealthy++;
        }
    }
    QCOMPARE(failing.count(), unhealthy);
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_aMisspelledPropertyIsToldApartFromANullOne()
{
#ifdef QGC_RUST_CORE
    const QJsonObject typo = take(qgc_bridge_get("settings.appSettings.audioMutedd"));
    QCOMPARE(typo.value(QStringLiteral("kind")).toString(), QStringLiteral("value"));
    QVERIFY2(typo.value(QStringLiteral("found")).toBool(true) == false,
             "a name that does not exist has to say so; without it a head reading a typo gets a plausible null and draws its default, with no error anywhere in the chain");
    QVERIFY(typo.value(QStringLiteral("value")).isNull());

    const QJsonObject real = take(qgc_bridge_get("settings.appSettings.audioMuted"));
    QVERIFY2(!real.contains(QStringLiteral("found")),
             "a name that does exist says nothing extra, so no head and no recorded contract sees a new key on a path that works");
    QVERIFY(!real.value(QStringLiteral("value")).isNull());

    const QJsonObject nullValued = take(qgc_bridge_get("plan.currentPlanFile"));
    QVERIFY2(!nullValued.contains(QStringLiteral("found")),
             "a property that exists and is genuinely null is a different answer from one that does not exist, which is the whole point");
    QVERIFY(nullValued.value(QStringLiteral("value")).isNull() || nullValued.value(QStringLiteral("value")).toString().isEmpty());

    const QJsonObject noSuchRoot = take(qgc_bridge_get("settingsss.appSettings.audioMuted"));
    QCOMPARE(noSuchRoot.value(QStringLiteral("kind")).toString(), QStringLiteral("null"));
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_missionStatisticsAnswerOnThePlanTheEditorHolds()
{
#ifdef QGC_RUST_CORE
    (void) take(qgc_bridge_invoke("plan.start", "[]"));
    const auto restore = []() { (void) take(qgc_bridge_invoke("plan.removeAll", "[]")); };
    const auto leaveNoPlanBehind = qScopeGuard(restore);
    restore();

    const auto number = [](const char *name) {
        return take(qgc_bridge_get(QStringLiteral("plan.missionController.%1").arg(QString::fromUtf8(name)).toUtf8().constData()))
            .value(QStringLiteral("value")).toDouble(-1.0);
    };
    const QStringList statistics = {
        QStringLiteral("missionTotalDistance"), QStringLiteral("missionPlannedDistance"), QStringLiteral("missionTime"),
        QStringLiteral("missionHoverDistance"), QStringLiteral("missionCruiseDistance"), QStringLiteral("missionMaxTelemetry"),
        QStringLiteral("batteriesRequired"), QStringLiteral("minAMSLAltitude"), QStringLiteral("maxAMSLAltitude"),
    };
    QStringList unreadable;
    for (const QString &name : statistics) {
        if (take(qgc_bridge_get(QStringLiteral("plan.missionController.%1").arg(name).toUtf8().constData())).contains(QStringLiteral("found"))) {
            unreadable.append(name);
        }
    }
    QVERIFY2(unreadable.isEmpty(), qPrintable(QStringLiteral("these are not properties on the controller at all: %1").arg(unreadable.join(QStringLiteral(", ")))));

    QVERIFY2(take(qgc_core_invoke("mission.insert", "[\"takeoff\", 47.3960, 8.5440, -1]")).value(QStringLiteral("ok")).toBool(false), "the takeoff was refused");
    QVERIFY2(take(qgc_core_invoke("mission.insert", "[\"waypoint\", 47.3990, 8.5480, -1]")).value(QStringLiteral("ok")).toBool(false), "the waypoint was refused");
    QVERIFY2(take(qgc_core_invoke("mission.insert", "[\"waypoint\", 47.3990, 8.5440, -1]")).value(QStringLiteral("ok")).toBool(false), "the second waypoint was refused");

    QTRY_VERIFY_WITH_TIMEOUT(number("missionTotalDistance") > 100.0, 10000);
    QVERIFY2(number("missionTotalDistance") > 100.0,
             "a plan spanning several hundred metres reports no distance, so anything built on these numbers would show an empty summary for every mission");
    QVERIFY2(number("missionTime") > 0.0, "a mission with distance takes time to fly");
    QVERIFY2(number("maxAMSLAltitude") >= number("minAMSLAltitude"), "the altitude range is the wrong way round");
    QVERIFY2(number("batteriesRequired") != 0.0, "batteries required reads zero, which is either a vehicle with no battery model or a number that never varies");
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_theRustCacheServesATileQtWroteIntoTheSameDatabase()
{
#ifdef QGC_RUST_CORE
    const QString databasePath = sharedTileCache();
    QTRY_VERIFY_WITH_TIMEOUT(tileCacheIsReady(databasePath), 20000);
    const auto closeWhenDone = qScopeGuard([]() { qgc_core_tile_close(); });

    const QString type = QStringLiteral("Bing Road");
    const int x = 8523;
    const int y = 5606;
    const int zoom = 14;
    const QString hash = UrlFactory::getTileHash(type, x, y, zoom);

    QByteArray drawn(2048, char(0));
    for (int index = 0; index < drawn.size(); index++) {
        drawn[index] = char(index % 251);
    }
    QGCCacheTile *const tile = new QGCCacheTile(hash, drawn, QStringLiteral("png"), type);
    QVERIFY2(QGCMapEngine::instance()->addTask(new QGCSaveTileTask(tile)), "the map engine would not take the tile");

    const QJsonObject opened = take(qgc_core_tile_open(databasePath.toUtf8().constData()));
    QVERIFY2(opened.value(QStringLiteral("ok")).toBool(false), qPrintable(QStringLiteral("the Rust cache could not open the database the map engine is using: %1").arg(opened.value(QStringLiteral("reason")).toString())));

    // The worker writes on its own thread, so wait for the row rather than for the file.
    QElapsedTimer waitingForTheTile;
    waitingForTheTile.start();
    while (qgc_core_tile_size(hash.toUtf8().constData()) != drawn.size() && waitingForTheTile.elapsed() < 20000) {
        QTest::qWait(100);
        (void) take(qgc_core_tile_open(databasePath.toUtf8().constData()));
    }
    // Read the size once. Calling it in the condition and again in the assertion let the two
    // disagree - the Qt worker is still writing to the same database - so the diagnostic block was
    // skipped while the assertion failed, which is how this failure produced an empty report every
    // time it fired.
    const qint64 served = qgc_core_tile_size(hash.toUtf8().constData());
    QString stored;
    if (served != drawn.size()) {
        {
        QSqlDatabase probe = QSqlDatabase::addDatabase(QStringLiteral("QSQLITE"), QStringLiteral("tileRowProbe"));
        probe.setDatabaseName(databasePath);
        QVERIFY(probe.open());
        {
        QSqlQuery rows(probe);
        stored = QStringLiteral("file %1 bytes; ").arg(QFileInfo(databasePath).size());
        if (rows.exec(QStringLiteral("SELECT hash, size, typeof(type) FROM Tiles"))) {
            int counted = 0;
            while (rows.next()) {
                counted++;
                stored += QStringLiteral("\n  %1 %2 bytes, type stored as %3").arg(rows.value(0).toString()).arg(rows.value(1).toInt()).arg(rows.value(2).toString());
            }
            stored += QStringLiteral("\n  %1 rows in Tiles, and the one wanted is %2").arg(counted).arg(hash);
        } else {
            stored += QStringLiteral("the Tiles table could not be read at all: %1").arg(rows.lastError().text());
        }
        }
        probe.close();
        }
        QSqlDatabase::removeDatabase(QStringLiteral("tileRowProbe"));
    }
    QVERIFY2(served == drawn.size(),
             qPrintable(QStringLiteral("the Rust cache served %1 bytes where Qt wrote %2. What the database holds:\n%3").arg(served).arg(drawn.size()).arg(stored)));


    QByteArray read(drawn.size(), char(0));
    const qint64 copied = qgc_core_tile_copy(hash.toUtf8().constData(), reinterpret_cast<unsigned char *>(read.data()), read.size());
    QVERIFY2(copied != -3, "the database could not be read, which is a different fact from the tile being absent and used to be reported as one");
    QCOMPARE(copied, qint64(drawn.size()));
    QVERIFY2(read == drawn, "the bytes came back changed, so the two sides disagree about what is stored rather than about where");

    QByteArray tooSmall(drawn.size() - 1, char(0));
    QCOMPARE(qgc_core_tile_copy(hash.toUtf8().constData(), reinterpret_cast<unsigned char *>(tooSmall.data()), tooSmall.size()), qint64(-2));
    QCOMPARE(qgc_core_tile_size("0000000000000000000000000000"), qint64(-1));

    // The hash is the whole gate: a key computed differently is a tile downloaded again.
    char *const spelled = qgc_core_tile_hash(qgc_core_tile_provider(type.toUtf8().constData()), x, y, zoom);
    QCOMPARE(QString::fromUtf8(spelled), hash);
    qgc_core_free(spelled);
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_theMissionSummaryArrivesOnItsOwnAfterAnEdit()
{
#ifdef QGC_RUST_CORE
    (void) take(qgc_bridge_invoke("plan.start", "[]"));
    const auto restore = []() { (void) take(qgc_bridge_invoke("plan.removeAll", "[]")); };
    const auto leaveNoPlanBehind = qScopeGuard([&restore]() {
        restore();
        qgc_bridge_watch("");
        qgc_bridge_set_event_handler(nullptr);
    });
    restore();

    const auto distance = []() {
        return take(qgc_core_get("view.missionSummary")).value(QStringLiteral("distanceMetres")).toDouble(-1.0);
    };
    QVERIFY2(take(qgc_core_invoke("mission.insert", "[\"takeoff\", 47.3960, 8.5440, -1]")).value(QStringLiteral("ok")).toBool(false), "the takeoff was refused");

    paths.clear();
    qgc_bridge_set_event_handler(onEvent);
    qgc_bridge_watch("view.missionSummary");

    // The controller recomputes its totals after the insert returns, so a head that reads once and
    // stops is a mission behind for as long as it stays stopped. The answer is not for the insert
    // to block - it runs on the same thread the recompute is queued on - it is that the view says
    // so when it changes.
    QVERIFY2(take(qgc_core_invoke("mission.insert", "[\"waypoint\", 47.3990, 8.5480, -1]")).value(QStringLiteral("ok")).toBool(false), "the waypoint was refused");
    QTRY_VERIFY_WITH_TIMEOUT(paths.contains(QStringLiteral("view.missionSummary")), 10000);
    QTRY_VERIFY_WITH_TIMEOUT(distance() > 100.0, 10000);

    const double afterTwo = distance();
    paths.clear();
    QVERIFY2(take(qgc_core_invoke("mission.insert", "[\"waypoint\", 47.3990, 8.5440, -1]")).value(QStringLiteral("ok")).toBool(false), "the second waypoint was refused");
    QTRY_VERIFY_WITH_TIMEOUT(paths.contains(QStringLiteral("view.missionSummary")), 10000);
    QTRY_VERIFY_WITH_TIMEOUT(distance() > afterTwo, 10000);
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_aSignalWithNoPropertyBehindItStillWakesAView()
{
#ifdef QGC_RUST_CORE
    (void) take(qgc_bridge_invoke("plan.start", "[]"));
    const auto restore = []() { (void) take(qgc_bridge_invoke("plan.removeAll", "[]")); };
    const auto leaveNoPlanBehind = qScopeGuard([&restore]() {
        restore();
        qgc_bridge_watch("");
        qgc_bridge_set_event_handler(nullptr);
    });
    restore();

    const QString fires = QStringLiteral("plan.missionController@recalcTerrainProfile");
    const QString absent = QStringLiteral("plan.missionController@noSuchSignalExists");
    const auto counted = [](const QString &path) { return paths.count(path); };
    const auto payloadFor = [](const QString &path) {
        const qsizetype index = paths.indexOf(path);
        return (index < 0) ? QString() : payloads.at(index);
    };

    paths.clear();
    payloads.clear();
    qgc_bridge_set_event_handler(onEvent);
    const QByteArray watched = (fires + QLatin1Char(',') + absent).toUtf8();
    qgc_bridge_watch(watched.constData());

    // Terrain heights arrive long after the item count and the dirty flag have settled, so a
    // profile watched through those alone is drawn once with nothing in it and never redrawn. The
    // controller already emits when the profile needs redrawing; that it carries no value was the
    // only reason the watcher could not bind to it.
    const QStringList inserts = {
        QStringLiteral("[\"takeoff\", 47.3960, 8.5440, -1]"),
        QStringLiteral("[\"waypoint\", 47.3990, 8.5480, -1]"),
        QStringLiteral("[\"waypoint\", 47.3990, 8.5440, -1]"),
    };
    for (const QString &insert : inserts) {
        const QByteArray args = insert.toUtf8();
        QVERIFY2(take(qgc_core_invoke("mission.insert", args.constData())).value(QStringLiteral("ok")).toBool(false), "an insert was refused");
        const int before = counted(fires);
        QTRY_VERIFY_WITH_TIMEOUT(counted(fires) > before, 10000);
    }

    QVERIFY2(counted(fires) >= inserts.count(), "the controller's redraw signal did not reach a head watching it");

    // A name that matches no signal is not a signal path at all - it falls back to being read as a
    // property, which is how the head learns the path is wrong. Answering it with a fired counter
    // instead would defeat the value dedup that keeps unresolvable paths quiet, so a mistyped
    // signal name would recompute its view on every poll tick while looking like a watch that
    // works. The tell is in what the event carries, not how often it arrives.
    const QString mistyped = payloadFor(absent);
    QVERIFY2(!mistyped.contains(QStringLiteral("fired")), qPrintable(QStringLiteral("a signal that does not exist reported itself as firing: ") + mistyped));
    QVERIFY2(mistyped.isEmpty() || mistyped.contains(QStringLiteral("false")), qPrintable(QStringLiteral("a path naming no signal and no property answered as though it resolved: ") + mistyped));
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_everyDependencyAViewDeclaresNamesSomethingTheBridgeHas()
{
#ifdef QGC_RUST_CORE
    // Without a vehicle, every chain through vehicle or the inspector's active system breaks at an
    // absent object and answers found:false, which is indistinguishable from a misspelling. The
    // check is only meaningful against a connected vehicle.
    _connectMockLink(MAV_AUTOPILOT_PX4);
    QTRY_VERIFY_WITH_TIMEOUT(MultiVehicleManager::instance()->activeVehicle() != nullptr, 10000);

    const QJsonArray views = take(qgc_core_get("view.dependencies")).value(QStringLiteral("views")).toArray();
    QVERIFY2(views.count() > 40, "the core served no dependency list, so this test would pass by finding nothing");

    // A dep that names no property binds to no signal. The watcher then falls back to a re-read
    // that only runs while the event loop is idle, and the value dedup silences it after the first
    // miss - so a misspelled dep makes its view quietly stop updating rather than fail anywhere.
    // found:false is the only thing that separates a name the bridge does not have from a value
    // that is legitimately absent because nothing is connected.
    QStringList unknown;
    int checked = 0;
    for (const QJsonValue &view : views) {
        const QString path = view.toObject().value(QStringLiteral("path")).toString();
        for (const QJsonValue &dep : view.toObject().value(QStringLiteral("deps")).toArray()) {
            const QString named = dep.toString();
            if (named.contains(QLatin1Char('@'))) {
                continue;
            }
            // Which parameters a vehicle has is the vehicle's business: the mode-slot names differ
            // between firmware families and only one family is ever present, so an unresolved
            // parameter here is the expected state rather than a misspelling. Everything else has
            // to resolve.
            if (named.startsWith(QStringLiteral("vehicle.parameterManager.getParameter("))) {
                continue;
            }
            checked++;
            const QByteArray utf8 = named.toUtf8();
            const QJsonObject read = take(qgc_bridge_get(utf8.constData()));
            if (read.value(QStringLiteral("found")).toBool(true) == false) {
                unknown.append(QStringLiteral("%1 declares %2").arg(path, named));
            }
        }
    }
    QVERIFY2(checked > 100, qPrintable(QStringLiteral("only %1 deps were resolved, which is too few to be the whole set").arg(checked)));
    QVERIFY2(unknown.isEmpty(), qPrintable(QStringLiteral("these deps name nothing the bridge has, so their views never recompute: %1").arg(unknown.join(QStringLiteral(", ")))));
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_changingAModeSlotWakesThePanelThatShowsIt()
{
#ifdef QGC_RUST_CORE
    _connectMockLink(MAV_AUTOPILOT_ARDUPILOTMEGA);
    QTRY_VERIFY_WITH_TIMEOUT(MultiVehicleManager::instance()->activeVehicle() && MultiVehicleManager::instance()->activeVehicle()->parameterManager()->parametersReady(), 20000);

    const auto leaveNothingWatched = qScopeGuard([]() {
        qgc_bridge_watch("");
        qgc_bridge_set_event_handler(nullptr);
    });

    const QJsonObject panel = take(qgc_core_get("view.modeSlots"));
    QVERIFY2(panel.value(QStringLiteral("available")).toBool(false), qPrintable(QStringLiteral("this vehicle does not offer mode slots, so the test proves nothing: %1").arg(panel.value(QStringLiteral("reason")).toString())));

    paths.clear();
    qgc_bridge_set_event_handler(onEvent);
    qgc_bridge_watch("view.modeSlots");
    QTRY_VERIFY_WITH_TIMEOUT(paths.contains(QStringLiteral("view.modeSlots")), 10000);

    // The slot parameters are the whole content of this panel and were read through getParameter,
    // which no dep named - so changing a mode left the panel showing the old one until something
    // unrelated happened to fire. Watching the parameter is only worth anything if the watcher can
    // actually bind to it, which is what this asserts rather than assumes.
    const QJsonArray before = take(qgc_core_get("view.modeSlots")).value(QStringLiteral("slots")).toArray();
    QVERIFY(before.count() >= 2);
    const QString firstMode = before.at(0).toObject().value(QStringLiteral("mode")).toString();

    // ArduCopter names these MODE1..6 and ArduPlane FLTMODE1..6, which is exactly why both
    // families are in the deps. The test has to ask rather than assume, or it writes to a name
    // this vehicle does not have and reports the watch broken when it is the write that missed.
    const bool copterNames = take(qgc_bridge_invoke("vehicle.parameterManager.parameterExists", "[-1,\"MODE_CH\"]")).value(QStringLiteral("result")).toBool(false);
    const QString slotName = copterNames ? QStringLiteral("MODE1") : QStringLiteral("FLTMODE1");
    const QByteArray slotPath = QStringLiteral("vehicle.parameterManager.getParameter(-1,%1).rawValue").arg(slotName).toUtf8();

    const double current = take(qgc_bridge_get(slotPath.constData())).value(QStringLiteral("value")).toDouble(-1.0);
    QVERIFY2(current >= 0.0, qPrintable(QStringLiteral("%1 did not read back, so the write below would prove nothing").arg(slotName)));
    const double wanted = (current == 7.0) ? 5.0 : 7.0;

    paths.clear();
    const QByteArray writeValue = QStringLiteral("{\"value\":%1}").arg(wanted).toUtf8();
    const QJsonObject written = take(qgc_bridge_set(slotPath.constData(), writeValue.constData()));
    QVERIFY2(written.value(QStringLiteral("ok")).toBool(false),
             qPrintable(QStringLiteral("%1 would not take a write: %2").arg(slotName, written.value(QStringLiteral("reason")).toString())));

    // Order matters: if the value never changes, the watch has nothing to report and a failure
    // here would be blamed on the watch. Prove the write landed first, then that it woke anyone.
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_core_get("view.modeSlots")).value(QStringLiteral("slots")).toArray().at(0).toObject().value(QStringLiteral("mode")).toString() != firstMode, 15000);
    QVERIFY2(paths.contains(QStringLiteral("view.modeSlots")),
             qPrintable(QStringLiteral("the slot changed from %1 to %2 and no one watching the panel was told")
                            .arg(firstMode, take(qgc_core_get("view.modeSlots")).value(QStringLiteral("slots")).toArray().at(0).toObject().value(QStringLiteral("mode")).toString())));
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_replacingAPlanWithOneTheSameLengthStillWakesTheItemList()
{
#ifdef QGC_RUST_CORE
    const QString fixture = QFileInfo(QString::fromUtf8(__FILE__)).dir().filePath(QStringLiteral("../MissionManager/SectionTest.plan"));
    const QByteArray loadArgs = QJsonDocument(QJsonArray { fixture }).toJson(QJsonDocument::Compact);

    (void) take(qgc_bridge_invoke("plan.start", "[]"));
    const auto leaveNoPlanBehind = qScopeGuard([]() {
        (void) take(qgc_bridge_invoke("plan.removeAll", "[]"));
        qgc_bridge_watch("");
        qgc_bridge_set_event_handler(nullptr);
    });
    QVERIFY2(take(qgc_bridge_invoke("plan.loadFromFile", loadArgs.constData())).value(QStringLiteral("result")).toBool(false), "the loader refused the fixture");
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_core_get("view.missionItems")).value(QStringLiteral("items")).toArray().count() > 1, 10000);
    const int held = take(qgc_core_get("view.missionItems")).value(QStringLiteral("items")).toArray().count();

    paths.clear();
    qgc_bridge_set_event_handler(onEvent);
    qgc_bridge_watch("view.missionItems");
    QTRY_VERIFY_WITH_TIMEOUT(paths.contains(QStringLiteral("view.missionItems")), 10000);

    // A fly view never edits this plan - it receives one from the vehicle or from a file. Loading
    // the same file again replaces every item and leaves the count, the current index and
    // containsItems exactly as they were, so the three property deps cannot see it. Without the
    // controller's own rebuild signal a head watching this view would sit on the old plan and a
    // 2Hz poll would be the only thing that noticed.
    paths.clear();
    QVERIFY2(take(qgc_bridge_invoke("plan.loadFromFile", loadArgs.constData())).value(QStringLiteral("result")).toBool(false), "the loader refused the second load");
    QCOMPARE(take(qgc_core_get("view.missionItems")).value(QStringLiteral("items")).toArray().count(), held);
    QVERIFY2(paths.contains(QStringLiteral("view.missionItems")),
             qPrintable(QStringLiteral("the plan was replaced with another of %1 items and nobody watching the list was told").arg(held)));
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_everyDependencyAViewDeclaresActuallyBindsToASignal()
{
#ifdef QGC_RUST_CORE
    _connectMockLink(MAV_AUTOPILOT_PX4);
    QTRY_VERIFY_WITH_TIMEOUT(MultiVehicleManager::instance()->activeVehicle() != nullptr, 10000);
    (void) take(qgc_bridge_invoke("plan.start", "[]"));
    const auto leaveNothingWatched = qScopeGuard([]() {
        qgc_bridge_watch("");
        qgc_bridge_set_event_handler(nullptr);
    });
    // Binding happens inside the poll, and the poll returns immediately when no event handler is
    // set - so without this every dep reads as unbound and the test reports the entire watch
    // system broken. It did exactly that twice before this line existed.
    paths.clear();
    qgc_bridge_set_event_handler(onEvent);

    // A dep that does not bind is not an error and says so nowhere: it falls through to a re-read
    // that only runs while the event loop is idle, and no test can tell the two apart because
    // QTRY spins that loop. Asking the watcher directly is the only way to see it.
    // Each of these names a list model or a bare object root whose Q_PROPERTY was read and found
    // to be CONSTANT. Not because list models are CONSTANT - 33 of them in this tree are and 21
    // are not, visualItems among the latter - but because these six were checked one at a time. That is a recorded decision per entry rather than an
    // accident, and anything new arriving here has to earn its place:
    //
    //   fence polygons, circles, rally points - CONSTANT list models. Replacement is covered by
    //     plan.geoFenceController@loadComplete; drawing a fence is not, and the only signal that
    //     would cover it carries an argument, which the watcher does not bind to. Left polled
    //     deliberately: a watch right for two of three edit shapes is worse than no watch.
    //   links.linkConfigurations - CONSTANT list model, and link edits are rare enough that a
    //     200ms re-read costs nothing.
    //   logDownload.model - CONSTANT list model, but the list arriving is already covered:
    //     requestingList goes false when the fetch completes and that dep does bind.
    //   mavlinkInspector.activeSystem.messages - CONSTANT list model, and message arrival is a
    //     flood. A rate display wants sampling, not an event per packet.
    //   radioCal, sensorsCal - bare object roots, which name no property at all.
    const QStringList knowinglyPolled = {
        QStringLiteral("plan.geoFenceController.polygons"),
        QStringLiteral("plan.geoFenceController.circles"),
        QStringLiteral("plan.rallyPointController.points"),
        QStringLiteral("links.linkConfigurations"),
        QStringLiteral("logDownload.model"),
        QStringLiteral("mavlinkInspector.activeSystem.messages"),
        QStringLiteral("radioCal"),
        QStringLiteral("sensorsCal"),
        QStringLiteral("vehicle.id"),
    };

    const QJsonArray views = take(qgc_core_get("view.dependencies")).value(QStringLiteral("views")).toArray();
    QVERIFY(views.count() > 40);
    QStringList everyView;
    for (const QJsonValue &view : views) {
        if (!view.toObject().value(QStringLiteral("deps")).toArray().isEmpty()) {
            everyView.append(view.toObject().value(QStringLiteral("path")).toString());
        }
    }

    // Watching them all at once and reading the result once avoids racing the queued setPaths
    // per view, which is what made the first version of this report every dep unbound.
    const QByteArray asked = everyView.join(QLatin1Char(',')).toUtf8();
    qgc_bridge_watch(asked.constData());
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_watch_status()).value(QStringLiteral("paths")).toArray().count() > 80, 10000);

    const QJsonArray reported = take(qgc_bridge_watch_status()).value(QStringLiteral("paths")).toArray();
    QStringList unbound;
    for (const QJsonValue &entry : reported) {
        const QString dep = entry.toObject().value(QStringLiteral("path")).toString();
        if (dep.startsWith(QStringLiteral("vehicle.parameterManager.getParameter(")) || knowinglyPolled.contains(dep)) {
            continue;
        }
        if (!entry.toObject().value(QStringLiteral("bound")).toBool(false)) {
            unbound.append(dep);
        }
    }
    QVERIFY2(unbound.isEmpty(),
             qPrintable(QStringLiteral("%1 of %2 deps bound to no signal, so their views are served by the idle poll rather than watched: %3")
                            .arg(unbound.count()).arg(reported.count()).arg(unbound.mid(0, 14).join(QStringLiteral(", ")))));
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_theCoreWorksOutTheSameFlownDistanceTheControllerDoes()
{
#ifdef QGC_RUST_CORE
    (void) take(qgc_bridge_invoke("plan.start", "[]"));
    const auto restore = []() { (void) take(qgc_bridge_invoke("plan.removeAll", "[]")); };
    const auto leaveNoPlanBehind = qScopeGuard(restore);
    restore();

    const auto agree = [](const char *shape) {
        const QJsonObject summary = take(qgc_core_get("view.missionSummary(verify)"));
        const double controller = summary.value(QStringLiteral("distanceMetres")).toDouble(-1.0);
        const double core = summary.value(QStringLiteral("distanceComputedMetres")).toDouble(-1.0);
        QVERIFY2(controller > 0.0, qPrintable(QStringLiteral("%1: the controller reported no distance, so there is nothing to agree with").arg(shape)));
        QVERIFY2(qAbs(core - controller) < qMax(1.0, controller * 0.001),
                 qPrintable(QStringLiteral("%1: the core makes it %2 m and the controller %3 m").arg(shape).arg(core).arg(controller)));

        // The furthest point from launch, which the controller measures over a pattern's transect
        // points rather than its entry corner - so a survey's far side counts and the corner it
        // starts at does not decide it.
        const double reachQt = summary.value(QStringLiteral("maxTelemetryMetres")).toDouble(-1.0);
        const double reachCore = summary.value(QStringLiteral("maxTelemetryComputedMetres")).toDouble(-1.0);
        QVERIFY2(reachQt > 0.0, qPrintable(QStringLiteral("%1: the controller reported no telemetry reach").arg(shape)));
        QVERIFY2(qAbs(reachCore - reachQt) < qMax(1.0, reachQt * 0.001),
                 qPrintable(QStringLiteral("%1: the core reaches %2 m and the controller %3 m").arg(shape).arg(reachCore).arg(reachQt)));
    };

    const auto insert = [](const char *args) {
        const QJsonObject answered = take(qgc_core_invoke("mission.insert", args));
        QVERIFY2(answered.value(QStringLiteral("ok")).toBool(false),
                 qPrintable(QStringLiteral("%1 was refused: %2").arg(QString::fromUtf8(args), answered.value(QStringLiteral("reason")).toString())));
    };

    // The arithmetic has rules that are invisible from the answer: the leg is measured from the
    // previous item's exit rather than its entry, a pattern contributes its own path on top of the
    // leg into it, the walk does not count a first leg out of the plan's settings entry, and a
    // landing ends the route so the leg after it is not counted at all.
    insert("[\"takeoff\", 47.3960, 8.5440, -1]");
    insert("[\"waypoint\", 47.3990, 8.5480, -1]");
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_core_get("view.missionSummary")).value(QStringLiteral("distanceMetres")).toDouble(0.0) > 0.0, 10000);
    agree("two waypoints");

    insert("[\"waypoint\", 47.4020, 8.5440, -1]");
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_core_get("view.missionSummary")).value(QStringLiteral("distanceMetres")).toDouble(0.0) > 500.0, 10000);
    agree("three waypoints");

    insert("[\"survey\", 47.3975, 8.5460, -1]");
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_core_get("view.missionItems")).value(QStringLiteral("items")).toArray().count() >= 5, 20000);
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_core_get("view.missionSummary")).value(QStringLiteral("distanceMetres")).toDouble(0.0) > 1000.0, 20000);
    agree("with a survey in the middle");

    // A region of interest has a position and earns a marker, and the aircraft never flies to it.
    // A walk that counts every placed item doglegs out to it and back - which is exactly the
    // defect the macOS head found drawing the route on its map this morning, from the same
    // conflation between "has a coordinate" and "is on the route".
    const double beforeRoi = take(qgc_core_get("view.missionSummary")).value(QStringLiteral("distanceMetres")).toDouble(-1.0);
    insert("[\"roi\", 47.4100, 8.5600, -1]");
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_core_get("view.missionItems")).value(QStringLiteral("items")).toArray().count() >= 6, 20000);
    agree("with a region of interest off the route");
    QVERIFY2(qAbs(take(qgc_core_get("view.missionSummary")).value(QStringLiteral("distanceMetres")).toDouble(-1.0) - beforeRoi) < 1.0,
             "a region of interest is not flown to, so adding one a kilometre away must not lengthen the mission");

    insert("[\"land\", 47.3960, 8.5440, -1]");
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_core_get("view.missionItems")).value(QStringLiteral("items")).toArray().count() >= 7, 20000);
    agree("ending in a landing");

    // A landing that is last cannot distinguish "the leg after it is dropped" from "there is no
    // leg after it". This is the shape that separates them, and the first rule I wrote - negating
    // on a land command - walked straight past a return to launch into an item that is uploaded
    // and never reached. The macOS head found it by building a plan my tests did not.
    // The shape that exposed the first version of this rule - an item left beyond the end of the
    // route - cannot be built here. The plan editor refuses to append after a landing, and the
    // core's own insert refuses to put a landing in front of places the vehicle flies through. It
    // arises from a return to launch, which nothing here can insert, so the rule is pinned as a
    // unit test over a hand-built list instead: missionsummary::the_walk_stops_where_the_route_ends.
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_everyClassTheCatalogueNamesIsTheClassTheEditorBuilds()
{
#ifdef QGC_RUST_CORE
    (void) take(qgc_bridge_invoke("plan.start", "[]"));
    const auto restore = []() { (void) take(qgc_bridge_invoke("plan.removeAll", "[]")); };
    const auto leaveNoPlanBehind = qScopeGuard(restore);
    restore();

    // className is a hand-written C++ class name, and a hand-written list of names rots exactly as
    // silently as the translated ones it replaced: misspell one and by_class matches nothing, the
    // item falls back to "complex", and it loses its geometry again with nothing failing. Building
    // one of each and asking what the editor actually made is the only check that cannot drift.
    const QJsonArray kinds = take(qgc_core_get("view.missionKinds")).value(QStringLiteral("kinds")).toArray();
    QVERIFY2(kinds.count() > 3, "the catalogue served nothing, so this would pass by checking none of it");

    int checked = 0;
    for (const QJsonValue &entry : kinds) {
        const QString named = entry.toObject().value(QStringLiteral("className")).toString();
        if (named.isEmpty()) {
            continue;
        }
        const QString id = entry.toObject().value(QStringLiteral("id")).toString();
        restore();
        // The core refuses a pattern before a takeoff, so each round needs a plan it will accept.
        QVERIFY2(take(qgc_core_invoke("mission.insert", "[\"takeoff\", 47.3960, 8.5440, -1]")).value(QStringLiteral("ok")).toBool(false), "the takeoff was refused");
        const QByteArray args = QStringLiteral("[\"%1\", 47.3970, 8.5460, -1]").arg(id).toUtf8();
        const QJsonObject answered = take(qgc_core_invoke("mission.insert", args.constData()));
        QVERIFY2(answered.value(QStringLiteral("ok")).toBool(false),
                 qPrintable(QStringLiteral("%1 was refused: %2").arg(id, answered.value(QStringLiteral("reason")).toString())));
        QTRY_VERIFY_WITH_TIMEOUT(take(qgc_core_get("view.missionItems")).value(QStringLiteral("items")).toArray().count() >= 3, 20000);

        const QJsonArray items = take(qgc_core_get("view.missionItems")).value(QStringLiteral("items")).toArray();
        QVERIFY2(items.last().toObject().value(QStringLiteral("kind")).toString() == id,
                 qPrintable(QStringLiteral("the catalogue calls it %1 and the item reads back as %2, so the class name does not match what the editor built")
                                .arg(id, items.last().toObject().value(QStringLiteral("kind")).toString())));
        checked++;
    }
    QVERIFY2(checked >= 2, qPrintable(QStringLiteral("only %1 classes were checked, too few to be the complex kinds").arg(checked)));
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_aTakeoffReportedInsertedHasAPlaceOnTheMap()
{
#ifdef QGC_RUST_CORE
    (void) take(qgc_bridge_invoke("plan.start", "[]"));
    const auto restore = []() { (void) take(qgc_bridge_invoke("plan.removeAll", "[]")); };
    const auto leaveNoPlanBehind = qScopeGuard(restore);
    restore();

    // The insert answers ok when the write answers ok, and a write answers ok when setProperty
    // accepted a value - which is not the same as the item having a position afterwards. The
    // Android head sees a takeoff reported inserted with no position on it and none on the plan's
    // own entry either, so the claim of success has to be checked against what the item reads back.
    const QJsonObject answered = take(qgc_core_invoke("mission.insert", "[\"takeoff\", 47.3960, 8.5440, -1]"));
    QVERIFY2(answered.value(QStringLiteral("ok")).toBool(false),
             qPrintable(QStringLiteral("the takeoff was refused: %1").arg(answered.value(QStringLiteral("reason")).toString())));

    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_core_get("view.missionItems")).value(QStringLiteral("items")).toArray().count() >= 2, 10000);
    const QJsonArray items = take(qgc_core_get("view.missionItems")).value(QStringLiteral("items")).toArray();

    const QJsonObject settings = items.at(0).toObject();
    const QJsonObject takeoff = items.at(1).toObject();
    QCOMPARE(settings.value(QStringLiteral("kind")).toString(), QStringLiteral("settings"));
    QCOMPARE(takeoff.value(QStringLiteral("kind")).toString(), QStringLiteral("takeoff"));

    QVERIFY2(!settings.value(QStringLiteral("coordinate")).isNull(),
             "setLaunchCoordinate places the plan's own entry, so a launch position that landed shows there first");
    QVERIFY2(!takeoff.value(QStringLiteral("coordinate")).isNull(),
             "a takeoff reported inserted and carrying no position is an insert that claimed a success it did not have");
    QVERIFY2(takeoff.value(QStringLiteral("movable")).toBool(false),
             "and a head cannot draw a pin for it either, which is how this is seen rather than read");
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_theItemListNamesWhatTheControllerHolds()
{
#ifdef QGC_RUST_CORE
    (void) take(qgc_bridge_invoke("plan.start", "[]"));
    const auto restore = []() { (void) take(qgc_bridge_invoke("plan.removeAll", "[]")); };
    const auto leaveNoPlanBehind = qScopeGuard(restore);
    restore();

    const QJsonObject empty = take(qgc_core_get("view.missionItems"));
    QCOMPARE(empty.value(QStringLiteral("available")).toBool(true), false);

    QVERIFY2(take(qgc_core_invoke("mission.insert", "[\"takeoff\", 47.3960, 8.5440, -1]")).value(QStringLiteral("ok")).toBool(false), "the takeoff was refused");
    QVERIFY2(take(qgc_core_invoke("mission.insert", "[\"waypoint\", 47.3990, 8.5480, -1]")).value(QStringLiteral("ok")).toBool(false), "the waypoint was refused");
    QVERIFY2(take(qgc_core_invoke("mission.insert", "[\"survey\", 47.3975, 8.5460, -1]")).value(QStringLiteral("ok")).toBool(false), "the survey was refused");

    const int held = take(qgc_bridge_get("plan.missionController.visualItems.count")).value(QStringLiteral("value")).toInt(-1);
    QTRY_COMPARE_WITH_TIMEOUT(take(qgc_core_get("view.missionItems")).value(QStringLiteral("items")).toArray().count(), held, 10000);

    const QJsonArray listed = take(qgc_core_get("view.missionItems")).value(QStringLiteral("items")).toArray();
    QStringList wrong;
    for (int index = 0; index < held; index++) {
        const QString path = QStringLiteral("plan.missionController.visualItems.%1").arg(index);
        const QJsonObject controller = take(qgc_bridge_get(path.toUtf8().constData()));
        const QJsonObject shown = listed.at(index).toObject();
        if (shown.value(QStringLiteral("sequence")).toInt(-1) != controller.value(QStringLiteral("sequenceNumber")).toInt(-2)) {
            wrong.append(QStringLiteral("item %1 sequence").arg(index));
        }
        if (shown.value(QStringLiteral("name")).toString() != controller.value(QStringLiteral("commandName")).toString()) {
            wrong.append(QStringLiteral("item %1 name: %2 against %3").arg(index).arg(shown.value(QStringLiteral("name")).toString(), controller.value(QStringLiteral("commandName")).toString()));
        }
        if (shown.value(QStringLiteral("abbreviation")).toString() != controller.value(QStringLiteral("abbreviation")).toString()) {
            wrong.append(QStringLiteral("item %1 abbreviation").arg(index));
        }
    }
    QVERIFY2(wrong.isEmpty(), qPrintable(QStringLiteral("the list disagrees with the controller it is describing: %1").arg(wrong.join(QStringLiteral(", ")))));

    QStringList kinds;
    for (const QJsonValue &entry : listed) {
        kinds.append(entry.toObject().value(QStringLiteral("kind")).toString());
    }
    QVERIFY2(kinds.first() == QStringLiteral("settings"), qPrintable(QStringLiteral("the first entry is the plan's own, and it read as %1").arg(kinds.first())));
    QVERIFY2(kinds.contains(QStringLiteral("takeoff")), qPrintable(kinds.join(QStringLiteral(", "))));
    QVERIFY2(kinds.contains(QStringLiteral("survey")), qPrintable(kinds.join(QStringLiteral(", "))));
    QVERIFY2(kinds.contains(QStringLiteral("waypoint")), qPrintable(kinds.join(QStringLiteral(", "))));
    QVERIFY2(!kinds.contains(QString()), "an item the core could not classify would draw as nothing at all");

    const QJsonObject takeoff = listed.at(kinds.indexOf(QStringLiteral("takeoff"))).toObject();
    QVERIFY2(takeoff.value(QStringLiteral("coordinate")).toObject().value(QStringLiteral("latitude")).toDouble() != 0.0,
             "an item that has a place on the map has to carry it, or the list is a list of names");
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_theModeSlotsReadTheChannelTheVehicleNames()
{
#ifdef QGC_RUST_CORE
    _connectMockLink(MAV_AUTOPILOT_ARDUPILOTMEGA);
    const auto disconnectWhenDone = qScopeGuard([this]() { _disconnectMockLink(); });
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get("vehicle.parameterManager.parametersReady")).value(QStringLiteral("value")).toBool(false), 90000);

    const QJsonObject exists = take(qgc_bridge_invoke("vehicle.parameterManager.parameterExists", "[-1,\"FLTMODE_CH\"]"));
    QVERIFY2(exists.value(QStringLiteral("result")).toBool(false), "this vehicle has no FLTMODE_CH, so the test below would pass by checking nothing");

    const double named = take(qgc_bridge_get("vehicle.parameterManager.getParameter(-1,FLTMODE_CH).rawValue")).value(QStringLiteral("value")).toDouble(-1.0);
    QVERIFY2(named > 0.0, "the vehicle did not answer which channel carries the mode switch");

    const QJsonObject offered = take(qgc_core_get("view.modeSlots"));
    QVERIFY2(offered.value(QStringLiteral("available")).toBool(false), qPrintable(offered.value(QStringLiteral("reason")).toString()));
    QVERIFY2(offered.value(QStringLiteral("channel")).toInt(-1) == int(named),
             qPrintable(QStringLiteral("the vehicle says the switch is on channel %1 and the view read channel %2. Reading the parameter wrongly leaves every vehicle on channel five.")
                            .arg(named).arg(offered.value(QStringLiteral("channel")).toInt(-1))));

    const QJsonArray listed = offered.value(QStringLiteral("slots")).toArray();
    QCOMPARE(listed.count(), 6);
    QStringList wrong;
    for (int slot = 1; slot <= listed.count(); slot++) {
        const QString spelled = take(qgc_bridge_get(QStringLiteral("vehicle.parameterManager.getParameter(-1,FLTMODE%1)").arg(slot).toUtf8().constData()))
                                    .value(QStringLiteral("enumOrValueString")).toString();
        const QString shown = listed.at(slot - 1).toObject().value(QStringLiteral("mode")).toString();
        if (shown != spelled) {
            wrong.append(QStringLiteral("slot %1: vehicle says %2, view says %3").arg(slot).arg(spelled, shown));
        }
    }
    QVERIFY2(wrong.isEmpty(), qPrintable(wrong.join(QStringLiteral("; "))));
    QVERIFY2(!listed.first().toObject().value(QStringLiteral("mode")).toString().isEmpty(),
             "every slot read as empty, which is what reading the wrong parameter names looks like");
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_everyEditablePathTheCoreNamesAcceptsAWrite()
{
#ifdef QGC_RUST_CORE
    (void) take(qgc_bridge_invoke("plan.start", "[]"));
    const auto restore = []() { (void) take(qgc_bridge_invoke("plan.removeAll", "[]")); };
    const auto leaveNoPlanBehind = qScopeGuard(restore);
    restore();

    QStringList checked;
    QStringList unwritable;
    for (const QString &kind : { QStringLiteral("takeoff"), QStringLiteral("waypoint"), QStringLiteral("survey"), QStringLiteral("corridor") }) {
        const QJsonObject added = take(qgc_core_invoke("mission.insert", QStringLiteral("[\"%1\", 47.3975, 8.5460, -1]").arg(kind).toUtf8().constData()));
        QVERIFY2(added.value(QStringLiteral("ok")).toBool(false), qPrintable(QStringLiteral("%1: %2").arg(kind, added.value(QStringLiteral("reason")).toString())));

        QJsonObject editing;
        QTRY_VERIFY_WITH_TIMEOUT(!(editing = take(qgc_core_get("view.missionItems")).value(QStringLiteral("editing")).toObject()).isEmpty(), 10000);
        const QJsonArray fields = editing.value(QStringLiteral("fields")).toArray();
        const QJsonArray everyItem = take(qgc_core_get("view.missionItems(fields)")).value(QStringLiteral("items")).toArray();
        if (kind == QStringLiteral("survey") || kind == QStringLiteral("corridor")) {
            // A pattern's transects are computed after the insert returns, so they are empty on the
            // read that follows it. Same late arrival as the terrain heights and the shot count: a
            // head that reads once draws a boundary with nothing inside it.
            const auto lastShaped = []() {
                const QJsonArray shaped = take(qgc_core_get("view.missionItems(geometry)")).value(QStringLiteral("items")).toArray();
                return shaped.isEmpty() ? QJsonObject() : shaped.last().toObject();
            };
            QTRY_VERIFY_WITH_TIMEOUT(lastShaped().value(QStringLiteral("geometry")).toObject().value(QStringLiteral("transects")).toArray().count() >= 2, 20000);
            const QJsonObject last = lastShaped();
            const QJsonObject geometry = last.value(QStringLiteral("geometry")).toObject();
            QCOMPARE(geometry.value(QStringLiteral("shape")).toString(), kind == QStringLiteral("survey") ? QStringLiteral("area") : QStringLiteral("line"));
            QVERIFY2(geometry.value(QStringLiteral("vertices")).toArray().count() >= 2,
                     qPrintable(QStringLiteral("a %1 draws as its own shape, and asking for geometry gave %2 vertices")
                                    .arg(kind).arg(geometry.value(QStringLiteral("vertices")).toArray().count())));
            if (kind == QStringLiteral("survey")) {
                QVERIFY2(last.value(QStringLiteral("cameraShots")).toInt(0) > 0,
                         "a survey with transects takes pictures, and the count is what an operator sizes a card by");
            }
        }
        QVERIFY2(everyItem.last().toObject().value(QStringLiteral("fields")).toArray().count() == fields.count(),
                 "asking for fields has to answer for every item, not only the one being edited");
        QVERIFY2(!fields.isEmpty(), qPrintable(QStringLiteral("%1 offers nothing to edit, which is what naming no paths looks like").arg(kind)));

        for (const QJsonValue &entry : fields) {
            const QJsonObject field = entry.toObject();
            const QString path = field.value(QStringLiteral("path")).toString();
            checked.append(path);

            const QJsonObject read = take(qgc_bridge_get(path.toUtf8().constData()));
            if (read.contains(QStringLiteral("found"))) {
                unwritable.append(QStringLiteral("%1 names nothing").arg(path));
                continue;
            }
            const QJsonObject written = take(qgc_bridge_set(path.toUtf8().constData(),
                                                            QJsonDocument(QJsonObject { { QStringLiteral("value"), field.value(QStringLiteral("value")) } })
                                                                .toJson(QJsonDocument::Compact)
                                                                .constData()));
            if (!written.value(QStringLiteral("ok")).toBool(false)) {
                unwritable.append(QStringLiteral("%1 refused a write of its own value: %2").arg(path, written.value(QStringLiteral("reason")).toString()));
            }
        }
    }
    restore();

    QVERIFY2(checked.count() > 8, qPrintable(QStringLiteral("only %1 paths were checked, so this proves very little").arg(checked.count())));
    QVERIFY2(unwritable.isEmpty(),
             qPrintable(QStringLiteral("the core named %1 paths a head cannot write, and a write that does not resolve does not happen and says nothing:\n%2")
                            .arg(unwritable.count())
                            .arg(unwritable.join(QStringLiteral("\n")))));
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_theFleetIsNamedAndTheCommandedOneCanBeChosen()
{
#ifdef QGC_RUST_CORE
    const QJsonObject none = take(qgc_core_get("view.vehicles"));
    QCOMPARE(none.value(QStringLiteral("count")).toInt(-1), 0);
    QCOMPARE(none.value(QStringLiteral("ambiguous")).toBool(true), false);

    _connectMockLink(MAV_AUTOPILOT_PX4);
    const auto disconnectWhenDone = qScopeGuard([this]() { _disconnectMockLink(); });
    QTRY_COMPARE_WITH_TIMEOUT(take(qgc_core_get("view.vehicles")).value(QStringLiteral("count")).toInt(-1), 1, 10000);

    const QJsonObject fleet = take(qgc_core_get("view.vehicles"));
    const QJsonObject only = fleet.value(QStringLiteral("vehicles")).toArray().first().toObject();
    const int id = only.value(QStringLiteral("id")).toInt(-1);
    QVERIFY2(id > 0, "the vehicle has no id, so nothing could name it");
    QCOMPARE(fleet.value(QStringLiteral("activeId")).toInt(-1), id);
    QCOMPARE(only.value(QStringLiteral("active")).toBool(), true);
    QVERIFY2(!only.value(QStringLiteral("name")).toString().isEmpty(), "an operator with two aircraft up needs each one named");
    QVERIFY2(!only.value(QStringLiteral("link")).toString().isEmpty(),
             "which link it arrived on is how an operator tells two identical airframes apart, and it read as nothing");
    QCOMPARE(fleet.value(QStringLiteral("ambiguous")).toBool(true), false);

    const QJsonObject chosen = take(qgc_core_invoke("vehicles.setActive", QStringLiteral("[%1]").arg(id).toUtf8().constData()));
    QVERIFY2(chosen.value(QStringLiteral("ok")).toBool(false),
             qPrintable(QStringLiteral("choosing the one connected vehicle failed: %1").arg(chosen.value(QStringLiteral("reason")).toString())));
    // The answer names what was asked for, not what is true yet: setActiveVehicle defers the
    // change through a 20ms singleShot, so the core cannot confirm it without waiting and does not
    // pretend to. A head learns it happened from view.vehicles, which is watched.
    QCOMPARE(chosen.value(QStringLiteral("activating")).toInt(-1), id);
    QVERIFY2(chosen.value(QStringLiteral("active")).isUndefined(), "the answer must not claim the vehicle is already the active one");
    QTRY_COMPARE_WITH_TIMEOUT(take(qgc_core_get("view.vehicles")).value(QStringLiteral("activeId")).toInt(-1), id, 5000);

    const QJsonObject absent = take(qgc_core_invoke("vehicles.setActive", "[99]"));
    QCOMPARE(absent.value(QStringLiteral("ok")).toBool(true), false);
    QCOMPARE(take(qgc_core_get("view.vehicles")).value(QStringLiteral("activeId")).toInt(-1), id);
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}

void QGCCoreCTest::_theTerrainProfileIsSampledThroughASurveyRatherThanAtItsCorner()
{
#ifdef QGC_RUST_CORE
    (void) take(qgc_bridge_invoke("plan.start", "[]"));
    const auto restore = []() { (void) take(qgc_bridge_invoke("plan.removeAll", "[]")); };
    const auto leaveNoPlanBehind = qScopeGuard(restore);
    restore();

    const auto profilePoints = []() {
        return take(qgc_core_get("view.terrainProfile")).value(QStringLiteral("points")).toArray().count();
    };
    QVERIFY2(take(qgc_core_invoke("mission.insert", "[\"takeoff\", 47.3960, 8.5440, -1]")).value(QStringLiteral("ok")).toBool(false), "the takeoff was refused");
    QVERIFY2(take(qgc_core_invoke("mission.insert", "[\"waypoint\", 47.3990, 8.5480, -1]")).value(QStringLiteral("ok")).toBool(false), "the waypoint was refused");
    QTRY_VERIFY_WITH_TIMEOUT(profilePoints() >= 2, 10000);
    const int withoutSurvey = profilePoints();

    QVERIFY2(take(qgc_core_invoke("mission.insert", "[\"survey\", 47.3975, 8.5460, -1]")).value(QStringLiteral("ok")).toBool(false), "the survey was refused");

    // A survey covering ground between its entry and its exit has to appear on the profile as the
    // path it flies, not as the corner it starts at, or the operator reads level ground under it.
    QTRY_VERIFY_WITH_TIMEOUT(profilePoints() > withoutSurvey + 1, 20000);

    const QJsonArray points = take(qgc_core_get("view.terrainProfile")).value(QStringLiteral("points")).toArray();
    QVERIFY2(points.count() > withoutSurvey + 1,
             qPrintable(QStringLiteral("the plan had %1 profile points and gained %2 by adding a survey, which is one point for the whole pattern")
                            .arg(withoutSurvey).arg(points.count() - withoutSurvey)));

    QList<double> distances;
    for (const QJsonValue &point : points) {
        distances.append(point.toObject().value(QStringLiteral("distance")).toDouble());
    }

    // The core sorts the profile before serving it, so asserting the result is sorted asserts
    // nothing. What the sampling can actually get wrong is stacking every sample of a pattern on
    // one x, which is what reading a sample spacing as a segment length did - the points were all
    // present, in order, and on top of each other.
    QHash<double, int> atDistance;
    for (const double distance : distances) {
        atDistance[distance]++;
    }
    // Consecutive segments share an endpoint, so two points on one x is the boundary between them.
    // More than two is samples piling up somewhere they were never flown.
    int worst = 0;
    double crowded = 0.0;
    for (auto entry = atDistance.cbegin(); entry != atDistance.cend(); ++entry) {
        if (entry.value() > worst) {
            worst = entry.value();
            crowded = entry.key();
        }
    }
    QVERIFY2(worst <= 2,
             qPrintable(QStringLiteral("%1 of the %2 profile points sit at %3 m, so a stretch of the pattern is stacked on one distance rather than walked")
                            .arg(worst).arg(distances.count()).arg(crowded)));

    // The x-axis has to span the mission the summary is describing. A profile that samples inside
    // the pattern but stops the axis at the pattern's entry squashes the whole chart into the
    // direct legs, and every altitude on it is drawn against the wrong ground.
    const double axis = take(qgc_core_get("view.terrainProfile")).value(QStringLiteral("totalDistanceMeters")).toDouble();
    const double flown = take(qgc_core_get("view.missionSummary")).value(QStringLiteral("distanceMetres")).toDouble();
    QVERIFY2(flown > 0.0, "the summary reported no distance for a plan with a survey in it");
    QVERIFY2(axis > flown * 0.9,
             qPrintable(QStringLiteral("the profile axis spans %1 m for a mission the summary says is %2 m").arg(axis).arg(flown)));
#else
    QSKIP("the Rust core is not linked into this build");
#endif
}
