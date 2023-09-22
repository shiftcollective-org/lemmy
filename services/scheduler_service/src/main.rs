use clap::Parser;
use lemmy_server::{init_logging, start_lemmy_scheduler, CmdArgs};
use lemmy_utils::{error::LemmyError, settings::SETTINGS};

#[tokio::main]
pub async fn main() -> Result<(), LemmyError> {
  let args = CmdArgs::parse();

  init_logging(&SETTINGS.opentelemetry_url)?;
  start_lemmy_scheduler(args).await?;

  Ok(())
}
