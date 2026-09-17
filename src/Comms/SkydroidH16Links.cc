#include "SkydroidH16Links.h"
#include "AutoConnectSettings.h"
#include "LinkManager.h"
#include "QGCLoggingCategory.h"
#include "QmlObjectListModel.h"
#include "UDPLink.h"
#include "VideoSettings.h"

#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QMetaObject>
#include <QtCore/QPointer>
#include <QtCore/QThread>
#include <QtCore/QTimer>
#include <QtNetwork/QTcpSocket>

#ifdef Q_OS_ANDROID
#include <QtCore/QJniObject>
#endif

QGC_LOGGING_CATEGORY(SkydroidH16LinksLog, "qgc.comms.skydroidh16links")

namespace {

bool hasUdpConfigOnLocalPort(LinkManager *linkManager, quint16 localPort)
{
    QmlObjectListModel *configs = linkManager->linkConfigurations();
    for (int i = 0; i < configs->count(); i++) {
        const auto *config = qobject_cast<const LinkConfiguration *>(configs->get(i));
        if (!config || config->type() != LinkConfiguration::TypeUdp) {
            continue;
        }
        if (static_cast<const UDPConfiguration *>(config)->localPort() == localPort) {
            return true;
        }
    }
    return false;
}

UDPConfiguration *makeAutoUdpConfig(const QString &name, quint16 localPort)
{
    auto *config = new UDPConfiguration(name);
    config->setAutoConnect(true);
    config->setLocalPort(localPort);
    return config;
}

bool rtspPathIsLive(const QString &path)
{
    QTcpSocket socket;
    socket.connectToHost(SkydroidH16Links::kAirUnitHost, SkydroidH16Links::kRtspPort);
    if (!socket.waitForConnected(800)) {
        return false;
    }
    const QByteArray request = QStringLiteral("DESCRIBE %1 RTSP/1.0\r\nCSeq: 1\r\nAccept: application/sdp\r\n\r\n")
                                   .arg(SkydroidH16Links::cameraUrl(path)).toUtf8();
    socket.write(request);
    if (!socket.waitForBytesWritten(500)) {
        return false;
    }
    QByteArray response;
    while (response.size() < 32 && socket.waitForReadyRead(1200)) {
        response += socket.readAll();
    }
    return response.startsWith("RTSP/1.0 200");
}

QString cameraName(int index)
{
    return QObject::tr("Camera %1").arg(index + 1);
}

QString extrasJson(const QStringList &livePaths)
{
    QJsonArray extras;
    for (int i = 1; i < livePaths.size(); ++i) {
        QJsonObject camera;
        camera.insert(QStringLiteral("name"), cameraName(i));
        camera.insert(QStringLiteral("source"), QString::fromUtf8(VideoSettings::videoSourceRTSP));
        camera.insert(QStringLiteral("url"), SkydroidH16Links::cameraUrl(livePaths.at(i)));
        extras.append(camera);
    }
    return QString::fromUtf8(QJsonDocument(extras).toJson(QJsonDocument::Compact));
}

bool isH16Managed(VideoSettings *video)
{
    const QString url = video->rtspUrl()->rawValue().toString().trimmed();
    if (url.isEmpty()) {
        return true;
    }
    for (const QString &path : SkydroidH16Links::kCameraPaths) {
        if (url == SkydroidH16Links::cameraUrl(path)) {
            return true;
        }
    }
    return false;
}

} // namespace

QString SkydroidH16Links::cameraUrl(const QString &path)
{
    return QStringLiteral("rtsp://%1:%2/%3").arg(kAirUnitHost).arg(kRtspPort).arg(path);
}

bool SkydroidH16Links::isThisRemote()
{
#ifdef Q_OS_ANDROID
    const QString model = QJniObject::getStaticObjectField<jstring>("android/os/Build", "MODEL").toString();
    const QString manufacturer = QJniObject::getStaticObjectField<jstring>("android/os/Build", "MANUFACTURER").toString();
    return model.compare(QStringLiteral("song"), Qt::CaseInsensitive) == 0
        || manufacturer.compare(QStringLiteral("Fishsemi"), Qt::CaseInsensitive) == 0;
#else
    return false;
#endif
}

