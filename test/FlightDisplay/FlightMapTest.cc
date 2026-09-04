#include "FlightMapTest.h"

#include "QuickInteractionTestHelpers.h"

#include <QtGui/QWheelEvent>
#include <QtPositioning/QGeoCoordinate>

static void sendWheel(QQuickView& view, const QPointF& pos, const QPoint& delta, Qt::ScrollPhase phase, const QPointingDevice* device, Qt::KeyboardModifiers modifiers = Qt::NoModifier)
{
    QWheelEvent event(pos, view.mapToGlobal(pos.toPoint()), delta, delta, Qt::NoButton, modifiers, phase, false, Qt::MouseEventNotSynthesized, device);
    QCoreApplication::sendEvent(&view, &event);
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

    const QGeoCoordinate panned = map->property("center").value<QGeoCoordinate>();
    QVERIFY(panned.latitude() < before.latitude());
    QCOMPARE(panned.longitude(), before.longitude());
    QCOMPARE(map->property("zoomLevel").toReal(), zoom);
    QCOMPARE(map->property("panStarts").toInt(), 1);
    QCOMPARE(map->property("panStops").toInt(), 1);

    sendWheel(view, center, QPoint(0, 120), Qt::NoScrollPhase, QPointingDevice::primaryPointingDevice());
    QCOMPARE(map->property("zoomLevel").toReal(), zoom + 1);
    QVERIFY(map->property("center").value<QGeoCoordinate>().distanceTo(panned) < 1);

    sendWheel(view, center, QPoint(0, 120), Qt::NoScrollPhase, &trackpad, Qt::MetaModifier);
    QCOMPARE(map->property("zoomLevel").toReal(), zoom + 2);
}
