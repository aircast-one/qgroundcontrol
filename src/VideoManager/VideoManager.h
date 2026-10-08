#pragma once

#include <array>
#include <atomic>
#include <chrono>

#include <QtCore/QFuture>
#include <QtCore/QHash>
#include <QtCore/QJsonArray>
#include <QtCore/QMutex>
#include <QtCore/QPointer>
#include <QtCore/QPromise>
#include <QtCore/QObject>
#include <QtCore/QSize>
#include <QtCore/QStringList>
#ifndef QGC_HEADLESS_CORE
#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickWindow>
#endif
#include <QtQmlIntegration/QtQmlIntegration>

#include "QGCVideoC.h"

#ifdef QGC_UNITTEST_BUILD
#include <functional>
#endif

class QQuickWindow;
class QQuickItem;
class QNetworkAccessManager;
class SubtitleWriter;
class VideoCloudFailover;
class Vehicle;
class QGCVideoStreamInfo;
class VideoReceiver;
class VideoSettings;

class VideoManager : public QObject
{
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("")
    Q_MOC_INCLUDE("Vehicle.h")

    Q_PROPERTY(bool     gstreamerEnabled        READ gstreamerEnabled                           CONSTANT)
    Q_PROPERTY(bool     qtmultimediaEnabled     READ qtmultimediaEnabled                        CONSTANT)
    Q_PROPERTY(bool     uvcEnabled              READ uvcEnabled                                 CONSTANT)
    Q_PROPERTY(bool     autoStreamConfigured    READ autoStreamConfigured                       NOTIFY activeVideoSourceChanged)
    Q_PROPERTY(bool     decoding                READ decoding                                   NOTIFY decodingChanged)
    Q_PROPERTY(QStringList cameraStatuses       READ cameraStatuses                             NOTIFY camerasChanged)
    Q_PROPERTY(QVariantList cameraConnecting    READ cameraConnecting                           NOTIFY camerasChanged)
    Q_PROPERTY(QStringList cameraSignals        READ cameraSignals                              NOTIFY camerasChanged)
    Q_PROPERTY(QVariantList cameraRecording     READ cameraRecording                            NOTIFY recordingChanged)
    Q_PROPERTY(QVariantList cameraConfigured    READ cameraConfigured                           NOTIFY camerasChanged)
    Q_PROPERTY(QVariantList cameraUsable        READ cameraUsable                               NOTIFY camerasChanged)
    Q_PROPERTY(QVariantList cameraFromDrone     READ cameraFromDrone                            NOTIFY camerasChanged)
    Q_PROPERTY(QStringList cameraNames          READ cameraNames                                NOTIFY camerasChanged)
    Q_PROPERTY(QStringList cameraSources        READ cameraSources                              NOTIFY camerasChanged)
    Q_PROPERTY(QStringList cameraUrls           READ cameraUrls                                 NOTIFY camerasChanged)
    Q_PROPERTY(int      activeVideoSource       READ activeVideoSource                          NOTIFY activeVideoSourceChanged)
    Q_PROPERTY(int      pipCameraNumber         READ pipCameraNumber                            NOTIFY activeVideoSourceChanged)
    Q_PROPERTY(QVariant pipSlot                 READ pipSlot                                    NOTIFY activeVideoSourceChanged)
    Q_PROPERTY(bool     hasMultipleVideoSources READ hasMultipleVideoSources                    NOTIFY activeVideoSourceChanged)
    Q_PROPERTY(QString  activeSourceLabel       READ activeSourceLabel                          NOTIFY activeVideoSourceChanged)
    Q_PROPERTY(int      videoSourceCount        READ videoSourceCount                           NOTIFY activeVideoSourceChanged)
    Q_PROPERTY(bool     fullScreen              READ fullScreen             WRITE setfullScreen NOTIFY fullScreenChanged)
    Q_PROPERTY(bool     hasThermal              READ hasThermal                                 NOTIFY decodingChanged)
    Q_PROPERTY(bool     hasVideo                READ hasVideo                                   NOTIFY hasVideoChanged)
    Q_PROPERTY(bool     isStreamSource          READ isStreamSource                             NOTIFY isStreamSourceChanged)
    Q_PROPERTY(bool     isUvc                   READ isUvc                                      NOTIFY isUvcChanged)
    Q_PROPERTY(bool     recording               READ recording                                  NOTIFY recordingChanged)
    Q_PROPERTY(bool     streaming               READ streaming                                  NOTIFY streamingChanged)
    Q_PROPERTY(double   aspectRatio             READ aspectRatio                                NOTIFY aspectRatioChanged)
    Q_PROPERTY(double   hfov                    READ hfov                                       NOTIFY aspectRatioChanged)
    Q_PROPERTY(double   thermalAspectRatio      READ thermalAspectRatio                         NOTIFY aspectRatioChanged)
    Q_PROPERTY(double   thermalHfov             READ thermalHfov                                NOTIFY aspectRatioChanged)
    Q_PROPERTY(QSize    videoSize               READ videoSize                                  NOTIFY videoSizeChanged)
    Q_PROPERTY(QString  videoStats              READ videoStats                                 NOTIFY videoStatsChanged)
    Q_PROPERTY(QString  imageFile               READ imageFile                                  NOTIFY imageFileChanged)
    Q_PROPERTY(QString  uvcVideoSourceID        READ uvcVideoSourceID                           NOTIFY uvcVideoSourceIDChanged)

