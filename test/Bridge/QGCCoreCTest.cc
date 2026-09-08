#include "QGCCoreCTest.h"

#include "MockLink.h"
#include "QGCBridgeC.h"

#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
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
    QCOMPARE(offline.value(QStringLiteral("showing")).toBool(true), false);
    QVERIFY(offline.value(QStringLiteral("armingBlocker")).isNull());

    _connectMockLink(MAV_AUTOPILOT_PX4);
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get("view.guidedActions")).value(QStringLiteral("connected")).toBool(false), 5000);
    const QJsonObject online = take(qgc_bridge_get("view.warnings"));
    QVERIFY(online.value(QStringLiteral("warnings")).isArray());
    QCOMPARE(online.value(QStringLiteral("showing")).toBool(), !online.value(QStringLiteral("warnings")).toArray().isEmpty());
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
    QVERIFY(first.value(QStringLiteral("path")).toString().startsWith(QStringLiteral("mavlinkInspector.systems.0.messages.")));
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

void QGCCoreCTest::_videoAndCameraAreServed()
{
    const QJsonObject video = take(qgc_bridge_get("view.video"));
    QCOMPARE(video.value(QStringLiteral("class")).toString(), QStringLiteral("Video"));
    QVERIFY(!video.value(QStringLiteral("summary")).toString().isEmpty());
    QVERIFY(video.value(QStringLiteral("cameras")).isArray());
    const QJsonObject camera = take(qgc_bridge_get("view.camera"));
    QCOMPARE(camera.value(QStringLiteral("present")).toBool(true), false);
    QCOMPARE(camera.value(QStringLiteral("shotsText")).toString(), QStringLiteral("00000"));
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

const char *const kViewPaths[] = {
    "view.messages", "view.plan", "view.guidedActions", "view.guidedAltitude", "view.guidedAltitude(30)",
    "view.guidedTakeoff", "view.guidedTakeoff(10)", "view.guidedSpeed", "view.guidedSpeed(3)", "view.battery",
    "view.preflight", "view.warnings", "view.label(altitudeRelative)", "view.instruments", "view.vibration",
    "view.sensors", "view.control(settings.appSettings.audioMuted)", "view.links", "view.linkForm(udp,,14550)",
    "view.mapScale(120)", "view.terrainProfile", "view.missionKinds", "view.missionSeed(survey,47,8)",
    "view.calibration", "view.radio", "view.logs", "view.inspector", "view.flightModes", "view.settings",
    "view.settings(General)", "view.surveyStats(0)", "view.fences", "view.polygon", "view.setup",
    "view.setup(Safety)", "view.video", "view.camera",
};

} // namespace

void QGCCoreCTest::_viewShapesMatchTheRecordedContract()
{
    _connectMockLink(MAV_AUTOPILOT_PX4);
    QTRY_VERIFY_WITH_TIMEOUT(take(qgc_bridge_get("view.guidedActions")).value(QStringLiteral("connected")).toBool(false), 5000);

    QJsonObject recorded;
    for (const char *path : kViewPaths) {
        recorded.insert(QString::fromUtf8(path), shapeOf(take(qgc_bridge_get(path))));
    }
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
    for (const char *path : kViewPaths) {
        const QString key = QString::fromUtf8(path);
        const QByteArray was = QJsonDocument(expected.value(key).toObject()).toJson(QJsonDocument::Compact);
        const QByteArray now = QJsonDocument(recorded.value(key).toObject()).toJson(QJsonDocument::Compact);
        QVERIFY2(was == now, qPrintable(QStringLiteral("%1 changed shape\n was: %2\n now: %3").arg(key, QString::fromUtf8(was), QString::fromUtf8(now))));
    }
}

void QGCCoreCTest::_tlogSummaryDecodesTheSampleLog()
{
    const QString sample = QFileInfo(QString::fromUtf8(__FILE__)).dir().filePath(QStringLiteral("../../mav.tlog"));
    const QJsonObject summary = take(qgc_bridge_get(QStringLiteral("view.tlog(%1)").arg(sample).toUtf8().constData()));
    QCOMPARE(summary.value(QStringLiteral("readable")).toBool(false), true);
    QVERIFY(summary.value(QStringLiteral("frames")).toInt() > 1000);
    QVERIFY(summary.value(QStringLiteral("byName")).toObject().contains(QStringLiteral("HEARTBEAT")));
    QCOMPARE(take(qgc_bridge_get("view.tlog(/nonexistent.tlog)")).value(QStringLiteral("readable")).toBool(true), false);
    QCOMPARE(take(qgc_bridge_get("view.tlog")).value(QStringLiteral("kind")).toString(), QStringLiteral("null"));
}
