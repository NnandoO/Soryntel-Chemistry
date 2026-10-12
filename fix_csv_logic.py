import re

with open("src/base_datos/gestor_db.cpp", "r") as f:
    content = f.read()

# First, remove the previously injected method which is outside the namespace
pattern = re.compile(r"bool GestorDB::cargar_desde_csv\(const std::string& ruta\) \{.*?return true;\n}", re.DOTALL)
content = pattern.sub("", content)

# Remove the previously injected headers which might be duplicate
content = content.replace("#include \"gestor_db.hpp\"\n#include <fstream>\n#include <sstream>\n#include <iostream>\n", "#include \"gestor_db.hpp\"\n")

# Now inject headers properly at the top
content = content.replace("#include \"gestor_db.hpp\"", "#include \"gestor_db.hpp\"\n#include <fstream>\n#include <sstream>\n#include <iostream>")

new_method = """
bool GestorDB::cargar_desde_csv(const std::string& ruta) {
    std::ifstream archivo(ruta);
    if (!archivo.is_open()) {
        std::cerr << "Error al abrir CSV: " << ruta << std::endl;
        return false;
    }
    
    std::string linea;
    bool primera_linea = true;
    
    // Asumimos un ID dinámico que empiece después de los hardcodeados
    int64_t next_id = 10000;
    if (!especies_.empty()) {
        next_id = especies_.rbegin()->first + 1;
    }
    
    while (std::getline(archivo, linea)) {
        if (primera_linea) {
            primera_linea = false;
            // Omitir cabecera si existe
            continue;
        }
        
        if (linea.empty() || linea[0] == '#' || linea[0] == '/') continue;
        
        std::stringstream ss(linea);
        std::string token;
        std::vector<std::string> columnas;
        
        while (std::getline(ss, token, ',')) {
            // Eliminar espacios al inicio y final
            size_t start = token.find_first_not_of(" \\r\\n\\t");
            size_t end = token.find_last_not_of(" \\r\\n\\t");
            if (start != std::string::npos && end != std::string::npos) {
                columnas.push_back(token.substr(start, end - start + 1));
            } else {
                columnas.push_back("");
            }
        }
        
        if (columnas.size() >= 6) {
            nucleo::Especie esp;
            esp.id = next_id++;
            esp.nombre = columnas[0];
            esp.formula = columnas[1];
            try {
                esp.masa_molar = std::stod(columnas[2]);
                esp.h_formacion = std::stod(columnas[3]);
                esp.s_formacion = std::stod(columnas[4]);
                int f = std::stoi(columnas[5]);
                if (f == 0) esp.fase_std = nucleo::Fase::Gas;
                else if (f == 1) esp.fase_std = nucleo::Fase::Liquido;
                else esp.fase_std = nucleo::Fase::Solido;
                
                especies_[esp.id] = esp;
                nombre_a_id_especie_[esp.nombre] = esp.id;
            } catch (const std::exception& e) {
                std::cerr << "Error parseando CSV en linea: " << linea << std::endl;
            }
        }
    }
    
    return true;
}
"""

# Insert inside namespace cma { namespace base_datos {
content = content.replace("} // namespace base_datos", new_method + "\n} // namespace base_datos")

with open("src/base_datos/gestor_db.cpp", "w") as f:
    f.write(content)
print("CSV logic fixed")
