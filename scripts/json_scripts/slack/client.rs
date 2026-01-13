use async_trait::async_trait;
use color_eyre::eyre::{Result, eyre};
use reqwest::{
    Client,
    header::{HeaderMap, HeaderValue},
};
use std::{collections::HashMap, env};

use crate::json_scripts::PlatformClient;

use super::types::{SlackUser, SlackUsersResponse};

const SLACK_TOKEN_ENV_VAR: &str = "SLACK_TOKEN";

pub struct SlackClient {
    client: Client,
}

impl SlackClient {
    pub fn new(token: Option<String>) -> Result<Self> {
        let mut token = token.unwrap_or_default();

        if token.is_empty()
            && let Ok(value) = env::var(SLACK_TOKEN_ENV_VAR)
        {
            token = value;
        }

        if token.is_empty() {
            return Err(eyre!("must provide slack token"));
        }

        let mut headers = HeaderMap::new();

        let token_value = HeaderValue::from_str(&format!("Bearer {token}"))?;

        headers.append("Authorization", token_value);

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self { client })
    }

    async fn get_users(&self) -> Result<Vec<SlackUser>> {
        let mut all_users = vec![];
        let mut cursor: Option<String> = None;
        let limit = 200;

        loop {
            let mut req =
                reqwest::Url::parse("https://slack.com/api/users.list")?;

            // Add parameters
            {
                let mut queries = req.query_pairs_mut();
                queries.append_pair("limit", &limit.to_string());
                if let Some(c) = &cursor {
                    queries.append_pair("cursor", c);
                }
            }

            // Send the request
            let response = self
                .client
                .get(req)
                .send()
                .await?
                .error_for_status()?
                .json::<SlackUsersResponse>()
                .await?;

            if !response.ok {
                return Err(eyre!(format!("Slack API error: {:?}", response)));
            }

            // Add the fetched users to our total list
            all_users.extend(response.members);

            // Check for the next cursor
            cursor = response
                .response_metadata
                .and_then(|meta| meta.next_cursor)
                .filter(|c| !c.is_empty()); // An empty cursor indicates no more pages

            // Break the loop if there are no more pages
            if cursor.is_none() {
                break;
            }
        }

        Ok(all_users)
    }
}

#[async_trait]
impl PlatformClient for SlackClient {
    async fn get_user_name_tag_hash(&self) -> Result<HashMap<String, String>> {
        let users = self.get_users().await?;

        println!("found {} users from total slack", users.len());

        let mut map = HashMap::new();

        for member in users.into_iter() {
            let tag = format!("<@{}>", member.id);

            if member.is_bot || member.deleted {
                continue;
            }

            if let Some(name) = member.real_name.as_ref() {
                map.insert(name.clone(), tag);
                continue;
            }

            if let Some(name) = member.profile.real_name.as_ref() {
                map.insert(name.clone(), tag);
                continue;
            }

            if let Some(name) = member.profile.display_name.as_ref() {
                map.insert(name.clone(), tag);
                continue;
            }

            if let Some(name) = member.profile.real_name_normalized.as_ref() {
                map.insert(name.clone(), tag);
                continue;
            }

            if let Some(name) = member.profile.display_name_normalized.as_ref()
            {
                map.insert(name.clone(), tag);
                continue;
            }

            map.insert(member.name.clone(), tag);
        }

        Ok(map)
    }
}
