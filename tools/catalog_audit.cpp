#include "base_datos/gestor_db.hpp"
#include "adapters/persistence/MinteqImporter.hpp"
#include "application/CatalogAudit.hpp"
#include <nlohmann/json.hpp>
#include <iostream>
int main(int argc,char** argv) {
    cma::base_datos::GestorDB repository;
    const auto report=cma::application::audit_catalog(repository);
    nlohmann::json output={{"records",report.records},{"unverified",report.unverified},
        {"missing_references",report.missing_references},{"unknown_temperature_ranges",report.unknown_ranges},
        {"issue_count",report.issues.size()},{"issues",nlohmann::json::array()}};
    for (const auto& issue : report.issues) output["issues"].push_back({{"id",issue.id},{"code",issue.code},{"detail",issue.detail}});
    if (argc>1) {
        cma::adapters::MinteqImporter importer;
        cma::domain::AqueousDatabase catalog;
        const bool ok=importer.load(argv[1],catalog);
        output["minteq"]={{"loaded",ok},{"components",catalog.components.size()},{"reactions",catalog.reactions.size()},
            {"diagnostics",catalog.diagnostics},{"error",importer.error()}};
        if (!ok) { std::cout<<output.dump(2)<<'\n'; return 1; }
    }
    std::cout<<output.dump(2)<<'\n';
    return 0; // Findings are review signals, not fabricated corrections.
}
