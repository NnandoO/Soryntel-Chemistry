add_library(cma_tools STATIC
    HeatLossCalc.cpp
    SpeciesConverter.cpp
    PeriodicTable.cpp
    UnitConverter.cpp
    LCAEvaluator.cpp
    DataFitter.cpp
)
target_include_directories(cma_tools PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})
target_include_directories(cma_tools PRIVATE ${CMAKE_SOURCE_DIR}/src)
target_link_libraries(cma_tools PUBLIC cma_domain cma_platform)
