target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        ApplePlatformTheme.cc
        ApplePlatformTheme.h
        FluentPlatformTheme.cc
        FluentPlatformTheme.h
        MaterialPlatformTheme.cc
        MaterialPlatformTheme.h
        PlatformTheme.cc
        PlatformTheme.h
)

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})
