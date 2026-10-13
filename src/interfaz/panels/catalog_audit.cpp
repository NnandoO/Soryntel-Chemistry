#include "scientific.hpp"
#include <imgui.h>
#include <nlohmann/json.hpp>
#include <fstream>

namespace cma::interfaz {
void CatalogAuditPanel::render() {
    ImGui::TextUnformatted("Auditoría de datos termodinámicos");
    ImGui::TextWrapped("Las señales requieren revisión de fuentes, unidades y rangos. No se alteran valores ni se certifican datos automáticamente.");
    if (!loaded_ || ImGui::Button("Actualizar auditoría")) { report_=application::audit_catalog(repository_); loaded_=true; }
    ImGui::Text("Registros: %zu · Sin verificación: %zu",report_.records,report_.unverified);
    ImGui::Text("Sin referencia: %zu · Sin rango de temperatura: %zu",report_.missing_references,report_.unknown_ranges);
    ImGui::Text("Señales de revisión: %zu",report_.issues.size());
    if (ImGui::Button("Exportar informe JSON")) {
        const auto path=dialog_.select("Guardar auditoría","*.json",true);
        if (!path.empty()) {
            nlohmann::json json={{"records",report_.records},{"unverified",report_.unverified},{"issues",nlohmann::json::array()}};
            for (const auto& item : report_.issues) json["issues"].push_back({{"id",item.id},{"code",item.code},{"detail",item.detail}});
            std::ofstream output(path); output<<json.dump(2)<<'\n'; output.close();
            message_=output?"Informe guardado.":"No se pudo guardar el informe; revise la ruta y los permisos.";
        }
    }
    if (!message_.empty()) ImGui::TextWrapped("%s",message_.c_str());
    if (ImGui::BeginTable("catalog_issues",3,ImGuiTableFlags_Borders|ImGuiTableFlags_RowBg|ImGuiTableFlags_ScrollY,ImVec2(0,300))) {
        ImGui::TableSetupColumn("ID"); ImGui::TableSetupColumn("Revisión"); ImGui::TableSetupColumn("Detalle"); ImGui::TableHeadersRow();
        ImGuiListClipper clipper; clipper.Begin(static_cast<int>(report_.issues.size()));
        while (clipper.Step()) for (int i=clipper.DisplayStart;i<clipper.DisplayEnd;++i) {
            const auto& issue=report_.issues[static_cast<size_t>(i)];
            ImGui::TableNextRow(); ImGui::TableNextColumn(); ImGui::Text("%lld",static_cast<long long>(issue.id));
            ImGui::TableNextColumn(); ImGui::TextUnformatted(issue.code.c_str());
            ImGui::TableNextColumn(); ImGui::TextUnformatted(issue.detail.c_str());
        }
        ImGui::EndTable();
    }
}
}
