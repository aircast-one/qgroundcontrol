/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#pragma once

#include <QtCore/QObject>
#include <QtCore/QTimer>

class GCSBattery : public QObject
{
    Q_OBJECT

    Q_PROPERTY(int  level     READ level     NOTIFY stateChanged)
    Q_PROPERTY(bool charging  READ charging  NOTIFY stateChanged)
    Q_PROPERTY(bool available READ available NOTIFY stateChanged)

public:
    explicit GCSBattery(QObject *parent = nullptr);

    int  level() const { return _level; }
    bool charging() const { return _charging; }
    bool available() const { return _level >= 0; }

signals:
    void stateChanged();

private:
    void _poll();

    QTimer _timer;
    int _level = -1;
    bool _charging = false;
};
