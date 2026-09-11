#include "QGCHostNotices.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QDateTime>
#include <QtCore/QVariantMap>

#include <algorithm>

namespace
{
constexpr int kMaxNotices = 64;

constexpr int kKeepOldest = 8;
}

QGCHostNotices *QGCHostNotices::instance()
{
    static QGCHostNotices *const notices = []() {
        QGCHostNotices *const created = new QGCHostNotices();
        created->moveToThread(QCoreApplication::instance()->thread());
        created->setParent(QCoreApplication::instance());
        return created;
    }();
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
        const QString token = QGCHostNotices::token(kind);
        if (!_notices.isEmpty()) {
            const QVariantMap newest = _notices.last().toMap();
            const bool same = newest.value(QStringLiteral("kind")).toString() == token
                    && newest.value(QStringLiteral("title")).toString() == title
                    && newest.value(QStringLiteral("text")).toString() == text;
            if (same) {
                QVariantMap repeated = newest;
                repeated.insert(QStringLiteral("repeated"), newest.value(QStringLiteral("repeated")).toInt() + 1);
                repeated.insert(QStringLiteral("lastAt"), QDateTime::currentMSecsSinceEpoch());
                _notices.replace(_notices.count() - 1, repeated);
                locked.unlock();
                emit noticesChanged();
                return;
            }
        }

        while (_notices.count() >= kMaxNotices) {
            _notices.removeAt(qMin(kKeepOldest, _notices.count() - 1));
            _dropped++;
        }

        _notices.append(QVariantMap {
            { QStringLiteral("id"), _nextId++ },
            { QStringLiteral("kind"), token },
            { QStringLiteral("repeated"), 0 },
            { QStringLiteral("title"), title },
            { QStringLiteral("text"), text },
            { QStringLiteral("at"), QDateTime::currentMSecsSinceEpoch() },
            { QStringLiteral("lastAt"), QDateTime::currentMSecsSinceEpoch() },
        });
    }
    emit noticesChanged();
}

bool QGCHostNotices::postNotice(const QString &kind, const QString &title, const QString &text)
{
    static const QList<Kind> kinds = { Message, VehicleError, Navigation };
    const auto wanted = std::find_if(kinds.cbegin(), kinds.cend(), [&kind](Kind candidate) { return token(candidate) == kind; });
    if (wanted == kinds.cend()) {
        return false;
    }
    post(*wanted, title, text);
    return true;
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
