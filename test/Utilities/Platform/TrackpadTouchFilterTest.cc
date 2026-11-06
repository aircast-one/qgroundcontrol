#include "TrackpadTouchFilterTest.h"

#include <QtGui/QPointingDevice>
#include <QtGui/QTouchEvent>
#include <QtGui/QWheelEvent>

#include "TrackpadTouchFilter.h"

static QPointingDevice* device(QInputDevice::DeviceType type)
{
    return new QPointingDevice(QStringLiteral("test"), 1, type, QPointingDevice::PointerType::Finger,
                               QInputDevice::Capability::Position, 10, 0);
}

void TrackpadTouchFilterTest::_testDropsTrackpadTouches()
{
    TrackpadTouchFilter filter;
    const QScopedPointer<QPointingDevice> trackpad(device(QInputDevice::DeviceType::TouchPad));
    const QList<QEvent::Type> types{QEvent::TouchBegin, QEvent::TouchUpdate, QEvent::TouchEnd, QEvent::TouchCancel};
    for (const QEvent::Type type : types) {
        QTouchEvent event(type, trackpad.data());
        QVERIFY(filter.eventFilter(nullptr, &event));
    }
}

void TrackpadTouchFilterTest::_testKeepsTouchscreenTouches()
{
    TrackpadTouchFilter filter;
    const QScopedPointer<QPointingDevice> screen(device(QInputDevice::DeviceType::TouchScreen));
    QTouchEvent event(QEvent::TouchBegin, screen.data());
    QVERIFY(!filter.eventFilter(nullptr, &event));
}

void TrackpadTouchFilterTest::_testKeepsTrackpadScroll()
{
    TrackpadTouchFilter filter;
    const QScopedPointer<QPointingDevice> trackpad(device(QInputDevice::DeviceType::TouchPad));
    QWheelEvent event(QPointF(), QPointF(), QPoint(0, 4), QPoint(0, 120), Qt::NoButton, Qt::NoModifier,
                      Qt::ScrollUpdate, false, Qt::MouseEventNotSynthesized, trackpad.data());
    QVERIFY(!filter.eventFilter(nullptr, &event));
}

UT_REGISTER_TEST(TrackpadTouchFilterTest, TestLabel::Unit, TestLabel::Utilities)
