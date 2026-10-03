#include "RadioComponentController.h"
#include "Fact.h"
#include "MultiVehicleManager.h"
#include "ParameterManager.h"
#include "QGCLoggingCategory.h"
#include "Vehicle.h"

#include <QtCore/QCoreApplication>
#include <QtCore/QPointer>
#include <QtCore/QSettings>
#include "QmlObjectListModel.h"

QGC_LOGGING_CATEGORY(RadioComponentControllerLog, "AutoPilotPlugins.RadioComponentController")
QGC_LOGGING_CATEGORY(RadioComponentControllerVerboseLog, "AutoPilotPlugins.RadioComponentController:verbose")

RadioComponentController::RadioComponentController(QObject *parent)
    : RemoteControlCalibrationController(parent)
{

    _calDefaultMinValue = 1000;
    _calDefaultMaxValue = 2000;
    _calCenterPoint= ((_calDefaultMaxValue - _calDefaultMinValue) / 2.0f) + _calDefaultMinValue;
    _calRoughCenterDelta = 50;
    _calMoveDelta = 300;
    _calSettleDelta = 20;

    int valueRange = _calDefaultMaxValue - _calDefaultMinValue;
    _calValidMinValue = _calDefaultMinValue + (valueRange * 0.3f);
    _calValidMaxValue = _calDefaultMaxValue - (valueRange * 0.3f);

    if (parameterExists(ParameterManager::defaultComponentId, QStringLiteral("RC1_REVERSED"))) {
        _revParamFormat = "RC%1_REVERSED";
        _revParamIsBool = true;
    } else {
        _revParamFormat = "RC%1_REV";
        _revParamIsBool = false;
    }

    _vehicle->startCalibration(QGCMAVLink::CalibrationRadio);
}

RadioComponentController *RadioComponentController::forActiveVehicle()
{
    static QPointer<RadioComponentController> controller;
    static QPointer<Vehicle> boundVehicle;

    Vehicle *const activeVehicle = MultiVehicleManager::instance()->activeVehicle();
    if (!activeVehicle) {
        delete controller;
        boundVehicle.clear();
        return nullptr;
    }

    if (!controller || (boundVehicle != activeVehicle)) {
        delete controller;
        controller = new RadioComponentController(QCoreApplication::instance());
        boundVehicle = activeVehicle;

        ParameterManager *const parameterManager = activeVehicle->parameterManager();
        (void) QObject::connect(parameterManager, &ParameterManager::parametersReadyChanged,
                                controller, [](bool ready) {
            if (ready && controller) {
                controller->start();
            }
        });
        if (parameterManager->parametersReady()) {
            controller->start();
        }
    }

    return controller;
}

RadioComponentController::~RadioComponentController()
{
    if (_vehicle && QCoreApplication::instance() && MultiVehicleManager::instance()->vehicles()->contains(_vehicle)) {
        _vehicle->stopCalibration(_vehicle->px4Firmware());
    }
}

void RadioComponentController::start(void)
{
    if (_throttleReversed) {
        _throttleReversed = false;
        emit throttleReversedChanged();
    }
    RemoteControlCalibrationController::start();
    (void) connect(_vehicle, &Vehicle::rcChannelsClampedChanged, this, &RemoteControlCalibrationController::_clampedChannelValuesChanged);

}

void RadioComponentController::spektrumBindMode(int mode)
{
    _vehicle->pairRX(RC_TYPE_SPEKTRUM, mode);
}

void RadioComponentController::crsfBindMode()
{
    _vehicle->pairRX(RC_TYPE_CRSF, 0);
}

bool RadioComponentController::_channelReversedParamValue(int channel)
{
    Fact *const paramFact = getParameterFact(ParameterManager::defaultComponentId, _revParamFormat.arg(channel+1));
    if (paramFact) {
        if (_revParamIsBool) {
            return paramFact->rawValue().toBool();
        } else {
            bool convertOk;
            float floatReversed = paramFact->rawValue().toFloat(&convertOk);
            if (!convertOk) {
                floatReversed = 1.0f;
            }

            return floatReversed == -1.0f;
        }
    }

    return false;
}

void RadioComponentController::_setChannelReversedParamValue(int channel, bool reversed)
{
    Fact *const paramFact = getParameterFact(ParameterManager::defaultComponentId, _revParamFormat.arg(channel+1));
    if (paramFact) {
        if (_revParamIsBool) {
            paramFact->setRawValue(reversed);
        } else {
            paramFact->setRawValue(reversed ? -1.0f : 1.0f);
        }
    }
}

