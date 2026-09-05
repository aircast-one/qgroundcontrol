target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        DeviceInfo.cc
        DeviceInfo.h
        GCSBattery.cc
        GCSBattery.h
        JsonHelper.cc
        JsonHelper.h
        FileSystem/QGCCachedFileDownload.cc
        FileSystem/QGCCachedFileDownload.h
        FileSystem/QGCFileDownload.cc
        FileSystem/QGCFileDownload.h
        QGC.cc
        QGC.h
        QGCLogging.cc
        QGCLogging.h
        QGCLoggingCategory.cc
        QGCLoggingCategory.h
        FileSystem/QGCTemporaryFile.cc
        FileSystem/QGCTemporaryFile.h
        StateMachine.cc
        StateMachine.h
)

if(APPLE)
    target_sources(${CMAKE_PROJECT_NAME} PRIVATE Haptics.h Haptics.mm)
    set_source_files_properties(Haptics.mm TARGET_DIRECTORY ${CMAKE_PROJECT_NAME} PROPERTIES LANGUAGE CXX SKIP_PRECOMPILE_HEADERS TRUE COMPILE_OPTIONS "-x;objective-c++;-fobjc-arc")
else()
    target_sources(${CMAKE_PROJECT_NAME} PRIVATE Haptics.h Haptics.cc)
endif()

if(LINUX)
    target_sources(${CMAKE_PROJECT_NAME}
        PRIVATE
            SignalHandler.cc
            SignalHandler.h
    )
elseif(IOS)
    target_sources(${CMAKE_PROJECT_NAME}
        PRIVATE
            MobileScreenMgr.mm
            MobileScreenMgr.h
    )
elseif(APPLE)
    target_link_libraries(${CMAKE_PROJECT_NAME} PRIVATE "-framework IOKit" "-framework AppKit")
endif()

target_include_directories(${CMAKE_PROJECT_NAME}
    PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}
        FileSystem
)

#===========================================================================#

add_subdirectory(Audio)
add_subdirectory(Compression)
add_subdirectory(Geo)
add_subdirectory(Shape)