int SkydroidH16Links::ensure(LinkManager *linkManager, AutoConnectSettings *autoConnect, VideoSettings *video)
{
    int added = 0;
    if (video->rtspUrl()->rawValue().toString().trimmed().isEmpty()) {
        // Seed the first camera so video shows immediately; the watcher fills in the rest.
        video->videoSource()->setRawValue(QString::fromUtf8(VideoSettings::videoSourceRTSP));
        video->rtspUrl()->setRawValue(cameraUrl(kCameraPaths.first()));
        video->primaryCameraName()->setRawValue(cameraName(0));
        qCDebug(SkydroidH16LinksLog) << "Seeded air unit video" << cameraUrl(kCameraPaths.first());
        added++;
    }
    if (!hasUdpConfigOnLocalPort(linkManager, kTelemetryLocalPort)) {
        linkManager->addConfiguration(makeAutoUdpConfig(kTelemetryLinkName, kTelemetryLocalPort));
        added++;
    }
    if (!hasUdpConfigOnLocalPort(linkManager, kCameraLocalPort)) {
        UDPConfiguration *camera = makeAutoUdpConfig(kCameraLinkName, kCameraLocalPort);
        camera->addHost(kRouterHost, kCameraRouterPort);
        linkManager->addConfiguration(camera);
        added++;
    }
    if (autoConnect->autoConnectUDP()->rawValue().toBool()) {
        autoConnect->autoConnectUDP()->setRawValue(false);
    }
    if (added > 0) {
        qCDebug(SkydroidH16LinksLog) << "Configured" << added << "SkyDroid H16 settings";
        linkManager->saveLinkConfigurationList();
    }
    return added;
}

SkydroidH16CameraWatcher::SkydroidH16CameraWatcher(VideoSettings *video, QObject *parent)
    : QObject(parent)
    , _video(video)
    , _timer(new QTimer(this))
{
    _timer->setInterval(30000);
    connect(_timer, &QTimer::timeout, this, &SkydroidH16CameraWatcher::refreshOnce);
}

void SkydroidH16CameraWatcher::start()
{
    refreshOnce();
    _timer->start();
}

void SkydroidH16CameraWatcher::refreshOnce()
{
    // This air unit has one video encoder and binds it to the lowest-numbered RTSP
    // path that has any session. A DESCRIBE on a path BELOW the one being played steals
    // the encoder and freezes the live picture; a probe of a HIGHER path is harmless.
    // So probe only paths above the active source's index, and never at or below it.
    // Camera 1 (index 0, /H264Video) is always assumed present, so it never needs probing.
    const int activeIndex = qMax(0, _video->currentIndex());
    QPointer<SkydroidH16CameraWatcher> self(this);
    QThread *worker = QThread::create([self, activeIndex]() {
        // Paths at or below the active index are assumed live (Camera 1, and whatever is
        // on screen); only higher paths are actually probed.
        QStringList live;
        for (int i = 0; i < SkydroidH16Links::kCameraPaths.size(); ++i) {
            const QString &path = SkydroidH16Links::kCameraPaths.at(i);
            if (i <= activeIndex || rtspPathIsLive(path)) {
                live.append(path);
            } else {
                break; // paths are contiguous: the first absent camera ends the list
            }
        }
        if (!self) {
            return;
        }
        QMetaObject::invokeMethod(self, [self, live]() {
            if (self) {
                self->_apply(live);
            }
        }, Qt::QueuedConnection);
    });
    connect(worker, &QThread::finished, worker, &QObject::deleteLater);
    worker->start();
}

void SkydroidH16CameraWatcher::_apply(const QStringList &livePaths)
{
    if (livePaths.isEmpty() || !isH16Managed(_video)) {
        return;
    }

    const QString desiredPrimary = SkydroidH16Links::cameraUrl(livePaths.first());
    if (_video->rtspUrl()->rawValue().toString().trimmed() != desiredPrimary) {
        _video->videoSource()->setRawValue(QString::fromUtf8(VideoSettings::videoSourceRTSP));
        _video->rtspUrl()->setRawValue(desiredPrimary);
        _video->primaryCameraName()->setRawValue(cameraName(0));
    }

    const QString desiredExtras = extrasJson(livePaths);
    if (_video->extraVideoSources()->rawValue().toString() != desiredExtras) {
        qCDebug(SkydroidH16LinksLog) << "Air unit now serving" << livePaths.size() << "cameras";
        _video->extraVideoSources()->setRawValue(desiredExtras);
    }
}
