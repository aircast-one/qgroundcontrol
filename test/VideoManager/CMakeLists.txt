target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        AirUnitCameraControlTest.cc
        AirUnitCameraControlTest.h
        VideoManagerTest.cc
        VideoManagerTest.h
)

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})
