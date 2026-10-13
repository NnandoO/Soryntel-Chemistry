import re

with open("src/interfaz/panel_pourbaix.cpp", "r") as f:
    content = f.read()

# Revert the OpenPopup inside the button
old_btn_element = """    if (ImGui::Button("Select Element", ImVec2(-1, 0))) {
        show_select_element_modal_ = true;
        ImGui::OpenPopup("Select Element##Modal");
    }"""
new_btn_element = """    if (ImGui::Button("Select Element", ImVec2(-1, 0))) {
        show_select_element_modal_ = true;
    }"""
content = content.replace(old_btn_element, new_btn_element)

old_btn_species = """    if (ImGui::Button("Select Species", ImVec2(-1, 0))) {
        show_select_species_modal_ = true;
        ImGui::OpenPopup("Select Species##Modal");
    }"""
new_btn_species = """    if (ImGui::Button("Select Species", ImVec2(-1, 0))) {
        show_select_species_modal_ = true;
    }"""
content = content.replace(old_btn_species, new_btn_species)

# Insert the trigger OpenPopup just before BeginPopupModal at the root level
old_modal_element = """    ImGui::SetNextWindowSize(ImVec2(800, 600), ImGuiCond_FirstUseEver);
    if (ImGui::BeginPopupModal("Select Element##Modal", &show_select_element_modal_)) {"""
new_modal_element = """    if (show_select_element_modal_ && !ImGui::IsPopupOpen("Select Element##Modal")) {
        ImGui::OpenPopup("Select Element##Modal");
    }
    ImGui::SetNextWindowSize(ImVec2(800, 600), ImGuiCond_FirstUseEver);
    if (ImGui::BeginPopupModal("Select Element##Modal", &show_select_element_modal_)) {"""
content = content.replace(old_modal_element, new_modal_element)

old_modal_species = """    ImGui::SetNextWindowSize(ImVec2(400, 500), ImGuiCond_FirstUseEver);
    if (ImGui::BeginPopupModal("Select Species##Modal", &show_select_species_modal_)) {"""
new_modal_species = """    if (show_select_species_modal_ && !ImGui::IsPopupOpen("Select Species##Modal")) {
        ImGui::OpenPopup("Select Species##Modal");
    }
    ImGui::SetNextWindowSize(ImVec2(400, 500), ImGuiCond_FirstUseEver);
    if (ImGui::BeginPopupModal("Select Species##Modal", &show_select_species_modal_)) {"""
content = content.replace(old_modal_species, new_modal_species)

with open("src/interfaz/panel_pourbaix.cpp", "w") as f:
    f.write(content)
print("Popups fixed.")
