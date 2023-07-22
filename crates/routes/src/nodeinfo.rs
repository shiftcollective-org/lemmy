use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct NodeInfo {
  pub version: Option<String>,
  pub software: Option<NodeInfoSoftware>,
  pub protocols: Option<Vec<String>>,
  pub usage: Option<NodeInfoUsage>,
  pub open_registrations: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(default)]
pub struct NodeInfoSoftware {
  pub name: Option<String>,
  pub version: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct NodeInfoUsage {
  pub users: Option<NodeInfoUsers>,
  pub local_posts: Option<i64>,
  pub local_comments: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct NodeInfoUsers {
  pub total: Option<i64>,
  pub active_halfyear: Option<i64>,
  pub active_month: Option<i64>,
}
