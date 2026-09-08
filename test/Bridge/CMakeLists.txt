target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        QGCBridgeCoreTest.cc
        QGCBridgeCoreTest.h
)

if(TARGET qgc_core_cargo)
    target_sources(${CMAKE_PROJECT_NAME}
        PRIVATE
            QGCCoreCTest.cc
            QGCCoreCTest.h
    )
endif()

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})
