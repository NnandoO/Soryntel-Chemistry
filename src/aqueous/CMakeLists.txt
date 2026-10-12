add_library(cma_aqueous STATIC
    SteamTables.cpp
    ElectrolyteSolver.cpp
)
target_include_directories(cma_aqueous PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
target_include_directories(cma_aqueous PRIVATE ${CMAKE_SOURCE_DIR}/src)
target_link_libraries(cma_aqueous PUBLIC Eigen3::Eigen SQLite3::SQLite3)
