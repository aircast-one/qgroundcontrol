# ============================================================================
# Qt Location Plugin Module
# Custom map tile providers and caching engine
# ============================================================================

set(QGC_LOCATION_CACHE_SOURCES
    Providers/BingMapProvider.cpp
    Providers/BingMapProvider.h
    Providers/ElevationMapProvider.cpp
    Providers/ElevationMapProvider.h
    Providers/EsriMapProvider.cpp
    Providers/EsriMapProvider.h
    Providers/GenericMapProvider.cpp
    Providers/GenericMapProvider.h
    Providers/GoogleMapProvider.cpp
    Providers/GoogleMapProvider.h
    Providers/MapboxMapProvider.cpp
    Providers/MapboxMapProvider.h
    Providers/MapProvider.cpp
    Providers/MapProvider.h
    Providers/TianDiTuProvider.cpp
    Providers/TianDiTuProvider.h
    QGCCachedTileSet.cpp
    QGCCachedTileSet.h
    QGCCacheTile.h
    QGCMapEngine.cpp
    QGCMapEngine.h
    QGCMapEngineManager.cc
    QGCMapEngineManager.h
    QGCMapTaskBase.h
    QGCMapTasks.cpp
    QGCMapTasks.h
    QGCMapUrlEngine.cpp
    QGCMapUrlEngine.h
    QGCTile.h
    QGCTileCacheDatabase.cpp
    QGCTileCacheDatabase.h
    QGCTileCacheTypes.h
    QGCTileCacheWorker.cpp
    QGCTileCacheWorker.h
    QGCTileCache.cpp
    QGCTileCache.h
    QGCTileFetchReply.cpp
    QGCTileFetchReply.h
    QGCTileSet.h
)

set(QGC_LOCATION_PLUGIN_SOURCES
    QGeoFileTileCacheQGC.cpp
    QGeoFileTileCacheQGC.h
    QGeoMapReplyQGC.cpp
    QGeoMapReplyQGC.h
    QGeoServiceProviderPluginQGC.cpp
    QGeoServiceProviderPluginQGC.h
    QGeoTiledMappingManagerEngineQGC.cpp
    QGeoTiledMappingManagerEngineQGC.h
    QGeoTiledMapQGC.cpp
    QGeoTiledMapQGC.h
    QGeoTileFetcherQGC.cpp
    QGeoTileFetcherQGC.h
)

if(QGC_HEADLESS_CORE)
    qt_add_library(QGCLocation STATIC ${QGC_LOCATION_CACHE_SOURCES})
else()
    qt_add_plugin(QGCLocation
        STATIC
        CLASS_NAME QGeoServiceProviderFactoryQGC
        OUTPUT_TARGETS QGCLocation_targets
        ${QGC_LOCATION_CACHE_SOURCES}
        ${QGC_LOCATION_PLUGIN_SOURCES}
    )
endif()

if(IOS)
    target_compile_definitions(QGCLocation PRIVATE QGC_NO_GOOGLE_MAPS)
endif()

target_link_libraries(QGCLocation
    PRIVATE
        Qt6::Positioning
        QGCCompression
        QGCDatabase
        QGCFileSystem
        QGCFormat
        QGCLogging
        QGCNetwork
    PUBLIC
        Qt6::Core
        $<$<NOT:$<BOOL:${QGC_HEADLESS_CORE}>>:Qt6::Location>
        $<$<NOT:$<BOOL:${QGC_HEADLESS_CORE}>>:Qt6::LocationPrivate>
        Qt6::Network
)

target_include_directories(QGCLocation
    PUBLIC
        ${CMAKE_CURRENT_SOURCE_DIR}
        ${CMAKE_CURRENT_SOURCE_DIR}/Providers
    PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}/..
        ${CMAKE_CURRENT_SOURCE_DIR}/../FactSystem
        ${CMAKE_CURRENT_SOURCE_DIR}/../QmlControls
        ${CMAKE_CURRENT_SOURCE_DIR}/../Terrain
        ${CMAKE_CURRENT_SOURCE_DIR}/../Terrain/Providers
        ${CMAKE_CURRENT_SOURCE_DIR}/../Settings
        ${CMAKE_CURRENT_SOURCE_DIR}/../Utilities
)

target_link_libraries(${CMAKE_PROJECT_NAME} PRIVATE QGCLocation)

target_precompile_headers(QGCLocation PRIVATE
    <QtCore/QDir>
    <QtCore/QLoggingCategory>
    <QtCore/QObject>
    <QtCore/QString>
    <QtCore/QStringList>
    <QtCore/QVariant>
    <QtNetwork/QNetworkRequest>
    <QtPositioning/QGeoCoordinate>
)

qt_add_resources(${CMAKE_PROJECT_NAME} qtlocationplugin_resources
    PREFIX "/res"
    BASE "${CMAKE_SOURCE_DIR}/resources"
    FILES
        "${CMAKE_SOURCE_DIR}/resources/notile.png"
)
