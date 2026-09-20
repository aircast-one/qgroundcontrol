#include "VideoManager.h"
#include "AppSettings.h"
#include "MavlinkCameraControlInterface.h"
#include "MultiVehicleManager.h"
#include "AppMessages.h"
#include "QGCApplication.h"
#include "QGCCameraManager.h"
#include "QGCCorePlugin.h"
#include "QGCLoggingCategory.h"
#include "QGCVideoStreamInfo.h"
#include "SettingsManager.h"
#include "SubtitleWriter.h"
#include "Vehicle.h"
#include "VehicleLinkManager.h"
#include "VideoReceiver.h"
#include "VideoSettings.h"
#ifndef QGC_HEADLESS_CORE
#include "UVCReceiver.h"
#endif
#include "VideoBackend.h"

#include <algorithm>
#include <climits>

#include <QtConcurrent/QtConcurrent>
#include <QtCore/QApplicationStatic>
#include <QtCore/QCoreApplication>
#include <QtCore/QDir>
#include <QtCore/QEventLoop>
#include <QtCore/QFutureWatcher>
#include <QtCore/QPointer>
#include <QtCore/QRunnable>
#include <QtCore/QTimer>
#include <QtCore/QUrl>
#ifndef QGC_HEADLESS_CORE
#include <QtQml/QQmlEngine>
#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickWindow>
#endif

QGC_LOGGING_CATEGORY(VideoManagerLog, "Video.VideoManager")

namespace {
constexpr uint32_t kSwitchStallHoldSeconds = 20;
constexpr int kEncoderHandoffMs = 500;
}

static constexpr const char *kMainReceiverName = "videoContent";

static constexpr const char *kFileExtension[VideoReceiver::FILE_FORMAT_MAX + 1] = {
    "mkv",
    "mov",
    "mp4"
};

Q_APPLICATION_STATIC(VideoManager, _videoManagerInstance);

VideoManager::VideoManager(QObject *parent)
    : QObject(parent)
    , _subtitleWriter(new SubtitleWriter(this))
    , _videoSettings(SettingsManager::instance()->videoSettings())
{
    qCDebug(VideoManagerLog) << this;

    (void) qRegisterMetaType<VideoReceiver::STATUS>("STATUS");

    if (VideoBackend::needsAsyncInit()) {
        _backendDisabledForTests = VideoBackend::disabledForUnitTests();
        if (_backendDisabledForTests) {
            qCInfo(VideoManagerLog) << "Skipping video backend initialization for unit tests";
        }
    }
}

VideoManager::~VideoManager()
{
    qCDebug(VideoManagerLog) << this;
}

VideoManager *VideoManager::instance()
{
    return _videoManagerInstance();
}

void VideoManager::startVideoBackendInit()
{
    if (!VideoBackend::needsAsyncInit()) return;

    if (_backendDisabledForTests) {
        _initState.store(InitState::BackendReady);
        qCInfo(VideoManagerLog) << "video initialization disabled for unit tests";
        return;
    }

    // CAS-gate NotStarted -> Pending: init() (GUI thread) and waitForVideoBackendReady() (other threads)
    // both enter here; without it both launch VideoBackend::initialize() -> double-init SIGABRT.
    // The mutex holds back waiters until _backendInitFuture is assigned.
    QMutexLocker lock(&_initFutureMutex);
    InitState expected = InitState::NotStarted;
    if (!_initState.compare_exchange_strong(expected, InitState::Pending)) {
        qCWarning(VideoManagerLog) << "video init already started";
        return;
    }

    const VideoBackend::EnvPrepResult envResult = VideoBackend::prepareEnvironment();
    // Snapshot argv + env result here on the GUI thread; QCoreApplication::arguments() is not thread-safe.
    _backendInitFuture = QtConcurrent::run(&VideoBackend::initialize, QCoreApplication::arguments(), envResult);

    _backendInitFuture.then(this, [this](bool success) {
        _onBackendInitComplete(success);
    }).onCanceled(this, [this] {
        _onBackendInitComplete(false);
    });
}

bool VideoManager::waitForVideoBackendReady(std::chrono::milliseconds timeout)
{
    if (!VideoBackend::needsAsyncInit()) return true;

    if (_backendDisabledForTests) {
        return true;
    }

    if (_initState.load() == InitState::NotStarted) {
        startVideoBackendInit();
    }

    switch (_initState.load()) {
    case InitState::Failed:
        return false;
    case InitState::BackendReady:
    case InitState::Running:
        return true;
    case InitState::NotStarted:
    case InitState::Pending:
    case InitState::QmlReady:
        break;
    }

    QFuture<bool> future;
    {
        QMutexLocker lock(&_initFutureMutex);
        future = _backendInitFuture;
    }
    if (!future.isValid()) {
        qCCritical(VideoManagerLog) << "waitForVideoBackendReady: no valid future";
        return false;
    }

    QEventLoop loop;
    QTimer timer;
    timer.setSingleShot(true);
    QFutureWatcher<bool> watcher;
    (void) connect(&watcher, &QFutureWatcher<bool>::finished, &loop, &QEventLoop::quit);
    (void) connect(&timer, &QTimer::timeout, &loop, &QEventLoop::quit);

    watcher.setFuture(future);
    if (!watcher.isFinished()) {
        timer.start(timeout);
        loop.exec();
    }

    if (!watcher.isFinished()) {
        qCCritical(VideoManagerLog) << "Timed out waiting for video init";
        return false;
    }

    const bool success = watcher.result();
    if (_initState.load() == InitState::Pending || _initState.load() == InitState::QmlReady) {
        _onBackendInitComplete(success);
    }
    return _initState.load() != InitState::Failed;
}

