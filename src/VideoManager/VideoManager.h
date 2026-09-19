#pragma once

#include <atomic>
#include <chrono>

#include <QtCore/QFuture>
#include <QtCore/QHash>
#include <QtCore/QMutex>
#include <QtCore/QPointer>
#include <QtCore/QPromise>
#include <QtCore/QObject>
#include <QtCore/QSize>
#include <QtCore/QStringList>
#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickWindow>
// #include <QtQmlIntegration/QtQmlIntegration>
#include <QtQmlIntegration/QtQmlIntegration>

#ifdef QGC_UNITTEST_BUILD
#include <functional>
#endif

class QQuickWindow;
class QQuickItem;
class SubtitleWriter;
class Vehicle;
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
    Q_PROPERTY(bool     autoStreamConfigured    READ autoStreamConfigured                       NOTIFY autoStreamConfiguredChanged)
    Q_PROPERTY(bool     decoding                READ decoding                                   NOTIFY decodingChanged)
    Q_PROPERTY(QStringList cameraStatuses       READ cameraStatuses                             NOTIFY camerasChanged)
    Q_PROPERTY(QVariantList cameraConnecting    READ cameraConnecting                           NOTIFY camerasChanged)
    Q_PROPERTY(QVariantList cameraRecording     READ cameraRecording                            NOTIFY recordingChanged)
    Q_PROPERTY(int      activeVideoSource       READ activeVideoSource                          NOTIFY activeVideoSourceChanged)
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
    Q_INVOKABLE void setNativeRendering(bool nativeRendering);
    Q_INVOKABLE void switchActiveVideoSource();
    Q_INVOKABLE int maxVideoTiles() const;
    Q_INVOKABLE int tileCameraNumber(int slot) const;
    Q_INVOKABLE void promoteTile(int slot);
    Q_INVOKABLE void registerTileItem(int slot, QQuickItem *item);
    Q_INVOKABLE QString cameraName(int index) const;
    QStringList cameraStatuses() const;
    /// True per camera index while a connection attempt is in flight, as opposed to a state
    /// that will not change on its own (no URL, bad URL). Only the former earns a spinner.
    QVariantList cameraConnecting() const;
    QVariantList cameraRecording() const;
    /// Decoded-frame counter and last-frame timestamp for the camera at `index` (0 when unknown).
    quint64 cameraFramesDecoded(int index) const;
    quint64 cameraBytesReceived(int index) const;
    qint64 cameraSecondsSinceLastFrame(int index) const;

    void init(QQuickWindow *mainWindow);
    Q_INVOKABLE bool initForItem(QQuickItem *item) { init(item ? item->window() : nullptr); return _initialized; }
    Q_INVOKABLE bool initNative() { init(nullptr); return _initialized; }
    void startVideoBackendInit();
    bool waitForVideoBackendReady(std::chrono::milliseconds timeout = std::chrono::minutes(1));
    void cleanup();
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
    void autoStreamConfiguredChanged();
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

private slots:
    void _communicationLostChanged(bool communicationLost);
    void _setActiveVehicle(Vehicle *vehicle);
    void _videoSourceChanged();

private:
    bool _nativeRendering = false;


    friend class VideoManagerTest;
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
    void _initVideoReceiver(VideoReceiver *receiver, QQuickWindow *window);
    bool _updateAutoStream(VideoReceiver *receiver);
    bool _updateUVC(VideoReceiver *receiver);
    bool _updateSettings(VideoReceiver *receiver);
    bool _updateVideoUri(VideoReceiver *receiver, const QString &uri);
    QString _sourceToUri(const QString &source, const QString &url) const;
    int _cameraIndexForReceiver(const VideoReceiver *receiver) const;
    QString _cameraStatus(int index) const;
    QQuickItem *_widgetForCamera(int cameraIndex) const;
    void _rebindWidgets();
    void _refreshActiveReceiverState();
    void _setReceiverStatus(VideoReceiver *receiver, const QString &status, bool connecting = false);
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
        QSize videoSize;
        QString status;
    };

    QList<VideoReceiver*> _videoReceivers;
    QHash<int, QPointer<QQuickItem>> _tileWidgets;
    QPointer<QQuickItem> _mainWidget;
    QHash<QString, ReceiverState> _receiverState;
    SubtitleWriter *_subtitleWriter = nullptr;
    VideoSettings *_videoSettings = nullptr;
    QQuickWindow *_mainWindow = nullptr;
    Vehicle *_activeVehicle = nullptr;

    std::atomic<InitState> _initState = InitState::NotStarted;
    // Orders _backendInitFuture publication against cross-thread waiters.
    QMutex _initFutureMutex;
    QFuture<bool> _backendInitFuture;
    bool _initialized = false;
    bool _backendDisabledForTests = false;
    bool _fullScreen = false;

    QAtomicInteger<bool> _decoding = false;
    QAtomicInteger<bool> _recording = false;
    QAtomicInteger<bool> _streaming = false;
    QSize _videoSize;
    QString _imageFile;
    QString _uvcVideoSourceID;

#ifdef QGC_UNITTEST_BUILD
    std::function<void()> _createVideoReceiversForTest;
#endif
};
