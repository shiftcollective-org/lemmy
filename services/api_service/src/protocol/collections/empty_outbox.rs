use activitypub_federation::kinds::collection::OrderedCollectionType;
use serde::{Deserialize, Serialize};
use url::Url;

/// Empty placeholder outbox used for Person, Instance, which dont implement a proper outbox yet.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EmptyOutbox {
  r#type: OrderedCollectionType,
  id: Url,
  ordered_items: Vec<()>,
  total_items: i32,
}
