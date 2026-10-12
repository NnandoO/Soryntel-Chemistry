#include "i18n.h"

namespace utils {

I18n::I18n() : idioma_actual_(Idioma::ESPANOL) {
    // Diccionario de Español -> Inglés
    
    // Menú Archivo
    dict_en_["Archivo"] = "File";
    dict_en_["Nueva simulación"] = "New Simulation";
    dict_en_["Cargar ejemplo##sub"] = "Load Example##sub";
    dict_en_["SMR - Reformado de Metano"] = "SMR - Methane Reforming";
    dict_en_["Flotación de Cobre"] = "Copper Flotation";
    dict_en_["Alto Horno de Hierro"] = "Iron Blast Furnace";
    dict_en_["Abrir proyecto (.cma)"] = "Open Project (.cma)";
    dict_en_["Guardar proyecto (.cma)"] = "Save Project (.cma)";
    dict_en_["Guardar como..."] = "Save As...";
    dict_en_["Salir"] = "Exit";

    // Menú Simulación
    dict_en_["Simulación"] = "Simulation";
    dict_en_["Ejecutar"] = "Run";
    dict_en_["Resolver Recirculaciones"] = "Solve Recycles";
    dict_en_["Minimizar Gibbs"] = "Gibbs Minimization";
    dict_en_["Encajar Vista"] = "Fit to Screen";
    dict_en_["Limpiar resultados"] = "Clear Results";

    // Menú Herramientas
    dict_en_["Herramientas"] = "Tools";
    dict_en_["Calculadora termodinámica"] = "Thermodynamic Calculator";
    dict_en_["Electroquímica"] = "Electrochemistry";
    dict_en_["Cinética química"] = "Chemical Kinetics";

    // Menú Módulos (the ones not already in English)
    dict_en_["Módulos"] = "Modules";
    
    // Menú Ver & Ayuda
    dict_en_["Ver"] = "View";
    dict_en_["Modo Claro"] = "Light Mode";
    dict_en_["Modo Oscuro"] = "Dark Mode";
    dict_en_["Ayuda"] = "Help";
    dict_en_["Manual de usuario"] = "User Manual";
    dict_en_["Acerca de CMA Chemistry"] = "About CMA Chemistry";
    
    // Paneles y Ventanas
    dict_en_["Panel Izquierdo"] = "Left Panel";
    dict_en_["Resultados"] = "Results";
    dict_en_["Propiedades"] = "Properties";
    dict_en_["Caja de Herramientas"] = "Toolbox";
    dict_en_["Lienzo"] = "Canvas";
    
    // Pestañas
    dict_en_["Unidades (Nodos)"] = "Units (Nodes)";
    dict_en_["Corrientes (Flujos)"] = "Streams (Flows)";
    dict_en_["Registro de Simulación:"] = "Simulation Log:";
    dict_en_["Simulación lista."] = "Simulation ready.";
    dict_en_["Haga clic en 'Play' en la barra superior."] = "Click 'Play' on the top bar.";
    
    // General
    dict_en_["Idioma"] = "Language";
    dict_en_["Español"] = "Spanish";
    dict_en_["Inglés"] = "English";
    
    // Botones
    dict_en_["Aceptar"] = "OK";
    dict_en_["Cancelar"] = "Cancel";
    dict_en_["Cerrar"] = "Close";
    dict_en_["Calcular"] = "Calculate";
}

const char* I18n::traducir(const char* texto_original) const {
    if (idioma_actual_ == Idioma::ESPANOL) {
        return texto_original;
    } else if (idioma_actual_ == Idioma::INGLES) {
        auto it = dict_en_.find(texto_original);
        if (it != dict_en_.end()) {
            return it->second.c_str();
        }
        // Si no encuentra la traducción, devuelve el original (útil para cadenas ya en inglés)
        return texto_original;
    }
    return texto_original;
}

} // namespace utils
