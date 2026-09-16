#include "SkydroidH16LinksTest.h"
#include "AutoConnectSettings.h"
#include "LinkManager.h"
#include "QmlObjectListModel.h"
#include "SettingsManager.h"
#include "SkydroidH16Links.h"
#include "UDPLink.h"
#include "VideoSettings.h"

#include <QtTest/QTest>

namespace {

const UDPConfiguration *udpConfigOnPort(quint16 localPort)
{
    QmlObjectListModel *configs = LinkManager::instance()->linkConfigurations();
    for (int i = 0; i < configs->count(); i++) {
        const auto *config = qobject_cast<const LinkConfiguration *>(configs->get(i));
        if (!config || config->type() != LinkConfiguration::TypeUdp) {
            continue;
        }
        const auto *udp = static_cast<const UDPConfiguration *>(config);
        if (udp->localPort() == localPort) {
            return udp;
        }
    }
    return nullptr;
}

} // namespace

void SkydroidH16LinksTest::_ensureCreatesBothLinksOnce()
{
    AutoConnectSettings *autoConnect = SettingsManager::instance()->autoConnectSettings();
    VideoSettings *video = SettingsManager::instance()->videoSettings();
    const QVariant savedAutoConnectUdp = autoConnect->autoConnectUDP()->rawValue();
    const QVariant savedVideoSource = video->videoSource()->rawValue();
    const QVariant savedRtspUrl = video->rtspUrl()->rawValue();
    autoConnect->autoConnectUDP()->setRawValue(true);
    video->rtspUrl()->setRawValue(QString());
    const int before = LinkManager::instance()->linkConfigurations()->count();

    QCOMPARE(SkydroidH16Links::ensure(LinkManager::instance(), autoConnect, video), 3);
    QCOMPARE(LinkManager::instance()->linkConfigurations()->count(), before + 2);
    QVERIFY(!autoConnect->autoConnectUDP()->rawValue().toBool());
    QCOMPARE(video->videoSource()->rawValue().toString(), QString::fromUtf8(VideoSettings::videoSourceRTSP));
    QCOMPARE(video->rtspUrl()->rawValue().toString(), SkydroidH16Links::kVideoUrl);

    const UDPConfiguration *telemetry = udpConfigOnPort(SkydroidH16Links::kTelemetryLocalPort);
    QVERIFY(telemetry);
    QVERIFY(telemetry->isAutoConnect());
    QVERIFY(telemetry->hostList().isEmpty());

    const UDPConfiguration *camera = udpConfigOnPort(SkydroidH16Links::kCameraLocalPort);
    QVERIFY(camera);
    QVERIFY(camera->isAutoConnect());
    QCOMPARE(camera->hostList(), QStringList{QStringLiteral("127.0.0.1:15552")});

    QCOMPARE(SkydroidH16Links::ensure(LinkManager::instance(), autoConnect, video), 0);
    QCOMPARE(LinkManager::instance()->linkConfigurations()->count(), before + 2);

    video->rtspUrl()->setRawValue(QStringLiteral("rtsp://10.0.0.5/mine"));
    QCOMPARE(SkydroidH16Links::ensure(LinkManager::instance(), autoConnect, video), 0);
    QCOMPARE(video->rtspUrl()->rawValue().toString(), QStringLiteral("rtsp://10.0.0.5/mine"));

    LinkManager::instance()->removeConfiguration(const_cast<UDPConfiguration *>(telemetry));
    LinkManager::instance()->removeConfiguration(const_cast<UDPConfiguration *>(camera));
    autoConnect->autoConnectUDP()->setRawValue(savedAutoConnectUdp);
    video->videoSource()->setRawValue(savedVideoSource);
    video->rtspUrl()->setRawValue(savedRtspUrl);
}
