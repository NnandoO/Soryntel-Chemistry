#pragma once

#include <string>
#include <unordered_map>
#include <memory>
#include <vector>

namespace utils {

class I18n {
public:
    enum class Idioma {
        ESPANOL,
        INGLES
    };

    static I18n& instancia() {
        static I18n instancia_;
        return instancia_;
    }

    // Configurar idioma
    void cambiar_idioma(Idioma idioma) {
        idioma_actual_ = idioma;
    }

    Idioma idioma_actual() const {
        return idioma_actual_;
    }

    // Traducir cadena
    const char* traducir(const char* texto_original) const;

    // Obtener nombres de idiomas disponibles
    std::vector<std::string> idiomas_disponibles() const {
        return {"Español", "English"};
    }

private:
    I18n();
    ~I18n() = default;

    Idioma idioma_actual_;
    std::unordered_map<std::string, std::string> dict_en_;
};

} // namespace utils

// Macro helper
#define tr(text) utils::I18n::instancia().traducir(text)
