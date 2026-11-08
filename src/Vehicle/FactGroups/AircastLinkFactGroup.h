#pragma once

#include <QtCore/QVariantList>

#include "FactGroup.h"

class AircastLinkFactGroup : public FactGroup
{
    Q_OBJECT
    Q_PROPERTY(Fact *quality        READ quality        CONSTANT)
    Q_PROPERTY(Fact *radioType      READ radioType      CONSTANT)
    Q_PROPERTY(Fact *status         READ status         CONSTANT)
    Q_PROPERTY(Fact *videoBitrate   READ videoBitrate   CONSTANT)
    Q_PROPERTY(Fact *uplink         READ uplink         CONSTANT)
    Q_PROPERTY(QVariantList qualityHistory READ qualityHistory NOTIFY historyChanged)
    Q_PROPERTY(QVariantList bitrateHistory READ bitrateHistory NOTIFY historyChanged)
    Q_PROPERTY(bool         cellular       READ cellular       NOTIFY historyChanged)
    Q_PROPERTY(bool         uplinkKnown    READ uplinkKnown    NOTIFY linkChanged)
    Q_PROPERTY(QString      linkText       READ linkText       NOTIFY linkChanged)
    Q_PROPERTY(QVariantList uplinkSwitches READ uplinkSwitches NOTIFY linkChanged)

public:
    static constexpr int kHistoryLength = 720;
    static constexpr int kUplinkSwitchesKept = 20;
    static constexpr int kQualityUnknown = UINT8_MAX;
    static constexpr int kUplinkLte = 3;

    explicit AircastLinkFactGroup(QObject *parent = nullptr);

    Fact *quality()      { return &_qualityFact; }
    Fact *radioType()    { return &_radioTypeFact; }
    Fact *status()       { return &_statusFact; }
    Fact *videoBitrate() { return &_videoBitrateFact; }
    Fact *uplink()       { return &_uplinkFact; }
    QVariantList qualityHistory() const { return _qualityHistory; }
    QVariantList bitrateHistory() const { return _bitrateHistory; }
    bool cellular() const { return !_qualityHistory.isEmpty(); }
    bool uplinkKnown() const { return _uplinkKnown; }
    QString linkText() const;
    QVariantList uplinkSwitches() const { return _uplinkSwitches; }

    void handleMessage(Vehicle *vehicle, const mavlink_message_t &message) final;

signals:
    void historyChanged();
    void linkChanged();

private:
    void _handleCellularStatus(const mavlink_message_t &message);
    void _handleNamedValueInt(const mavlink_message_t &message);
    QString _uplinkName(int uplink) const;
    static void _remember(QVariantList &history, const QVariant &value);

    Fact _qualityFact      = Fact(0, QStringLiteral("quality"),      FactMetaData::valueTypeUint8);
    Fact _radioTypeFact    = Fact(0, QStringLiteral("radioType"),    FactMetaData::valueTypeUint8);
    Fact _statusFact       = Fact(0, QStringLiteral("status"),       FactMetaData::valueTypeUint8);
    Fact _videoBitrateFact = Fact(0, QStringLiteral("videoBitrate"), FactMetaData::valueTypeUint32);
    Fact _uplinkFact       = Fact(0, QStringLiteral("uplink"),       FactMetaData::valueTypeUint8);
    QVariantList _qualityHistory;
    QVariantList _bitrateHistory;
    QVariantList _uplinkSwitches;
    bool _uplinkKnown = false;
};
