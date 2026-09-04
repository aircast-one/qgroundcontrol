#include "FlightMapTest.h"

#include "QuickInteractionTestHelpers.h"

#include <QtGui/QWheelEvent>
#include <QtGui/QNativeGestureEvent>
#include <QtPositioning/QGeoCoordinate>

static void sendWheel(QQuickView& view, const QPointF& pos, const QPoint& delta, Qt::ScrollPhase phase, const QPointingDevice* device, Qt::KeyboardModifiers modifiers = Qt::NoModifier)
{
    QWheelEvent event(pos, view.mapToGlobal(pos.toPoint()), delta, delta, Qt::NoButton, modifiers, phase, false, Qt::MouseEventNotSynthesized, device);
    QCoreApplication::sendEvent(&view, &event);
}

static void sendPinch(QQuickView& view, const QPointF& pos, Qt::NativeGestureType type, qreal value, const QPointingDevice* device)
{
    QNativeGestureEvent event(type, device, 2, pos, pos, view.mapToGlobal(pos.toPoint()), value, QPointF());
    QCoreApplication::sendEvent(&view, &event);
}

static QGeoCoordinate coordinateAt(QQuickItem* map, const QPointF& pos)
{
    QGeoCoordinate coordinate;
    QMetaObject::invokeMethod(map, "toCoordinate", Q_RETURN_ARG(QGeoCoordinate, coordinate), Q_ARG(QPointF, pos), Q_ARG(bool, false));
    return coordinate;
}

void FlightMapTest::_trackpadScrollPansAndMouseWheelZooms()
{
    QQuickView view;
    QVERIFY(loadTestView(view, QStringLiteral("qrc:/unittest/FlightMapWheelTest.qml")));
    QQuickItem* const map = view.rootObject();
    QTRY_VERIFY(map->property("mapReady").toBool());

    const QPointF center(300, 200);
    const QGeoCoordinate before = map->property("center").value<QGeoCoordinate>();
    const qreal zoom = map->property("zoomLevel").toReal();

    const QPointingDevice trackpad(QStringLiteral("trackpad"), 1, QInputDevice::DeviceType::TouchPad, QPointingDevice::PointerType::Finger, QInputDevice::Capability::Scroll, 1, 3);
    sendWheel(view, center, QPoint(0, -80), Qt::ScrollBegin, &trackpad);
    sendWheel(view, center, QPoint(0, 0), Qt::ScrollEnd, &trackpad);
    sendWheel(view, center, QPoint(0, -40), Qt::ScrollMomentum, &trackpad);
    sendWheel(view, center, QPoint(0, 0), Qt::ScrollEnd, &trackpad);

    const QGeoCoordinate panned = map->property("center").value<QGeoCoordinate>();
    QVERIFY(panned.latitude() < before.latitude());
    QCOMPARE(panned.longitude(), before.longitude());
    QCOMPARE(map->property("zoomLevel").toReal(), zoom);
    QPointF beforeNow;
    QMetaObject::invokeMethod(map, "fromCoordinate", Q_RETURN_ARG(QPointF, beforeNow), Q_ARG(QGeoCoordinate, before), Q_ARG(bool, false));
    QVERIFY((beforeNow - QPointF(300, 120)).manhattanLength() < 2);
    QCOMPARE(map->property("panStarts").toInt(), 1);
    QCOMPARE(map->property("panStops").toInt(), 2);

    sendWheel(view, center, QPoint(0, 120), Qt::NoScrollPhase, QPointingDevice::primaryPointingDevice());
    QCOMPARE(map->property("zoomLevel").toReal(), zoom + 1);
    QVERIFY(map->property("center").value<QGeoCoordinate>().distanceTo(panned) < 1);

    sendWheel(view, center, QPoint(0, 120), Qt::NoScrollPhase, &trackpad, Qt::MetaModifier);
    QCOMPARE(map->property("zoomLevel").toReal(), zoom + 2);

    const QPointF fingers(120, 90);
    const QGeoCoordinate underFingers = coordinateAt(map, fingers);
    sendPinch(view, fingers, Qt::BeginNativeGesture, 0, &trackpad);
    sendPinch(view, fingers, Qt::ZoomNativeGesture, 1.0, &trackpad);
    QCOMPARE(map->property("zoomLevel").toReal(), zoom + 3);
    QVERIFY(coordinateAt(map, fingers).distanceTo(underFingers) < 1);

    sendWheel(view, fingers, QPoint(0, -80), Qt::ScrollUpdate, &trackpad);
    sendPinch(view, fingers, Qt::ZoomNativeGesture, 1.0, &trackpad);
    sendPinch(view, fingers, Qt::EndNativeGesture, 0, &trackpad);
    QCOMPARE(map->property("zoomLevel").toReal(), zoom + 4);
    QPointF moved;
    QMetaObject::invokeMethod(map, "fromCoordinate", Q_RETURN_ARG(QPointF, moved), Q_ARG(QGeoCoordinate, underFingers), Q_ARG(bool, false));
    QVERIFY((moved - QPointF(120, 90 - 160)).manhattanLength() < 2);
}
