/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#pragma once

#include <QtCore/QHash>
#include <QtCore/QLoggingCategory>
#include <QtCore/QObject>
#include <QtCore/QPointer>
#include <QtCore/QRunnable>
#include <QtCore/QSize>
#include <QtCore/QStringList>
#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickWindow>
// #include <QtQmlIntegration/QtQmlIntegration>

Q_DECLARE_LOGGING_CATEGORY(VideoManagerLog)

class QQuickWindow;
class QQuickItem;
class FinishVideoInitialization;
class SubtitleWriter;
class Vehicle;
class VideoReceiver;
class VideoSettings;

class VideoManager : public QObject
{
    Q_OBJECT
    // QML_ELEMENT
    // QML_UNCREATABLE("")
    Q_MOC_INCLUDE("Vehicle.h")
    Q_PROPERTY(bool     gstreamerEnabled        READ gstreamerEnabled                           CONSTANT)
    Q_PROPERTY(bool     qtmultimediaEnabled     READ qtmultimediaEnabled                        CONSTANT)
    Q_PROPERTY(bool     uvcEnabled              READ uvcEnabled                                 CONSTANT)
    Q_PROPERTY(bool     autoStreamConfigured    READ autoStreamConfigured                       NOTIFY autoStreamConfiguredChanged)
    Q_PROPERTY(bool     decoding                READ decoding                                   NOTIFY decodingChanged)
    Q_PROPERTY(QStringList cameraStatuses       READ cameraStatuses                             NOTIFY camerasChanged)
    Q_PROPERTY(QVariantList cameraConnecting    READ cameraConnecting                           NOTIFY camerasChanged)
    Q_PROPERTY(QVariantList cameraRecording     READ cameraRecording                            NOTIFY recordingChanged)
    Q_PROPERTY(bool     fullScreen              READ fullScreen             WRITE setfullScreen NOTIFY fullScreenChanged)
    Q_PROPERTY(bool     hasThermal              READ hasThermal                                 NOTIFY decodingChanged)
    Q_PROPERTY(bool     hasVideo                READ hasVideo                                   NOTIFY hasVideoChanged)
    Q_PROPERTY(bool     isStreamSource          READ isStreamSource                             NOTIFY isStreamSourceChanged)
    Q_PROPERTY(bool     isUvc                   READ isUvc                                      NOTIFY isUvcChanged)
    Q_PROPERTY(int      activeVideoSource       READ activeVideoSource                          NOTIFY activeVideoSourceChanged)
    Q_PROPERTY(bool     hasMultipleVideoSources READ hasMultipleVideoSources                    NOTIFY activeVideoSourceChanged)
    Q_PROPERTY(bool     recording               READ recording                                  NOTIFY recordingChanged)
    Q_PROPERTY(bool     streaming               READ streaming                                  NOTIFY streamingChanged)
    Q_PROPERTY(double   aspectRatio             READ aspectRatio                                NOTIFY aspectRatioChanged)
    Q_PROPERTY(double   hfov                    READ hfov                                       NOTIFY aspectRatioChanged)
    Q_PROPERTY(double   thermalAspectRatio      READ thermalAspectRatio                         NOTIFY aspectRatioChanged)
    Q_PROPERTY(double   thermalHfov             READ thermalHfov                                NOTIFY aspectRatioChanged)
    Q_PROPERTY(QSize    videoSize               READ videoSize                                  NOTIFY videoSizeChanged)
    Q_PROPERTY(QString  imageFile               READ imageFile                                  NOTIFY imageFileChanged)
    Q_PROPERTY(QString  uvcVideoSourceID        READ uvcVideoSourceID                           NOTIFY uvcVideoSourceIDChanged)

public:
    explicit VideoManager(QObject *parent = nullptr);
    ~VideoManager();

    static VideoManager *instance();
    static void registerQmlTypes();

    Q_INVOKABLE void grabImage(const QString &imageFile = QString());
    Q_INVOKABLE void startRecording(const QString &videoFile = QString());
    Q_INVOKABLE void startVideo();
    Q_INVOKABLE void stopRecording();
    Q_INVOKABLE void stopVideo();
    Q_INVOKABLE void setActiveVideoSource(int index);
    Q_INVOKABLE void switchActiveVideoSource();

    /// Number of picture-in-picture tile slots available for simultaneous multi-view.
    Q_INVOKABLE int maxVideoTiles() const;
    /// 1-based camera number shown in tile `slot`, or 0 when the slot is unused/multi-view is off.
    Q_INVOKABLE int tileCameraNumber(int slot) const;
    /// Makes the camera shown in tile `slot` the main (active) view.
    Q_INVOKABLE void promoteTile(int slot);
    /// Hands a tile's video item to the manager so its sink can be created. The tile items
    /// are created independently of C++ init order, so binding happens whenever both exist.
    /// Pass null while the tile is collapsed: the receiver keeps streaming but stops
    /// feeding the invisible item.
    Q_INVOKABLE void registerTileItem(int slot, QQuickItem *item);
    /// Connection status per camera index; an empty entry means frames are rendering.
    /// Bindable: re-evaluates on camerasChanged.
    QStringList cameraStatuses() const;
    /// True per camera index while a connection attempt is in flight, as opposed to a state
    /// that will not change on its own (no URL, bad URL). Only the former earns a spinner.
    QVariantList cameraConnecting() const;
    QVariantList cameraRecording() const;
    /// Decoded-frame counter and last-frame timestamp for the camera at `index` (0 when unknown).
    quint64 cameraFramesDecoded(int index) const;
    quint64 cameraBytesReceived(int index) const;
    qint64 cameraSecondsSinceLastFrame(int index) const;
    /// Display name for the camera at `index` (its configured name, or "Camera N").
    Q_INVOKABLE QString cameraName(int index) const;

    void init(QQuickWindow *rootWindow);
    Q_INVOKABLE void initForItem(QQuickItem *item) { init(item ? item->window() : nullptr); }
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
    void recordingChanged();
    void recordingStarted(const QString &filename);
    void streamingChanged();
    void uvcVideoSourceIDChanged();
    void videoSizeChanged();

private slots:
    void _communicationLostChanged(bool communicationLost);
    void _setActiveVehicle(Vehicle *vehicle);
    void _videoSourceChanged();

private:
    friend class VideoManagerTest;

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

    bool _initialized = false;
    bool _fullScreen = false;
    QAtomicInteger<bool> _decoding = false;
    QAtomicInteger<bool> _recording = false;
    QAtomicInteger<bool> _streaming = false;
    QSize _videoSize;
    QString _imageFile;
    QString _uvcVideoSourceID;
    Vehicle *_activeVehicle = nullptr;
};

/*===========================================================================*/

class FinishVideoInitialization : public QRunnable
{
public:
    FinishVideoInitialization();
    ~FinishVideoInitialization();

    void run() final;
};