    friend class VideoManagerInitTest;
    friend class VideoManagerTest;
    friend class VideoCameraSwitchTest;

public:
    explicit VideoManager(QObject *parent = nullptr);
    ~VideoManager();

    static VideoManager *instance();

    Q_INVOKABLE void grabImage(const QString &imageFile = QString());
    Q_INVOKABLE void startRecording(const QString &videoFile = QString());
    Q_INVOKABLE void startVideo();
    Q_INVOKABLE void stopRecording();
    Q_INVOKABLE void stopVideo();
    Q_INVOKABLE void setActiveVideoSource(int index);
    Q_INVOKABLE void storeCameras(const QString &list, int active);
    Q_INVOKABLE void setNativeRendering(bool nativeRendering);
    Q_INVOKABLE void switchActiveVideoSource();
    int pipCameraNumber() const;
    QVariant pipSlot() const;
    Q_INVOKABLE void promotePip();
#ifndef QGC_HEADLESS_CORE
    Q_INVOKABLE void registerPipItem(QQuickItem *item);
#endif
    Q_INVOKABLE QString cameraName(int index) const;
    QStringList cameraStatuses() const;
    QVariantList cameraConnecting() const;
    QStringList cameraSignals() const;
    QVariantList cameraRecording() const;
    QVariantList cameraConfigured() const;
    QVariantList cameraUsable() const;
    QVariantList cameraFromDrone() const;
    QStringList cameraNames() const;
    QStringList cameraSources() const;
    QStringList cameraUrls() const;
    quint64 cameraFramesDecoded(int index) const;
    quint64 cameraBytesReceived(int index) const;
    qint64 cameraSecondsSinceLastFrame(int index) const;

#ifdef QGC_HEADLESS_CORE
    void init();
    Q_INVOKABLE bool initNative() { init(); return _initialized; }
#else
    void init(QQuickWindow *mainWindow);
    Q_INVOKABLE bool initForItem(QQuickItem *item) { init(item ? item->window() : nullptr); return _initialized; }
    Q_INVOKABLE bool initNative() { init(nullptr); return _initialized; }
#endif
    void startVideoBackendInit();
    bool waitForVideoBackendReady(std::chrono::milliseconds timeout = std::chrono::minutes(1));
    void cleanup();
    void setCloudDevice(const QString &host, const QString &sfu, const QString &deviceId);
    VideoCloudFailover *cloudFailover() const { return _cloudFailover; }
    bool autoStreamConfigured() const;
    bool decoding() const { return _decoding; }
    bool fullScreen() const { return _fullScreen; }
    bool hasThermal() const;
    bool hasVideo() const;
    bool isStreamSource() const;
    bool isUvc() const;
    int activeVideoSource() const;
    bool hasMultipleVideoSources() const;
    QString activeSourceLabel() const;
    int videoSourceCount() const;
    bool recording() const { return _recording; }
    bool streaming() const { return _streaming; }
    double aspectRatio() const;
    double hfov() const;
    double thermalAspectRatio() const;
    double thermalHfov() const;
    QSize videoSize() const { return _videoSize; }
    QString videoStats() const { return _videoStats; }
    static QString formatVideoStats(int latencyMs, int fps, int height);
    QString imageFile() const { return _imageFile; }
    QString uvcVideoSourceID() const { return _uvcVideoSourceID; }
    void setfullScreen(bool on);
    static bool gstreamerEnabled();
    static bool qtmultimediaEnabled();
    static bool uvcEnabled();

signals:
    void activeVideoSourceChanged();
    void camerasChanged();
    void aspectRatioChanged();
    void decodingChanged();
    void fullScreenChanged();
    void hasVideoChanged();
    void imageFileChanged(const QString &filename);
    void isAutoStreamChanged();
    void isStreamSourceChanged();
    void isUvcChanged();
    void recordingChanged(bool recording);
    void recordingStarted(const QString &filename);
    void streamingChanged();
    void uvcVideoSourceIDChanged();
    void videoSizeChanged();
    void videoStatsChanged();

private slots:
    void _communicationLostChanged(bool communicationLost);
    void _setActiveVehicle(Vehicle *vehicle);
    void _videoSourceChanged();
    void _streamEnabledChanged();

private:
    bool _nativeRendering = false;

