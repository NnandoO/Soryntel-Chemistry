#pragma once
#include <atomic>
#include <cmath>
#include <imgui.h>
#include <string>
#include <thread>
#include <vector>
#include "diagrama_flujo/flowsheet.hpp"
#include "domain/ports/ThermoRepository.hpp"
#include "motores/estimador_propiedades.hpp"
#include "motores/red_neuronal.hpp"
#include "tools/DataFitter.hpp"
#include "tools/SpeciesConverter.hpp"

namespace cma { namespace interfaz {

// ── Panel de Convertidor de Especies (análisis químico a minerales) ─────
class PanelConvertidorEspecies {
public:
    PanelConvertidorEspecies();
    void establecer_repositorio(const domain::ThermoRepository* repo) { repo_ = repo; }
    void renderizar();
private:
    const domain::ThermoRepository* repo_ = nullptr;
    struct InputItem {
        char element[32];
        double wt_pct;
        double calc_elem;
    };
    struct OutputItem {
        char species[32];
        double calc_wt;
        double target_wt;
        double weight_coeff;
        char type[16];
    };

    std::vector<InputItem> inputs_;
    std::vector<OutputItem> outputs_;

    double input_total_wt_ = 0.0;
    double output_total_wt_ = 0.0;

    bool mostrar_grafico_ = false;

    cma::application::modules::SpeciesConverter motor_;

    char save_filename_[128] = "conversion.json";
    char open_filename_[128] = "conversion.json";

    void calcular();
    void guardar_json(const char* path);
    void cargar_json(const char* path);
};

// ── Panel de Unidades (conversor) ───────────────────────────────────────
class PanelUnidades {
public:
    void renderizar();
private:
    bool db_inicializada_ = false;
    int categoria_ = 0;
    int unidad_origen_ = 0;
    int unidad_destino_ = 1;

    struct FilaUnidades {
        double d1 = 0.0;
        double d2 = 0.0;
        std::string cat = "";
        int u1 = 0;
        int u2 = 1;
        std::string nom_u1 = "";
        std::string nom_u2 = "";
        bool activa = false;
    };
    std::vector<FilaUnidades> filas_;
};

// ── Panel de Estimación de Propiedades ──────────────────────────────────
class PanelEstimacionProp {
public:
    explicit PanelEstimacionProp(const domain::ThermoRepository& repository) : repo_(&repository), motor_(repository) {}
    void establecer_repositorio(const domain::ThermoRepository* repo) { repo_ = repo; }
    void renderizar();
private:
    const domain::ThermoRepository* repo_ = nullptr;
    char formula_[64] = "CuSO4";
    double temperatura_ = 25.0;
    bool es_celsius_ = true;
    int tipo_especie_ = 1; // 0=Not Specified, 1=Inorganic, 2=Organic
    bool mostrar_grafico_ = false;
    bool grafico_calculado_ = false;
    std::vector<double> exp_h_;
    std::vector<double> est_h_;
    std::vector<std::string> db_formulas_;
    double mean_error_ = 0.0;
    double median_error_ = 0.0;
    int species_count_ = 0;
    motores::EstimadorPropiedades motor_;
    std::vector<motores::PropiedadesEstimadasHSC> resultados_;
};

// ── Panel de Método Benson ──────────────────────────────────────────────
struct Vec3 {
    float x,y,z;
    Vec3 operator+(const Vec3& o) const { return {x+o.x, y+o.y, z+o.z}; }
    Vec3 operator-(const Vec3& o) const { return {x-o.x, y-o.y, z-o.z}; }
    Vec3 operator*(float s) const { return {x*s, y*s, z*s}; }
    Vec3 cross(const Vec3& o) const { return {y*o.z - z*o.y, z*o.x - x*o.z, x*o.y - y*o.x}; }
    float dot(const Vec3& o) const { return x*o.x + y*o.y + z*o.z; }
    Vec3 normalize() const {
        float l = std::sqrt(x*x+y*y+z*z);
        if (l < 1e-6f) return {1.0f,0.0f,0.0f};
        return {x/l, y/l, z/l};
    }
};

struct Atomo3D {
    float x, y, z;
    std::string elemento;
    std::vector<Vec3> valencias; // vectores direccionales disponibles para enlaces
};
struct Enlace3D {
    int atomo1, atomo2;
};

class PanelBenson {
public:
    void renderizar();
private:
    char formula_[128] = "";
    double mw_ = 0.0;
    double h_est_ = 0.0;
    double s_est_ = 0.0;
    double cp_est_ = 0.0;

    std::vector<Atomo3D> atomos_;
    std::vector<Enlace3D> enlaces_;

    float atom_size_ = 1.0f;
    float opacity_ = 1.0f;

