/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "SysStatusSensorInfo.h"
#include "QGCLoggingCategory.h"
#include "QGCMAVLink.h"

QGC_LOGGING_CATEGORY(SysStatusSensorInfoLog, "qgc.mavlink.sysstatussensorinfo")

SysStatusSensorInfo::SysStatusSensorInfo(QObject *parent)
    : QObject(parent)
{
    // qCDebug(SysStatusSensorInfoLog) << Q_FUNC_INFO << this;
}

SysStatusSensorInfo::~SysStatusSensorInfo()
{
    // qCDebug(SysStatusSensorInfoLog) << Q_FUNC_INFO << this;
}

void SysStatusSensorInfo::update(const mavlink_sys_status_t &sysStatus)
{
    bool dirty = false;

    // Walk the bits
    for (int bitPosition = 0; bitPosition < 32; bitPosition++) {
        const MAV_SYS_STATUS_SENSOR sensorBitMask = static_cast<MAV_SYS_STATUS_SENSOR>(1 << bitPosition);
        if (sysStatus.onboard_control_sensors_present & sensorBitMask) {
            if (_sensorInfoMap.contains(sensorBitMask)) {
                SensorInfo &sensorInfo = _sensorInfoMap[sensorBitMask];

                const bool newEnabled = sysStatus.onboard_control_sensors_enabled & sensorBitMask;
                if (sensorInfo.enabled != newEnabled) {
                    dirty = true;
                    sensorInfo.enabled = newEnabled;
                }

                const bool newHealthy = sysStatus.onboard_control_sensors_health & sensorBitMask;
                if (sensorInfo.healthy != newHealthy) {
                    dirty = true;
                    sensorInfo.healthy = newHealthy;
                }
            } else {
                dirty = true;
                const SensorInfo sensorInfo = { !!(sysStatus.onboard_control_sensors_enabled & sensorBitMask), !!(sysStatus.onboard_control_sensors_health & sensorBitMask) };
                _sensorInfoMap[sensorBitMask] = sensorInfo;
            }
        } else if (_sensorInfoMap.contains(sensorBitMask)) {
            dirty = true;
            (void) _sensorInfoMap.remove(sensorBitMask);
        }
    }

    if (dirty) {
        emit sensorInfoChanged();
    }
}

QList<QPair<MAV_SYS_STATUS_SENSOR, SysStatusSensorInfo::SensorInfo>> SysStatusSensorInfo::_orderedSensors() const
{
    QList<QPair<MAV_SYS_STATUS_SENSOR, SensorInfo>> unhealthy;
    QList<QPair<MAV_SYS_STATUS_SENSOR, SensorInfo>> healthy;
    QList<QPair<MAV_SYS_STATUS_SENSOR, SensorInfo>> disabled;

    for (std::pair<MAV_SYS_STATUS_SENSOR, const SensorInfo&> sensor : _sensorInfoMap.asKeyValueRange()) {
        QList<QPair<MAV_SYS_STATUS_SENSOR, SensorInfo>> &bucket =
            !sensor.second.enabled ? disabled : (sensor.second.healthy ? healthy : unhealthy);
        bucket.append({sensor.first, sensor.second});
    }

    return unhealthy + healthy + disabled;
}

QStringList SysStatusSensorInfo::sensorNames() const
{
    QStringList rgNames;
    for (const auto &sensor : _orderedSensors()) {
        rgNames.append(QGCMAVLink::mavSysStatusSensorToString(sensor.first));
    }
    return rgNames;
}

QVariantList SysStatusSensorInfo::sensorHealthy() const
{
    QVariantList rgHealthy;
    for (const auto &sensor : _orderedSensors()) {
        rgHealthy.append(sensor.second.enabled && sensor.second.healthy);
    }
    return rgHealthy;
}

QVariantList SysStatusSensorInfo::sensorEnabled() const
{
    QVariantList rgEnabled;
    for (const auto &sensor : _orderedSensors()) {
        rgEnabled.append(sensor.second.enabled);
    }
    return rgEnabled;
}

QStringList SysStatusSensorInfo::sensorStatus() const
{
    QStringList rgStatus;
    for (const auto &sensor : _orderedSensors()) {
        rgStatus.append(!sensor.second.enabled ? tr("Disabled")
                                               : (sensor.second.healthy ? tr("Normal") : tr("Error")));
    }
    return rgStatus;
}
