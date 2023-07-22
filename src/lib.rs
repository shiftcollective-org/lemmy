pub mod code_migrations;
pub mod root_span_builder;
pub mod scheduled_tasks;
#[cfg(feature = "console")]
pub mod telemetry;

use crate::code_migrations::run_advanced_migrations;
use actix_web::Result;
use lemmy_api_common::{
  context::LemmyContext,
  lemmy_db_views::structs::SiteView,
  request::build_user_agent,
  utils::local_site_rate_limit_to_rate_limit_config,
};
use lemmy_db_schema::{
  source::secret::Secret,
  utils::{build_db_pool, get_database_url, run_migrations},
};
use lemmy_utils::{
  error::LemmyError,
  rate_limit::RateLimitCell,
  settings::SETTINGS,
};
use reqwest::Client;
use reqwest_middleware::ClientBuilder;
use reqwest_tracing::TracingMiddleware;
use std::{env, thread, time::Duration};
use tracing::subscriber::set_global_default;
use tracing_error::ErrorLayer;
use tracing_log::LogTracer;
use tracing_subscriber::{filter::Targets, layer::SubscriberExt, Layer, Registry};
use url::Url;

/// Max timeout for http requests
pub(crate) const REQWEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Placing the main function in lib.rs allows other crates to import it and embed Lemmy
pub async fn start_lemmy_server() -> Result<(), LemmyError> {
  let args: Vec<String> = env::args().collect();

  let scheduled_tasks_enabled = args.get(1) != Some(&"--disable-scheduled-tasks".to_string());

  let settings = SETTINGS.to_owned();

  // Run the DB migrations
  let db_url = get_database_url(Some(&settings));
  run_migrations(&db_url);

  // Set up the connection pool
  let pool = build_db_pool(&settings).await?;

  // Run the Code-required migrations
  run_advanced_migrations(&mut (&pool).into(), &settings).await?;

  // Initialize the secrets
  let secret = Secret::init(&mut (&pool).into())
    .await
    .expect("Couldn't initialize secrets.");

  // Make sure the local site is set up.
  let site_view = SiteView::read_local(&mut (&pool).into())
    .await
    .expect("local site not set up");

  // Set up the rate limiter
  let rate_limit_config =
    local_site_rate_limit_to_rate_limit_config(&site_view.local_site_rate_limit);
  let rate_limit_cell = RateLimitCell::new(rate_limit_config).await;

  let user_agent = build_user_agent(&settings);
  let reqwest_client = Client::builder()
    .user_agent(user_agent.clone())
    .timeout(REQWEST_TIMEOUT)
    .connect_timeout(REQWEST_TIMEOUT)
    .build()?;

  let client = ClientBuilder::new(reqwest_client.clone())
    .with(TracingMiddleware::default())
    .build();

  let context = LemmyContext::create(
    pool.clone(),
    client.clone(),
    secret.clone(),
    rate_limit_cell.clone(),
  );

  if scheduled_tasks_enabled {
    // Schedules various cleanup tasks for the DB
    thread::spawn({
      let context = context.clone();
      move || {
        scheduled_tasks::setup(db_url, user_agent, context)
          .expect("Couldn't set up scheduled_tasks");
      }
    });
  }

  Ok(())
}

pub fn init_logging(opentelemetry_url: &Option<Url>) -> Result<(), LemmyError> {
  LogTracer::init()?;

  let log_description = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into());

  let targets = log_description
    .trim()
    .trim_matches('"')
    .parse::<Targets>()?;

  let format_layer = {
    #[cfg(feature = "json-log")]
    let layer = tracing_subscriber::fmt::layer().json();
    #[cfg(not(feature = "json-log"))]
    let layer = tracing_subscriber::fmt::layer();

    layer.with_filter(targets.clone())
  };

  let subscriber = Registry::default()
    .with(format_layer)
    .with(ErrorLayer::default());

  if let Some(_url) = opentelemetry_url {
    #[cfg(feature = "console")]
    telemetry::init_tracing(_url.as_ref(), subscriber, targets)?;
    #[cfg(not(feature = "console"))]
    tracing::error!("Feature `console` must be enabled for opentelemetry tracing");
  } else {
    set_global_default(subscriber)?;
  }

  Ok(())
}
