#include "QGCHostNotices.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QDateTime>
#include <QtCore/QVariantMap>

#include <algorithm>

namespace
{
constexpr int kMaxNotices = 64;

// Under a flood the first notice is usually the one that explains the rest, so it is not the one
// to lose. The oldest few are kept and the drop is taken from behind them.
constexpr int kKeepOldest = 8;
}

QGCHostNotices *QGCHostNotices::instance()
{
    static QGCHostNotices *const notices = new QGCHostNotices(QCoreApplication::instance());
    return notices;
}

QString QGCHostNotices::token(Kind kind)
{
    switch (kind) {
    case Message:
        return QStringLiteral("message");
    case VehicleError:
        return QStringLiteral("vehicleError");
    case Navigation:
        return QStringLiteral("navigation");
    }
    return QStringLiteral("message");
}

QVariantList QGCHostNotices::notices() const
{
    QMutexLocker locked(&_lock);
    return _notices;
}

int QGCHostNotices::count() const
{
    QMutexLocker locked(&_lock);
    return _notices.count();
}

int QGCHostNotices::dropped() const
{
    QMutexLocker locked(&_lock);
    return _dropped;
}

void QGCHostNotices::post(Kind kind, const QString &title, const QString &text)
{
    {
        QMutexLocker locked(&_lock);
        while (_notices.count() >= kMaxNotices) {
            _notices.removeAt(qMin(kKeepOldest, _notices.count() - 1));
            _dropped++;
        }

        _notices.append(QVariantMap {
            { QStringLiteral("id"), _nextId++ },
            { QStringLiteral("kind"), token(kind) },
            { QStringLiteral("title"), title },
            { QStringLiteral("text"), text },
            { QStringLiteral("at"), QDateTime::currentMSecsSinceEpoch() },
        });
    }
    emit noticesChanged();
}

bool QGCHostNotices::acknowledge(qint64 id)
{
    {
        QMutexLocker locked(&_lock);
        const auto found = std::find_if(_notices.cbegin(), _notices.cend(), [id](const QVariant &notice) {
            return notice.toMap().value(QStringLiteral("id")).toLongLong() == id;
        });
        if (found == _notices.cend()) {
            return false;
        }
        _notices.removeAt(static_cast<int>(std::distance(_notices.cbegin(), found)));
    }
    emit noticesChanged();
    return true;
}

int QGCHostNotices::acknowledgeThrough(qint64 id)
{
    int removed = 0;
    {
        QMutexLocker locked(&_lock);
        const int before = _notices.count();
        QVariantList kept;
        for (const QVariant &notice : std::as_const(_notices)) {
            if (notice.toMap().value(QStringLiteral("id")).toLongLong() > id) {
                kept.append(notice);
            }
        }
        _notices = kept;
        removed = before - _notices.count();
    }
    if (removed > 0) {
        emit noticesChanged();
    }
    return removed;
}