    void agregar_grupo(const std::string& nombre, const std::string& elemento_central);
    void deshacer();
    void recalcular();
};

// ── Panel Dat - Data (Redes Neuronales FFNN estilo HSC) ────────────────
class PanelDatosANN {
public:
    PanelDatosANN() = default;
    ~PanelDatosANN();
    void renderizar();

private:
    friend struct PanelDatosANNTestAcceso;  // acceso para el harness de pruebas headless
    // ── Hoja de datos ──
    std::vector<std::vector<double>> datos_;  // filas × columnas
    std::vector<int> regla_;                  // por columna: 0=X, 1=Y
    std::vector<std::vector<double>> ynn_;    // predicciones (fila × n_sal) tras entrenar

    // ── Hiperparámetros (panel Properties) ──
    double tasa_ = 0.02;
    double decaimiento_ = 1e-6;
    double momento_ = 0.1;
    int act_oculta_ = 0;      // 0=TanH 1=Sigmoid 2=ReLU 3=Linear
    int act_salida_ = 3;      // Linear
    char capas_buf_[64] = "16;16";
    int particion_val_ = 20;  // % validación (100-val = entrenamiento)
    int batch_size_ = 1;      // Batch Size (HSC default = 1)
    double rmse_objetivo_ = 1e-5; // Stopping MSE
    int max_epocas_ = 2500;
    int tipo_entrenamiento_ = 0; // 0=FFNN 1=PSO 2=RNN aleatoria
    int particulas_pso_ = 40;

    // ── Motor y hilo de entrenamiento ──
    motores::RedNeuronal red_;
    motores::ResultadoANN resultado_;    // escrito por el hilo, leído solo al consumir
    motores::ResultadoANN resultado_ui_; // copia estable que lee la UI cada frame
    std::thread hilo_;
    std::atomic<bool> calculando_{false};
    std::atomic<bool> listo_{false};
    std::atomic<bool> cancelar_{false};
    std::atomic<float> progreso_{0.0f};
    bool ejemplo_cargado_ = false;       // el ejemplo inicial se carga una sola vez

    // ── UI ──
    bool ver_propiedades_ = true;
    bool ver_red_ = false;
    bool ver_xml_ = false;
    bool ver_paridad_ = true, ver_rendimiento_ = true, ver_comparar_ = true;
    bool mostrar_validacion_ = true, mostrar_entrenamiento_ = true;
    bool mostrar_nn_comp_ = true, mostrar_orig_comp_ = true;
    int  tab_hoja_ = 0;                  // 0 = Data, 1 = Info
    int  prop_activa_ = 4;               // índice de propiedad para caja de ayuda inferior
    std::string error_;

    // ── Hoja estilo Excel: celda activa, foco teclado y barra de nombre/fórmula ──
    int celda_fila_ = 0;        // fila activa (índice en datos_)
    int celda_colo_ = 0;        // columna activa (índice de columna RULE)
    int foco_fila_ = -1;         // objetivo de foco teclado pendiente (Enter/Tab)
    int foco_colo_ = -1;
    char nombre_celda_buf_[24] = "A1";
    int nombre_fila_prev_ = -1, nombre_colo_prev_ = -1;

    void cargar_ejemplo_hsc();
    void aplicar_datos();
    void importar_csv(const std::string& ruta);
    void exportar_csv(const std::string& ruta);
    void lanzar_entrenamiento();
    void calcular_ynn();
    void dibujar_red(const ImVec2& tam);
};

// ── Panel FIT – NUMERICAL DATA FIT ─────────────────────────────────────────
class PanelDataFit {
public:
    PanelDataFit();
    void renderizar();
    void establecer_flowsheet(diagrama_flujo::Flowsheet* fs) { flowsheet_ = fs; }
private:
    diagrama_flujo::Flowsheet* flowsheet_ = nullptr;
    struct DataPoint { float x; float y; };
    std::vector<DataPoint> data_;

    int func_type_ = 0; // 0=Linear, 1=Polynomial, 2=Rational

    // Regression parameters (A, B, C, D, E, F, G)
    float params_[7] = {1.0f, 1.0f, 1.0f, 1.0f, 1.0f, 1.0f, 1.0f};

    // Results
    float error_ = 0.0f;
    float r2_ = 0.0f;
    float ssr_ = 0.0f;

    // Gradient Descent Settings
    int iterations_max_ = 1000;
    float learning_rate_ = 0.0001f;

    // Input
    void refrescar_datos();
    void fit_curve();
    float evaluate_func(float x) const;
};

class PanelStatisticalAnalysis {
public:
    PanelStatisticalAnalysis();
    void renderizar();
    void establecer_flowsheet(diagrama_flujo::Flowsheet* fs) { flowsheet_ = fs; }
private:
    diagrama_flujo::Flowsheet* flowsheet_ = nullptr;
    struct StatDataPoint {
        int id;
        float al2o3;
        float cr2o3;
        float zn_rec;
        float cu_rec;
        int group; // 0 or 1 for coloring
    };
    std::vector<StatDataPoint> db_;

    // UI state
    int x_axis_col_ = 0; // 0=Al2O3, 1=Cr2O3, 2=Zn, 3=Cu
    int y_axis_col_ = 1;

    // Variance data for PCA mock
    std::vector<float> pca_variance_;
    std::vector<std::string> pca_labels_;

    void refrescar_datos();
};

}} // namespace cma::interfaz
