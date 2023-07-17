use lemmy_db_schema::{
  impls::actor_language::UNDETERMINED_ID,
  newtypes::LanguageId,
  source::language::Language,
  utils::DbPool,
};
use lemmy_utils::error::LemmyError;
use serde::{Deserialize, Serialize};
use url::Url;

pub(crate) mod chat_message;
pub(crate) mod group;
pub(crate) mod instance;
pub(crate) mod note;
pub(crate) mod page;
pub(crate) mod person;
pub(crate) mod tombstone;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Endpoints {
  pub shared_inbox: Url,
}

/// As specified in https://schema.org/Language
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LanguageTag {
  pub(crate) identifier: String,
  pub(crate) name: String,
}

impl LanguageTag {
  pub(crate) async fn new_single(
    lang: LanguageId,
    pool: &mut DbPool<'_>,
  ) -> Result<Option<LanguageTag>, LemmyError> {
    let lang = Language::read_from_id(pool, lang).await?;

    // undetermined
    if lang.id == UNDETERMINED_ID {
      Ok(None)
    } else {
      Ok(Some(LanguageTag {
        identifier: lang.code,
        name: lang.name,
      }))
    }
  }

  pub(crate) async fn new_multiple(
    lang_ids: Vec<LanguageId>,
    pool: &mut DbPool<'_>,
  ) -> Result<Vec<LanguageTag>, LemmyError> {
    let mut langs = Vec::<Language>::new();

    for l in lang_ids {
      langs.push(Language::read_from_id(pool, l).await?);
    }

    let langs = langs
      .into_iter()
      .map(|l| LanguageTag {
        identifier: l.code,
        name: l.name,
      })
      .collect();
    Ok(langs)
  }

  pub(crate) async fn to_language_id_single(
    lang: Option<Self>,
    pool: &mut DbPool<'_>,
  ) -> Result<Option<LanguageId>, LemmyError> {
    let identifier = lang.map(|l| l.identifier);
    let language = Language::read_id_from_code(pool, identifier.as_deref()).await?;

    Ok(language)
  }

  pub(crate) async fn to_language_id_multiple(
    langs: Vec<Self>,
    pool: &mut DbPool<'_>,
  ) -> Result<Vec<LanguageId>, LemmyError> {
    let mut language_ids = Vec::new();

    for l in langs {
      let id = l.identifier;
      language_ids.push(Language::read_id_from_code(pool, Some(&id)).await?);
    }

    Ok(language_ids.into_iter().flatten().collect())
  }
}
