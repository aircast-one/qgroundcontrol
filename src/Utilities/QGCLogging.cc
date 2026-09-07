/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "QGCLogging.h"
#include "AppSettings.h"
#include "QGCApplication.h"
#include "QGCLoggingCategory.h"
#include "SettingsManager.h"

#include <QtConcurrent/QtConcurrentRun>
#include <QtCore/QGlobalStatic>
#include <QtCore/QStringListModel>
#include <QtCore/QTextStream>

QGC_LOGGING_CATEGORY(QGCLoggingLog, "QGCLoggingLog")

Q_GLOBAL_STATIC(QGCLogging, _qgcLogging)

static QtMessageHandler defaultHandler = nullptr;

static void msgHandler(QtMsgType type, const QMessageLogContext &context, const QString &msg)
{
    if (!QLoggingCategory(context.category).isDebugEnabled()) {
        return;
    }

    const QString message = qFormatLogMessage(type, context, msg);

    QGCLogging *const logger = QGCLogging::instance();
    if (logger && !QString(context.category).startsWith("qt.quick")) {
        logger->log(message);
    }

    if (defaultHandler) {
        defaultHandler(type, context, msg);
    }
}

QGCLogging *QGCLogging::instance()
{
    return _qgcLogging();
}

QGCLogging::QGCLogging(QObject *parent)
    : QStringListModel(parent)
{

    _flushTimer.setInterval(kFlushIntervalMSecs);
    _flushTimer.setSingleShot(false);
    (void) connect(&_flushTimer, &QTimer::timeout, this, &QGCLogging::_flushToDisk);
    _flushTimer.start();

#if defined(Q_OS_ANDROID) || defined(Q_OS_IOS)
    const Qt::ConnectionType conntype = Qt::QueuedConnection;
#else
    const Qt::ConnectionType conntype = Qt::AutoConnection;
#endif
    (void) connect(this, &QGCLogging::emitLog, this, &QGCLogging::_threadsafeLog, conntype);
}

void QGCLogging::installHandler()
{
    qSetMessagePattern(QStringLiteral("%{time process} - %{type}: %{message} (%{category}:%{function}:%{line})"));

    defaultHandler = qInstallMessageHandler(msgHandler);
}

void QGCLogging::log(const QString &message)
{
    if (!_ioError) {
        emit emitLog(message);
    }
}

void QGCLogging::_threadsafeLog(const QString &message)
{
    const int line = rowCount();
    beginInsertRows(QModelIndex(), line, line);
    (void) QStringListModel::insertRows(line, 1);
    (void) setData(index(line, 0), message, Qt::DisplayRole);
    endInsertRows();

    static constexpr const int kMaxLogRows = kMaxLogFileSize / 100;
    if (rowCount() > kMaxLogRows) {
        const int removeCount = rowCount() - kMaxLogRows;
        beginRemoveRows(QModelIndex(), 0, removeCount - 1);
        (void) removeRows(0, removeCount);
        endRemoveRows();
    }

    _pendingDiskWrites.append(message);
}

void QGCLogging::_rotateLogs()
{
    _logFile.close();

    const QString basePath = _logFile.fileName();
    const QFileInfo fileInfo(basePath);
    const QString dir = fileInfo.absolutePath();
    const QString name = fileInfo.baseName();
    const QString ext = fileInfo.completeSuffix();

    for (int i = kMaxBackupFiles - 1; i >= 1; --i) {
        const QString from = QStringLiteral("%1/%2.%3.%4").arg(dir, name).arg(i).arg(ext);
        const QString to = QStringLiteral("%1/%2.%3.%4").arg(dir, name).arg(i+1).arg(ext);
        if (QFile::exists(to)) {
            (void) QFile::remove(to);
        }
        if (QFile::exists(from)) {
            (void) QFile::rename(from, to);
        }
    }

    const QString firstBackup = QStringLiteral("%1/%2.1.%3").arg(dir, name, ext);
    if (QFile::exists(firstBackup)) {
        (void) QFile::remove(firstBackup);
    }
    (void) QFile::rename(basePath, firstBackup);

    _logFile.setFileName(basePath);
    if (!_logFile.open(QIODevice::WriteOnly | QIODevice::Append | QIODevice::Text)) {
        _ioError = true;
        qgcApp()->showAppMessage(tr("Unable to reopen log file %1: %2").arg(_logFile.fileName(), _logFile.errorString()));
    }
}

void QGCLogging::_flushToDisk()
{
    if (_pendingDiskWrites.isEmpty() || _ioError) {
        return;
    }

    if (!_logFile.isOpen()) {
        if (!qgcApp()->logOutput()) {
            _pendingDiskWrites.clear();
            return;
        }

        const QString saveDirPath = SettingsManager::instance()->appSettings()->crashSavePath();
        const QDir saveDir(saveDirPath);
        const QString saveFilePath = saveDir.absoluteFilePath("QGCConsole.log");

        _logFile.setFileName(saveFilePath);
        if (!_logFile.open(QIODevice::WriteOnly | QIODevice::Append | QIODevice::Text)) {
            _ioError = true;
            qgcApp()->showAppMessage(tr("Open console log output file failed %1 : %2").arg(_logFile.fileName(), _logFile.errorString()));
            return;
        }
    }

    if (_logFile.size() >= kMaxLogFileSize) {
        _rotateLogs();
    }

    QTextStream out(&_logFile);
    for (const QString &line : std::as_const(_pendingDiskWrites)) {
        out << line << '\n';
        if (out.status() != QTextStream::Ok) {
            _ioError = true;
            qCWarning(QGCLoggingLog) << "Error writing to log file:" << _logFile.errorString();
            break;
        }
    }
    (void) _logFile.flush();
    _pendingDiskWrites.clear();
}

void QGCLogging::writeMessages(const QString &destFile)
{
    const QStringList logs = stringList();

    (void) QtConcurrent::run([this, destFile, logs]() {
        emit writeStarted();
        bool success = false;
        QSaveFile file(destFile);
        if (file.open(QIODevice::WriteOnly | QIODevice::Text)) {
            QTextStream out(&file);
            for (const QString &line : logs) {
                out << line << '\n';
            }
            success = ((out.status() == QTextStream::Ok) && file.commit());
        } else {
            qCWarning(QGCLoggingLog) << "write failed:" << file.errorString();
        }
        emit writeFinished(success);
    });
}