#ifdef QGC_HEADLESS_CORE
void VideoManager::init()
{
    if (_initialized) {
        qCDebug(VideoManagerLog) << "Video Manager already initialized";
        return;
    }
#else
void VideoManager::init(QQuickWindow *mainWindow)
{
    if (_initialized) {
        qCDebug(VideoManagerLog) << "Video Manager already initialized";
        return;
    }

    if (!mainWindow && !_nativeRendering) {
        qCCritical(VideoManagerLog) << "Failed To Init Video Manager - window is NULL";
        return;
    }
    _mainWindow = mainWindow;

    if (mainWindow) {
        VideoBackend::onMainWindowReady(mainWindow);
    }
#endif

    (void) connect(_videoSettings->videoSource(), &Fact::rawValueChanged, this, &VideoManager::_videoSourceChanged);
    (void) connect(_videoSettings->udpUrl(), &Fact::rawValueChanged, this, &VideoManager::_videoSourceChanged);
    (void) connect(_videoSettings->rtspUrl(), &Fact::rawValueChanged, this, &VideoManager::_videoSourceChanged);
    (void) connect(_videoSettings->tcpUrl(), &Fact::rawValueChanged, this, &VideoManager::_videoSourceChanged);
    (void) connect(_videoSettings->whepUrl(), &Fact::rawValueChanged, this, &VideoManager::_videoSourceChanged);
    (void) connect(_videoSettings->extraVideoSources(), &Fact::rawValueChanged, this, &VideoManager::_videoSourceChanged);
    (void) connect(_videoSettings->extraVideoSources(), &Fact::rawValueChanged, this, &VideoManager::activeVideoSourceChanged);
    (void) connect(_videoSettings->activeVideoSource(), &Fact::rawValueChanged, this, &VideoManager::_videoSourceChanged);
    (void) connect(_videoSettings->activeVideoSource(), &Fact::rawValueChanged, this, &VideoManager::activeVideoSourceChanged);
    (void) connect(_videoSettings->activeVideoSource(), &Fact::rawValueChanged, this, &VideoManager::_holdStallRestartWhileSwitching);
    (void) connect(_videoSettings->multiViewEnabled(), &Fact::rawValueChanged, this, &VideoManager::_videoSourceChanged);
    (void) connect(_videoSettings->multiViewEnabled(), &Fact::rawValueChanged, this, &VideoManager::activeVideoSourceChanged);
    (void) connect(_videoSettings->primaryCameraName(), &Fact::rawValueChanged, this, &VideoManager::activeVideoSourceChanged);
    (void) connect(this, &VideoManager::activeVideoSourceChanged, this, &VideoManager::camerasChanged);
    (void) connect(_videoSettings->aspectRatio(), &Fact::rawValueChanged, this, &VideoManager::aspectRatioChanged);
    (void) connect(_videoSettings->lowLatencyMode(), &Fact::rawValueChanged, this, [this](const QVariant &value) { Q_UNUSED(value); _restartAllVideos(); });
    // rtpJitterLatencyMs needs a pipeline restart; route through _videoSourceChanged so _updateSettings
    // pushes the new value to each receiver and restarts exactly once (no double restart).
    (void) connect(_videoSettings->rtpJitterLatencyMs(), &Fact::rawValueChanged, this, [this](const QVariant &value) { Q_UNUSED(value); _videoSourceChanged(); });
    // autoReconnect is a live setting — push without restart so an in-flight reconnect
    // can be cancelled mid-backoff.
    (void) connect(_videoSettings->rtspAutoReconnect(), &Fact::rawValueChanged, this, [this](const QVariant &value) {
        const bool enabled = value.toBool();
        for (VideoReceiver *receiver : std::as_const(_videoReceivers)) {
            receiver->setAutoReconnect(enabled);
        }
    });
    VideoBackend::bindDebugLevelFact(SettingsManager::instance()->appSettings()->gstDebugLevel(), this);
    (void) connect(MultiVehicleManager::instance(), &MultiVehicleManager::activeVehicleChanged, this, &VideoManager::_setActiveVehicle);

    (void) connect(this, &VideoManager::autoStreamConfiguredChanged, this, &VideoManager::_videoSourceChanged);

    if (VideoBackend::needsAsyncInit() && _initState.load() == InitState::NotStarted) {
        startVideoBackendInit();
    }

#ifndef QGC_HEADLESS_CORE
    if (_mainWindow) {
        _mainWindow->scheduleRenderJob(
            QRunnable::create([this] {
                QMetaObject::invokeMethod(this, &VideoManager::_initAfterQmlIsReady, Qt::QueuedConnection);
            }),
            QQuickWindow::AfterSynchronizingStage);
    } else
#endif
    {
        QMetaObject::invokeMethod(this, &VideoManager::_initAfterQmlIsReady, Qt::QueuedConnection);
    }

    _initialized = true;
}

void VideoManager::_initAfterQmlIsReady()
{
#ifndef QGC_HEADLESS_CORE
    if (!_mainWindow && !_nativeRendering) {
        qCCritical(VideoManagerLog) << "_initAfterQmlIsReady called with NULL mainWindow";
        return;
    }
#endif

    qCDebug(VideoManagerLog) << "_initAfterQmlIsReady";

    if (VideoBackend::needsAsyncInit()) {
        switch (_initState.load()) {
        case InitState::Pending:
            _initState.store(InitState::QmlReady);
            qCDebug(VideoManagerLog) << "QML ready, waiting for video";
            return;
        case InitState::BackendReady:
            _initState.store(InitState::Running);
            qCDebug(VideoManagerLog) << "QML ready, video already done — creating receivers";
            break;
        case InitState::Failed:
            qCWarning(VideoManagerLog) << "QML ready but video init failed";
            return;
        case InitState::NotStarted:
        case InitState::QmlReady:
        case InitState::Running:
            qCWarning(VideoManagerLog) << "_initAfterQmlIsReady: unexpected state" << static_cast<int>(_initState.load());
            return;
        }
    }
    _createVideoReceivers();
}

void VideoManager::_onBackendInitComplete(bool success)
{
    if (!success) {
        _initState.store(InitState::Failed);
        qCCritical(VideoManagerLog) << "video initialization failed";
        return;
    }

    if (VideoBackend::needsAsyncInit() && _videoSettings) {
        _videoSettings->pruneUnavailableDecoders();
        VideoBackend::applyDecoderPriorities(_videoSettings->forceVideoDecoder()->rawValue().toInt());
    }

    switch (_initState.load()) {
    case InitState::Pending:
        _initState.store(InitState::BackendReady);
        qCDebug(VideoManagerLog) << "video ready, waiting for QML";
        return;
    case InitState::QmlReady:
        _initState.store(InitState::Running);
        qCDebug(VideoManagerLog) << "video ready, QML already done — creating receivers";
        _createVideoReceivers();
        return;
    default:
        qCWarning(VideoManagerLog) << "_onBackendInitComplete: unexpected state" << static_cast<int>(_initState.load());
        return;
    }
}

void VideoManager::_createVideoReceivers()
{
#ifdef QGC_UNITTEST_BUILD
    if (_createVideoReceiversForTest) {
        _createVideoReceiversForTest();
        return;
    }
#endif
    QStringList videoStreamList = {
        kMainReceiverName,
        "thermalVideo"
    };
    for (int i = 0; i < kMaxVideoTiles; ++i) {
        videoStreamList.append(_tileReceiverName(i));
    }

#ifndef QGC_HEADLESS_CORE
    _mainWidget = _mainWindow ? _mainWindow->findChild<QQuickItem*>(QLatin1String(kMainReceiverName)) : nullptr;
    if (!_mainWidget && _mainWindow) {
        qCCritical(VideoManagerLog) << "main video widget not found";
    }
#endif

    QStringList existing;
    existing.reserve(_videoReceivers.size());
    for (const VideoReceiver *receiver : std::as_const(_videoReceivers)) {
        existing.append(receiver->name());
    }

    for (const QString &streamName : videoStreamList) {
        // Skip only names that already initialized; a once-failed receiver was removed from the
        // list, so re-entry retries it instead of being blocked by an all-or-nothing guard.
        if (existing.contains(streamName)) {
            continue;
        }
        VideoReceiver *receiver = QGCCorePlugin::instance()->createVideoReceiver(this);
        if (!receiver) {
            continue;
        }
        receiver->setName(streamName);

#ifdef QGC_HEADLESS_CORE
        _initVideoReceiver(receiver);
#else
        _initVideoReceiver(receiver, _mainWindow);
#endif
    }

    _rebindWidgets();
}

void VideoManager::cleanup()
{
    for (VideoReceiver *receiver : std::as_const(_videoReceivers)) {
        QGCCorePlugin::instance()->releaseVideoSink(receiver->sink());
    }
}

void VideoManager::_cleanupOldVideos()
{
    if (!SettingsManager::instance()->videoSettings()->enableStorageLimit()->rawValue().toBool()) {
        return;
    }

    const QString savePath = SettingsManager::instance()->appSettings()->videoSavePath();
    QDir videoDir = QDir(savePath);
    videoDir.setFilter(QDir::Files | QDir::Readable | QDir::NoSymLinks | QDir::Writable);
    videoDir.setSorting(QDir::Time);

    QStringList nameFilters;
    for (size_t i = 0; i < std::size(kFileExtension); i++) {
        nameFilters << QStringLiteral("*.") + kFileExtension[i];
    }

    videoDir.setNameFilters(nameFilters);
    QFileInfoList vidList = videoDir.entryInfoList();
    if (vidList.isEmpty()) {
        return;
    }

    uint64_t total = 0;
    for (const QFileInfo &video : std::as_const(vidList)) {
        total += video.size();
    }

    const uint64_t maxSize = SettingsManager::instance()->videoSettings()->maxVideoSize()->rawValue().toUInt() * qPow(1024, 2);
    while ((total >= maxSize) && !vidList.isEmpty()) {
        const QFileInfo info = vidList.takeLast();
        total -= info.size();
        const QString path = info.filePath();
        qCDebug(VideoManagerLog) << "Removing old video file:" << path;
        (void) QFile::remove(path);
    }
}

void VideoManager::startRecording(const QString &videoFile)
{
    const VideoReceiver::FILE_FORMAT fileFormat = static_cast<VideoReceiver::FILE_FORMAT>(_videoSettings->recordingFormat()->rawValue().toInt());
    if (!VideoReceiver::isValidFileFormat(fileFormat)) {
        QGC::showAppMessage(tr("Invalid video format defined."));
        return;
    }

    _cleanupOldVideos();

    const QString savePath = SettingsManager::instance()->appSettings()->videoSavePath();
    if (savePath.isEmpty()) {
        QGC::showAppMessage(tr("Unabled to record video. Video save path must be specified in Settings."));
        return;
    }

    const QString videoFileUrl = videoFile.isEmpty() ? QDateTime::currentDateTime().toString("yyyy-MM-dd_hh.mm.ss") : videoFile;
    const QString ext = kFileExtension[fileFormat];

    const QString videoFileNameTemplate = savePath + "/" + videoFileUrl + ".%1" + ext;

    for (VideoReceiver *receiver : std::as_const(_videoReceivers)) {
        if (!receiver->started()) {
            qCDebug(VideoManagerLog) << "Video receiver is not ready.";
            continue;
        }
        const QString streamName = (receiver->name() == QStringLiteral("videoContent")) ? "" : (receiver->name() + ".");
        const QString videoFileName = videoFileNameTemplate.arg(streamName);
        receiver->startRecording(videoFileName, fileFormat);
    }
}

void VideoManager::stopRecording()
{
    for (VideoReceiver *receiver : std::as_const(_videoReceivers)) {
        receiver->stopRecording();
    }
}

void VideoManager::grabImage(const QString &imageFile)
{
    if (imageFile.isEmpty()) {
        _imageFile = SettingsManager::instance()->appSettings()->photoSavePath();
        _imageFile += QStringLiteral("/") + QDateTime::currentDateTime().toString("yyyy-MM-dd_hh.mm.ss.zzz") + QStringLiteral(".jpg");
    } else {
        _imageFile = imageFile;
    }

    emit imageFileChanged(_imageFile);

    for (VideoReceiver *receiver : std::as_const(_videoReceivers)) {
        receiver->takeScreenshot(_imageFile);
        // QSharedPointer<QQuickItemGrabResult> result = receiver->widget()->grabToImage(const QSize &targetSize = QSize())
    }
}

double VideoManager::aspectRatio() const
{
    // Live decoded resolution wins — set by VideoReceiver::videoSizeChanged once frames flow.
    if (!_videoSize.isEmpty()) {
        return static_cast<double>(_videoSize.width()) / _videoSize.height();
    }

    for (VideoReceiver *receiver : _videoReceivers) {
        QGCVideoStreamInfo *pInfo = receiver->videoStreamInfo();
        if (!receiver->isThermal() && pInfo && !pInfo->isThermal()) {
            return pInfo->aspectRatio();
        }
    }

    return _videoSettings->aspectRatio()->rawValue().toDouble();
}

double VideoManager::thermalAspectRatio() const
{
    for (VideoReceiver *receiver : _videoReceivers) {
        QGCVideoStreamInfo *pInfo = receiver->videoStreamInfo();
        if (receiver->isThermal() && pInfo && pInfo->isThermal()) {
            return pInfo->aspectRatio();
        }
    }

    return 1.0;
}

double VideoManager::hfov() const
{
    for (VideoReceiver *receiver : _videoReceivers) {
        QGCVideoStreamInfo *pInfo = receiver->videoStreamInfo();
        if (!receiver->isThermal() && pInfo && !pInfo->isThermal()) {
            return pInfo->hfov();
        }
    }

    return 1.0;
}

double VideoManager::thermalHfov() const
{
    for (VideoReceiver *receiver : _videoReceivers) {
        QGCVideoStreamInfo *pInfo = receiver->videoStreamInfo();
        if (receiver->isThermal() && pInfo && pInfo->isThermal()) {
            return pInfo->hfov();
        }
    }

    return _videoSettings->aspectRatio()->rawValue().toDouble();
}

bool VideoManager::hasThermal() const
{
    for (VideoReceiver *receiver : _videoReceivers) {
        QGCVideoStreamInfo *pInfo = receiver->videoStreamInfo();
        if (receiver->isThermal() && pInfo && pInfo->isThermal()) {
            return true;
        }
    }

    return false;
}

bool VideoManager::hasVideo() const
{
    return (_videoSettings->streamEnabled()->rawValue().toBool() && _videoSettings->streamConfigured());
}

bool VideoManager::isUvc() const
{
    return (!_uvcVideoSourceID.isEmpty() && uvcEnabled() && hasVideo());
}

bool VideoManager::gstreamerEnabled()
{
#ifdef QGC_GST_STREAMING
    return true;
#else
    return false;
#endif
}

bool VideoManager::uvcEnabled()
{
#ifdef QGC_HEADLESS_CORE
    return false;
#else
    return UVCReceiver::enabled();
#endif
}

bool VideoManager::qtmultimediaEnabled()
{
    return !gstreamerEnabled();
}

void VideoManager::setfullScreen(bool on)
{
    if (on && !hasVideo()) {
        on = false;
    }

    if (on != _fullScreen) {
        _fullScreen = on;
        emit fullScreenChanged();
    }
}

bool VideoManager::isStreamSource() const
{
    static const QStringList videoSourceList = {
        VideoSettings::videoSourceUDPH264,
        VideoSettings::videoSourceUDPH265,
        VideoSettings::videoSourceRTSP,
        VideoSettings::videoSourceTCP,
        VideoSettings::videoSourceMPEGTS,
        VideoSettings::videoSourceWebRTC,
        VideoSettings::videoSource3DRSolo,
        VideoSettings::videoSourceParrotDiscovery,
        VideoSettings::videoSourceYuneecMantisG,
        VideoSettings::videoSourceHerelinkAirUnit,
        VideoSettings::videoSourceHerelinkHotspot,
    };
    const QString videoSource = _videoSettings->currentVideoSourceName();
    return (videoSourceList.contains(videoSource) || autoStreamConfigured());
}

int VideoManager::activeVideoSource() const
{
    return _videoSettings->currentIndex();
}

bool VideoManager::hasMultipleVideoSources() const
{
    return _videoSettings->switchableIndices().size() > 1;
}

int VideoManager::videoSourceCount() const
{
    return _videoSettings->videoSourceCount();
}

QString VideoManager::activeSourceLabel() const
{
    return cameraName(activeVideoSource());
}

void VideoManager::setActiveVideoSource(int index)
{
    const int count = _videoSettings->videoSourceCount();
    const int clamped = qBound(0, index, count - 1);
    if (clamped == _videoSettings->activeVideoSource()->rawValue().toInt()) {
        return;
    }
    _videoSettings->activeVideoSource()->setRawValue(clamped);
}

void VideoManager::switchActiveVideoSource()
{
    const QList<int> indices = _videoSettings->switchableIndices();
    if (indices.size() <= 1) {
        return;
    }
    const int pos = indices.indexOf(activeVideoSource());
    const int next = indices.at(((pos < 0 ? 0 : pos) + 1) % indices.size());
    setActiveVideoSource(next);
}

int VideoManager::maxVideoTiles() const
{
    return kMaxVideoTiles;
}

int VideoManager::tileCameraNumber(int slot) const
{
    if (!_videoSettings->multiViewEnabled()->rawValue().toBool()) {
        return 0;
    }
    const QList<int> tiles = _videoSettings->tileCameraIndices();
    if (slot < 0 || slot >= tiles.size() || slot >= kMaxVideoTiles) {
        return 0;
    }
    return tiles.at(slot) + 1;
}

void VideoManager::promoteTile(int slot)
{
    const QList<int> tiles = _videoSettings->tileCameraIndices();
    if (slot >= 0 && slot < tiles.size()) {
        setActiveVideoSource(tiles.at(slot));
    }
}

QString VideoManager::_tileReceiverName(int slot)
{
    return QStringLiteral("extraVideo%1").arg(slot);
}

#ifndef QGC_HEADLESS_CORE
void VideoManager::registerTileItem(int slot, QQuickItem *item)
{
    _tileWidgets.insert(slot, item);
    _rebindWidgets();
}
#endif

QStringList VideoManager::cameraStatuses() const
{
    QStringList statuses;
    const int count = _videoSettings->videoSourceCount();
    statuses.reserve(count);
    for (int i = 0; i < count; ++i) {
        statuses.append(_cameraStatus(i));
    }
    return statuses;
}

QVariantList VideoManager::cameraConnecting() const
{
    QVariantList connecting;
    const int count = _videoSettings->videoSourceCount();
    connecting.reserve(count);
    for (int i = 0; i < count; ++i) {
        connecting.append(_cameraConnecting(i));
    }
    return connecting;
}

QVariantList VideoManager::cameraRecording() const
{
    QVariantList recording;
    const int count = _videoSettings->videoSourceCount();
    recording.reserve(count);
    for (int i = 0; i < count; ++i) {
        recording.append(_cameraRecording(i));
    }
    return recording;
}

bool VideoManager::_cameraRecording(int index) const
{
    for (VideoReceiver *receiver : _videoReceivers) {
        if (!receiver->isThermal() && (_cameraIndexForReceiver(receiver) == index)) {
            return _receiverState.value(receiver->name()).recording;
        }
    }
    return false;
}

bool VideoManager::_cameraConnecting(int index) const
{
    for (VideoReceiver *receiver : _videoReceivers) {
        if (receiver->isThermal() || (_cameraIndexForReceiver(receiver) != index)) {
            continue;
        }
        const ReceiverState state = _receiverState.value(receiver->name());
        if (state.decoding) {
            return false;
        }
        // Streaming without frames yet is still progress being made.
        return state.streaming || state.connecting;
    }
    return false;
}

QString VideoManager::_cameraStatus(int index) const
{
    for (VideoReceiver *receiver : _videoReceivers) {
        if (receiver->isThermal() || (_cameraIndexForReceiver(receiver) != index)) {
            continue;
        }
        const ReceiverState state = _receiverState.value(receiver->name());
        if (state.decoding) {
            return QString();
        }
        if (state.streaming) {
            return tr("Connected, waiting for frames");
        }
        return state.status.isEmpty() ? tr("Not started") : state.status;
    }
    return tr("No video source");
}

void VideoManager::_setReceiverStatus(VideoReceiver *receiver, const QString &status, bool connecting)
{
    if (receiver->isThermal()) {
        return;
    }
    ReceiverState &state = _receiverState[receiver->name()];
    if ((state.status == status) && (state.connecting == connecting)) {
        return;
    }
    state.status = status;
    state.connecting = connecting;
    emit camerasChanged();
}

quint64 VideoManager::cameraFramesDecoded(int index) const
{
    for (VideoReceiver *receiver : _videoReceivers) {
        if (!receiver->isThermal() && (_cameraIndexForReceiver(receiver) == index)) {
            return receiver->framesDecoded();
        }
    }
    return 0;
}

quint64 VideoManager::cameraBytesReceived(int index) const
{
    for (VideoReceiver *receiver : _videoReceivers) {
        if (!receiver->isThermal() && (_cameraIndexForReceiver(receiver) == index)) {
            return receiver->bytesReceived();
        }
    }
    return 0;
}

qint64 VideoManager::cameraSecondsSinceLastFrame(int index) const
{
    for (VideoReceiver *receiver : _videoReceivers) {
        if (!receiver->isThermal() && (_cameraIndexForReceiver(receiver) == index)) {
            const qint64 last = receiver->lastFrameSeconds();
            return (last > 0) ? (QDateTime::currentSecsSinceEpoch() - last) : -1;
        }
    }
    return -1;
}

QString VideoManager::cameraName(int index) const
{
    const QString name = _videoSettings->cameraName(index);
    return name.isEmpty() ? tr("Camera %1").arg(index + 1) : name;
}

int VideoManager::_cameraIndexForReceiver(const VideoReceiver *receiver) const
{
    const QString name = receiver->name();
    int cameraIndex = -1;
    if (name == QLatin1String(kMainReceiverName)) {
        cameraIndex = 0;
    } else {
        static const QString extraPrefix = QStringLiteral("extraVideo");
        if (name.startsWith(extraPrefix)) {
            cameraIndex = name.mid(extraPrefix.size()).toInt() + 1;
        }
    }
    if (cameraIndex < 0 || cameraIndex >= _videoSettings->videoSourceCount()) {
        return -1;
    }
    if ((cameraIndex != _videoSettings->currentIndex()) && !_videoSettings->multiViewEnabled()->rawValue().toBool()) {
        return -1;
    }
    return cameraIndex;
}

#ifndef QGC_HEADLESS_CORE
QQuickItem *VideoManager::_widgetForCamera(int cameraIndex) const
{
    if (cameraIndex < 0) {
        return nullptr;
    }
    if (cameraIndex == _videoSettings->currentIndex()) {
        return _mainWidget;
    }
    if (!_videoSettings->multiViewEnabled()->rawValue().toBool()) {
        return nullptr;
    }
    const int slot = _videoSettings->tileCameraIndices().indexOf(cameraIndex);
    if (slot < 0 || slot >= kMaxVideoTiles) {
        return nullptr;
    }
    return _tileWidgets.value(slot, nullptr);
}

#endif

void VideoManager::setNativeRendering(bool nativeRendering)
{
    if (_nativeRendering == nativeRendering) {
        return;
    }
    _nativeRendering = nativeRendering;
    _rebindWidgets();
}

void VideoManager::_rebindWidgets()
{
    for (VideoReceiver *receiver : std::as_const(_videoReceivers)) {
        if (receiver->isThermal()) {
            continue;
        }
        // A native head that has asked to render video itself must not also be handed a
        // QML item. On Android the QML fly view is still hosted, so init() finds its video
        // item by name and _widgetForCamera returns it — which silently kept the native
        // sink from ever being created, however the flag was set.
#ifndef QGC_HEADLESS_CORE
        QQuickItem *desired =
            _nativeRendering ? nullptr : _widgetForCamera(_cameraIndexForReceiver(receiver));
#else
        constexpr void *desired = nullptr;
#endif
        if (_nativeRendering && !desired && !receiver->sink()) {
            void *nativeSink = QGCCorePlugin::instance()->createNativeVideoSink(receiver);
            qCDebug(VideoManagerLog) << "native sink rebind" << receiver->name()
                                     << (nativeSink != nullptr) << "started:" << receiver->started();
            if (nativeSink) {
                receiver->setSink(nativeSink);
                if (receiver->started()) {
                    receiver->startDecoding(nativeSink);
                }
            }
            continue;
        }
#ifndef QGC_HEADLESS_CORE
        if (receiver->widget() == desired) {
            continue;
        }
        receiver->setWidget(desired);
        if (!desired) {
            continue;
        }
        if (!receiver->sink()) {
            void *sink = QGCCorePlugin::instance()->createVideoSink(desired, receiver);
            if (!sink) {
                qCCritical(VideoManagerLog) << "createVideoSink() failed" << receiver->name();
                continue;
            }
            receiver->setSink(sink);
            if (receiver->started()) {
                receiver->startDecoding(sink);
            }
        }
        VideoBackend::attachSink(receiver, receiver->sink(), desired);
#endif
    }
    _refreshActiveReceiverState();
}

void VideoManager::_refreshActiveReceiverState()
{
    const int active = _videoSettings->currentIndex();
    bool streaming = false;
    bool decoding = false;
    QSize size;
    for (VideoReceiver *receiver : std::as_const(_videoReceivers)) {
        if (receiver->isThermal() || (_cameraIndexForReceiver(receiver) != active)) {
            continue;
        }
        const ReceiverState state = _receiverState.value(receiver->name());
        streaming = state.streaming;
        decoding = state.decoding;
        size = state.videoSize;
        break;
    }
    if (_streaming != streaming) {
        _streaming = streaming;
        emit streamingChanged();
    }
    if (_decoding != decoding) {
        _decoding = decoding;
        emit decodingChanged();
    }
    if (size.isValid() && (_videoSize != size)) {
        _videoSize = size;
        emit videoSizeChanged();
        emit aspectRatioChanged();
    }
}

void VideoManager::_videoSourceChanged()
{
    QList<VideoReceiver*> changedReceivers;
    QGCCameraManager* camMgr = _activeVehicle ? _activeVehicle->cameraManager() : nullptr;
    for (VideoReceiver *receiver : std::as_const(_videoReceivers)) {
        QGCVideoStreamInfo* info = nullptr;
        if (camMgr) {
            info = receiver->isThermal() ? camMgr->thermalStreamInstance() : camMgr->currentStreamInstance();
        }
        receiver->setVideoStreamInfo(info);
        if (_updateSettings(receiver)) {
            changedReceivers.append(receiver);
        }
    }

    // hasVideo/isStreamSource/isUvc derive from the ACTIVE camera's source type; with pinned
    // URIs a switch changes no receiver settings, so re-emit unconditionally.
    emit hasVideoChanged();
    emit isStreamSourceChanged();
    emit isUvcChanged();
    emit isAutoStreamChanged();

    if (!changedReceivers.isEmpty()) {
        if (hasVideo()) {
            // A camera switch stops one receiver and starts another. Some air units serve a
            // single video encoder shared across their streams and will not feed a new stream
            // until the previous one is fully torn down. So stop first, then release the
            // incoming starts a short beat later.
            QList<VideoReceiver*> toStart;
            bool stoppedAny = false;
            for (VideoReceiver *receiver : std::as_const(changedReceivers)) {
                if (receiver->uri().isEmpty()) {
                    _restartVideo(receiver);
                    stoppedAny = true;
                } else if (receiver->started()) {
                    _restartVideo(receiver);
                } else {
                    toStart.append(receiver);
                }
            }
            for (VideoReceiver *receiver : std::as_const(toStart)) {
                if (stoppedAny) {
                    QPointer<VideoReceiver> guard(receiver);
                    QTimer::singleShot(kEncoderHandoffMs, this, [this, guard]() {
                        if (guard) {
                            _restartVideo(guard);
                        }
                    });
                } else {
                    _restartVideo(receiver);
                }
            }
        } else {
            stopVideo();
        }

        qCDebug(VideoManagerLog) << "New Video Source:" << _videoSettings->videoSource()->rawValue().toString();
    }

    _rebindWidgets();
}

bool VideoManager::_updateUVC(VideoReceiver * /*receiver*/)
{
    bool result = false;

    const QString oldUvcVideoSrcID = _uvcVideoSourceID;

#ifndef QGC_HEADLESS_CORE

    if (!UVCReceiver::enabled() || !hasVideo() || isStreamSource()) {
        _uvcVideoSourceID = QString();
    } else {
        _uvcVideoSourceID = UVCReceiver::getSourceId();
    }

#endif

    if (oldUvcVideoSrcID != _uvcVideoSourceID) {
        qCDebug(VideoManagerLog) << "UVC changed from [" << oldUvcVideoSrcID << "] to [" << _uvcVideoSourceID << "]";
#ifndef QGC_HEADLESS_CORE
        if (!_uvcVideoSourceID.isEmpty()) {
            UVCReceiver::checkPermission();
        }
#endif
        result = true;
        emit uvcVideoSourceIDChanged();
        emit isUvcChanged();
    }

    return result;
}

bool VideoManager::autoStreamConfigured() const
{
    for (VideoReceiver *receiver : _videoReceivers) {
        QGCVideoStreamInfo *pInfo = receiver->videoStreamInfo();
        if (!receiver->isThermal() && pInfo && !pInfo->isThermal()) {
            return !pInfo->uri().isEmpty();
        }
    }

    return false;
}

bool VideoManager::_updateAutoStream(VideoReceiver *receiver)
{
    const QGCVideoStreamInfo *pInfo = receiver->videoStreamInfo();
    if (!pInfo) {
        return false;
    }

    qCDebug(VideoManagerLog) << QString("Configure stream (%1):").arg(receiver->name()) << pInfo->uri();

    QString source, url;
    switch (pInfo->type()) {
    case VIDEO_STREAM_TYPE_RTSP:
        source = VideoSettings::videoSourceRTSP;
        url = pInfo->uri();
        if (source == VideoSettings::videoSourceRTSP) {
            _videoSettings->rtspUrl()->setRawValue(url);
        }
        break;
    case VIDEO_STREAM_TYPE_TCP_MPEG:
        source = VideoSettings::videoSourceTCP;
        url = pInfo->uri();
        break;
    case VIDEO_STREAM_TYPE_RTPUDP:
        if (pInfo->encoding() == VIDEO_STREAM_ENCODING_H265) {
            source = VideoSettings::videoSourceUDPH265;
            url = pInfo->uri().contains("udp265://") ? pInfo->uri() : QStringLiteral("udp265://0.0.0.0:%1").arg(pInfo->uri());
        } else {
            source = VideoSettings::videoSourceUDPH264;
            url = pInfo->uri().contains("udp://") ? pInfo->uri() : QStringLiteral("udp://0.0.0.0:%1").arg(pInfo->uri());
        }
        break;
    case VIDEO_STREAM_TYPE_MPEG_TS:
        source = VideoSettings::videoSourceMPEGTS;
        url = pInfo->uri().contains("mpegts://") ? pInfo->uri() : QStringLiteral("mpegts://0.0.0.0:%1").arg(pInfo->uri());
        break;
    default:
        qCWarning(VideoManagerLog) << "Unknown VIDEO_STREAM_TYPE";
        source = VideoSettings::videoSourceNoVideo;
        url = pInfo->uri();
        break;
    }

    const bool settingsChanged = _updateVideoUri(receiver, url);
    if (settingsChanged) {
        if (!receiver->isThermal()) {
            _videoSettings->videoSource()->setRawValue(source);
        }

        emit autoStreamConfiguredChanged();
    }

    return settingsChanged;
}

bool VideoManager::_updateVideoUri(VideoReceiver *receiver, const QString &uri)
{
    if (!receiver) {
        qCDebug(VideoManagerLog) << "VideoReceiver is NULL";
        return false;
    }

    if ((uri == receiver->uri()) && !receiver->uri().isNull()) {
        return false;
    }

    qCDebug(VideoManagerLog) << "New Video URI" << uri;

    receiver->setUri(uri);

    return true;
}

bool VideoManager::_updateSettings(VideoReceiver *receiver)
{
    if (!receiver) {
        qCDebug(VideoManagerLog) << "VideoReceiver is NULL";
        return false;
    }

    bool settingsChanged = false;

    const bool lowLatency = _videoSettings->lowLatencyMode()->rawValue().toBool();
    if (lowLatency != receiver->lowLatency()) {
        receiver->setLowLatency(lowLatency);
        settingsChanged = true;
    }

    const int rtpLatencyMs = static_cast<int>(std::min(_videoSettings->rtpJitterLatencyMs()->rawValue().toUInt(), static_cast<uint>(INT_MAX)));
    if (rtpLatencyMs != receiver->rtpJitterLatencyMs()) {
        receiver->setRtpJitterLatencyMs(rtpLatencyMs);
        settingsChanged = true;
    }

    const bool autoReconnect = _videoSettings->rtspAutoReconnect()->rawValue().toBool();
    if (autoReconnect != receiver->autoReconnect()) {
        receiver->setAutoReconnect(autoReconnect);
        // No settingsChanged: autoReconnect is live, doesn't require pipeline restart.
    }

    if (receiver->isThermal()) {
        return settingsChanged;
    }

    settingsChanged |= _updateUVC(receiver);

    const int cameraIndex = _cameraIndexForReceiver(receiver);
    if (cameraIndex < 0) {
        settingsChanged |= _updateVideoUri(receiver, QString());
        return settingsChanged;
    }

    if (receiver->name() == QLatin1String(kMainReceiverName) && cameraIndex == 0) {
        settingsChanged |= _updateAutoStream(receiver);
    }

    const QString source = _videoSettings->videoSourceNameAt(cameraIndex);
    settingsChanged |= _updateVideoUri(receiver, _sourceToUri(source, _videoSettings->videoUrlAt(cameraIndex)));

    return settingsChanged;
}

QString VideoManager::_sourceToUri(const QString &source, const QString &url) const
{
    if (source == VideoSettings::videoSourceUDPH264) {
        return QStringLiteral("udp://%1").arg(url);
    }
    if (source == VideoSettings::videoSourceUDPH265) {
        return QStringLiteral("udp265://%1").arg(url);
    }
    if (source == VideoSettings::videoSourceMPEGTS) {
        return QStringLiteral("mpegts://%1").arg(url);
    }
    if (source == VideoSettings::videoSourceRTSP) {
        return url;
    }
    if (source == VideoSettings::videoSourceTCP) {
        return QStringLiteral("tcp://%1").arg(url);
    }
    if (source == VideoSettings::videoSourceWebRTC) {
        const QString whepInput = url.trimmed();
        return whepInput.isEmpty() ? QString() : QUrl::fromUserInput(whepInput).toString();
    }
    if (source == VideoSettings::videoSource3DRSolo) {
        return QStringLiteral("udp://0.0.0.0:5600");
    }
    if (source == VideoSettings::videoSourceParrotDiscovery) {
        return QStringLiteral("udp://0.0.0.0:8888");
    }
    if (source == VideoSettings::videoSourceYuneecMantisG) {
        return QStringLiteral("rtsp://192.168.42.1:554/live");
    }
    if (source == VideoSettings::videoSourceHerelinkAirUnit) {
        return QStringLiteral("rtsp://192.168.0.10:8554/H264Video");
    }
    if (source == VideoSettings::videoSourceHerelinkHotspot) {
        return QStringLiteral("rtsp://192.168.43.1:8554/fpv_stream");
    }
    if ((source != VideoSettings::videoDisabled) && (source != VideoSettings::videoSourceNoVideo) && !isUvc()) {
        qCCritical(VideoManagerLog) << "Video source URI \"" << source << "\" is not supported. Please add support!";
    }
    return QString();
}

void VideoManager::_setActiveVehicle(Vehicle *vehicle)
{
    qCDebug(VideoManagerLog) << Q_FUNC_INFO << "new vehicle" << vehicle << "old active vehicle" << _activeVehicle;

    if (_activeVehicle) {
        (void) disconnect(_activeVehicle->vehicleLinkManager(), &VehicleLinkManager::communicationLostChanged, this, &VideoManager::_communicationLostChanged);
        auto cameraManager = _activeVehicle->cameraManager();
        if (cameraManager) {
            MavlinkCameraControlInterface *pCamera = cameraManager->currentCameraInstance();
            if (pCamera) {
                pCamera->stopStream();
            }
            (void) disconnect(cameraManager, &QGCCameraManager::streamChanged, this, &VideoManager::_videoSourceChanged);
        }

        for (VideoReceiver *receiver : std::as_const(_videoReceivers)) {
            // disconnect(receiver->videoStreamInfo(), &QGCVideoStreamInfo::infoChanged, ))
            receiver->setVideoStreamInfo(nullptr);
        }
    }

    _activeVehicle = vehicle;
    if (_activeVehicle) {
        (void) connect(_activeVehicle->vehicleLinkManager(), &VehicleLinkManager::communicationLostChanged, this, &VideoManager::_communicationLostChanged);
        if (_activeVehicle->cameraManager()) {
            (void) connect(_activeVehicle->cameraManager(), &QGCCameraManager::streamChanged, this, &VideoManager::_videoSourceChanged);
            MavlinkCameraControlInterface *pCamera = _activeVehicle->cameraManager()->currentCameraInstance();
            if (pCamera) {
                pCamera->resumeStream();
            }
        }

        for (VideoReceiver *receiver : std::as_const(_videoReceivers)) {
            if (_activeVehicle->cameraManager()) {
                if (receiver->isThermal()) {
                    receiver->setVideoStreamInfo(_activeVehicle->cameraManager()->thermalStreamInstance());
                } else {
                    receiver->setVideoStreamInfo(_activeVehicle->cameraManager()->currentStreamInstance());
                }
            } else {
                receiver->setVideoStreamInfo(nullptr);
            }
            // connect(receiver->videoStreamInfo(), &QGCVideoStreamInfo::infoChanged, ))
        }
    } else {
        setfullScreen(false);
    }
}

void VideoManager::_communicationLostChanged(bool connectionLost)
{
    if (connectionLost) {
        setfullScreen(false);
    }
}

void VideoManager::_restartAllVideos()
{
    for (VideoReceiver *videoReceiver : std::as_const(_videoReceivers)) {
        _restartVideo(videoReceiver);
    }
}

void VideoManager::_restartVideo(VideoReceiver *receiver)
{
    if (!receiver) {
        qCDebug(VideoManagerLog) << "VideoReceiver is NULL";
        return;
    }

    qCDebug(VideoManagerLog) << "Restart video receiver" << receiver->name();

    if (receiver->started()) {
        _stopReceiver(receiver);
        // onStopComplete Signal Will Restart It
    } else {
        _startReceiver(receiver);
    }
}

void VideoManager::_stopReceiver(VideoReceiver *receiver)
{
    if (!receiver) {
        qCDebug(VideoManagerLog) << "VideoReceiver is NULL";
        return;
    }

    if (receiver->started()) {
        receiver->stop();
    }
}

void VideoManager::stopVideo()
{
    for (VideoReceiver *receiver : std::as_const(_videoReceivers)) {
        _stopReceiver(receiver);
    }
}

void VideoManager::_startReceiver(VideoReceiver *receiver)
{
    if (!receiver) {
        qCDebug(VideoManagerLog) << "VideoReceiver is NULL";
        return;
    }

    if (receiver->started()) {
        qCDebug(VideoManagerLog) << "VideoReceiver is already started" << receiver->name();
        return;
    }

    if (receiver->uri().isEmpty()) {
        qCDebug(VideoManagerLog) << "VideoUri is NULL" << receiver->name();
        _setReceiverStatus(receiver, tr("No stream URL"));
        return;
    }
    _setReceiverStatus(receiver, tr("Connecting…"), true);

    receiver->start(_stallTimeoutFor(receiver));
}

uint32_t VideoManager::_stallTimeoutFor(const VideoReceiver *receiver) const
{
    const int cameraIndex = _cameraIndexForReceiver(receiver);
    const QString source = (cameraIndex >= 0) ? _videoSettings->videoSourceNameAt(cameraIndex) : _videoSettings->currentVideoSourceName();
    const bool needsNegotiationTime = (source == VideoSettings::videoSourceRTSP) || (source == VideoSettings::videoSourceWebRTC);
    return needsNegotiationTime ? _videoSettings->rtspTimeout()->rawValue().toUInt() : 3;
}

void VideoManager::_holdStallRestartWhileSwitching()
{
    for (VideoReceiver *receiver : std::as_const(_videoReceivers)) {
        receiver->setTimeout(kSwitchStallHoldSeconds);
    }
    QTimer::singleShot(kSwitchStallHoldSeconds * 1000, this, [this]() {
        for (VideoReceiver *receiver : std::as_const(_videoReceivers)) {
            receiver->setTimeout(_stallTimeoutFor(receiver));
        }
    });
}

#ifdef QGC_HEADLESS_CORE
void VideoManager::_initVideoReceiver(VideoReceiver *receiver)
{
    if (_videoReceivers.contains(receiver)) {
        qCWarning(VideoManagerLog) << "Receiver already initialized";
        return;
    }

    // Register before any setup so re-entry is blocked at every point below; error paths remove it.
    _videoReceivers.append(receiver);
#else
void VideoManager::_initVideoReceiver(VideoReceiver *receiver, QQuickWindow *window)
{
    if (_videoReceivers.contains(receiver)) {
        qCWarning(VideoManagerLog) << "Receiver already initialized";
        return;
    }

    // Register before any setup so re-entry is blocked at every point below; error paths remove it.
    _videoReceivers.append(receiver);

    // The thermal stream keeps its fixed widget; all camera receivers get their widget
    // assigned by role (main view vs tile) in _rebindWidgets().
    if (receiver->isThermal() && window) {
        QQuickItem *widget = window->findChild<QQuickItem*>(receiver->name());
        if (!widget) {
            qCCritical(VideoManagerLog) << "stream widget not found" << receiver->name();
            _videoReceivers.removeOne(receiver);
            receiver->deleteLater();
            return;
        }
        receiver->setWidget(widget);

        void *sink = QGCCorePlugin::instance()->createVideoSink(receiver->widget(), receiver);
        if (!sink) {
            qCCritical(VideoManagerLog) << "createVideoSink() failed" << receiver->name();
            _videoReceivers.removeOne(receiver);
            receiver->deleteLater();
            return;
        }
        receiver->setSink(sink);

        VideoBackend::attachSink(receiver, sink, widget);
    }
#endif

    (void) connect(receiver, &VideoReceiver::onStartComplete, this, [this, receiver](VideoReceiver::STATUS status) {
        qCDebug(VideoManagerLog) << "Video" << receiver->name() << "Start complete, status:" << status;
        switch (status) {
        case VideoReceiver::STATUS_OK:
            receiver->setStarted(true);
            if (_nativeRendering && !receiver->sink() && !receiver->isThermal()) {
                void *nativeSink = QGCCorePlugin::instance()->createNativeVideoSink(receiver);
                if (nativeSink) {
                    receiver->setSink(nativeSink);
                }
            }
            if (receiver->sink()) {
                receiver->startDecoding(receiver->sink());
            }
            _setReceiverStatus(receiver, tr("Connecting…"), true);
            break;
        case VideoReceiver::STATUS_INVALID_URL:
            _setReceiverStatus(receiver, tr("Invalid stream URL"));
            break;
        case VideoReceiver::STATUS_INVALID_STATE:
            break;
        default:
            _setReceiverStatus(receiver, tr("Connection failed, retrying"), true);
            QTimer::singleShot(1000, receiver, [this, receiver]() {
                _restartVideo(receiver);
            });
            break;
        }
    });

    (void) connect(receiver, &VideoReceiver::onStopComplete, this, [this, receiver](VideoReceiver::STATUS status) {
        qCDebug(VideoManagerLog) << "Stop complete" << receiver->name() << receiver->uri()  << ", status:" << status;
        receiver->setStarted(false);
        if (status == VideoReceiver::STATUS_INVALID_URL) {
            qCDebug(VideoManagerLog) << "Invalid video URL. Not restarting";
            _setReceiverStatus(receiver, tr("Invalid stream URL"));
        } else {
            _setReceiverStatus(receiver, tr("Reconnecting…"), true);
            QTimer::singleShot(1000, receiver, [this, receiver]() {
                qCDebug(VideoManagerLog) << "Restarting video receiver" << receiver->name() << receiver->uri();
                _startReceiver(receiver);
            });
        }
    });

    (void) connect(receiver, &VideoReceiver::streamingChanged, this, [this, receiver](bool active) {
        qCDebug(VideoManagerLog) << "Video" << receiver->name() << "streaming changed, active:" << (active ? "yes" : "no");
        if (!receiver->isThermal()) {
            _receiverState[receiver->name()].streaming = active;
            emit camerasChanged();
            if (_cameraIndexForReceiver(receiver) == _videoSettings->currentIndex()) {
                _streaming = active;
                emit streamingChanged();
            }
        }
    });

    (void) connect(receiver, &VideoReceiver::decodingChanged, this, [this, receiver](bool active) {
        qCDebug(VideoManagerLog) << "Video" << receiver->name() << "decoding changed, active:" << (active ? "yes" : "no");
        if (!receiver->isThermal()) {
            _receiverState[receiver->name()].decoding = active;
            emit camerasChanged();
            // The main-view visibility tracks the active camera's receiver; tiles must not clobber it.
            if (_cameraIndexForReceiver(receiver) == _videoSettings->currentIndex()) {
                _decoding = active;
                emit decodingChanged();
            }
        }
    });

    (void) connect(receiver, &VideoReceiver::recordingChanged, this, [this, receiver](bool active) {
        qCDebug(VideoManagerLog) << "Video" << receiver->name() << "recording changed, active:" << (active ? "yes" : "no");
        if (!receiver->isThermal()) {
            _receiverState[receiver->name()].recording = active;
            _recording = active;
            if (!active) {
                _subtitleWriter->stopCapturingTelemetry();
            }
            emit recordingChanged(_recording);
        }
    });

    (void) connect(receiver, &VideoReceiver::recordingStarted, this, [this, receiver](const QString &filename) {
        qCDebug(VideoManagerLog) << "Video" << receiver->name() << "recording started";
        if (!receiver->isThermal()) {
            _subtitleWriter->startCapturingTelemetry(filename, videoSize());
        }
    });

    (void) connect(receiver, &VideoReceiver::videoSizeChanged, this, [this, receiver](QSize size) {
        qCDebug(VideoManagerLog) << "Video" << receiver->name() << "resized. New resolution:" << size.width() << "x" << size.height();
        if (!receiver->isThermal()) {
            _receiverState[receiver->name()].videoSize = size;
            if (_cameraIndexForReceiver(receiver) == _videoSettings->currentIndex()) {
                _videoSize = size;
                emit videoSizeChanged();
                emit aspectRatioChanged();
            }
        }
    });

    (void) connect(receiver, &VideoReceiver::onTakeScreenshotComplete, this, [receiver](VideoReceiver::STATUS status) {
        if (status == VideoReceiver::STATUS_OK) {
            qCDebug(VideoManagerLog) << "Video" << receiver->name() << "screenshot taken";
        } else {
            qCWarning(VideoManagerLog) << "Video" << receiver->name() << "screenshot failed";
        }
    });

    (void) connect(receiver, &VideoReceiver::videoStreamInfoChanged, this, [this, receiver]() {
        const QGCVideoStreamInfo *videoStreamInfo = receiver->videoStreamInfo();
        qCDebug(VideoManagerLog) << "Video" << receiver->name() << "stream info:" << (videoStreamInfo ? "received" : "lost");

        (void) _updateAutoStream(receiver);
    });

    (void) _updateSettings(receiver);

    if (hasVideo()) {
        _startReceiver(receiver);
    }
}

void VideoManager::startVideo()
{
    qCDebug(VideoManagerLog) << "startVideo";

    if (!hasVideo()) {
        qCDebug(VideoManagerLog) << "Stream not enabled/configured";
        return;
    }

    _restartAllVideos();
}
