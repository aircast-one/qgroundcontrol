#include "TrackpadTouchFilter.h"

#include <QtGui/QInputDevice>
#include <QtGui/QTouchEvent>

bool TrackpadTouchFilter::eventFilter(QObject* watched, QEvent* event)
{
    switch (event->type()) {
        case QEvent::TouchBegin:
        case QEvent::TouchUpdate:
        case QEvent::TouchEnd:
        case QEvent::TouchCancel:
            return static_cast<QTouchEvent*>(event)->device()->type() == QInputDevice::DeviceType::TouchPad;
        default:
            return QObject::eventFilter(watched, event);
    }
}
