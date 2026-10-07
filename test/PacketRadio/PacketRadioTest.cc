#include "PacketRadioTest.h"

#include "Fact.h"
#include "PacketRadioManager.h"
#include "PacketRadioSettings.h"
#include "SettingsManager.h"
#include "VideoSettings.h"

#ifdef emit
#undef emit
#endif
#include "wfbng_link.h"

#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QTemporaryDir>
#include <QtTest/QTest>

#include <vector>

namespace {

class FactRestorer
{
public:
    explicit FactRestorer(std::initializer_list<Fact *> facts)
    {
        for (Fact *fact : facts) {
            _saved.append({fact, fact->rawValue()});
        }
    }

    ~FactRestorer()
    {
        for (const auto &entry : _saved) {
            entry.first->setRawValue(entry.second);
        }
    }

private:
    QList<QPair<Fact *, QVariant>> _saved;
};

DeviceId makeDevice(const char *name, bool known)
{
    DeviceId device{};
    device.vendor_id = 0x0bda;
    device.product_id = 0x881a;
    device.display_name = name;
    device.known_adapter = known;
    return device;
}

}

void PacketRadioTest::_videoSettingsRoundTripTest()
{
    VideoSettings *video = SettingsManager::instance()->videoSettings();
    FactRestorer restore({video->cameras(), video->activeVideoSource(), video->lowLatencyMode()});

    const QString mine = QStringLiteral(R"([{"name":"Front","source":"RTSP Video Stream","url":"rtsp://10.0.0.5:8554/front"}])");
    video->cameras()->setRawValue(mine);
    video->activeVideoSource()->setRawValue(0);
    video->lowLatencyMode()->setRawValue(false);

    PacketRadioManager manager;

    manager._applyVideoSettings(QStringLiteral("H265"));
    QCOMPARE(video->videoSourceCount(), 2);
    QCOMPARE(video->cameraName(1), QStringLiteral("Packet radio"));
    QCOMPARE(video->videoSourceNameAt(1), QString(VideoSettings::videoSourceUDPH265));
    QCOMPARE(video->videoUrlAt(1), QStringLiteral("0.0.0.0:5600"));
    QCOMPARE(video->currentIndex(), 1);
    QVERIFY(video->lowLatencyMode()->rawValue().toBool());

    manager._applyVideoSettings(QStringLiteral("H264"));
    QCOMPARE(video->videoSourceCount(), 2);
    QCOMPARE(video->videoSourceNameAt(1), QString(VideoSettings::videoSourceUDPH264));

    manager._restoreVideoSettings();
    QCOMPARE(video->cameras()->rawValue().toString(), mine);
    QCOMPARE(video->currentIndex(), 0);
    QVERIFY(!video->lowLatencyMode()->rawValue().toBool());
}

void PacketRadioTest::_keyPathTest()
{
    PacketRadioSettings *settings = SettingsManager::instance()->packetRadioSettings();
    FactRestorer restore({settings->keyFile()});

    PacketRadioManager manager;

    settings->keyFile()->setRawValue(QString());
    const QString fallback = manager._resolveKeyPath();
    QVERIFY(!fallback.isEmpty());
    QVERIFY(QFile::exists(fallback));
    QCOMPARE(QFileInfo(fallback).size(), 64);

    settings->keyFile()->setRawValue(QStringLiteral("/nonexistent/gs.key"));
    QVERIFY(manager._resolveKeyPath().isEmpty());

    QTemporaryDir dir;
    QVERIFY(dir.isValid());
    const QString custom = dir.filePath(QStringLiteral("gs.key"));
    QFile file(custom);
    QVERIFY(file.open(QIODevice::WriteOnly));
    file.write(QByteArray(64, '\0'));
    file.close();

    settings->keyFile()->setRawValue(custom);
    QCOMPARE(manager._resolveKeyPath(), custom);

    QFile::remove(fallback);
}

void PacketRadioTest::_selectAdapterTest()
{
    QVERIFY(PacketRadioManager::selectAdapter({}, "") == nullptr);

    const std::vector<DeviceId> unsupportedOnly = {makeDevice("Some Hub [0:1]", false)};
    QVERIFY(PacketRadioManager::selectAdapter(unsupportedOnly, "") == nullptr);

    const std::vector<DeviceId> two = {
        makeDevice("Some Hub [0:1]", false),
        makeDevice("RTL8812AU [0:2]", true),
        makeDevice("RTL8812AU-VS [0:3]", true),
    };

    const DeviceId *automatic = PacketRadioManager::selectAdapter(two, "");
    QVERIFY(automatic != nullptr);
    QCOMPARE(QString::fromStdString(automatic->display_name), QStringLiteral("RTL8812AU [0:2]"));

    const DeviceId *saved = PacketRadioManager::selectAdapter(two, "RTL8812AU-VS [0:3]");
    QVERIFY(saved != nullptr);
    QCOMPARE(QString::fromStdString(saved->display_name), QStringLiteral("RTL8812AU-VS [0:3]"));

    const DeviceId *missing = PacketRadioManager::selectAdapter(two, "RTL8812AU [9:9]");
    QVERIFY(missing != nullptr);
    QCOMPARE(QString::fromStdString(missing->display_name), QStringLiteral("RTL8812AU [0:2]"));

    const std::vector<DeviceId> unsupportedNameMatch = {makeDevice("Some Hub [0:1]", false)};
    QVERIFY(PacketRadioManager::selectAdapter(unsupportedNameMatch, "Some Hub [0:1]") == nullptr);
}

UT_REGISTER_TEST(PacketRadioTest, TestLabel::Unit)
