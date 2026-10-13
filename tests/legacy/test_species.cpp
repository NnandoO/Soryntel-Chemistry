#include <iostream>
#include <string>
#include <vector>
#include "src/base_datos/gestor_db.hpp"
#include "src/nucleo/tipos.hpp"

int main() {
    auto especies = cma::base_datos::GestorDB::instancia().todas_especies();
    std::cout << "Total especies: " << especies.size() << std::endl;
    int count = 0;
    std::string elemento_ = "Fe";
    for (auto* esp : especies) {
        if (esp->formula.find(elemento_) != std::string::npos) {
            count++;
            std::cout << "Match: " << esp->formula << std::endl;
        }
    }
    std::cout << "Total matches for " << elemento_ << ": " << count << std::endl;
    return 0;
}
