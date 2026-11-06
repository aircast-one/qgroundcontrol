#include "AppUpdateInstallTest.h"

#include <QtCore/QDir>
#include <QtCore/QTemporaryDir>

#include "AppUpdateInstall.h"

using namespace AppUpdateInstall;

void AppUpdateInstallTest::_theChecksumListingGivesTheSha256_test()
{
    QCOMPARE(sha256FromListing(QByteArrayLiteral(
                 "580F1BA728870859914723C0B70BD557B73CAFA8C096E43B08B2FA0CA0D360CC  AircastQGC-v5.5.13.dmg\n")),
             QStringLiteral("580f1ba728870859914723c0b70bd557b73cafa8c096e43b08b2fa0ca0d360cc"));
    QVERIFY(sha256FromListing(QByteArrayLiteral("<html>Not Found</html>")).isEmpty());
    QVERIFY(sha256FromListing(QByteArrayLiteral("580f1ba7  short")).isEmpty());
}

void AppUpdateInstallTest::_eachPlatformInstallsItsOwnWay_test()
{
    const QString file = QStringLiteral("/tmp/Updates/AircastQGC-v5.5.14.dmg");
    QCOMPARE(launch("winnt", "windows", "C:/Updates/setup.exe", {}, {}, 7),
             (Launch{Way::RunInstaller, QStringLiteral("C:/Updates/setup.exe"), {QStringLiteral("/S")}}));
    QCOMPARE(launch("linux", "android", "/data/app.apk", {}, {}, 7),
             (Launch{Way::HandToAndroid, QStringLiteral("/data/app.apk"), {}}));

    const Launch mac = launch("darwin", "macos", file, QStringLiteral("/Applications/Aircast QGC.app"), {}, 42);
    QCOMPARE(mac.way, Way::ReplaceBundle);
    QCOMPARE(mac.program, QStringLiteral("/bin/sh"));
    QVERIFY(mac.arguments.at(1).startsWith(QStringLiteral("while kill -0 42 ")));
    QVERIFY2(mac.arguments.at(1).contains(QStringLiteral("ditto \"$APP\" '/Applications/Aircast QGC.app'")),
             qPrintable(mac.arguments.at(1)));
    QVERIFY(mac.arguments.at(1).endsWith(QStringLiteral("open -n '/Applications/Aircast QGC.app'")));
    QCOMPARE(launch("darwin", "macos", file, {}, {}, 42).way, Way::OpenFile);

    const Launch linux = launch("linux", "ubuntu", QStringLiteral("/tmp/new.AppImage"), {},
                                QStringLiteral("/opt/Aircast's.AppImage"), 9);
    QCOMPARE(linux.way, Way::ReplaceAppImage);
    QVERIFY2(linux.arguments.at(1).contains(QStringLiteral("mv -f '/opt/Aircast'\\''s.AppImage'.new")),
             qPrintable(linux.arguments.at(1)));
    QVERIFY(linux.arguments.at(1).endsWith(QStringLiteral("exec '/opt/Aircast'\\''s.AppImage'")));
    QCOMPARE(launch("linux", "ubuntu", QStringLiteral("/tmp/new.AppImage"), {}, {}, 9).way, Way::OpenFile);
}

void AppUpdateInstallTest::_aBundleIsReplacedOnlyWhereTheAppCanWrite_test()
{
    QTemporaryDir home;
    QVERIFY(home.isValid());
    const QString bundle = home.path() + QStringLiteral("/Aircast QGC.app");
    QVERIFY(QDir().mkpath(bundle + QStringLiteral("/Contents/MacOS")));
    QCOMPARE(bundleOf(bundle + QStringLiteral("/Contents/MacOS")), QDir(bundle).absolutePath());
    QVERIFY2(bundleOf(home.path()).isEmpty(), "not inside a .app bundle");
    QVERIFY2(bundleOf(QStringLiteral("/Volumes/AircastQGC/AircastQGC.app/Contents/MacOS")).isEmpty(),
             "a bundle on a read-only disk image is not replaced in place");
}

UT_REGISTER_TEST(AppUpdateInstallTest, TestLabel::Unit)
