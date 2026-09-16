#pragma once

#include <QtCore/QString>

class LinkManager;
class AutoConnectSettings;

namespace SkydroidH16Links
{
    constexpr quint16 kTelemetryLocalPort = 14551;
    constexpr quint16 kCameraLocalPort = 15553;
    constexpr quint16 kCameraRouterPort = 15552;
    inline const QString kRouterHost = QStringLiteral("127.0.0.1");
    inline const QString kTelemetryLinkName = QStringLiteral("H16 telemetry");
    inline const QString kCameraLinkName = QStringLiteral("H16 camera");

    bool isThisRemote();
    int ensure(LinkManager *linkManager, AutoConnectSettings *autoConnect);
}
