use my_crate::Sampler;
use my_crate::SimulationServiceImpl; // Adjust to your crate's module structure.
use rand::Rng;
use rand::SeedableRng;
use rand_distr::Normal;
use aegis_athena_contracts::simulation_service_server::{SimulationService, SimulationServiceServer};
use aegis_athena_contracts::{Portfolio, SimulationBatchRequest, SimulationBatchResult, SimulationScenario};
use tonic::transport::Server;
use tonic::{Request, Response, Status}; // Import your sampler type.

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
