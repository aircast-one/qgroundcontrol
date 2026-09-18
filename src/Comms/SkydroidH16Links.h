#pragma once

#include <QtCore/QString>
#include <QtCore/QStringList>

class LinkManager;
class AutoConnectSettings;
class VideoSettings;

namespace SkydroidH16Links
{
    constexpr quint16 kTelemetryLocalPort = 14551;
    constexpr quint16 kCameraLocalPort = 15553;
    constexpr quint16 kCameraRouterPort = 15552;
    inline const QString kRouterHost = QStringLiteral("127.0.0.1");
    inline const QString kTelemetryLinkName = QStringLiteral("H16 telemetry");
    inline const QString kCameraLinkName = QStringLiteral("H16 camera");
    inline const QString kAirUnitHost = QStringLiteral("192.168.0.10");
    constexpr quint16 kRtspPort = 8554;
    inline const QStringList kCameraPaths = {
        QStringLiteral("H264Video"),
        QStringLiteral("H264Video1"),
    };

    QString cameraUrl(const QString &path);
    QString cameraName(int index);
    QString extraCamerasJson();

    bool isThisRemote();
    int ensure(LinkManager *linkManager, AutoConnectSettings *autoConnect, VideoSettings *video);
}
