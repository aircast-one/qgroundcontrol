#pragma once

#include <QtCore/QString>
#include <QtCore/QStringList>
#include <optional>

namespace AppUpdateInstall {

enum class Way
{
    RunInstaller,
    ReplaceBundle,
    ReplaceAppImage,
    HandToAndroid,
    OpenFile
};

struct Launch
{
    Way way = Way::OpenFile;
    QString program;
    QStringList arguments;
    bool operator==(const Launch& other) const = default;
};

QString sha256FromListing(const QByteArray& listing);
QString bundleOf(const QString& applicationDirPath);
Launch launch(const QString& kernelType, const QString& productType, const QString& file, const QString& bundle,
              const QString& appImage, qint64 pid);

}  // namespace AppUpdateInstall
