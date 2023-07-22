use lemmy_server::{init_logging, start_lemmy_scheduler};
use lemmy_utils::{error::LemmyError, settings::SETTINGS};

#[tokio::main]
pub async fn main() -> Result<(), LemmyError> {
  init_logging(&SETTINGS.opentelemetry_url)?;
  start_lemmy_scheduler().await?;

  Ok(())
}
