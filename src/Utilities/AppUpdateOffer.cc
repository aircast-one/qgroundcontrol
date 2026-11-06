#include "AppUpdateOffer.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QRegularExpression>
#include <algorithm>
#include <ranges>
#include <tuple>

namespace AppUpdateOffer {

namespace {

std::optional<std::tuple<int, int, int>> numbers(const QString& version)
{
    static const QRegularExpression pattern(QStringLiteral("v(\\d+)\\.(\\d+)\\.(\\d+)"));
    const QRegularExpressionMatch match = pattern.match(version);
    if (!match.hasMatch()) {
        return std::nullopt;
    }
    return std::make_tuple(match.captured(1).toInt(), match.captured(2).toInt(), match.captured(3).toInt());
}

}  // namespace

QString assetSuffix(const QString& productType, const QString& kernelType, const QString& arch)
{
    const bool arm = arch.startsWith(QStringLiteral("arm"), Qt::CaseInsensitive);
    return productType == QLatin1String("ios")       ? QString()
           : productType == QLatin1String("android") ? QStringLiteral(".apk")
           : kernelType == QLatin1String("winnt")
               ? (arm ? QStringLiteral("-installer-arm64.exe") : QStringLiteral("-installer.exe"))
           : kernelType == QLatin1String("darwin") ? QStringLiteral(".dmg")
           : kernelType == QLatin1String("linux")  ? QStringLiteral("-x86_64.AppImage")
                                                   : QString();
}

std::optional<Offer> parse(const QByteArray& releaseJson, const QString& productType, const QString& kernelType,
                           const QString& arch)
{
    const QJsonObject release = QJsonDocument::fromJson(releaseJson).object();
    const QString tag = release.value(QStringLiteral("tag_name")).toString();
    if (!numbers(tag) || release.value(QStringLiteral("prerelease")).toBool() ||
        release.value(QStringLiteral("draft")).toBool()) {
        return std::nullopt;
    }
    const QUrl page(release.value(QStringLiteral("html_url")).toString(QString::fromLatin1(kReleasesPage)));
    const QString suffix = assetSuffix(productType, kernelType, arch);
    const QJsonArray assets = release.value(QStringLiteral("assets")).toArray();
    const auto wanted = std::ranges::find_if(assets, [&suffix](const QJsonValue& asset) {
        return !suffix.isEmpty() && asset.toObject().value(QStringLiteral("name")).toString().endsWith(suffix);
    });
    const QUrl download = wanted == assets.cend()
                              ? page
                              : QUrl(wanted->toObject().value(QStringLiteral("browser_download_url")).toString());
    return Offer{tag.startsWith(QStringLiteral("aircast-")) ? tag.mid(8) : tag, download, page};
}

bool newer(const QString& current, const QString& latest)
{
    const auto here = numbers(current);
    const auto there = numbers(latest);
    return here && there && *here < *there;
}

QString text(const QString& appName, const QString& current, const Offer& offer)
{
    const auto here = numbers(current);
    const QString running =
        here ? QStringLiteral("v%1.%2.%3").arg(std::get<0>(*here)).arg(std::get<1>(*here)).arg(std::get<2>(*here))
             : current;
    return QCoreApplication::translate("AppUpdateOffer",
                                       "%1 %2 is available. You are running %3.\n\nOK opens the download in your "
                                       "browser; install it the same way as the first time.")
        .arg(appName, offer.version, running);
}

}  // namespace AppUpdateOffer
