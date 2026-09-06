#pragma once

#include <functional>

#include <QtCore/QString>
#include <QtCore/QStringList>

namespace QGCBridgeCore
{
    using EventHandler = std::function<void(const QString &path, const QString &json)>;

    QString get(const QString &path);
    QString set(const QString &path, const QString &valueJson);
    QString invoke(const QString &path, const QString &argsJson);
    void watch(const QStringList &paths);
    void setEventHandler(EventHandler handler);
}
