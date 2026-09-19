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
    set_source_files_properties(QGCCoreCTest.cc TARGET_DIRECTORY ${CMAKE_PROJECT_NAME} PROPERTIES COMPILE_OPTIONS "-Wno-shadow")
    add_qgc_test(QGCCoreCTest LABELS Unit TIMEOUT 1800)
endif()

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})
add_qgc_test(QGCBridgeCoreTest LABELS Unit)