void RadioComponentController::_saveStoredCalibrationValues()
{
    if (!_vehicle->px4Firmware() && ((_vehicle->vehicleType() == MAV_TYPE_HELICOPTER) || _vehicle->multiRotor()) && _rgChannelInfo[_rgFunctionChannelMapping[stickFunctionThrottle]].channelReversed) {
        _throttleReversed = true;
        emit throttleReversedChanged();
        emit throttleReversedCalFailure();
    } else {
        _validateAndAdjustCalibrationValues();

        const QString minTpl("RC%1_MIN");
        const QString maxTpl("RC%1_MAX");
        const QString trimTpl("RC%1_TRIM");

        for (int chan = 0; chan<_chanMax; chan++) {
            ChannelInfo *const info = &_rgChannelInfo[chan];
            const int oneBasedChannel = chan + 1;

            if (!parameterExists(ParameterManager::defaultComponentId, minTpl.arg(chan+1))) {
                continue;
            }

            Fact* paramFact = getParameterFact(ParameterManager::defaultComponentId, trimTpl.arg(oneBasedChannel));
            if (paramFact) {
                paramFact->setRawValue(static_cast<float>(info->channelTrim));
            }
            paramFact = getParameterFact(ParameterManager::defaultComponentId, minTpl.arg(oneBasedChannel));
            if (paramFact) {
                paramFact->setRawValue(static_cast<float>(info->channelMin));
            }
            paramFact = getParameterFact(ParameterManager::defaultComponentId, maxTpl.arg(oneBasedChannel));
            if (paramFact) {
                paramFact->setRawValue(static_cast<float>(info->channelMax));
            }

            if (_vehicle->px4Firmware() || _vehicle->multiRotor()) {
                bool reversed;
                if (_vehicle->px4Firmware() || info->stickFunction != stickFunctionPitch) {
                    reversed = info->channelReversed;
                } else {
                    reversed = !info->channelReversed;
                }
                _setChannelReversedParamValue(chan, reversed);
            }
        }

        for (size_t stickFunctionIndex = 0; stickFunctionIndex < stickFunctionMaxRadio; stickFunctionIndex++) {
            int32_t paramChannel;
            if (_rgFunctionChannelMapping[stickFunctionIndex] == _chanMax) {
                paramChannel = 0;
            } else {
                paramChannel = _rgFunctionChannelMapping[stickFunctionIndex] + 1;
            }

            QString paramName = _stickFunctionToParamName(static_cast<StickFunction>(stickFunctionIndex));
            Fact* paramFact = getParameterFact(ParameterManager::defaultComponentId, paramName);

            if (paramFact && paramFact->rawValue().toInt() != paramChannel) {
                paramFact = getParameterFact(ParameterManager::defaultComponentId, paramName);
                if (paramFact) {
                    paramFact->setRawValue(paramChannel);
                }
            }
        }
    }

    if (_vehicle->px4Firmware()) {
        if (parameterExists(ParameterManager::defaultComponentId, QStringLiteral("RC_CHAN_CNT"))) {
            getParameterFact(ParameterManager::defaultComponentId, QStringLiteral("RC_CHAN_CNT"))->setRawValue(_chanCount);
        }
    }

    _readStoredCalibrationValues();
}

void RadioComponentController::_readStoredCalibrationValues()
{

    for (int i = 0; i < _chanMax; i++) {
        ChannelInfo *const info = &_rgChannelInfo[i];
        info->stickFunction = stickFunctionMaxRadio;
    }

    for (size_t i = 0; i < stickFunctionMaxRadio; i++) {
        _rgFunctionChannelMapping[i] = _chanMax;
    }

    const QString minTpl("RC%1_MIN");
    const QString maxTpl("RC%1_MAX");
    const QString trimTpl("RC%1_TRIM");

    for (int i = 0; i < _chanMax; ++i) {
        ChannelInfo *const info = &_rgChannelInfo[i];

        if (!parameterExists(ParameterManager::defaultComponentId, minTpl.arg(i+1))) {
            info->channelTrim = 1500;
            info->channelMin = 1100;
            info->channelMax = 1900;
            info->channelReversed = false;
            continue;
        }

        Fact *paramFact = getParameterFact(ParameterManager::defaultComponentId, trimTpl.arg(i+1));
        if (paramFact) {
            info->channelTrim = paramFact->rawValue().toInt();
        }

        paramFact = getParameterFact(ParameterManager::defaultComponentId, minTpl.arg(i+1));
        if (paramFact) {
            info->channelMin = paramFact->rawValue().toInt();
        }

        paramFact = getParameterFact(ParameterManager::defaultComponentId, maxTpl.arg(i+1));
        if (paramFact) {
            info->channelMax = getParameterFact(ParameterManager::defaultComponentId, maxTpl.arg(i+1))->rawValue().toInt();
        }

        info->channelReversed = _channelReversedParamValue(i);
    }

    for (int i=0; i<stickFunctionMaxRadio; i++) {
        int32_t paramChannel;

        QString paramName = _stickFunctionToParamName(static_cast<StickFunction>(i));
        Fact *const paramFact = getParameterFact(ParameterManager::defaultComponentId, paramName);
        if (paramFact) {
            paramChannel = paramFact->rawValue().toInt();

            if (paramChannel > 0 && paramChannel <= _chanMax) {
                _rgFunctionChannelMapping[i] = paramChannel - 1;
                _rgChannelInfo[paramChannel - 1].stickFunction = static_cast<StickFunction>(i);
            }
        }
    }

    _signalAllAttitudeValueChanges();
}

QString RadioComponentController::_stickFunctionToParamName(RemoteControlCalibrationController::StickFunction function) const
{
    static const QHash<StickFunction, QString> rgStickFunctionParamsPX4({
        { stickFunctionRoll,     "RC_MAP_ROLL"     },
        { stickFunctionPitch,    "RC_MAP_PITCH"    },
        { stickFunctionYaw,      "RC_MAP_YAW"      },
        { stickFunctionThrottle, "RC_MAP_THROTTLE" },
    });

    static const QHash<StickFunction, QString> rgStickFunctionParamsAPM({
        { stickFunctionRoll,     "RCMAP_ROLL"     },
        { stickFunctionPitch,    "RCMAP_PITCH"    },
        { stickFunctionYaw,      "RCMAP_YAW"      },
        { stickFunctionThrottle, "RCMAP_THROTTLE" },
    });

    const auto &rgStickFunctionParams = _vehicle->px4Firmware() ? rgStickFunctionParamsPX4 : rgStickFunctionParamsAPM;

    if (!rgStickFunctionParams.contains(function)) {
        qCWarning(RadioComponentControllerLog) << "Internal Error: Invalid stick function";
        return QString();
    }

    return rgStickFunctionParams.value(function);
}
