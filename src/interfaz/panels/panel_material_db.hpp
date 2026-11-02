#ifndef CMA_INTERFAZ_PANEL_MATERIAL_DB_HPP
#define CMA_INTERFAZ_PANEL_MATERIAL_DB_HPP

#include "db/MaterialDB.hpp"
#include "diagrama_flujo/flowsheet.hpp"
#include "domain/ports/ThermoRepository.hpp"
#include <functional>
#include <string>
#include <vector>

namespace cma { namespace interfaz {

class PanelMaterialDB {
public:
    explicit PanelMaterialDB(const domain::ThermoRepository& repository);

    void renderizar(bool* abierto = nullptr);
    void establecer_flowsheet(diagrama_flujo::Flowsheet* fs) { flowsheet_ = fs; }

    cma::application::modules::MaterialDB& db() { return db_; }
    const cma::application::modules::MaterialDB& db() const { return db_; }

    // Permite enviar el material activo al módulo de Balance de Calor y Materiales (Bal)
    std::function<void(const cma::application::modules::RegistroMaterial&, double masa_kg, double temp_c)>
        al_enviar_a_balance;

private:
    const domain::ThermoRepository& repo_;
    cma::application::modules::MaterialDB db_;
    diagrama_flujo::Flowsheet* flowsheet_ = nullptr;

    int idx_material_ = 0;
    int idx_categoria_filtro_ = 0;
    char filtro_busqueda_[128] = "";

    // Estado estilo hoja de cálculo HSC (barra de fórmulas + celda activa)
    std::string ref_celda_activa_ = "A7";
    char buffer_formula_bar_[256] = "";
    int fila_sel_ = 4;   // Fila dentro de la tabla de datos (0-based -> A7 es fila 4 de datos)
    int col_sel_ = 0;    // Columna activa (0=A..7=H)

    // Controles para agregar componente / sub-material / propiedad
    int idx_submaterial_combo_ = 0;
    char nueva_especie_buf_[64] = "";
    double nuevo_wt_pct_ = 5.0;
    char nueva_ref_buf_[128] = "";

    // Controles para nuevo material
    char nuevo_mat_nombre_[128] = "New Concentrate Blend";
    int nuevo_mat_cat_idx_ = 1;
    char nuevo_mat_codigo_[64] = "MAT-NEW-001";
    char nuevo_mat_desc_[256] = "";

    // Controles para propiedades definidas por el usuario
    char prop_usr_nombre_[128] = "Bond Work Index (BWi)";
    double prop_usr_valor_ = 14.2;
    char prop_usr_unidad_[64] = "kWh/t";
    char prop_usr_ref_[128] = "Lab test";

    // Controles para inyección a HSC-Sim Flowsheet / Heat Balance
    double caudal_sim_t_h_ = 100.0;
    double temp_sim_c_ = 25.0;
    int idx_flujo_destino_ = 0;
    char ruta_archivo_io_[256] = "hsc_material_database.tsv";
    char ruta_csv_export_[256] = "smelter_feed_blend.csv";
    std::string mensaje_estado_;
    bool estado_ok_ = true;

    void dibujar_barra_herramientas(cma::application::modules::RegistroMaterial& mat);
    void dibujar_barra_formulas(cma::application::modules::RegistroMaterial& mat,
                                const std::vector<cma::application::modules::EspecieResuelta>& especies,
                                const std::vector<cma::application::modules::ElementoResuelto>& elementos);
    void dibujar_panel_izquierdo();
    void dibujar_hoja_composicion(cma::application::modules::RegistroMaterial& mat,
                                  const std::vector<cma::application::modules::EspecieResuelta>& especies,
                                  const std::vector<cma::application::modules::ElementoResuelto>& elementos);
    void dibujar_pestana_propiedades(cma::application::modules::RegistroMaterial& mat);
    void dibujar_pestana_integracion_sim(const cma::application::modules::RegistroMaterial& mat,
                                         const std::vector<cma::application::modules::EspecieResuelta>& especies,
                                         const std::vector<cma::application::modules::ElementoResuelto>& elementos);
    void inyectar_en_flowsheet(const cma::application::modules::RegistroMaterial& mat,
                               const std::vector<cma::application::modules::EspecieResuelta>& especies,
                               const std::vector<cma::application::modules::ElementoResuelto>& elementos);
};

}} // namespace cma::interfaz

#endif // CMA_INTERFAZ_PANEL_MATERIAL_DB_HPP
