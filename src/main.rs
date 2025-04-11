use crate::Sampler;
use crate::service::SimulationServiceImpl; 
use rand::Rng;
use rand::SeedableRng;
use aegis_athena_contracts::simulation::simulation_service_server::{SimulationService, SimulationServiceServer};
use aegis_athena_contracts::simulation::{Portfolio, SimulationBatchRequest, SimulationBatchResult, SimulationScenario};
use tonic::transport::Server;
use tonic::{Request, Response, Status}; 
use rayon::prelude::*;

pub const FLOAT_COMPARISON_EPSILON: f64 = 1e-9; // it would be a good idea to add this to athena-contracts or some central thing
fn compute_portfolio_performance(
    returns: &[Vec<f64>],
    weights: &[f64],
    money_to_invest: f64,
    risk_free_rate: f64,
    time_horizon_in_days: f64,
) -> PortfolioPerformance {
    // --- Edge Case Checks ---
    // Check 1: Invalid Configuration for Time/Money (Panic)
    if time_horizon_in_days.abs() < FLOAT_COMPARISON_EPSILON {
        panic!("Configuration Error: time_horizon_in_days cannot be zero.");
    }
    if money_to_invest.abs() < FLOAT_COMPARISON_EPSILON {
        panic!("Configuration Error: money_to_invest cannot be zero.");
    }

    let number_of_periods = returns.len() as f64;

    // Check 2: Insufficient Return Periods for Volatility/Sharpe (Panic)
    if number_of_periods < 2.0 {
        panic!(
        "Configuration Error: Cannot compute volatility or Sharpe ratio with fewer than 2 return periods (found {}). \
         Check 'periods_to_sample' in Sampler configuration.",
         returns.len()
     );
    }

    // --- Main Calculation (Now guaranteed N >= 2) ---
    let portfolio_returns = returns
        .par_iter()
        .map(|row| {
            row.par_iter()
                .zip(weights.par_iter())
                .map(|(log_return, weight)| {
                    ((log_return.exp() - 1.0) * *weight) * money_to_invest
                })
                .sum::<f64>()
        })
        .collect::<Vec<f64>>();

    let average_return = portfolio_returns.iter().sum::<f64>() / number_of_periods;

    // Calculate variance (N-1 in denominator is now safe)
    let variance = portfolio_returns
        .iter()
        .map(|ret| (ret - average_return).powi(2))
        .sum::<f64>()
        / (number_of_periods - 1.0);
    let volatility = variance.sqrt(); // Standard deviation (dollar terms)

    // Annualizing!
    let time_horizon_in_years = time_horizon_in_days / 365.0;
    let periods_per_year = number_of_periods / time_horizon_in_years;

    let annualized_return = average_return * periods_per_year;
    let annualized_volatility = volatility * periods_per_year.sqrt();
    let percent_annualized_volatility = annualized_volatility / money_to_invest;

    // Adjust risk-free rate
    let risk_free_return = money_to_invest * risk_free_rate; // Annual dollar risk-free

    // Calculate Sharpe
    let sharpe_ratio = if annualized_volatility.abs() >= FLOAT_COMPARISON_EPSILON {
        // CASE 1: Volatility is significantly NON-ZERO
        (annualized_return - risk_free_return) / annualized_volatility
    } else {
        // CASE 2: Volatility IS effectively ZERO
        // Throwaway cause that's a useless portfolio (just cap it at 0. sharpe tadum-tsh)
        0.0
    };

    PortfolioPerformance {
        portfolio_returns,
        annualized_return,
        percent_annualized_volatility,
        sharpe_ratio,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing/logging to capture logs.
    tracing_subscriber::fmt::init();

    // Define the address where the Athena simulation service will listen.
    let addr = "0.0.0.0:50051".parse()?;

    // Create an instance of your Sampler.
    let sampler = Sampler::default();

    // Instantiate your simulation service with the sampler.
    let simulation_service = SimulationServiceImpl { sampler };

    println!("Athena Simulation Service listening on {}", addr);

    // Build and serve the gRPC server.
    Server::builder()
        .add_service(SimulationServiceServer::new(simulation_service))
        .serve(addr)
        .await?;

    Ok(())
}
