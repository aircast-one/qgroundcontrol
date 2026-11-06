#pragma once

#include <QtCore/QByteArray>
#include <QtCore/QString>
#include <QtCore/QUrl>
#include <optional>

namespace AppUpdateOffer {

constexpr auto kLatestReleaseUrl = "https://api.github.com/repos/aircast-one/qgroundcontrol/releases/latest";
constexpr auto kReleasesPage = "https://github.com/aircast-one/qgroundcontrol/releases";

struct Offer
{
    QString version;
    QUrl download;
    QUrl page;
    bool operator==(const Offer& other) const = default;
};

QString assetSuffix(const QString& productType, const QString& kernelType, const QString& arch);
std::optional<Offer> parse(const QByteArray& releaseJson, const QString& productType, const QString& kernelType,
                           const QString& arch);
bool newer(const QString& current, const QString& latest);
QString text(const QString& appName, const QString& current, const Offer& offer, bool downloaded);

}  // namespace AppUpdateOffer
