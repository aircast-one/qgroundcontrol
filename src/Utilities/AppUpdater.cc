#include "AppUpdater.h"

#include <QtCore/QApplicationStatic>
#include <QtCore/QCoreApplication>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QFileInfo>
#include <QtCore/QLatin1StringView>
#include <QtCore/QProcess>
#include <QtCore/QStandardPaths>
#include <QtCore/QSysInfo>
#include <QtCore/QUrl>
#include <QtGui/QDesktopServices>
#ifdef Q_OS_ANDROID
#include <QtCore/QJniObject>
#endif

#include "AppUpdateInstall.h"
#include "MultiVehicleManager.h"
#include "QGCCorePlugin.h"
#include "QGCFileDownload.h"
#include "QGCFileHelper.h"
#include "QGCLoggingCategory.h"

QGC_LOGGING_CATEGORY(AppUpdaterLog, "Utilities.AppUpdater")

Q_APPLICATION_STATIC(AppUpdater, _appUpdater);

namespace {

constexpr QLatin1StringView kReady("ready");
constexpr QLatin1StringView kIdle("idle");
constexpr QLatin1StringView kChecking("checking");
constexpr QLatin1StringView kDownloading("downloading");
constexpr QLatin1StringView kFailed("failed");
constexpr QLatin1StringView kInstalling("installing");

QString listingUrl(const QUrl& download)
{
    return download.toString() + QStringLiteral(".sha256");
}

}  // namespace

AppUpdater::AppUpdater(QObject* parent) : QObject(parent)
{
    _timer.setInterval(kFirstCheckAfterMs);
    _timer.setSingleShot(true);
    (void) connect(&_timer, &QTimer::timeout, this, &AppUpdater::_tick);
    _offerTimer.setInterval(kOfferAgainAfterMs);
    (void) connect(&_offerTimer, &QTimer::timeout, this, &AppUpdater::_offerIfFree);
}

AppUpdater* AppUpdater::instance()
{
    return _appUpdater();
}

void AppUpdater::start()
{
    _timer.start();
}

bool AppUpdater::_vehicleConnected()
{
    return MultiVehicleManager::instance() && MultiVehicleManager::instance()->activeVehicle();
}

QString AppUpdater::_updatesDir() const
{
    const QString dir = QStandardPaths::writableLocation(QStandardPaths::AppDataLocation) + QStringLiteral("/Updates");
    (void) QDir().mkpath(dir);
    return dir;
}

void AppUpdater::_setState(const QString& state, const QString& error)
{
    if (state == _state && error == _error) {
        return;
    }
    _state = state;
    _error = error;
    emit stateChanged();
}

void AppUpdater::_tick()
{
    _timer.setInterval(kCheckEveryMs);
    _timer.start();
    if (!_vehicleConnected()) {
        check();
    }
}

QGCFileDownload* AppUpdater::_fetch(const QString& url, void (AppUpdater::*done)(bool, const QString&, const QString&))
{
    QGCFileDownload* const download = new QGCFileDownload(this);
    (void) connect(download, &QGCFileDownload::finished, this, done);
    (void) connect(download, &QGCFileDownload::finished, download, &QObject::deleteLater);
    if (!download->start(url)) {
        qCWarning(AppUpdaterLog) << "Could not start" << url << download->errorString();
        download->deleteLater();
        return nullptr;
    }
    return download;
}

void AppUpdater::check()
{
    if (_checking || _state == kDownloading || _state == kInstalling) {
        return;
    }
    if (_state == kReady) {
        _offerIfFree();
        return;
    }
    _checking = true;
    _setState(QString(kChecking));
    if (!_fetch(QGCCorePlugin::instance()->stableVersionCheckFileUrl(), &AppUpdater::_onCheckFinished)) {
        _checking = false;
        _setState(QString(kFailed), tr("Could not reach the release server."));
    }
}

void AppUpdater::_onCheckFinished(bool success, const QString& localFile, const QString& errorMsg)
{
    _checking = false;
    QFile answer(localFile);
    if (!success || !answer.open(QIODevice::ReadOnly)) {
        _setState(QString(kFailed), errorMsg);
        return;
    }
    const std::optional<AppUpdateOffer::Offer> offer = AppUpdateOffer::parse(
        answer.readAll(), QSysInfo::productType(), QSysInfo::kernelType(), QSysInfo::currentCpuArchitecture());
    qCDebug(AppUpdaterLog) << "Latest stable release" << (offer ? offer->version : QStringLiteral("unknown"));
    if (!offer || !AppUpdateOffer::newer(QCoreApplication::applicationVersion(), offer->version)) {
        _offer.reset();
        _setState(QString(kIdle));
        return;
    }
    _offer = offer;
    if (offer->download == offer->page) {
        _file.clear();
        _setState(QString(kReady));
        _offerIfFree();
        return;
    }
    _setState(QString(kDownloading));
    if (!_fetch(listingUrl(offer->download), &AppUpdater::_onListingFinished)) {
        _setState(QString(kFailed), tr("Could not fetch the checksum."));
    }
}

