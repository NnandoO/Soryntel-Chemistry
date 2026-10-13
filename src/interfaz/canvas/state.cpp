#include "shared.hpp"
namespace cma { namespace interfaz {
Lienzo::Lienzo() {}

void Lienzo::guardar_estado() {
    if (flowsheet_) {
        undo_stack_.push_back(flowsheet_->a_json());
        // Limitar el historial a 50 estados para no consumir excesiva memoria
        if (undo_stack_.size() > 50)
            undo_stack_.erase(undo_stack_.begin());
        redo_stack_.clear();
        if (al_cambio_flowsheet_) al_cambio_flowsheet_();
    }
}

void Lienzo::deshacer() {
    if (flowsheet_ && !undo_stack_.empty()) {
        redo_stack_.push_back(flowsheet_->a_json());
        std::string estado = undo_stack_.back();
        undo_stack_.pop_back();
        std::string err;
        flowsheet_->desde_json(estado, err);
        id_nodo_seleccionado = -1;
        id_flujo_seleccionado = -1;
        id_anotacion_seleccionada = -1;
        if (al_cambio_flowsheet_) al_cambio_flowsheet_();
    }
}

void Lienzo::rehacer() {
    if (flowsheet_ && !redo_stack_.empty()) {
        undo_stack_.push_back(flowsheet_->a_json());
        std::string estado = redo_stack_.back();
        redo_stack_.pop_back();
        std::string err;
        flowsheet_->desde_json(estado, err);
        id_nodo_seleccionado = -1;
        id_flujo_seleccionado = -1;
        id_anotacion_seleccionada = -1;
        if (al_cambio_flowsheet_) al_cambio_flowsheet_();
    }
}

void Lienzo::limpiar_historial() {
    undo_stack_.clear();
    redo_stack_.clear();
}

void Lienzo::copiar() {
    if (id_nodo_seleccionado >= 0 && flowsheet_) {
        if (auto* n = flowsheet_->nodo_por_id(id_nodo_seleccionado)) {
            portapapeles_tipo_ = n->tipo;
            tiene_portapapeles_ = true;
        }
    }
}

void Lienzo::cortar() {
    copiar();
    if (id_nodo_seleccionado >= 0 && flowsheet_) {
        guardar_estado();
        flowsheet_->eliminar_nodo(id_nodo_seleccionado);
        id_nodo_seleccionado = -1;
    }
}

void Lienzo::pegar() {
    if (tiene_portapapeles_ && flowsheet_) {
        guardar_estado();
        int64_t nuevo_id = flowsheet_->agregar_nodo(portapapeles_tipo_);
        if (auto* n = flowsheet_->nodo_por_id(nuevo_id)) {
            n->canvas_x = coord_raton.x - NODO_ANCHO/2;
            n->canvas_y = coord_raton.y - NODO_ALTO/2;
            n->pagina = pagina_actual;
            id_nodo_seleccionado = nuevo_id;
        }
    }
}

void Lienzo::establecer_flowsheet(diagrama_flujo::Flowsheet* fs) {
    flowsheet_ = fs;
}

void Lienzo::centrar_en(float x, float y) {
    des_destino_ = ImVec2(-x, -y);
    animando_ = true;
}

void Lienzo::encajar_todo() {
    if (!flowsheet_ || flowsheet_->nodos().empty()) return;

    float xmin = 1e9, ymin = 1e9, xmax = -1e9, ymax = -1e9;
    bool has_nodes = false;
    for (auto& n : flowsheet_->nodos()) {
        if (n.pagina != pagina_actual) continue;
        has_nodes = true;
        xmin = std::min(xmin, n.canvas_x);
        ymin = std::min(ymin, n.canvas_y);
        xmax = std::max(xmax, n.canvas_x + NODO_ANCHO);
        ymax = std::max(ymax, n.canvas_y + NODO_ALTO);
    }
    if (!has_nodes) return;

    ImVec2 tam_lienzo = ultima_region_;
    if (tam_lienzo.x < 1 || tam_lienzo.y < 1) return;

    float cx = (xmin + xmax) * 0.5f;
    float cy = (ymin + ymax) * 0.5f;
    float escala_x = tam_lienzo.x / (xmax - xmin + 100);
    float escala_y = tam_lienzo.y / (ymax - ymin + 100);
    zoom = std::clamp(std::min(escala_x, escala_y), 0.1f, 5.0f);

    des_destino_ = ImVec2(tam_lienzo.x * 0.5f - cx * zoom,
                          tam_lienzo.y * 0.5f - cy * zoom);
    animando_ = true;
}


}} // namespace
