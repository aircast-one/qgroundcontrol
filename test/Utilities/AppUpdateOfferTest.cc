#include "AppUpdateOfferTest.h"

#include "AppUpdateOffer.h"

using namespace AppUpdateOffer;

namespace {

const QByteArray kRelease = QByteArrayLiteral(R"({
  "html_url": "https://github.com/aircast-one/qgroundcontrol/releases/tag/aircast-v5.5.13",
  "tag_name": "aircast-v5.5.13",
  "draft": false,
  "prerelease": false,
  "assets": [
    {"name": "AircastQGC-v5.5.13-installer-arm64.exe", "browser_download_url": "https://x/arm64.exe"},
    {"name": "AircastQGC-v5.5.13-installer-arm64.exe.sha256", "browser_download_url": "https://x/arm64.sha"},
    {"name": "AircastQGC-v5.5.13-installer.exe", "browser_download_url": "https://x/x64.exe"},
    {"name": "AircastQGC-v5.5.13-x86_64.AppImage", "browser_download_url": "https://x/linux"},
    {"name": "AircastQGC-v5.5.13.apk", "browser_download_url": "https://x/apk"},
    {"name": "AircastQGC-v5.5.13.dmg", "browser_download_url": "https://x/dmg"}
  ],
  "body": "Since v5.5.12: the radio's camera channels"
})");

}  // namespace

void AppUpdateOfferTest::_theLatestStableReleaseOffersTheInstallerForThisMachine_test()
{
    const auto on = [](const char* product, const char* kernel, const char* arch) {
        return parse(kRelease, QLatin1String(product), QLatin1String(kernel), QLatin1String(arch));
    };
    const QUrl page(QStringLiteral("https://github.com/aircast-one/qgroundcontrol/releases/tag/aircast-v5.5.13"));
    QCOMPARE(on("windows", "winnt", "x86_64"), (Offer{QStringLiteral("v5.5.13"), QUrl("https://x/x64.exe"), page}));
    QCOMPARE(on("windows", "winnt", "arm64")->download, QUrl("https://x/arm64.exe"));
    QCOMPARE(on("macos", "darwin", "arm64")->download, QUrl("https://x/dmg"));
    QCOMPARE(on("android", "linux", "arm64")->download, QUrl("https://x/apk"));
    QCOMPARE(on("ubuntu", "linux", "x86_64")->download, QUrl("https://x/linux"));
    QCOMPARE(on("freebsd", "freebsd", "x86_64")->download, page);
    QCOMPARE(on("ios", "darwin", "arm64")->download, page);
    QVERIFY(!parse(QByteArrayLiteral("{\"tag_name\":\"aircast-v5.6.5-beta.30\",\"prerelease\":true}"), "windows",
                   "winnt", "x86_64"));
    QVERIFY(!parse(QByteArrayLiteral("not json"), "windows", "winnt", "x86_64"));
    QVERIFY(!parse(QByteArrayLiteral("{\"tag_name\":\"nightly\",\"prerelease\":false}"), "windows", "winnt", "x86_64"));
}

void AppUpdateOfferTest::_onlyANewerStableVersionIsAnOffer_test()
{
    QVERIFY(newer(QStringLiteral("aircast-v5.5.12"), QStringLiteral("v5.5.13")));
    QVERIFY(newer(QStringLiteral("aircast-v5.5.11-15-ga28dbd3882"), QStringLiteral("v5.5.13")));
    QVERIFY(newer(QStringLiteral("aircast-v5.5.13"), QStringLiteral("v5.6.0")));
    QVERIFY(!newer(QStringLiteral("aircast-v5.5.13"), QStringLiteral("v5.5.13")));
    QVERIFY(!newer(QStringLiteral("aircast-v5.6.5-beta.30"), QStringLiteral("v5.5.13")));
    QVERIFY(!newer(QStringLiteral("unknown"), QStringLiteral("v5.5.13")));
    const Offer offer{QStringLiteral("v5.5.13"), QUrl(), QUrl()};
    const QString ready = text(QStringLiteral("Aircast QGC"), QStringLiteral("aircast-v5.5.12"), offer, true);
    QVERIFY(ready.startsWith(QStringLiteral("Aircast QGC v5.5.13 is available. You are running v5.5.12.")));
    QVERIFY(ready.endsWith(QStringLiteral("OK installs it and restarts Aircast QGC.")));
    QVERIFY(text(QStringLiteral("Aircast QGC"), QStringLiteral("aircast-v5.5.12"), offer, false)
                .contains(QStringLiteral("OK opens the download")));
}

UT_REGISTER_TEST(AppUpdateOfferTest, TestLabel::Unit)
