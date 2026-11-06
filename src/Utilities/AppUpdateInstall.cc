#include "AppUpdateInstall.h"

#include <QtCore/QDir>
#include <QtCore/QFileInfo>
#include <QtCore/QRegularExpression>

namespace AppUpdateInstall {

namespace {

QString quoted(const QString& path)
{
    return QStringLiteral("'") + QString(path).replace(QLatin1Char('\''), QStringLiteral("'\\''")) +
           QStringLiteral("'");
}

QString waitForExit(qint64 pid)
{
    return QStringLiteral("while kill -0 %1 2>/dev/null; do sleep 0.5; done; ").arg(pid);
}

}  // namespace

QString sha256FromListing(const QByteArray& listing)
{
    static const QRegularExpression hex(QStringLiteral("^\\s*([0-9a-fA-F]{64})\\b"));
    const QRegularExpressionMatch match = hex.match(QString::fromLatin1(listing));
    return match.hasMatch() ? match.captured(1).toLower() : QString();
}

QString bundleOf(const QString& applicationDirPath)
{
    const QDir bundle(applicationDirPath + QStringLiteral("/../.."));
    const QString path = bundle.absolutePath();
    const bool isBundle = path.endsWith(QStringLiteral(".app")) && QFileInfo(path).isDir();
    const bool writable = isBundle && QFileInfo(QFileInfo(path).absolutePath()).isWritable();
    return writable ? path : QString();
}

Launch launch(const QString& kernelType, const QString& productType, const QString& file, const QString& bundle,
              const QString& appImage, qint64 pid)
{
    if (productType == QLatin1String("android")) {
        return Launch{Way::HandToAndroid, file, {}};
    }
    if (kernelType == QLatin1String("winnt")) {
        return Launch{Way::RunInstaller, file, {QStringLiteral("/S")}};
    }
    if (kernelType == QLatin1String("darwin") && !bundle.isEmpty()) {
        const QString script =
            waitForExit(pid) + QStringLiteral(
                                   "M=$(mktemp -d) && hdiutil attach -nobrowse -quiet -mountpoint \"$M\" %1 && "
                                   "APP=$(ls -d \"$M\"/*.app | head -n 1) && rm -rf %2 && ditto \"$APP\" %2; "
                                   "hdiutil detach \"$M\" -quiet; open -n %2")
                                   .arg(quoted(file), quoted(bundle));
        return Launch{Way::ReplaceBundle, QStringLiteral("/bin/sh"), {QStringLiteral("-c"), script}};
    }
    if (kernelType == QLatin1String("linux") && !appImage.isEmpty()) {
        const QString script =
            waitForExit(pid) + QStringLiteral("cp %1 %2.new && chmod +x %2.new && mv -f %2.new %2 && exec %2")
                                   .arg(quoted(file), quoted(appImage));
        return Launch{Way::ReplaceAppImage, QStringLiteral("/bin/sh"), {QStringLiteral("-c"), script}};
    }
    return Launch{Way::OpenFile, file, {}};
}

}  // namespace AppUpdateInstall
