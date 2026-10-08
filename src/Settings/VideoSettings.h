#pragma once

#include <QtQmlIntegration/QtQmlIntegration>

#include "SettingsGroup.h"

#include <QtCore/QJsonArray>
#include <QtCore/QJsonObject>

#include <functional>
#include <optional>

class VideoSettings : public SettingsGroup
{
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("")
public:
    VideoSettings(QObject* parent = nullptr);
    DEFINE_SETTING_NAME_GROUP()

    DEFINE_SETTINGFACT(videoSource)
    DEFINE_SETTINGFACT(cameras)
    DEFINE_SETTINGFACT(activeVideoSource)
    DEFINE_SETTINGFACT(multiViewEnabled)
    DEFINE_SETTINGFACT(aspectRatio)
    DEFINE_SETTINGFACT(videoFit)
    DEFINE_SETTINGFACT(gridLines)
    DEFINE_SETTINGFACT(showRecControl)
    DEFINE_SETTINGFACT(recordingFormat)
    DEFINE_SETTINGFACT(maxVideoSize)
    DEFINE_SETTINGFACT(enableStorageLimit)
    DEFINE_SETTINGFACT(rtspTimeout)
    DEFINE_SETTINGFACT(streamEnabled)
    DEFINE_SETTINGFACT(disableWhenDisarmed)
    DEFINE_SETTINGFACT(lowLatencyMode)
    DEFINE_SETTINGFACT(rtpJitterLatencyMs)
    DEFINE_SETTINGFACT(rtspAutoReconnect)
    DEFINE_SETTINGFACT(forceVideoDecoder)
    DEFINE_SETTINGFACT(forceCpuVideoPath)
    DEFINE_SETTINGFACT(videoConversionElement)
    DEFINE_SETTINGFACT(disablePixelAspectRatio)

    Q_PROPERTY(bool     streamConfigured        READ streamConfigured       NOTIFY streamConfiguredChanged)
    Q_PROPERTY(QString  rtspVideoSource         READ rtspVideoSource        CONSTANT)
    Q_PROPERTY(QString  udp264VideoSource       READ udp264VideoSource      CONSTANT)
    Q_PROPERTY(QString  udp265VideoSource       READ udp265VideoSource      CONSTANT)
    Q_PROPERTY(QString  tcpVideoSource          READ tcpVideoSource         CONSTANT)
    Q_PROPERTY(QString  mpegtsVideoSource       READ mpegtsVideoSource      CONSTANT)
    Q_PROPERTY(QString  webrtcVideoSource       READ webrtcVideoSource      CONSTANT)
    Q_PROPERTY(QString  disabledVideoSource     READ disabledVideoSource    CONSTANT)

    bool     streamConfigured       ();

    int      videoSourceCount       ();
    int      currentIndex           ();
    QString  currentVideoSourceName ();
    QString  currentVideoUrl        ();
    QString  videoSourceNameAt      (int index);
    QString  videoUrlAt             (int index);
    QString  cameraName             (int index);

    QList<int> switchableIndices     ();
    Q_INVOKABLE bool sourceConfigured (int index);
    Q_INVOKABLE bool sourceEnabled    (int index);
    bool     sourceUsable           (int index);
    Q_INVOKABLE bool offeredSource  (const QString &source);
    Q_INVOKABLE QStringList offeredSources();
    int      pipCameraIndex         ();
    int      nextUsableIndex        ();