void AppUpdater::_onListingFinished(bool success, const QString& localFile, const QString& errorMsg)
{
    QFile listing(localFile);
    const QString sha = success && listing.open(QIODevice::ReadOnly)
                            ? AppUpdateInstall::sha256FromListing(listing.readAll())
                            : QString();
    if (sha.isEmpty() || !_offer) {
        _setState(QString(kFailed), errorMsg.isEmpty() ? tr("The release has no checksum.") : errorMsg);
        return;
    }
    _sha256 = sha;
    const QString target = _updatesDir() + QLatin1Char('/') + _offer->download.fileName();
    if (QFile::exists(target) && QGCFileHelper::computeFileHash(target).compare(sha, Qt::CaseInsensitive) == 0) {
        _file = target;
        _setState(QString(kReady));
        _offerIfFree();
        return;
    }
    QGCFileDownload* const download = new QGCFileDownload(this);
    download->setOutputPath(target);
    download->setExpectedHash(sha);
    (void) connect(download, &QGCFileDownload::finished, this, &AppUpdater::_onDownloadFinished);
    (void) connect(download, &QGCFileDownload::finished, download, &QObject::deleteLater);
    if (!download->start(_offer->download.toString())) {
        _setState(QString(kFailed), download->errorString());
        download->deleteLater();
    }
}

void AppUpdater::_onDownloadFinished(bool success, const QString& localFile, const QString& errorMsg)
{
    if (!success) {
        (void) QFile::remove(localFile);
        _setState(QString(kFailed), errorMsg);
        return;
    }
    _file = localFile;
    qCInfo(AppUpdaterLog) << "Downloaded" << _offer->version << "to" << localFile;
    _setState(QString(kReady));
    _offerIfFree();
}

void AppUpdater::_offerIfFree()
{
    if (_state != kReady || !_offer) {
        _offerTimer.stop();
        return;
    }
    if (_vehicleConnected()) {
        _offerTimer.start();
        return;
    }
    _offerTimer.stop();
    if (_askedFor == _offer->version) {
        return;
    }
    _askedFor = _offer->version;
    emit readyToInstall(AppUpdateOffer::text(QCoreApplication::applicationName(),
                                             QCoreApplication::applicationVersion(), *_offer, !_file.isEmpty()));
}

void AppUpdater::later()
{
    _askedFor.clear();
    _offerTimer.stop();
}

bool AppUpdater::install()
{
    if (_state != kReady || !_offer || _vehicleConnected()) {
        return false;
    }
    if (_file.isEmpty()) {
        return QDesktopServices::openUrl(_offer->download);
    }
    const AppUpdateInstall::Launch plan =
        AppUpdateInstall::launch(QSysInfo::kernelType(), QSysInfo::productType(), _file,
                                 AppUpdateInstall::bundleOf(QCoreApplication::applicationDirPath()),
                                 qEnvironmentVariable("APPIMAGE"), QCoreApplication::applicationPid());
    qCInfo(AppUpdaterLog) << "Installing" << _offer->version << "by" << static_cast<int>(plan.way);
    switch (plan.way) {
        case AppUpdateInstall::Way::HandToAndroid:
#ifdef Q_OS_ANDROID
            QJniObject::callStaticMethod<void>("org/mavlink/qgroundcontrol/QGCActivity", "installApk",
                                               "(Ljava/lang/String;)V", QJniObject::fromString(_file).object());
            return true;
#else
            return QDesktopServices::openUrl(QUrl::fromLocalFile(_file));
#endif
        case AppUpdateInstall::Way::OpenFile:
            return QDesktopServices::openUrl(QUrl::fromLocalFile(_file));
        case AppUpdateInstall::Way::RunInstaller:
        case AppUpdateInstall::Way::ReplaceBundle:
        case AppUpdateInstall::Way::ReplaceAppImage:
            if (!QProcess::startDetached(plan.program, plan.arguments)) {
                _setState(QString(kFailed), tr("Could not start the installer."));
                return false;
            }
            _setState(QString(kInstalling));
            QCoreApplication::quit();
            return true;
    }
    return false;
}
