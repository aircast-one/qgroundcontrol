#pragma once

#include <QtCore/QMap>
#include <QtCore/QObject>
#include <QtCore/QString>
#include <QtCore/QTimer>
#include <QtQmlIntegration/QtQmlIntegration>

#include "Fact.h"
#include "MAVLinkEnums.h"
#include "QGCMAVLinkTypes.h"

class QTextStream;

class ParameterEditorController;
class Vehicle;

class ParameterManager : public QObject
{
    Q_OBJECT
    QML_ELEMENT
    QML_UNCREATABLE("")
    Q_PROPERTY(bool parametersReady READ parametersReady NOTIFY parametersReadyChanged)
    Q_PROPERTY(bool missingParameters READ missingParameters NOTIFY missingParametersChanged)
    Q_PROPERTY(double loadProgress READ loadProgress NOTIFY loadProgressChanged)
    Q_PROPERTY(bool pendingWrites READ pendingWrites NOTIFY pendingWritesChanged)
    Q_PROPERTY(bool parameterDownloadSkipped READ parameterDownloadSkipped NOTIFY parameterDownloadSkippedChanged)
    friend class ParameterEditorController;

public:
    ParameterManager(Vehicle* vehicle);
    ~ParameterManager();

    bool parametersReady() const { return _parametersReady; }

    bool missingParameters() const { return _missingParameters; }

    double loadProgress() const { return _loadProgress; }

    bool parameterDownloadSkipped() const { return _parameterDownloadSkipped; }

    void setParameterDownloadSkipped(bool skipped);

    static QDir parameterCacheDir();

    static QString parameterCacheFile(int vehicleId, int componentId);

    void mavlinkMessageReceived(const mavlink_message_t& message);

    QList<int> componentIds() const;

    void refreshAllParameters(uint8_t componentID);

    Q_INVOKABLE void refreshAllParameters() { refreshAllParameters(MAV_COMP_ID_ALL); }

    void tryHashCheckCacheLoad();

    void refreshParameter(int componentId, const QString& paramName);

    void refreshParametersPrefix(int componentId, const QString& namePrefix);

    void bulkRefresh(int componentId, const QStringList& names, bool notifyFailure = true);

    void resetAllParametersToDefaults();
    void resetAllToVehicleConfiguration();

    bool parameterExists(int componentId, const QString& paramName) const;

    QStringList parameterNames(int componentId) const;

    Fact* getParameter(int componentId, const QString& paramName);

    void writeParametersToStream(QTextStream& stream) const;

    bool pendingWrites() const;

#ifdef QGC_UNITTEST_BUILD
    void setPendingWritesForTest(bool pending);
#endif

    Vehicle* vehicle();

    static MAV_PARAM_TYPE factTypeToMavType(FactMetaData::ValueType_t factType);
    static FactMetaData::ValueType_t mavTypeToFactType(MAV_PARAM_TYPE mavType);

    static constexpr int defaultComponentId = -1;

    static constexpr int kParamSetRetryCount = 2;
    static constexpr int kParamRequestReadRetryCount = 2;
    static constexpr int kWaitForParamValueAckMs = 1000;
    static constexpr int kMaxInitialRequestListRetry = 4;
    static constexpr int kHashCheckTimeoutMs = 1000;
    static constexpr int kTestHashCheckTimeoutMs = 200;
    static constexpr int kParamRequestListTimeoutMs = 5000;
    static constexpr int kTestInitialRequestIntervalMs = 500;
    static constexpr int kTestMaxInitialRequestTimeMs =
        (kMaxInitialRequestListRetry + 1) * kTestInitialRequestIntervalMs + 1000;

signals:
    void parametersReadyChanged(bool parametersReady);
    void missingParametersChanged(bool missingParameters);
    void loadProgressChanged(float value);
    void cacheCheckOnlyFailed();
    void pendingWritesChanged(bool pendingWrites);
    void parameterDownloadSkippedChanged();
    void factAdded(int componentId, Fact* fact);