    std::optional<QJsonArray> cameraList();
    Q_INVOKABLE bool camerasReadable();
    void     storeCameras           (const QJsonArray &cameras, int active);
    void     adoptCamera            (const QString &title, const QString &source, const QString &url);
    void     adoptDeviceCameras     (const QString &host, const QJsonArray &device);
    Q_INVOKABLE QString addCamera   (const QString &title, const QString &source, const QString &url);
    Q_INVOKABLE QString updateCamera(int index, const QString &title, const QString &source, const QString &url);
    Q_INVOKABLE void removeCamera   (int index);
    Q_INVOKABLE void moveCamera     (int from, int to);
    int      storedCameraCount      ();
    bool     cameraFromDrone        (int index);
    bool     setDroneCameras        (const QJsonArray &drone);
    QString  storedActiveSourceName ();
    static QJsonObject camera       (const QString &title, const QString &source, const QString &url);
    QString  problem                (const QString &source, const QString &url);
    static QString normalizedUrl    (const QString &source, const QString &url);
    static QString urlHost          (const QString &url);
    static QString streamUri        (const QString &source, const QString &url);
    static bool    sameStream       (const QString &a, const QString &b);
    static bool    pipCapable       (const QString &source);
    static int activeAfterAdd       (int active, int stored);
    static int activeAfterAppend    (int active, int stored, int appended);
    static int activeAfterRemoval   (int active, int removed);
    static int activeAfterMove      (int active, int from, int to);

    QString  rtspVideoSource        () { return videoSourceRTSP; }
    QString  udp264VideoSource      () { return videoSourceUDPH264; }
    QString  udp265VideoSource      () { return videoSourceUDPH265; }
    QString  tcpVideoSource         () { return videoSourceTCP; }
    QString  mpegtsVideoSource      () { return videoSourceMPEGTS; }
    QString  webrtcVideoSource      () { return videoSourceWebRTC; }
    QString  disabledVideoSource    () { return videoDisabled; }

    void pruneUnavailableDecoders();

    static constexpr const char* videoSourceNoVideo           = QT_TRANSLATE_NOOP("VideoSettings", "No Video Available");
    static constexpr const char* videoDisabled                = QT_TRANSLATE_NOOP("VideoSettings", "Video Stream Disabled");
    static constexpr const char* videoSourceRTSP              = QT_TRANSLATE_NOOP("VideoSettings", "RTSP Video Stream");
    static constexpr const char* videoSourceUDPH264           = QT_TRANSLATE_NOOP("VideoSettings", "UDP h.264 Video Stream");
    static constexpr const char* videoSourceUDPH265           = QT_TRANSLATE_NOOP("VideoSettings", "UDP h.265 Video Stream");
    static constexpr const char* videoSourceTCP               = QT_TRANSLATE_NOOP("VideoSettings", "TCP-MPEG2 Video Stream");
    static constexpr const char* videoSourceMPEGTS            = QT_TRANSLATE_NOOP("VideoSettings", "MPEG-TS Video Stream");
    static constexpr const char* videoSourceWebRTC            = QT_TRANSLATE_NOOP("VideoSettings", "WebRTC (WHEP) Video Stream");
    static constexpr const char* videoSource3DRSolo           = QT_TRANSLATE_NOOP("VideoSettings", "3DR Solo (requires restart)");
    static constexpr const char* videoSourceParrotDiscovery   = QT_TRANSLATE_NOOP("VideoSettings", "Parrot Discovery");
    static constexpr const char* videoSourceYuneecMantisG     = QT_TRANSLATE_NOOP("VideoSettings", "Yuneec Mantis G");
    static constexpr const char* videoSourceHerelinkAirUnit   = QT_TRANSLATE_NOOP("VideoSettings", "Herelink AirUnit");
    static constexpr const char* videoSourceHerelinkHotspot   = QT_TRANSLATE_NOOP("VideoSettings", "Herelink Hotspot");

signals:
    void streamConfiguredChanged    (bool configured);

private slots:
    void _configChanged             (QVariant value);

private:
    void _setForceVideoDecodeList();
    QJsonObject _cameraAt           (int index);
    QJsonArray _allCameras          ();
    static bool _sourceNeedsUrl     (const QString &source);
    static QString _streamIdentity  (const QString &uri);
    static std::optional<QString> _listenPort(const QString &uri);
    int _nextAfterCurrent           (const std::function<bool(int)> &eligible);
    static QStringList _schemes     (const QString &source);
    static QString _schemeOf        (const QString &source, const QString &url);
    static bool _schemeAdded        (const QString &source);

    QJsonArray _droneCameras;

};
