#pragma once

#include <QtCore/QString>

class QGCHostNotices
{
public:
    enum Kind {
        Message,
        VehicleError,
        Navigation,
    };

    static QString token(Kind kind);
    static void post(Kind kind, const QString &title, const QString &text);
};
