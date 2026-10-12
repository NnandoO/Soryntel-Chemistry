add_library(cma_sim STATIC
    SteadyStateEngine.cpp
    DynamicEngine.cpp
    MassReconciliation.cpp
)
target_include_directories(cma_sim PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
target_include_directories(cma_sim PRIVATE ${CMAKE_SOURCE_DIR}/src)
target_link_libraries(cma_sim PUBLIC Eigen3::Eigen SQLite3::SQLite3)
