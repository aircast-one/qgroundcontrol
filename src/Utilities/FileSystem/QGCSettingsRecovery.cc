#include "QGCSettingsRecovery.h"
#include "QGCLoggingCategory.h"

#include <QtCore/QFile>
#include <QtCore/QSettings>

QGC_LOGGING_CATEGORY(QGCSettingsRecoveryLog, "qgc.utilities.filesystem.qgcsettingsrecovery")

bool QGCSettingsRecovery::moveAsideIfUnwritable(QSettings &settings)
{
    const QString fileName = settings.fileName();
    if (settings.isWritable() || !QFile::exists(fileName)) {
        return false;
    }
    QVariantMap kept;
    for (const QString &key : settings.allKeys()) {
        kept.insert(key, settings.value(key));
    }
    const QString aside = fileName + QStringLiteral(".unwritable");
    QFile::remove(aside);
    if (!QFile::rename(fileName, aside)) {
        qCWarning(QGCSettingsRecoveryLog) << "Settings file is not writable and could not be moved aside" << fileName;
        return false;
    }
    qCWarning(QGCSettingsRecoveryLog) << "Settings file was not writable, moved aside and recreated" << fileName << "kept" << kept.size() << "keys";
    for (auto it = kept.cbegin(); it != kept.cend(); ++it) {
        settings.setValue(it.key(), it.value());
    }
    settings.sync();
    return true;
}
