use crate::fetcher::post_or_comment::PostOrComment;
use activitypub_federation::config::{Data, UrlVerifier};
use async_trait::async_trait;
use lemmy_api_common::context::LemmyContext;
use lemmy_db_schema::{
  source::{activity::ReceivedActivity, instance::Instance, local_site::LocalSite},
  utils::{ActualDbPool, DbPool},
};
use lemmy_utils::error::{LemmyError, LemmyErrorType, LemmyResult};
use moka::future::Cache;
use once_cell::sync::Lazy;
use tracing::subscriber::set_global_default;
use tracing_error::ErrorLayer;
use tracing_log::LogTracer;
use tracing_subscriber::{filter::Targets, Registry, prelude::__tracing_subscriber_SubscriberExt, Layer};
use std::{sync::Arc, time::Duration};
use url::Url;

pub mod api;
pub mod activities;
pub(crate) mod activity_lists;
pub(crate) mod collections;
pub mod fetcher;
// pub mod http;
pub(crate) mod mentions;
pub mod objects;
pub mod protocol;
pub mod routes;

pub const FEDERATION_HTTP_FETCH_LIMIT: u32 = 50;
/// All incoming and outgoing federation actions read the blocklist/allowlist and slur filters
/// multiple times. This causes a huge number of database reads if we hit the db directly. So we
/// cache these values for a short time, which will already make a huge difference and ensures that
/// changes take effect quickly.
const BLOCKLIST_CACHE_DURATION: Duration = Duration::from_secs(60);

static CONTEXT: Lazy<Vec<serde_json::Value>> = Lazy::new(|| {
  serde_json::from_str(include_str!("../assets/lemmy/context.json")).expect("parse context")
});

#[derive(Clone)]
pub struct VerifyUrlData(pub ActualDbPool);

#[async_trait]
impl UrlVerifier for VerifyUrlData {
  async fn verify(&self, url: &Url) -> Result<(), &'static str> {
    let local_site_data = local_site_data_cached(&mut (&self.0).into())
      .await
      .expect("read local site data");
    check_apub_id_valid(url, &local_site_data)?;
    Ok(())
  }
}

/// Checks if the ID is allowed for sending or receiving.
///
/// In particular, it checks for:
/// - federation being enabled (if its disabled, only local URLs are allowed)
/// - the correct scheme (either http or https)
/// - URL being in the allowlist (if it is active)
/// - URL not being in the blocklist (if it is active)
#[tracing::instrument(skip(local_site_data))]
fn check_apub_id_valid(apub_id: &Url, local_site_data: &LocalSiteData) -> Result<(), &'static str> {
  let domain = apub_id.domain().expect("apud id has domain").to_string();

  if !local_site_data
    .local_site
    .as_ref()
    .map(|l| l.federation_enabled)
    .unwrap_or(true)
  {
    return Err("Federation disabled");
  }

  if local_site_data
    .blocked_instances
    .iter()
    .any(|i| domain.eq(&i.domain))
  {
    return Err("Domain is blocked");
  }

  // Only check this if there are instances in the allowlist
  if !local_site_data.allowed_instances.is_empty()
    && !local_site_data
      .allowed_instances
      .iter()
      .any(|i| domain.eq(&i.domain))
  {
    return Err("Domain is not in allowlist");
  }

  Ok(())
}

#[derive(Clone)]
pub(crate) struct LocalSiteData {
  local_site: Option<LocalSite>,
  allowed_instances: Vec<Instance>,
  blocked_instances: Vec<Instance>,
}

pub(crate) async fn local_site_data_cached(
  pool: &mut DbPool<'_>,
) -> LemmyResult<Arc<LocalSiteData>> {
  static CACHE: Lazy<Cache<(), Arc<LocalSiteData>>> = Lazy::new(|| {
    Cache::builder()
      .max_capacity(1)
      .time_to_live(BLOCKLIST_CACHE_DURATION)
      .build()
  });
  Ok(
    CACHE
      .try_get_with((), async {
        let (local_site, allowed_instances, blocked_instances) =
          lemmy_db_schema::try_join_with_pool!(pool => (
            // LocalSite may be missing
            |pool| async {
              Ok(LocalSite::read(pool).await.ok())
            },
            Instance::allowlist,
            Instance::blocklist
          ))?;

        Ok::<_, diesel::result::Error>(Arc::new(LocalSiteData {
          local_site,
          allowed_instances,
          blocked_instances,
        }))
      })
      .await?,
  )
}

pub(crate) async fn check_apub_id_valid_with_strictness(
  apub_id: &Url,
  is_strict: bool,
  context: &LemmyContext,
) -> Result<(), LemmyError> {
  let domain = apub_id.domain().expect("apud id has domain").to_string();
  let local_instance = context
    .settings()
    .get_hostname_without_port()
    .expect("local hostname is valid");
  if domain == local_instance {
    return Ok(());
  }

  let local_site_data = local_site_data_cached(&mut context.pool()).await?;
  check_apub_id_valid(apub_id, &local_site_data).map_err(|err| match err {
    "Federation disabled" => LemmyErrorType::FederationDisabled,
    "Domain is blocked" => LemmyErrorType::DomainBlocked,
    "Domain is not in allowlist" => LemmyErrorType::DomainNotInAllowList,
    _ => panic!("Could not handle apub error!"),
  })?;

  // Only check allowlist if this is a community, and there are instances in the allowlist
  if is_strict && !local_site_data.allowed_instances.is_empty() {
    // need to allow this explicitly because apub receive might contain objects from our local
    // instance.
    let mut allowed_and_local = local_site_data
      .allowed_instances
      .iter()
      .map(|i| i.domain.clone())
      .collect::<Vec<String>>();
    let local_instance = context
      .settings()
      .get_hostname_without_port()
      .expect("local hostname is valid");
    allowed_and_local.push(local_instance);

    let domain = apub_id.domain().expect("apud id has domain").to_string();
    if !allowed_and_local.contains(&domain) {
      return Err(LemmyErrorType::FederationDisabledByStrictAllowList)?;
    }
  }
  Ok(())
}

/// Store received activities in the database.
///
/// This ensures that the same activity doesnt get received and processed more than once, which
/// would be a waste of resources.
#[tracing::instrument(skip(data))]
async fn insert_received_activity(
  ap_id: &Url,
  data: &Data<LemmyContext>,
) -> Result<(), LemmyError> {
  ReceivedActivity::create(&mut data.pool(), &ap_id.clone().into()).await?;
  Ok(())
}

#[async_trait::async_trait]
pub trait SendActivity: Sync {
  type Response: Sync + Send + Clone;

  async fn send_activity(
    _request: &Self,
    _response: &Self::Response,
    _context: &Data<LemmyContext>,
  ) -> Result<(), LemmyError> {
    Ok(())
  }
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
