add_library(cma_thermo STATIC
    ReactionSolver.cpp
    GibbsMinimizer.cpp
    HeatMaterialBalance.cpp
    ExergyCalculator.cpp
)
target_include_directories(cma_thermo PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
target_include_directories(cma_thermo PRIVATE ${CMAKE_SOURCE_DIR}/src)
target_link_libraries(cma_thermo PUBLIC cma_domain)
