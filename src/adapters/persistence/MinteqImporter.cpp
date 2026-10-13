#include "MinteqImporter.hpp"
#include "csv.hpp"
#include <filesystem>
#include <fstream>
#include <set>
#include <sstream>
namespace cma::adapters {
namespace {
std::string trim(std::string value) {
    const auto start = value.find_first_not_of(" \t\r\n");
    if (start == std::string::npos) return {};
    return value.substr(start,value.find_last_not_of(" \t\r\n") - start + 1);
}
int integer(const std::string& value) {
    const double number = csv_number(value);
    if (number != std::floor(number) || number < 0 || number > 1000000000)
        throw std::invalid_argument("ID/count VDB inválido");
    return static_cast<int>(number);
}
std::vector<std::string> row(std::ifstream& input) {
    std::string line;
    while (std::getline(input,line)) if (!trim(line).empty()) {
        auto fields = csv_row(line);
        for (auto& value : fields) value = trim(value);
        return fields;
    }
    return {};
}
void components(const std::filesystem::path& path,domain::AqueousDatabase& catalog) {
    std::ifstream input(path);
    if (!input) throw std::invalid_argument("No se pudo abrir " + path.string());
    while (true) {
        const auto fields = row(input);
        if (fields.empty()) break;
        if (fields.size() < 6) throw std::invalid_argument("Registro de componente incompleto");
        const int id = integer(fields[0]);
        if (id == 0) break;
        domain::AqueousComponent component;
        component.id = id; component.name = fields[1];
        component.charge = csv_number(fields[2]); component.molar_mass = csv_number(fields[5]);
        if (component.name.empty() || component.molar_mass < 0 || !catalog.components.emplace(id,component).second)
            throw std::invalid_argument("Componente VDB inválido o duplicado");
    }
    if (input.bad() || catalog.components.empty()) throw std::invalid_argument("Componentes VDB vacíos/corruptos");
}
void reactions(const std::filesystem::path& path,domain::AqueousPhase phase,domain::AqueousDatabase& catalog) {
    std::ifstream input(path);
    if (!input) throw std::invalid_argument("No se pudo abrir " + path.string());
    while (true) {
        const auto header = row(input);
        if (header.empty()) break;
        if (header.size() < 6) throw std::invalid_argument("Cabecera de reacción VDB incompleta");
        const int id = integer(header[0]);
        if (id == 0) break;
        const auto stoichiometry = row(input), source = row(input);
        const int count = integer(header.back());
        if (count <= 0 || count > 100 || stoichiometry.size() != static_cast<size_t>(count * 2) || source.size() < 3)
            throw std::invalid_argument("Estequiometría/fuente VDB incompleta, id=" + header[0]);
        domain::FormationReaction reaction;
        reaction.id = id; reaction.name = header[1]; reaction.phase = phase;
        reaction.reaction_enthalpy_kJ = csv_number(header[2]); reaction.log_k_298 = csv_number(header[3]);
        reaction.molar_mass = csv_number(source[0]);
        reaction.source_log_k = source[1]; reaction.source_enthalpy = source[2];
        reaction.enthalpy_available = source[2] != "n.a." && source[2] != "";
        reaction.source_file = path.filename().string();
        bool missing = false;
        for (int term = 0; term < count; ++term) {
            const double coefficient = csv_number(stoichiometry[static_cast<size_t>(2*term)]);
            const int component = integer(stoichiometry[static_cast<size_t>(2*term+1)]);
            if (!catalog.components.count(component)) missing = true;
            reaction.components[component] += coefficient;
            if (!missing) reaction.charge += coefficient * catalog.components.at(component).charge;
        }
        if (missing || reaction.name.empty() || reaction.molar_mass < 0) {
            catalog.diagnostics.push_back("Reacción " + header[0] + " excluida: componente/fórmula/masa inválida");
            continue;
        }
        if (catalog.reactions.count(id)) {
            const auto& previous = catalog.reactions.at(id);
            if (previous.components != reaction.components || previous.log_k_298 != reaction.log_k_298)
                catalog.diagnostics.push_back("Reacción " + header[0] + " duplicada/conflictiva; se conserva primera fuente");
            continue;
        }
        catalog.reactions.emplace(id,std::move(reaction));
    }
    if (input.bad()) throw std::invalid_argument("Lectura VDB falló");
}
}
bool MinteqImporter::load(const std::string& directory,domain::AqueousDatabase& destination) {
    error_.clear();
    try {
        domain::AqueousDatabase parsed;
        const std::filesystem::path path(directory);
        components(path / "comp_2008.vdb",parsed);
        // Explicit phase files take precedence over duplicated records in thermo.
        reactions(path / "type6.vdb",domain::AqueousPhase::Solid,parsed);
        reactions(path / "gases.vdb",domain::AqueousPhase::Gas,parsed);
        reactions(path / "thermo.vdb",domain::AqueousPhase::Dissolved,parsed);
        if (parsed.reactions.empty()) throw std::invalid_argument("Catálogo de reacciones vacío");
        destination = std::move(parsed);
        return true;
    } catch (const std::exception& e) { error_ = e.what(); return false; }
}
}
