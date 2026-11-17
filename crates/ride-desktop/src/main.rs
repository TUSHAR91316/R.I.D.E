use ride_agent::AgentExecutor;
use ride_core::{RideConfig, RIDE_APP_NAME, RIDE_VERSION};
use ride_discovery::ModelDetector;
use ride_moe::MoeEngineManager;
use ride_skills::SkillRegistry;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    info!("Starting {} v{}", RIDE_APP_NAME, RIDE_VERSION);

    let config = RideConfig::default();
    info!("Active UI Layout: {:?}", config.active_layout);
    info!("RAM optimization limit: {} MB", config.memory_limits.max_editor_ram_mb);

    let detector = ModelDetector::default();
    let statuses = detector.scan_all().await;
    let online_count = statuses.iter().filter(|s| s.is_online).count();
    info!("Local model discovery scanned {} endpoints. Online: {}", statuses.len(), online_count);

    let moe_manager = MoeEngineManager::new(Default::default());
    info!("MoE engine integration ready on port {}", moe_manager.config().port);

    let mut skills = SkillRegistry::new();
    let loaded = skills.load_from_dir(&config.skills_dir)?;
    info!("Loaded {} skills from {:?}", loaded, config.skills_dir);

    let skill_prompt = AgentExecutor::build_skills_prompt(&skills.list());
    info!("Agent prompt buffer initialized ({} bytes)", skill_prompt.len());

    info!("R.I.D.E desktop core initialized successfully.");
    Ok(())
}
