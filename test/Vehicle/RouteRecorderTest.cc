#include "RouteRecorderTest.h"

#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QTemporaryDir>
#include <QtTest/QTest>

#include "KMLHelper.h"
#include "RouteRecorder.h"

namespace {

const QGeoCoordinate kGround(41.7100, 44.7900, 600.0);

QList<QGeoCoordinate> vertical(const QGeoCoordinate& at, double fromAltitude, double toAltitude)
{
    QList<QGeoCoordinate> points;
    const double step = (toAltitude > fromAltitude) ? 3.0 : -3.0;
    for (double altitude = fromAltitude; (step > 0) ? (altitude <= toAltitude) : (altitude >= toAltitude);
         altitude += step) {
        points.append(QGeoCoordinate(at.latitude(), at.longitude(), altitude));
    }
    return points;
}

}  // namespace

void RouteRecorderTest::_climbAndDescentAtTheEndsAreTrimmed()
{
    const QGeoCoordinate top(kGround.latitude(), kGround.longitude(), 660.0);
    QList<QGeoCoordinate> path = vertical(kGround, 600.0, 660.0);
    for (int i = 1; i <= 50; i++) {
        QGeoCoordinate point = top.atDistanceAndAzimuth(i * 2.0, 0);
        point.setAltitude(660.0);
        path.append(point);
    }
    path.append(vertical(path.last(), 660.0, 600.0));

    const QList<QGeoCoordinate> route = RouteRecorder::routeFromPath(path);

    QCOMPARE(route.count(), 2);
    QCOMPARE(route.first().altitude(), 660.0);
    QCOMPARE(route.last().altitude(), 660.0);
    QVERIFY(route.first().distanceTo(route.last()) > 75.0);
}

void RouteRecorderTest::_hoveringRecordsNoRoute()
{
    QList<QGeoCoordinate> path = vertical(kGround, 600.0, 660.0);
    path.append(kGround.atDistanceAndAzimuth(4.0, 45, 60.0));

    QVERIFY(RouteRecorder::routeFromPath(path).isEmpty());
}

void RouteRecorderTest::_savedRouteLoadsBackWithAbsoluteAltitudes()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString fileName = dir.filePath(QStringLiteral("route.kml"));
    const QGeoCoordinate tower = kGround.atDistanceAndAzimuth(200, 60, 5.0);
    const QList<RouteRecorder::Sample> saved = {
        {kGround.atDistanceAndAzimuth(0, 0, 60.0), -30.0, 10.0, QGeoCoordinate()},
        {kGround.atDistanceAndAzimuth(150, 45, 70.0), -45.0, 0.0, tower},
        {kGround.atDistanceAndAzimuth(300, 90, 80.0), qQNaN(), qQNaN(), QGeoCoordinate()},
    };

    QVERIFY(RouteRecorder::saveKml(fileName, QStringLiteral("Route"), saved));

    KMLHelper::Route loaded;
    QString errorString;
    QVERIFY2(KMLHelper::loadRouteFromFile(fileName, loaded, errorString), qPrintable(errorString));
    QCOMPARE(loaded.altitude, KMLHelper::Route::Altitude::Absolute);
    QCOMPARE(loaded.path.count(), saved.count());
    for (qsizetype i = 0; i < saved.count(); i++) {
        QVERIFY(loaded.path[i].distanceTo(saved[i].coordinate) < 0.01);
        QCOMPARE(loaded.path[i].altitude(), saved[i].coordinate.altitude());
    }
    QCOMPARE(loaded.gimbalPitch.count(), 3);
    QCOMPARE(loaded.gimbalPitch[0], -30.0);
    QCOMPARE(loaded.gimbalYaw[0], 10.0);
    QVERIFY(qIsNaN(loaded.gimbalPitch[2]));
    QCOMPARE(loaded.lookAt.count(), 3);
    QVERIFY(!loaded.lookAt[0].isValid());
    QVERIFY(loaded.lookAt[1].distanceTo(tower) < 0.05);
    QCOMPARE(loaded.lookAt[1].altitude(), tower.altitude());
}

void RouteRecorderTest::_renameKeepsTheExtensionAndRefusesToOverwrite()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString original = dir.filePath(QStringLiteral("Route 1.kml"));
    const QString other = dir.filePath(QStringLiteral("Tower line.kml"));
    for (const QString& path : {original, other}) {
        QFile file(path);
        QVERIFY(file.open(QIODevice::WriteOnly));
    }

    RouteRecorder recorder;
    QCOMPARE(recorder.renameFile(original, QStringLiteral("Tower line")), original);
    const QString renamed = recorder.renameFile(original, QStringLiteral(" Tower 7/North "));
    QCOMPARE(renamed, dir.filePath(QStringLiteral("Tower 7-North.kml")));
    QVERIFY(QFileInfo::exists(renamed));
    QVERIFY(!QFileInfo::exists(original));
}

UT_REGISTER_TEST_LIGHTWEIGHT(RouteRecorderTest, TestLabel::Unit, TestLabel::Vehicle)
