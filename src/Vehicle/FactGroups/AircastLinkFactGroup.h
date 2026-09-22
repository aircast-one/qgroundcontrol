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
    Q_PROPERTY(QVariantList qualityHistory READ qualityHistory NOTIFY historyChanged)
    Q_PROPERTY(QVariantList bitrateHistory READ bitrateHistory NOTIFY historyChanged)

public:
    static constexpr int kHistoryLength = 720;
    static constexpr int kQualityUnknown = UINT8_MAX;

    explicit AircastLinkFactGroup(QObject *parent = nullptr);

    Fact *quality()      { return &_qualityFact; }
    Fact *radioType()    { return &_radioTypeFact; }
    Fact *status()       { return &_statusFact; }
    Fact *videoBitrate() { return &_videoBitrateFact; }
    QVariantList qualityHistory() const { return _qualityHistory; }
    QVariantList bitrateHistory() const { return _bitrateHistory; }

    void handleMessage(Vehicle *vehicle, const mavlink_message_t &message) final;

signals:
    void historyChanged();

private:
    void _handleCellularStatus(const mavlink_message_t &message);
    static void _remember(QVariantList &history, const QVariant &value);

    Fact _qualityFact      = Fact(0, QStringLiteral("quality"),      FactMetaData::valueTypeUint8);
    Fact _radioTypeFact    = Fact(0, QStringLiteral("radioType"),    FactMetaData::valueTypeUint8);
    Fact _statusFact       = Fact(0, QStringLiteral("status"),       FactMetaData::valueTypeUint8);
    Fact _videoBitrateFact = Fact(0, QStringLiteral("videoBitrate"), FactMetaData::valueTypeUint32);
    QVariantList _qualityHistory;
    QVariantList _bitrateHistory;
};
