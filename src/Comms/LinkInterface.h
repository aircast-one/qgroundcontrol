#pragma once

#include <QtCore/QElapsedTimer>
#include <QtQmlIntegration/QtQmlIntegration>

#include <memory>

#include "LinkConfiguration.h"
#include "MAVLinkMessageType.h"

class LinkManager;
class QLoggingCategory;
class QThread;
class SigningController;

Q_DECLARE_LOGGING_CATEGORY(LinkInterfaceLog)

/// \brief The link interface defines the interface for all links used to communicate with the ground station application.
///
class LinkInterface : public QObject
{
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("")
    friend class LinkManager;

public:
    virtual ~LinkInterface();

    Q_INVOKABLE virtual void disconnect() = 0; // Implementations should guard against multiple calls

    virtual bool isConnected() const = 0;
    virtual bool isLogReplay() const { return false; }
    virtual bool isSecureConnection() const { return false; }

    SharedLinkConfigurationPtr linkConfiguration() { return _config; }
    const SharedLinkConfigurationPtr linkConfiguration() const { return _config; }
    uint8_t mavlinkChannel() const;
    bool mavlinkChannelIsSet() const;
    bool decodedFirstMavlinkPacket() const { return _decodedFirstMavlinkPacket; }
    void setDecodedFirstMavlinkPacket(bool decodedFirstMavlinkPacket) {
        if (_decodedFirstMavlinkPacket == decodedFirstMavlinkPacket) {
            return;
        }
        _decodedFirstMavlinkPacket = decodedFirstMavlinkPacket;
        emit decodedFirstMavlinkPacketChanged();
    }
    void writeBytesThreadSafe(const char *bytes, int length);
    /// Single message-level send chokepoint: re-signs (if signing is active), serializes, then writes. All
    /// outbound mavlink_message_t sends must route through here so signing can't be bypassed.
    void sendMessageThreadSafe(mavlink_message_t &message);
    void addVehicleReference() { ++_vehicleReferenceCount; }
    void removeVehicleReference();
    /// Called for each received v1 message which QGC drops. The warning is deferred by a grace
    /// period since ArduPilot starts links in v1 and upgrades to v2 on first v2 message from QGC.
    void reportMavlinkV1Traffic();
    /// Called when a v2 message is received: permanently suppresses the v1-only warning for this link.
    void reportMavlinkV2Traffic() { _mavlinkV2TrafficSeen = true; }
    bool mavlinkV1TrafficReported() const { return _mavlinkV1TrafficReported; }

    /// Grace period a link is given to upgrade from MAVLink v1 to v2 (ArduPilot starts out in v1)
    /// before the v1-only warning is reported. Settable for tests.
    static constexpr int kMavlinkV1TrafficGraceMsecsDefault = 10000;
    static void setMavlinkV1TrafficGraceMsecs(int msecs) { _mavlinkV1TrafficGraceMsecs = msecs; }

    /// Per-link signing state and confirmation state machine. Non-null after channel allocation.
    SigningController* signing() { return _signingController.get(); }
    const SigningController* signing() const { return _signingController.get(); }

signals:
    void bytesReceived(LinkInterface *link, const QByteArray &data);
    void bytesSent(LinkInterface *link, const QByteArray &data);
    void connected();
    void disconnected();
    void communicationError(const QString &title, const QString &error, LinkConfiguration::ErrorRemedy remedy = LinkConfiguration::RemedyRetry);
    void decodedFirstMavlinkPacketChanged();

protected:

    explicit LinkInterface(SharedLinkConfigurationPtr &config, QObject *parent = nullptr);

    virtual bool _allocateMavlinkChannel();

    virtual void _freeMavlinkChannel();

    void _connectionRemoved();

    /// Stops a link's worker thread, bounding the wait so destruction can always proceed.
    /// Guarantees the thread is never left as a QObject child while running (which would abort
    /// in ~QThread when child cleanup deletes it) — NOT that execution has stopped on return:
    /// on quit timeout the thread is terminated, and as a last resort it is orphaned and leaked,
    /// still running, with the link configuration kept alive so a resumed worker can't
    /// dereference a destroyed config. Pass allowTerminate = false when the worker holds locks
    /// that termination could leave locked.
    void _shutdownWorkerThread(QThread *thread, const QLoggingCategory &category, bool allowTerminate = true);

    SharedLinkConfigurationPtr _config;

private slots:

    virtual void _writeBytes(const QByteArray &bytes) = 0;

private:

    virtual bool _connect() = 0;

    void _orphanWorkerThread(QThread *thread);

    uint8_t _mavlinkChannel = std::numeric_limits<uint8_t>::max();
    bool _decodedFirstMavlinkPacket = false;
    int _vehicleReferenceCount = 0;
    bool _mavlinkV1TrafficReported = false;
    bool _mavlinkV2TrafficSeen = false;
    QElapsedTimer _mavlinkV1FirstSeenTimer;
    static inline int _mavlinkV1TrafficGraceMsecs = kMavlinkV1TrafficGraceMsecsDefault;
    /// Must `reset()` in `_freeMavlinkChannel` before LinkManager frees the channel so the
    /// controller can flush the final timestamp.
    std::unique_ptr<SigningController> _signingController;
};

typedef std::shared_ptr<LinkInterface> SharedLinkInterfacePtr;
typedef std::weak_ptr<LinkInterface> WeakLinkInterfacePtr;
