#include "QGCHostNotices.h"

#include "QGCCoreC.h"

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

void QGCHostNotices::post(Kind kind, const QString &title, const QString &text)
{
    (void) qgc_core_post_notice(token(kind).toUtf8().constData(), title.toUtf8().constData(), text.toUtf8().constData());
}
