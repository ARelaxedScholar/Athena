use rayon::prelude::*;
use tonic::{Request, Response, Status};
use aegis_athena_contracts::simulation::simulation_service_server::{SimulationService, SimulationServiceServer};
use aegis_athena_contracts::simulation::{SimulationBatchRequest, SimulationBatchResult, SimulationScenario, EvolutionConfig, Portfolio};
use aegis_athena_contracts::common_portfolio_evolution_ds::compute_portfolio_performance; 
use aegis_athena_contracts::sampling::Sampler;                        

#[derive(Clone)]
pub struct SimulationServiceImpl {
    pub sampler: Sampler,
}

#[tonic::async_trait]
impl simulation::simulation_service_server::SimulationService for SimulationServiceImpl {
    async fn run_batch(
        &self,
        request: Request<SimulationBatchRequest>,
    ) -> Result<Response<SimulationBatchResult>, Status> {
        // Extract the batch request
        let req = request.into_inner();
        
        // Deserialize the portfolios blob using bincode.
        // The blob is assumed to be a bincode serialized Vec<Portfolio>.
        let portfolios: Vec<Portfolio> = bincode::deserialize(&req.portfolios_blob)
            .map_err(|e| Status::invalid_argument(format!("Failed to deserialize portfolios: {}", e)))?;
        let config = req.config; // Using config if needed; otherwise, you can ignore it.
        let iterations = req.iterations as usize;

        // Prepare accumulators
        let n = portfolios.len();
        let mut sum_returns = vec![0.0; n];
        let mut sum_vols = vec![0.0; n];
        let mut sum_sharpes = vec![0.0; n];
        let mut last_scenario = Vec::new();

        // Clone the sampler to use within the blocking task.
        let sampler = self.sampler.clone();

        // Run the batch *synchronously*, but wrap in spawn_blocking to avoid blocking the async executor.
        let (sr, sv, ss, ls) = tokio::task::spawn_blocking(move || {
            for i in 0..iterations {
                // sample scenario
                let scenario_returns = sampler.sample_returns();
                if i == iterations - 1 {
                    last_scenario = scenario_returns.clone();
                }

                // parallel evaluation of all portfolios
                let metrics: Vec<(f64, f64, f64)> = portfolios
                    .par_iter()
                    .map(|p| {
                        let perf = compute_portfolio_performance(
                            &scenario_returns,
                            &p.weights,
                            config.money_to_invest,
                            config.risk_free_rate,
                            config.time_horizon_in_days,
                        );
                        (perf.annualized_return, perf.percent_annualized_volatility, perf.sharpe_ratio)
                    })
                    .collect();

                // accumulate
                for (idx, (r, v, s)) in metrics.into_iter().enumerate() {
                    sum_returns[idx] += r;
                    sum_vols[idx]    += v;
                    sum_sharpes[idx] += s;
                }
            }
            (sum_returns, sum_vols, sum_sharpes, last_scenario)
        })
        .await
        .map_err(|e| Status::internal(format!("batch panicked: {}", e)))?;

        // Build the gRPC response
        let reply = PopulationPartialResult {
            sum_returns: sr,
            sum_volatilities: sv,
            sum_sharpes: ss,
            last_scenario: SimulationScenario { returns: ls },
        };
        Ok(Response::new(reply))
    }
}
