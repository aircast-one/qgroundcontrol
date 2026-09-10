#pragma once

#include <QtCore/QMutex>
#include <QtCore/QObject>
#include <QtCore/QVariantList>

class QGCHostNotices : public QObject
{
    Q_OBJECT

    Q_PROPERTY(QVariantList notices READ notices NOTIFY noticesChanged)
    Q_PROPERTY(int count READ count NOTIFY noticesChanged)
    Q_PROPERTY(int dropped READ dropped NOTIFY noticesChanged)

public:
    enum Kind {
        Message,
        VehicleError,
        Navigation,
    };
    Q_ENUM(Kind)

    static QGCHostNotices *instance();

    QVariantList notices() const;
    int count() const;
    int dropped() const;

    static QString token(Kind kind);

    Q_INVOKABLE bool acknowledge(qint64 id);
    Q_INVOKABLE int acknowledgeThrough(qint64 id);

    void post(Kind kind, const QString &title, const QString &text);

signals:
    void noticesChanged();

private:
    explicit QGCHostNotices(QObject *parent = nullptr) : QObject(parent) {}

    mutable QMutex _lock;
    QVariantList _notices;
    qint64 _nextId = 1;
    int _dropped = 0;
};
