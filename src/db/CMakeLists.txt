add_library(cma_db STATIC
    ThermoDB.cpp
    MineralDB.cpp
    MaterialDB.cpp
)
target_include_directories(cma_db PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
target_include_directories(cma_db PRIVATE ${CMAKE_SOURCE_DIR}/src)
target_link_libraries(cma_db PUBLIC Eigen3::Eigen SQLite3::SQLite3)