    enum class InitState : uint8_t {
        NotStarted,
        Pending,
        BackendReady,
        QmlReady,
        Running,
        Failed
    };

    void _initAfterQmlIsReady();
    void _onBackendInitComplete(bool success);
    void _createVideoReceivers();
#ifdef QGC_HEADLESS_CORE
    void _initVideoReceiver(VideoReceiver *receiver);
#else
    void _initVideoReceiver(VideoReceiver *receiver, QQuickWindow *window);
#endif
    bool _updateAutoStream(VideoReceiver *receiver);
    QJsonArray _droneCameras() const;
    static QString _droneCameraName(const QString &model, const QString &stream, bool several, int compId);
    static QString _droneCameraUrl(const QString &source, const QString &uri);
    static QPair<QString, QString> _announcedSource(const QGCVideoStreamInfo *info);
    bool _updateUVC(VideoReceiver *receiver);
    bool _updateSettings(VideoReceiver *receiver);
    bool _updateVideoUri(VideoReceiver *receiver, const QString &uri);
    int _cameraIndexForReceiver(const VideoReceiver *receiver) const;
    int _nativeChannelForReceiver(const VideoReceiver *receiver) const;
    void _bindNativeSink(VideoReceiver *receiver);
    void _releaseChannels(const VideoReceiver *receiver, int kept = -1);
    QString _cameraStatus(int index) const;
    QString _cameraSignal(int index) const;
    int _pipCamera() const;
    bool _cameraPlayed(int index) const;
#ifndef QGC_HEADLESS_CORE
    QQuickItem *_widgetForCamera(int cameraIndex) const;
#endif
    void _rebindWidgets();
    void _refreshActiveReceiverState();
    void _setReceiverStatus(VideoReceiver *receiver, const QString &status, bool connecting = false);
    void _setReceiverFailing(VideoReceiver *receiver, bool failing);
    bool _cameraConnecting(int index) const;
    bool _cameraRecording(int index) const;
    static QString _tileReceiverName(int slot);
    void _restartAllVideos();
    void _restartVideo(VideoReceiver *receiver);
    void _startReceiver(VideoReceiver *receiver);
    uint32_t _stallTimeoutFor(const VideoReceiver *receiver) const;
    void _holdStallRestartWhileSwitching();
    void _stopReceiver(VideoReceiver *receiver);
    static void _cleanupOldVideos();

    static constexpr int kMaxVideoTiles = 8;

    struct ReceiverState {
        bool streaming = false;
        bool decoding = false;
        bool connecting = false;
        bool recording = false;
        bool failing = false;
        bool stopRequested = false;
        QSize videoSize;
        QString status;
    };

    QList<VideoReceiver*> _videoReceivers;
#ifndef QGC_HEADLESS_CORE
    QPointer<QQuickItem> _pipWidget;
    QPointer<QQuickItem> _mainWidget;
#endif
    QHash<QString, ReceiverState> _receiverState;
    std::array<QString, QGC_VIDEO_CHANNELS> _channelHolders;
    SubtitleWriter *_subtitleWriter = nullptr;
    QNetworkAccessManager *_probeNetwork = nullptr;
    VideoCloudFailover *_cloudFailover = nullptr;
    VideoSettings *_videoSettings = nullptr;
#ifndef QGC_HEADLESS_CORE
    QQuickWindow *_mainWindow = nullptr;
#endif
    Vehicle *_activeVehicle = nullptr;

    std::atomic<InitState> _initState = InitState::NotStarted;
    QMutex _initFutureMutex;
    QFuture<bool> _backendInitFuture;
    bool _initialized = false;
    bool _backendDisabledForTests = false;
    bool _fullScreen = false;

    QAtomicInteger<bool> _decoding = false;
    QAtomicInteger<bool> _recording = false;
    QAtomicInteger<bool> _streaming = false;
    QSize _videoSize;
    QString _videoStats;
    quint64 _statsFramesDecoded = 0;
    const VideoReceiver *_statsReceiver = nullptr;
    void _updateVideoStats();
    QString _imageFile;
    QString _uvcVideoSourceID;

#ifdef QGC_UNITTEST_BUILD
    std::function<void()> _createVideoReceiversForTest;
#endif
};
