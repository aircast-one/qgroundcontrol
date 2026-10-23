#include "RouteRecorderTest.h"

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
    const QList<QGeoCoordinate> saved = {kGround.atDistanceAndAzimuth(0, 0, 60.0),
                                         kGround.atDistanceAndAzimuth(150, 45, 70.0),
                                         kGround.atDistanceAndAzimuth(300, 90, 80.0)};

    QVERIFY(RouteRecorder::saveKml(fileName, QStringLiteral("Route"), saved));

    KMLHelper::Route loaded;
    QString errorString;
    QVERIFY2(KMLHelper::loadRouteFromFile(fileName, loaded, errorString), qPrintable(errorString));
    QCOMPARE(loaded.altitude, KMLHelper::Route::Altitude::Absolute);
    QCOMPARE(loaded.path.count(), saved.count());
    for (qsizetype i = 0; i < saved.count(); i++) {
        QVERIFY(loaded.path[i].distanceTo(saved[i]) < 0.01);
        QCOMPARE(loaded.path[i].altitude(), saved[i].altitude());
    }
}

UT_REGISTER_TEST_LIGHTWEIGHT(RouteRecorderTest, TestLabel::Unit, TestLabel::Vehicle)
