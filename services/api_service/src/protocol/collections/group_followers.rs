use activitypub_federation::kinds::collection::CollectionType;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GroupFollowers {
  id: Url,
  r#type: CollectionType,
  total_items: i32,
  items: Vec<()>,
}
