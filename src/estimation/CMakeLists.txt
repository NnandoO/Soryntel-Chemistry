add_library(cma_estimation STATIC
    StatisticalEstimator.cpp
    BensonEstimator.cpp
)
target_include_directories(cma_estimation PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
target_include_directories(cma_estimation PRIVATE ${CMAKE_SOURCE_DIR}/src)
target_link_libraries(cma_estimation PUBLIC Eigen3::Eigen SQLite3::SQLite3)