    void _paramSetSuccess(int componentId, const QString& paramName);
    void _paramSetFailure(int componentId, const QString& paramName);
    void _paramRequestReadSuccess(int componentId, const QString& paramName, int paramIndex);
    void _paramRequestReadFailure(int componentId, const QString& paramName, int paramIndex);

private slots:
    void _factRawValueUpdated(const QVariant& rawValue);

private:
    void _handleParamValue(int componentId, const QString& parameterName, int parameterCount, int parameterIndex,
                           MAV_PARAM_TYPE mavParamType, const QVariant& parameterValue);
    void _mavlinkParamSet(int componentId, const QString& name, FactMetaData::ValueType_t valueType,
                          const QVariant& rawValue);
    void _waitingParamTimeout();
    void _tryCacheLookup();
    void _resetHashCheck();
    void _startParameterDownload(uint8_t componentId);
    void _hashCheckTimeout();
    void _paramRequestListTimeout();
    int _actualComponentId(int componentId) const;
    void _mavlinkParamRequestRead(int componentId, const QString& paramName, int paramIndex, bool notifyFailure);
    void _requestHashCheck(uint8_t componentId);
    void _writeLocalParamCache(int vehicleId, int componentId);
    void _tryCacheHashLoad(int vehicleId, int componentId, const QVariant& hashValue);
    void _loadMetaData();
    void _clearMetaData();
    QString _remapParamNameToVersion(const QString& paramName) const;
    bool _fillMavlinkParamUnion(FactMetaData::ValueType_t valueType, const QVariant& rawValue,
                                mavlink_param_union_t& paramUnion) const;
    bool _mavlinkParamUnionToVariant(const mavlink_param_union_t& paramUnion, QVariant& outValue) const;
    void _loadOfflineEditingParams();
    QString _logVehiclePrefix(int componentId) const;
    void _setLoadProgress(double loadProgress);
    bool _fillIndexBatchQueue(bool waitingParamTimeout);
    void _updateProgressBar();
    void _checkInitialLoadComplete();
    void _ftpDownloadComplete(const QString& fileName, const QString& errorMsg);
    void _ftpDownloadProgress(float progress);
    bool _parseParamFile(const QString& filename);
    void _incrementPendingWriteCount();
    void _decrementPendingWriteCount();
    QString _vehicleAndComponentString(int componentId) const;

    static QVariant _stringToTypedVariant(const QString& string, FactMetaData::ValueType_t type, bool failOk = false);

    Vehicle* _vehicle = nullptr;

    QMap<int, QMap<QString, Fact*>> _mapCompId2FactMap;

    double _loadProgress = 0;
    bool _parametersReady = false;
    bool _parameterDownloadSkipped = false;
    bool _missingParameters = false;
    bool _initialLoadComplete = false;
    bool _waitingForDefaultComponent = false;
    bool _metaDataAddedToFacts = false;
    bool _logReplay = false;
    bool _hashCheckDone = false;
    bool _cacheOnlyHashCheck = false;

    typedef QPair<int, QVariant> ParamTypeVal;
    typedef QMap<QString, ParamTypeVal> CacheMapName2ParamTypeVal;

    QMap<int, bool> _debugCacheCRC;
    QMap<int, CacheMapName2ParamTypeVal> _debugCacheMap;
    QMap<int, QMap<QString, bool>> _debugCacheParamSeen;

    int _prevWaitingReadParamIndexCount = 0;

    bool _readParamIndexProgressActive = false;

    static constexpr int _maxInitialRequestListRetry = kMaxInitialRequestListRetry;
    int _initialRequestRetryCount = 0;
    static constexpr int _maxInitialLoadRetrySingleParam = 5;
    bool _disableAllRetries = false;
    const int _waitForParamValueAckMs;

    bool _indexBatchQueueActive = false;
    QList<int> _indexBatchQueue;

    QMap<int, int> _paramCountMap;
    QMap<int, QMap<int, int>> _waitingReadParamIndexMap;
    QMap<int, QList<int>> _failedReadParamIndexMap;

    int _totalParamCount = 0;
    int _pendingWritesCount = 0;

    QTimer _hashCheckTimer;
    QTimer _paramRequestListTimer;
    QTimer _waitingParamTimeoutTimer;

    Fact _defaultFact;

    bool _tryftp = false;
    bool _vehicleBooting = false;
    bool _bootNoticeShown = false;
};
