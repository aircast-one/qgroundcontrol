#include "AircastLinkFactGroup.h"
#include "Vehicle.h"

#include <QtCore/QTime>

AircastLinkFactGroup::AircastLinkFactGroup(QObject *parent)
    : FactGroup(1000, QStringLiteral(":/json/Vehicle/AircastLinkFact.json"), parent)
{
    _addFact(&_qualityFact);
    _addFact(&_radioTypeFact);
    _addFact(&_statusFact);
    _addFact(&_videoBitrateFact);
    _addFact(&_uplinkFact);

    _qualityFact.setRawValue(kQualityUnknown);
}

void AircastLinkFactGroup::handleMessage(Vehicle *vehicle, const mavlink_message_t &message)
{
    Q_UNUSED(vehicle);

    switch (message.msgid) {
    case MAVLINK_MSG_ID_CELLULAR_STATUS:
        _handleCellularStatus(message);
        break;
    case MAVLINK_MSG_ID_NAMED_VALUE_INT:
        _handleNamedValueInt(message);
        break;
    default:
        break;
    }
}

QString AircastLinkFactGroup::linkText() const
{
    const int quality = _qualityFact.rawValue().toInt();
    const QString qualityText = (quality == kQualityUnknown) ? QStringLiteral("--") : QStringLiteral("%1%").arg(quality);
    if (!_uplinkKnown) {
        return qualityText;
    }

    const int uplink = _uplinkFact.rawValue().toInt();
    return (uplink == kUplinkLte) ? QStringLiteral("%1 %2").arg(_uplinkName(uplink), qualityText) : _uplinkName(uplink);
}

void AircastLinkFactGroup::_handleCellularStatus(const mavlink_message_t &message)
{
    mavlink_cellular_status_t cellular{};
    mavlink_msg_cellular_status_decode(&message, &cellular);

    const uint32_t bitrateKbps = (cellular.link_rx_rate == UINT32_MAX) ? 0 : qRound(cellular.link_rx_rate * 1024.0 * 8.0 / 1000.0);

    quality()->setRawValue(cellular.quality);
    radioType()->setRawValue(cellular.type);
    status()->setRawValue(cellular.status);
    videoBitrate()->setRawValue(bitrateKbps);

    _remember(_qualityHistory, (cellular.quality == kQualityUnknown) ? QVariant(-1) : QVariant(static_cast<int>(cellular.quality)));
    _remember(_bitrateHistory, bitrateKbps);
    emit historyChanged();
    emit linkChanged();

    _setTelemetryAvailable(true);
}

void AircastLinkFactGroup::_handleNamedValueInt(const mavlink_message_t &message)
{
    if (message.compid != MAV_COMP_ID_ONBOARD_COMPUTER) {
        return;
    }

    mavlink_named_value_int_t value{};
    mavlink_msg_named_value_int_decode(&message, &value);
    if (QString::fromLatin1(value.name, static_cast<qsizetype>(strnlen(value.name, sizeof(value.name)))) != QStringLiteral("UPLINK")) {
        return;
    }

    const int from = _uplinkFact.rawValue().toInt();
    if (_uplinkKnown && (from != value.value)) {
        _uplinkSwitches.prepend(QVariantMap{
            {QStringLiteral("clock"), QTime::currentTime().toString(QStringLiteral("HH:mm:ss"))},
            {QStringLiteral("from"),  _uplinkName(from)},
            {QStringLiteral("to"),    _uplinkName(value.value)},
        });
        if (_uplinkSwitches.size() > kUplinkSwitchesKept) {
            _uplinkSwitches.removeLast();
        }
    }

    _uplinkKnown = true;
    uplink()->setRawValue(value.value);
    emit linkChanged();

    _setTelemetryAvailable(true);
}

QString AircastLinkFactGroup::_uplinkName(int uplink) const
{
    return _uplinkFact.enumStrings().value(uplink, tr("Unknown"));
}

void AircastLinkFactGroup::_remember(QVariantList &history, const QVariant &value)
{
    history.append(value);
    if (history.size() > kHistoryLength) {
        history.removeFirst();
    }
}
