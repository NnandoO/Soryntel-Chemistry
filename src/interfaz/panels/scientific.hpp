#pragma once
#include "domain/AqueousDatabase.hpp"
#include "domain/ports/ThermoRepository.hpp"
#include "motores/especiacion_acuosa.hpp"
#include "motores/mineral_processing.hpp"
#include "application/CatalogAudit.hpp"
#include "application/ports/FileDialog.hpp"
#include "application/FlotationCalibration.hpp"
#include "domain/ports/AqueousEngine.hpp"
#include <array>

namespace cma::interfaz {
class AdvancedAqueousPanel {
public:
    explicit AdvancedAqueousPanel(const domain::AqueousEngine& engine) : engine_(engine) {}
    void render();
private:
    const domain::AqueousEngine& engine_;
    domain::AqueousRequest input_;
    domain::AqueousResponse result_;
    int model_=0;
    bool calculated_=false;
};
class NativeAqueousPanel {
public:
    explicit NativeAqueousPanel(const domain::AqueousDatabase& db) : database_(db) {}
    void render();
private:
    const domain::AqueousDatabase& database_;
    std::map<int,double> totals_{{140,.001},{150,.001},{180,.012},{500,.01}};
    int new_component_ = 150, model_ = 0;
    double ph_ = 7, kelvin_ = 298.15;
    bool calculated_ = false;
    motores::SpeciationResult result_;
};
class MineralProcessingPanel {
public:
    explicit MineralProcessingPanel(application::FileDialog& dialog) : dialog_(dialog) {}
    void render();
private:
    application::FileDialog& dialog_;
    application::FlotationFit fit_;
    double calibration_limit_=0;
    struct Row { std::array<char,64> name{}; double size=100,mass=0,rate=0,limit=0; };
    std::vector<Row> rows_;
    int operation_ = 0;
    double water_=0,d50_=100,sharpness_=2,bypass_=0,water_split_=0;
    double residence_=1,float_water_=0,index_=0,f80_=10000,p80_=100;
    bool calculated_=false;
    motores::ParticleSplit classification_, result_;
};
class CatalogAuditPanel {
public:
    CatalogAuditPanel(const domain::ThermoRepository& db,application::FileDialog& dialog)
        : repository_(db), dialog_(dialog) {}
    void render();
private:
    const domain::ThermoRepository& repository_;
    application::FileDialog& dialog_;
    application::CatalogAudit report_;
    bool loaded_ = false;
    std::string message_;
};
}
