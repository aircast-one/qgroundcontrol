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
    const QVariant savedExtras = video->extraVideoSources()->rawValue();
    const QVariant savedPrimaryName = video->primaryCameraName()->rawValue();
    const QVariant savedMultiView = video->multiViewEnabled()->rawValue();
    video->multiViewEnabled()->setRawValue(false);
    autoConnect->autoConnectUDP()->setRawValue(true);
    video->rtspUrl()->setRawValue(QString());
    video->extraVideoSources()->setRawValue(QStringLiteral("[]"));
    const int before = LinkManager::instance()->linkConfigurations()->count();

    QCOMPARE(SkydroidH16Links::ensure(LinkManager::instance(), autoConnect, video), 4);
    QCOMPARE(LinkManager::instance()->linkConfigurations()->count(), before + 2);
    QVERIFY(!autoConnect->autoConnectUDP()->rawValue().toBool());
    QCOMPARE(video->videoSource()->rawValue().toString(), QString::fromUtf8(VideoSettings::videoSourceRTSP));
    QCOMPARE(video->rtspUrl()->rawValue().toString(), SkydroidH16Links::cameraUrl(SkydroidH16Links::kCameraPaths.first()));
    QCOMPARE(video->primaryCameraName()->rawValue().toString(), QStringLiteral("Camera 1"));
    QCOMPARE(video->videoSourceCount(), 2);
    QCOMPARE(video->cameraName(1), QStringLiteral("Camera 2"));
    QCOMPARE(video->videoUrlAt(1), SkydroidH16Links::cameraUrl(SkydroidH16Links::kCameraPaths.at(1)));
    QVERIFY(video->multiViewEnabled()->rawValue().toBool());

    const UDPConfiguration *telemetry = udpConfigOnPort(SkydroidH16Links::kTelemetryLocalPort);
    QVERIFY(telemetry);
    QVERIFY(telemetry->isAutoConnect());
    QVERIFY(telemetry->hostList().isEmpty());

    const UDPConfiguration *camera = udpConfigOnPort(SkydroidH16Links::kCameraLocalPort);
    QVERIFY(camera);
    QVERIFY(camera->isAutoConnect());
    QCOMPARE(camera->hostList(), QStringList{QStringLiteral("127.0.0.1:15552")});

    video->multiViewEnabled()->setRawValue(false);
    QCOMPARE(SkydroidH16Links::ensure(LinkManager::instance(), autoConnect, video), 0);
    QCOMPARE(LinkManager::instance()->linkConfigurations()->count(), before + 2);
    QVERIFY(!video->multiViewEnabled()->rawValue().toBool());

    video->extraVideoSources()->setRawValue(QStringLiteral("[]"));
    QCOMPARE(SkydroidH16Links::ensure(LinkManager::instance(), autoConnect, video), 1);
    QCOMPARE(video->videoSourceCount(), 2);

    video->rtspUrl()->setRawValue(QStringLiteral("rtsp://10.0.0.5/mine"));
    video->extraVideoSources()->setRawValue(QStringLiteral("[]"));
    QCOMPARE(SkydroidH16Links::ensure(LinkManager::instance(), autoConnect, video), 0);
    QCOMPARE(video->rtspUrl()->rawValue().toString(), QStringLiteral("rtsp://10.0.0.5/mine"));
    QCOMPARE(video->videoSourceCount(), 1);

    LinkManager::instance()->removeConfiguration(const_cast<UDPConfiguration *>(telemetry));
    LinkManager::instance()->removeConfiguration(const_cast<UDPConfiguration *>(camera));
    autoConnect->autoConnectUDP()->setRawValue(savedAutoConnectUdp);
    video->videoSource()->setRawValue(savedVideoSource);
    video->rtspUrl()->setRawValue(savedRtspUrl);
    video->extraVideoSources()->setRawValue(savedExtras);
    video->primaryCameraName()->setRawValue(savedPrimaryName);
    video->multiViewEnabled()->setRawValue(savedMultiView);
}

UT_REGISTER_TEST(SkydroidH16LinksTest, TestLabel::Unit)
