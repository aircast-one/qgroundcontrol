#include "FlightMapTest.h"

#include "QuickInteractionTestHelpers.h"

#include <QtGui/QNativeGestureEvent>
#include <QtGui/QWheelEvent>
#include <QtPositioning/QGeoCoordinate>

static const QPointF kCenter(300, 200);
static const QPointingDevice kTrackpad(QStringLiteral("trackpad"), 1, QInputDevice::DeviceType::TouchPad, QPointingDevice::PointerType::Finger, QInputDevice::Capability::Scroll, 1, 3);

static QQuickItem* loadMap(QQuickView& view)
{
    if (!loadTestView(view, QStringLiteral("qrc:/unittest/FlightMapWheelTest.qml"))) {
        return nullptr;
    }
    QQuickItem* const map = view.rootObject();
    return QTest::qWaitFor([map] { return map->property("mapReady").toBool(); }) ? map : nullptr;
}

static void sendWheel(QQuickView& view, const QPointF& pos, const QPoint& pixelDelta, const QPoint& angleDelta, Qt::ScrollPhase phase, const QPointingDevice* device = &kTrackpad, Qt::KeyboardModifiers modifiers = Qt::NoModifier, Qt::MouseButtons buttons = Qt::NoButton)
{
    QWheelEvent event(pos, view.mapToGlobal(pos.toPoint()), pixelDelta, angleDelta, buttons, modifiers, phase, false, Qt::MouseEventNotSynthesized, device);
    QCoreApplication::sendEvent(&view, &event);
}

static void sendPinch(QQuickView& view, const QPointF& pos, Qt::NativeGestureType type, qreal value)
{
    QNativeGestureEvent event(type, &kTrackpad, 2, pos, pos, view.mapToGlobal(pos.toPoint()), value, QPointF());
    QCoreApplication::sendEvent(&view, &event);
}

static QGeoCoordinate coordinateAt(QQuickItem* map, const QPointF& pos)
{
    QGeoCoordinate coordinate;
    QMetaObject::invokeMethod(map, "toCoordinate", Q_RETURN_ARG(QGeoCoordinate, coordinate), Q_ARG(QPointF, pos), Q_ARG(bool, false));
    return coordinate;
}

static QPointF pointOf(QQuickItem* map, const QGeoCoordinate& coordinate)
{
    QPointF point;
    QMetaObject::invokeMethod(map, "fromCoordinate", Q_RETURN_ARG(QPointF, point), Q_ARG(QGeoCoordinate, coordinate), Q_ARG(bool, false));
    return point;
}

static bool near(const QPointF& a, const QPointF& b)
{
    return (a - b).manhattanLength() < 2;
}

void FlightMapTest::_trackpadOrMagicMouseScrollPans()
{
    QQuickView view;
    QQuickItem* const map = loadMap(view);
    QVERIFY(map);
    const QGeoCoordinate before = coordinateAt(map, kCenter);
    const qreal zoom = map->property("zoomLevel").toReal();

    sendWheel(view, kCenter, QPoint(0, -80), QPoint(0, -80), Qt::ScrollBegin);
    sendWheel(view, kCenter, QPoint(), QPoint(), Qt::ScrollEnd);

    QVERIFY(near(pointOf(map, before), kCenter - QPointF(0, 80)));
    QCOMPARE(map->property("zoomLevel").toReal(), zoom);
    QCOMPARE(map->property("panStarts").toInt(), 1);
    QCOMPARE(map->property("panStops").toInt(), 1);
}

void FlightMapTest::_momentumKeepsPanningAfterTheStopSignal()
{
    QQuickView view;
    QQuickItem* const map = loadMap(view);
    QVERIFY(map);
    const QGeoCoordinate before = coordinateAt(map, kCenter);

    sendWheel(view, kCenter, QPoint(0, -80), QPoint(0, -80), Qt::ScrollBegin);
    sendWheel(view, kCenter, QPoint(), QPoint(), Qt::ScrollEnd);
    sendWheel(view, kCenter, QPoint(0, -40), QPoint(0, -40), Qt::ScrollMomentum);
    sendWheel(view, kCenter, QPoint(), QPoint(), Qt::ScrollEnd);

    QVERIFY(near(pointOf(map, before), kCenter - QPointF(0, 120)));
    QCOMPARE(map->property("panStarts").toInt(), 1);
    QCOMPARE(map->property("panStops").toInt(), 1);
}

void FlightMapTest::_scrollWhileAButtonIsHeldIsIgnored()
{
    QQuickView view;
    QQuickItem* const map = loadMap(view);
    QVERIFY(map);
    const QGeoCoordinate before = coordinateAt(map, kCenter);

    sendWheel(view, kCenter, QPoint(0, -80), QPoint(0, -80), Qt::ScrollBegin, &kTrackpad, Qt::NoModifier, Qt::LeftButton);

    QVERIFY(near(pointOf(map, before), kCenter));
    QCOMPARE(map->property("panStarts").toInt(), 0);
}

