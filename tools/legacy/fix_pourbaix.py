import re

# 1. Fix diagramas.cpp
with open("src/motores/diagramas.cpp", "r") as f:
    content = f.read()

old_g_eff = "double g_eff = (c.G_T - c.o * mu_H2O - (c.h - 2.0*c.o) * mu_H - c.z * mu_e) / c.m;"
new_g_eff = "double g_eff = (c.G_T - c.o * mu_H2O - (c.h - 2.0*c.o) * mu_H - (c.h - 2.0*c.o - c.z) * mu_e) / c.m;"

content = content.replace(old_g_eff, new_g_eff)

with open("src/motores/diagramas.cpp", "w") as f:
    f.write(content)

# 2. Fix panel_pourbaix.cpp modal opening
with open("src/interfaz/panel_pourbaix.cpp", "r") as f:
    content = f.read()

# Remove the broken OpenPopup calls
content = content.replace("""    if (show_select_element_modal_) {
        ImGui::OpenPopup("Select Element##Modal");
    }
""", "")

content = content.replace("""    if (show_select_species_modal_) {
        ImGui::OpenPopup("Select Species##Modal");
    }
""", "")

# Add OpenPopup inside the button handlers
old_btn_element = """    if (ImGui::Button("Select Element", ImVec2(-1, 0))) {
        show_select_element_modal_ = true;
    }"""
new_btn_element = """    if (ImGui::Button("Select Element", ImVec2(-1, 0))) {
        show_select_element_modal_ = true;
        ImGui::OpenPopup("Select Element##Modal");
    }"""
content = content.replace(old_btn_element, new_btn_element)

old_btn_species = """    if (ImGui::Button("Select Species", ImVec2(-1, 0))) {
        show_select_species_modal_ = true;
    }"""
new_btn_species = """    if (ImGui::Button("Select Species", ImVec2(-1, 0))) {
        show_select_species_modal_ = true;
        ImGui::OpenPopup("Select Species##Modal");
    }"""
content = content.replace(old_btn_species, new_btn_species)

with open("src/interfaz/panel_pourbaix.cpp", "w") as f:
    f.write(content)

