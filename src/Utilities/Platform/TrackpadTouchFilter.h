#pragma once

#include <QtCore/QObject>

class TrackpadTouchFilter : public QObject
{
    Q_OBJECT

public:
    using QObject::QObject;

    bool eventFilter(QObject* watched, QEvent* event) override;
};
