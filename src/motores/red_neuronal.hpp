#pragma once
// Red neuronal feedforward (FFNN) + optimizadores (Backprop, PSO, aleatorio)
// Módulo Dat - Data (estilo HSC Chemistry)
#include <vector>
#include <string>
#include <functional>
#include <atomic>

namespace cma { namespace motores {

struct ConfigANN {
    std::vector<int> ocultas = {16, 16};  // neuronas por capa oculta
    int act_oculta = 0;                   // 0=TanH 1=Sigmoid 2=ReLU 3=Linear
    int act_salida = 3;                   // Linear
    double tasa = 0.02;                   // learning rate
    double decaimiento = 1e-6;            // learning rate decay
    double momento = 0.1;
    int batch_size = 1;                   // tamaño de mini-lote (HSC default = 1)
    int semilla = 12345;
    int tipo = 0;                         // 0=FFNN backprop 1=PSO 2=RNN aleatoria
    int particulas_pso = 40;
    bool normalizar = true;               // escalar X, Y a [-0.9, 0.9] como HSC Data
};

struct ResultadoANN {
    bool ok = false;
    bool cancelado = false;
    std::string mensaje;
    std::vector<double> epoca_hist;       // época de cada punto registrado
    std::vector<double> rmse_ent_hist;    // RMSE entrenamiento
    std::vector<double> rmse_val_hist;    // RMSE validación
    std::vector<double> mse_ent_hist;     // MSE entrenamiento (escala [-0.9, 0.9])
    std::vector<double> mse_val_hist;     // MSE validación (escala [-0.9, 0.9])
    double rmse_ent = 0.0;
    double rmse_val = 0.0;
    double mse_ent = 0.0;
    double mse_val = 0.0;
    int epocas = 0;
};

class RedNeuronal {
public:
    void inicializar(int n_entrada, const ConfigANN& cfg, int n_salida);
    std::vector<double> predecir(const std::vector<double>& entrada) const;
    std::vector<double> predecir_normalizado(const std::vector<double>& entrada) const;
    std::vector<double> normalizar_y(const std::vector<double>& y_raw) const;

    ResultadoANN entrenar(const std::vector<std::vector<double>>& X,
                          const std::vector<std::vector<double>>& Y,
                          const std::vector<int>& idx_entrenamiento,
                          const std::vector<int>& idx_validacion,
                          double mse_objetivo,
                          int max_epocas,
                          const std::function<void(float)>& progreso,
                          const std::atomic<bool>& cancelar);

    bool lista() const { return !W_.empty(); }
    int n_entrada() const { return n_ent_; }
    int n_salida() const { return n_sal_; }
    std::vector<int> topologia() const;
    const ConfigANN& config() const { return cfg_; }

    // Serialización (exportar/importar red entrenada en JSON y XML estilo HSC)
    std::string a_json() const;
    bool desde_json(const std::string& texto);
    std::string a_xml() const;

private:
    double activar(double v, int tipo) const;
    double activar_deriv(double v, double y, int tipo) const;

    std::vector<double> normalizar_x(const std::vector<double>& x_raw) const;
    std::vector<double> desnormalizar_y(const std::vector<double>& y_norm) const;

    size_t total_params() const;
    std::vector<double> obtener_params() const;
    void fijar_params(const std::vector<double>& p);

    // forward que además guarda activaciones (para backprop)
    void forward(const std::vector<double>& x, std::vector<std::vector<double>>& a) const;

    ConfigANN cfg_;
    int n_ent_ = 0, n_sal_ = 0;
    std::vector<std::vector<std::vector<double>>> W_; // capa → salida → entrada
    std::vector<std::vector<double>> B_;              // capa → sesgo

    // Escalado min-max a [-0.9, 0.9]
    bool normalizado_ = false;
    std::vector<double> x_min_, x_max_;
    std::vector<double> y_min_, y_max_;
};

}} // namespace
