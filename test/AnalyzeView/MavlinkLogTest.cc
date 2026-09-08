#include "MavlinkLogTest.h"
#include "AppSettings.h"
#include "MAVLinkProtocol.h"
#include "MavlinkSettings.h"
#include "QGCTemporaryFile.h"
#include "SettingsManager.h"

#include <QtCore/QStandardPaths>
#include <QtTest/QTest>

void MavlinkLogTest::init(void)
{
    UnitTest::init();
    SettingsManager::instance()->mavlinkSettings()->telemetrySave()->setRawValue(true);
    SettingsManager::instance()->mavlinkSettings()->telemetrySaveNotArmed()->setRawValue(false);
    SettingsManager::instance()->appSettings()->disableAllPersistence()->setRawValue(false);
    MAVLinkProtocol::deleteTempLogFiles();
    QVERIFY(_telemetryDir().exists());
}

void MavlinkLogTest::cleanup(void)
{
    QCOMPARE(_tempLogs().count(), 0);
    UnitTest::cleanup();
}

QDir MavlinkLogTest::_telemetryDir(void)
{
    const QString path = SettingsManager::instance()->appSettings()->telemetrySavePath();
    QDir().mkpath(path);
    return QDir(path);
}

QStringList MavlinkLogTest::_savedLogs(void)
{
    return _telemetryDir().entryList(QStringList(QStringLiteral("*.%1").arg(AppSettings::telemetryFileExtension)), QDir::Files);
}

QStringList MavlinkLogTest::_tempLogs(void)
{
    const QDir tmpDir(QStandardPaths::writableLocation(QStandardPaths::TempLocation));
    return tmpDir.entryList(QStringList(QStringLiteral("*.%1").arg(_logFileExtension)), QDir::Files);
}

void MavlinkLogTest::_removeSavedLogsNotIn(const QStringList& keep)
{
    QDir dir = _telemetryDir();
    for (const QString& name : _savedLogs()) {
        if (!keep.contains(name)) {
            QVERIFY(dir.remove(name));
        }
    }
}

void MavlinkLogTest::_createTempLogFile(bool zeroLength)
{
    QGCTemporaryFile tempLogFile(QStringLiteral("%1.%2").arg(_tempLogFileTemplate, _logFileExtension));
    tempLogFile.open();
    if (!zeroLength) {
        tempLogFile.write("foo");
    }
    tempLogFile.close();
}

void MavlinkLogTest::_orphanedLogIsSavedToTheTelemetryDirectory(void)
{
    const QStringList before = _savedLogs();
    _createTempLogFile(false);

    MAVLinkProtocol::instance()->checkForLostLogFiles();

    QTRY_COMPARE(_savedLogs().count(), before.count() + 1);
    QCOMPARE(_tempLogs().count(), 0);
    _removeSavedLogsNotIn(before);
}

void MavlinkLogTest::_zeroLengthOrphanIsDeleted(void)
{
    const QStringList before = _savedLogs();
    _createTempLogFile(true);

    MAVLinkProtocol::instance()->checkForLostLogFiles();

    QCOMPARE(_tempLogs().count(), 0);
    QCOMPARE(_savedLogs(), before);
}

void MavlinkLogTest::_deleteTempLogFilesEmptiesTheTempDirectory(void)
{
    _createTempLogFile(false);
    MAVLinkProtocol::deleteTempLogFiles();
    QCOMPARE(_tempLogs().count(), 0);
}
