pub(crate) mod root_span_builder;

use activitypub_federation::config::{FederationConfig, FederationMiddleware};
use actix_cors::Cors;
use actix_web::{HttpServer, App, middleware::{self, ErrorHandlers}, web::Data};
use federation_service::{FEDERATION_HTTP_FETCH_LIMIT, VerifyUrlData, http::routes, init_logging};
use lemmy_api_common::{utils::{check_private_instance_and_federation_enabled, local_site_rate_limit_to_rate_limit_config}, context::LemmyContext};
use lemmy_db_views::structs::SiteView;
use lemmy_utils::{error::LemmyError, settings::SETTINGS, rate_limit::RateLimitCell, version::VERSION, REQWEST_TIMEOUT, SYNCHRONOUS_FEDERATION, response::jsonify_plain_text_errors};
use lemmy_db_schema::{
  source::secret::Secret,
  utils::build_db_pool,
};
use reqwest::Client;
use reqwest_middleware::ClientBuilder;
use reqwest_tracing::TracingMiddleware;
use tracing_actix_web::TracingLogger;

use crate::root_span_builder::QuieterRootSpanBuilder;

#[tokio::main]
pub async fn main() -> Result<(), LemmyError> {
  init_logging(&SETTINGS.opentelemetry_url)?;

  let settings = SETTINGS.to_owned();

  // Set up the connection pool
  let pool = build_db_pool(&settings).await?;

  // Initialize the secrets
  let secret = Secret::init(&mut (&pool).into())
    .await
    .expect("Couldn't initialize secrets.");

  // Make sure the local site is set up.
  let site_view = SiteView::read_local(&mut (&pool).into())
    .await
    .expect("local site not set up");
  let local_site = site_view.local_site;
  let federation_enabled = local_site.federation_enabled;

  if federation_enabled {
    println!("federation enabled, host is {}", &settings.hostname);
  }

  check_private_instance_and_federation_enabled(&local_site)?;

  // Set up the rate limiter
  let rate_limit_config =
    local_site_rate_limit_to_rate_limit_config(&site_view.local_site_rate_limit);
  let rate_limit_cell = RateLimitCell::new(rate_limit_config).await;

  println!(
    "Starting http server at {}:{}",
    settings.bind, 8537 // TODO: Replace with settings.federation_service_port
  );

  let user_agent = format!(
    "LemmyApubService/{}; +{}",
    VERSION,
    settings.get_protocol_and_hostname()
  );
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

  let settings_bind = settings.clone();

  let federation_config = FederationConfig::builder()
    .domain(settings.hostname.clone())
    .app_data(context.clone())
    .client(client.clone())
    .http_fetch_limit(FEDERATION_HTTP_FETCH_LIMIT)
    .worker_count(settings.worker_count)
    .retry_count(settings.retry_count)
    .debug(*SYNCHRONOUS_FEDERATION)
    .http_signature_compat(true)
    .url_verifier(Box::new(VerifyUrlData(context.inner_pool().clone())))
    .build()
    .await?;

  // Create Http server with websocket support
  HttpServer::new(move || {
    let cors_origin = std::env::var("LEMMY_CORS_ORIGIN");
    let cors_config = match (cors_origin, cfg!(debug_assertions)) {
      (Ok(origin), false) => Cors::default()
        .allowed_origin(&origin)
        .allowed_origin(&settings.get_protocol_and_hostname()),
      _ => Cors::default()
        .allow_any_origin()
        .allow_any_method()
        .allow_any_header()
        .expose_any_header()
        .max_age(3600),
    };

    let app = App::new()
      .wrap(middleware::Logger::new(
        // This is the default log format save for the usage of %{r}a over %a to guarantee to record the client's (forwarded) IP and not the last peer address, since the latter is frequently just a reverse proxy
        "%{r}a '%r' %s %b '%{Referer}i' '%{User-Agent}i' %T",
      ))
      .wrap(middleware::Compress::default())
      .wrap(cors_config)
      .wrap(TracingLogger::<QuieterRootSpanBuilder>::new())
      .wrap(ErrorHandlers::new().default_handler(jsonify_plain_text_errors))
      .app_data(Data::new(context.clone()))
      .app_data(Data::new(rate_limit_cell.clone()))
      .wrap(FederationMiddleware::new(federation_config.clone()));

    // The routes
    app
      .configure(|cfg| {
        if federation_enabled {
          routes::config(cfg);
        }
      })
  })
  .bind((settings_bind.bind, 8537))?  // TODO: Replace with settings_bind.federation_service_port
  .run()
  .await?;

  Ok(())
}
