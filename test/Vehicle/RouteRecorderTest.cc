#include "RouteRecorderTest.h"

#include <QtCore/QTemporaryDir>
#include <QtTest/QTest>

#include "KMLHelper.h"
#include "RouteRecorder.h"

namespace {

const QGeoCoordinate kStart(41.7100, 44.7900, 600.0);

QList<QGeoCoordinate> leg(const QGeoCoordinate& from, double azimuth, int steps, double climbPerStep)
{
    QList<QGeoCoordinate> points;
    for (int i = 1; i <= steps; i++) {
        QGeoCoordinate point = from.atDistanceAndAzimuth(i * 5.0, azimuth);
        point.setAltitude(from.altitude() + (i * climbPerStep));
        points.append(point);
    }
    return points;
}

}  // namespace

void RouteRecorderTest::_straightLineKeepsOnlyTheEnds()
{
    const QList<QGeoCoordinate> path = QList<QGeoCoordinate>{kStart} + leg(kStart, 90, 40, 0);

    const QList<QGeoCoordinate> route = RouteRecorder::simplify(path, RouteRecorder::kSimplifyToleranceMeters);

    QCOMPARE(route.count(), 2);
    QCOMPARE(route.first(), path.first());
    QCOMPARE(route.last(), path.last());
}

void RouteRecorderTest::_cornerAndClimbAreKept()
{
    const QList<QGeoCoordinate> east = leg(kStart, 90, 20, 0);
    const QList<QGeoCoordinate> north = leg(east.last(), 0, 20, 0);
    const QList<QGeoCoordinate> climb = leg(north.last(), 0, 20, 2.0);
    const QList<QGeoCoordinate> level = leg(climb.last(), 0, 20, 0);
    const QList<QGeoCoordinate> path = QList<QGeoCoordinate>{kStart} + east + north + climb + level;

    const QList<QGeoCoordinate> route = RouteRecorder::simplify(path, RouteRecorder::kSimplifyToleranceMeters);

    QCOMPARE(route.count(), 5);
    QCOMPARE(route[1], east.last());
    QCOMPARE(route[2], north.last());
    QCOMPARE(route[3], climb.last());
}

void RouteRecorderTest::_savedRouteLoadsBackWithAbsoluteAltitudes()
{
    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString fileName = dir.filePath(QStringLiteral("route.kml"));
    const QList<QGeoCoordinate> saved = QList<QGeoCoordinate>{kStart} + leg(kStart, 45, 3, 10.0);

    QVERIFY(RouteRecorder::saveKml(fileName, QStringLiteral("Route"), saved));

    QList<QGeoCoordinate> loaded;
    bool absoluteAltitude = false;
    QString errorString;
    QVERIFY2(KMLHelper::loadRouteFromFile(fileName, loaded, absoluteAltitude, errorString), qPrintable(errorString));
    QVERIFY(absoluteAltitude);
    QCOMPARE(loaded.count(), saved.count());
    for (qsizetype i = 0; i < saved.count(); i++) {
        QVERIFY(loaded[i].distanceTo(saved[i]) < 0.01);
        QCOMPARE(loaded[i].altitude(), saved[i].altitude());
    }
}

UT_REGISTER_TEST_LIGHTWEIGHT(RouteRecorderTest, TestLabel::Unit, TestLabel::Vehicle)