void FlightMapTest::_touchpadWithoutPixelDeltaStillPans()
{
    QQuickView view;
    QQuickItem* const map = loadMap(view);
    QVERIFY(map);
    const QGeoCoordinate before = coordinateAt(map, kCenter);

    sendWheel(view, kCenter, QPoint(), QPoint(0, -160), Qt::NoScrollPhase);

    QVERIFY(near(pointOf(map, before), kCenter - QPointF(0, 80)));
}

void FlightMapTest::_mouseWheelZoomsAboutTheCursor()
{
    QQuickView view;
    QQuickItem* const map = loadMap(view);
    QVERIFY(map);
    const QPointF cursor(100, 50);
    const QGeoCoordinate underCursor = coordinateAt(map, cursor);
    const qreal zoom = map->property("zoomLevel").toReal();

    sendWheel(view, cursor, QPoint(), QPoint(0, 120), Qt::NoScrollPhase, QPointingDevice::primaryPointingDevice());

    QCOMPARE(map->property("zoomLevel").toReal(), zoom + 1);
    QVERIFY(near(pointOf(map, underCursor), cursor));
}

void FlightMapTest::_modifierScrollZooms()
{
    QQuickView view;
    QQuickItem* const map = loadMap(view);
    QVERIFY(map);
    const QGeoCoordinate before = coordinateAt(map, kCenter);
    const qreal zoom = map->property("zoomLevel").toReal();

    sendWheel(view, kCenter, QPoint(0, 120), QPoint(0, 120), Qt::NoScrollPhase, &kTrackpad, Qt::MetaModifier);

    QCOMPARE(map->property("zoomLevel").toReal(), zoom + 1);
    QVERIFY(near(pointOf(map, before), kCenter));
    QCOMPARE(map->property("panStarts").toInt(), 0);
}

void FlightMapTest::_pinchFollowsTheFingersWhilePanning()
{
    QQuickView view;
    QQuickItem* const map = loadMap(view);
    QVERIFY(map);
    const QPointF fingers(120, 90);
    const QPointF panDelta(0, 80);
    const QGeoCoordinate underFingers = coordinateAt(map, fingers);
    const qreal zoom = map->property("zoomLevel").toReal();

    sendPinch(view, fingers, Qt::BeginNativeGesture, 0);
    sendPinch(view, fingers, Qt::ZoomNativeGesture, 1.0);
    QCOMPARE(map->property("zoomLevel").toReal(), zoom + 1);
    QVERIFY(near(pointOf(map, underFingers), fingers));

    sendWheel(view, fingers, -panDelta.toPoint(), -panDelta.toPoint(), Qt::ScrollUpdate);
    sendPinch(view, fingers, Qt::ZoomNativeGesture, 1.0);
    sendPinch(view, fingers, Qt::EndNativeGesture, 0);
    QCOMPARE(map->property("zoomLevel").toReal(), zoom + 2);
    QVERIFY(near(pointOf(map, underFingers), fingers - panDelta * 2));
}

void FlightMapTest::_rightClickSignalsApartFromLeftClick()
{
    QQuickView view;
    QQuickItem* const map = loadMap(view);
    QVERIFY(map);

    QTest::mouseClick(&view, Qt::RightButton, Qt::NoModifier, kCenter.toPoint());
    QCOMPARE(map->property("rightClicks").toInt(), 1);
    QCOMPARE(map->property("leftClicks").toInt(), 0);
    QCOMPARE(map->property("lastClick").toPointF(), kCenter);

    const QPointF elsewhere(100, 50);
    QTest::mouseClick(&view, Qt::LeftButton, Qt::NoModifier, elsewhere.toPoint());
    QCOMPARE(map->property("leftClicks").toInt(), 1);
    QCOMPARE(map->property("rightClicks").toInt(), 1);
    QCOMPARE(map->property("lastClick").toPointF(), elsewhere);
}

static void flickMouse(QQuickView& view, const QPointF& from, const QPointF& to, int moveDelayMs)
{
    QTest::mousePress(&view, Qt::LeftButton, Qt::NoModifier, from.toPoint());
    QTest::mouseMove(&view, ((from + to) / 2).toPoint(), moveDelayMs);
    QTest::mouseMove(&view, to.toPoint(), moveDelayMs);
    QTest::mouseRelease(&view, Qt::LeftButton, Qt::NoModifier, to.toPoint());
}

void FlightMapTest::_mouseDragFlicksWithInertia()
{
    QQuickView view;
    QQuickItem* const map = loadMap(view);
    QVERIFY(map);
    const QGeoCoordinate before = coordinateAt(map, kCenter);

    flickMouse(view, kCenter, kCenter - QPointF(0, 60), 20);
    const QPointF atRelease = pointOf(map, before);
    QVERIFY(near(atRelease, kCenter - QPointF(0, 60)));
    QVERIFY(map->property("flicking").toBool());
    QCOMPARE(map->property("panStops").toInt(), 1);

    QTRY_VERIFY(!map->property("flicking").toBool());
    QVERIFY(pointOf(map, before).y() < atRelease.y() - 50);
}
