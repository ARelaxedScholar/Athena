use rayon::prelude::*;
use tonic::{Request, Response, Status};
use simulation::{SimulationBatchRequest, PopulationPartialResult, SimulationScenario};
use crate::compute_portfolio_performance; // your existing fn
use crate::Sampler;                        // your sampler type

#[derive(Clone)]
pub struct SimulationServiceImpl {
    pub sampler: Sampler,
}

#[tonic::async_trait]
impl simulation::simulation_service_server::SimulationService for SimulationServiceImpl {
    async fn run_batch(
        &self,
        request: Request<SimulationBatchRequest>,
    ) -> Result<Response<PopulationPartialResult>, Status> {
        let req = request.into_inner();
        let portfolios = req.portfolios;
        let config = req.config;
        let iterations = req.iterations as usize;

        // Prepare accumulators
        let n = portfolios.len();
        let mut sum_returns = vec![0.0; n];
        let mut sum_vols = vec![0.0; n];
        let mut sum_sharpes = vec![0.0; n];
        let mut last_scenario = Vec::new();

        // Run the batch *synchronously*, but wrap in spawn_blocking
        let (sr, sv, ss, ls) = tokio::task::spawn_blocking(move || {
            for i in 0..iterations {
                // sample scenario
                let scenario_returns = self.sampler.sample_returns();
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
