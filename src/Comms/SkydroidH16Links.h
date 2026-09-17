#pragma once

#include <QtCore/QObject>
#include <QtCore/QString>
#include <QtCore/QStringList>

class LinkManager;
class AutoConnectSettings;
class VideoSettings;
class QTimer;

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
    // The air unit serves one RTSP path per connected camera: /H264Video, then
    // /H264Video1, /H264Video2, ... A path answers 200 only while a camera is on it.
    inline const QStringList kCameraPaths = {
        QStringLiteral("H264Video"),
        QStringLiteral("H264Video1"),
        QStringLiteral("H264Video2"),
        QStringLiteral("H264Video3"),
    };

    QString cameraUrl(const QString &path);

    bool isThisRemote();
    int ensure(LinkManager *linkManager, AutoConnectSettings *autoConnect, VideoSettings *video);
}

// Discovers how many cameras the air unit is serving and keeps the video sources
// in step with them. Probes the RTSP paths on a worker thread, then rewrites the
// H16-managed video sources on the main thread, leaving a user's own URL untouched.
class SkydroidH16CameraWatcher : public QObject
{
    Q_OBJECT

public:
    explicit SkydroidH16CameraWatcher(VideoSettings *video, QObject *parent = nullptr);

    void start();
    void refreshOnce();

private:
    void _apply(const QStringList &livePaths);

    VideoSettings *_video = nullptr;
    QTimer *_timer = nullptr;
};
