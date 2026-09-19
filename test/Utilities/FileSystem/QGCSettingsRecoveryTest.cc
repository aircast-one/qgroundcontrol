#include "QGCSettingsRecoveryTest.h"
#include "QGCSettingsRecovery.h"

#include <QtCore/QFile>
#include <QtCore/QRegularExpression>
#include <QtCore/QSettings>
#include <QtCore/QTemporaryDir>
#include <QtTest/QTest>

namespace {

QString writeIni(const QTemporaryDir &dir, QFileDevice::Permissions permissions)
{
    const QString path = dir.filePath(QStringLiteral("Aircast QGC Daily.ini"));
    {
        QSettings seed(path, QSettings::IniFormat);
        seed.setValue(QStringLiteral("FlyView/showPhotoVideoControl"), true);
        seed.setValue(QStringLiteral("LinkConfigurations/count"), 2);
    }
    QFile::setPermissions(path, permissions);
    return path;
}

} // namespace

void QGCSettingsRecoveryTest::_unreadableFileIsReplaced()
{
    QTemporaryDir dir;
    const QString path = writeIni(dir, QFileDevice::Permissions());

    {
        QSettings settings(path, QSettings::IniFormat);
        QVERIFY(!settings.isWritable());
        expectLogMessage("qgc.utilities.filesystem.qgcsettingsrecovery", QtWarningMsg, QRegularExpression(QStringLiteral("moved aside and recreated")));
        QVERIFY(QGCSettingsRecovery::moveAsideIfUnwritable(settings));
        verifyExpectedLogMessage();
        QVERIFY(settings.isWritable());
        settings.setValue(QStringLiteral("General/appFontPointSize"), 11);
    }

    QSettings reopened(path, QSettings::IniFormat);
    QCOMPARE(reopened.value(QStringLiteral("General/appFontPointSize")).toInt(), 11);
    QVERIFY(QFile::exists(path + QStringLiteral(".unwritable")));
}

void QGCSettingsRecoveryTest::_readOnlyFileKeepsItsValues()
{
    QTemporaryDir dir;
    const QString path = writeIni(dir, QFileDevice::ReadOwner);

    {
        QSettings settings(path, QSettings::IniFormat);
        QVERIFY(!settings.isWritable());
        expectLogMessage("qgc.utilities.filesystem.qgcsettingsrecovery", QtWarningMsg, QRegularExpression(QStringLiteral("moved aside and recreated")));
        QVERIFY(QGCSettingsRecovery::moveAsideIfUnwritable(settings));
        verifyExpectedLogMessage();
        settings.setValue(QStringLiteral("General/appFontPointSize"), 11);
    }

    QSettings reopened(path, QSettings::IniFormat);
    QCOMPARE(reopened.value(QStringLiteral("FlyView/showPhotoVideoControl")).toBool(), true);
    QCOMPARE(reopened.value(QStringLiteral("LinkConfigurations/count")).toInt(), 2);
    QCOMPARE(reopened.value(QStringLiteral("General/appFontPointSize")).toInt(), 11);
}

void QGCSettingsRecoveryTest::_writableFileIsLeftAlone()
{
    QTemporaryDir dir;
    const QString path = writeIni(dir, QFileDevice::ReadOwner | QFileDevice::WriteOwner);

    QSettings settings(path, QSettings::IniFormat);
    QVERIFY(!QGCSettingsRecovery::moveAsideIfUnwritable(settings));
    QVERIFY(!QFile::exists(path + QStringLiteral(".unwritable")));
}

UT_REGISTER_TEST(QGCSettingsRecoveryTest, TestLabel::Unit)
