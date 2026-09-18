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

QString SkydroidH16Links::cameraName(int index)
{
    return QObject::tr("Camera %1").arg(index + 1);
}

QString SkydroidH16Links::extraCamerasJson()
{
    QJsonArray extras;
    for (int i = 1; i < kCameraPaths.size(); ++i) {
        QJsonObject camera;
        camera.insert(QStringLiteral("name"), cameraName(i));
        camera.insert(QStringLiteral("source"), QString::fromUtf8(VideoSettings::videoSourceRTSP));
        camera.insert(QStringLiteral("url"), cameraUrl(kCameraPaths.at(i)));
        extras.append(camera);
    }
    return QString::fromUtf8(QJsonDocument(extras).toJson(QJsonDocument::Compact));
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
    if (isH16Managed(video)) {
        if (video->rtspUrl()->rawValue().toString().trimmed().isEmpty()) {
            video->videoSource()->setRawValue(QString::fromUtf8(VideoSettings::videoSourceRTSP));
            video->rtspUrl()->setRawValue(cameraUrl(kCameraPaths.first()));
            video->primaryCameraName()->setRawValue(cameraName(0));
            added++;
        }
        if (video->extraVideoSources()->rawValue().toString() != extraCamerasJson()) {
            video->extraVideoSources()->setRawValue(extraCamerasJson());
            added++;
        }
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
