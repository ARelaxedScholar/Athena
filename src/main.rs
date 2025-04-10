/// Evaluates the performance of a given population of portfolios over multiple simulations.
///
/// Calculates per-portfolio average metrics and population-wide summary statistics.
///
/// # Arguments
/// * `population`: A slice of weight vectors representing the portfolios to evaluate.
/// * `config`: The base configuration containing simulation parameters and the sampler.
///
/// # Returns
/// A `PopulationEvaluationResult` struct. Returns default/empty values if the input population is empty.
fn evaluate_population_performance(
    population: &[Vec<f64>],
    config: &StandardEvolutionConfig,
) -> PopulationEvaluationResult {
    let population_size = population.len();
    if population_size == 0 {
        panic!(
            "I am pretty sure I got checks for that already, but we shouldn't be here with a population of 0."
        );
    }

    let simulations_per_generation = config.simulations_per_generation;

    // Initialize accumulators
    let mut accumulated_returns = vec![0.0; population_size];
    let mut accumulated_volatilities = vec![0.0; population_size];
    let mut accumulated_sharpe_ratios = vec![0.0; population_size];
    let mut last_scenario_returns: Vec<Vec<f64>> = vec![];

    // --- Simulation Loop ---
    for i in 0..simulations_per_generation {
        let scenario_returns = config.sampler.sample_returns();
        if i == simulations_per_generation - 1 {
            last_scenario_returns = scenario_returns.clone();
        }

        let performance_metrics_in_scenario: Vec<(f64, f64, f64)> = population
            .par_iter()
            .map(|portfolio_weights| {
                let performance = compute_portfolio_performance(
                    &scenario_returns,
                    portfolio_weights,
                    config.money_to_invest,
                    config.risk_free_rate,
                    config.time_horizon_in_days as f64,
                );
                (
                    performance.annualized_return,
                    performance.percent_annualized_volatility,
                    performance.sharpe_ratio,
                )
            })
            .collect();

        for (idx, (ret, vol, sharpe)) in performance_metrics_in_scenario.iter().enumerate() {
            accumulated_returns[idx] += ret;
            accumulated_volatilities[idx] += vol;
            accumulated_sharpe_ratios[idx] += sharpe;
        }
    } // --- End of Simulation Loop ---

    // --- Calculate Per-Portfolio Averages ---
    let sim_count_f64 = simulations_per_generation as f64;
    // These vectors hold the average performance for each *individual* portfolio
    let average_returns: Vec<f64> = accumulated_returns
        .par_iter()
        .map(|&sum| sum / sim_count_f64)
        .collect();
    let average_volatilities: Vec<f64> = accumulated_volatilities
        .par_iter()
        .map(|&sum| sum / sim_count_f64)
        .collect();
    let average_sharpe_ratios: Vec<f64> = accumulated_sharpe_ratios
        .par_iter()
        .map(|&sum| sum / sim_count_f64)
        .collect();

    // --- Calculate Population-Wide Summary Statistics ---
    let population_size_f64 = population_size as f64;

    // Use the calculated per-portfolio averages to get population stats
    let best_return = average_returns
        .par_iter()
        .fold(|| f64::NEG_INFINITY, |a, &b| a.max(b))
        .reduce(|| f64::NEG_INFINITY, |a, b| a.max(b));
    let population_average_return = average_returns.par_iter().sum::<f64>() / population_size_f64;

    let best_volatility = average_volatilities
        .par_iter()
        .fold(|| f64::INFINITY, |a, &b| a.min(b))
        .reduce(|| f64::INFINITY, |a, b| a.min(b));
    let population_average_volatility =
        average_volatilities.par_iter().sum::<f64>() / population_size_f64;

    let best_sharpe = average_sharpe_ratios
        .par_iter()
        .fold(|| f64::NEG_INFINITY, |a, &b| a.max(b))
        .reduce(|| f64::NEG_INFINITY, |a, b| a.max(b));
    let population_average_sharpe =
        average_sharpe_ratios.par_iter().sum::<f64>() / population_size_f64;

    // --- Return Results ---
    PopulationEvaluationResult {
        average_returns, // Per-portfolio averages
        average_volatilities,
        average_sharpe_ratios,
        last_scenario_returns, // Data from last sim run
        best_return,           // Population summary stats
        population_average_return,
        best_volatility,
        population_average_volatility,
        best_sharpe,
        population_average_sharpe,
    }
}

fn main() {}
