#pragma once

#include <QtCore/QCoreApplication>
#include <QtCore/QString>
#include <QtCore/QVariantMap>

class QGCFetchTileTask;

/// Disk/database side of the map tile cache. Free of QtLocation so a headless build that
/// draws its own map (MapKit on macOS) reaches the same cache the QML map plugin uses.
class QGCTileCache
{
    Q_DECLARE_TR_FUNCTIONS(QGCTileCache)

public:
    static quint32 getMaxDiskCacheSetting();
    static void cacheTile(const QString &type, int x, int y, int z, const QByteArray &image, const QString &format, qulonglong set = UINT64_MAX);
    static void cacheTile(const QString &type, const QString &hash, const QByteArray &image, const QString &format, qulonglong set = UINT64_MAX);
    static QGCFetchTileTask *createFetchTileTask(const QString &type, int x, int y, int z);

    /// Resolves the cache directory and database path on first call.
    static void ensureInitialized();
    static QString getDatabaseFilePath();
    static QString getCachePath();

    /// Directory the QtLocation file tile cache stores its own copies in.
    static QString cachePathForParameters(const QVariantMap &parameters);

private:
    static void _initCache();
    static bool _wipeDirectory(const QString &dirPath);
    static void _wipeOldCaches();

    static QString _databaseFilePath;
    static QString _cachePath;
    static std::atomic<bool> _cacheWasReset;
};
