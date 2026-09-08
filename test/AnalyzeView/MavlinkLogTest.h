#pragma once

#include "UnitTest.h"

#include <QtCore/QDir>
#include <QtCore/QStringList>

class MavlinkLogTest : public UnitTest
{
    Q_OBJECT

private slots:
    void init(void);
    void cleanup(void);

    void _orphanedLogIsSavedToTheTelemetryDirectory(void);
    void _zeroLengthOrphanIsDeleted(void);
    void _deleteTempLogFilesEmptiesTheTempDirectory(void);

signals:
    void checkForLostLogFiles(void);

private:
    void        _createTempLogFile(bool zeroLength);
    QDir        _telemetryDir(void);
    QStringList _savedLogs(void);
    QStringList _tempLogs(void);
    void        _removeSavedLogsNotIn(const QStringList& keep);

    static constexpr const char* _tempLogFileTemplate = "FlightDataXXXXXX";
    static constexpr const char* _logFileExtension    = "mavlink";
};
