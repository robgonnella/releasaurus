use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct SlackUserProfile {
    pub real_name: Option<String>,
    pub display_name: Option<String>,
    pub real_name_normalized: Option<String>,
    pub display_name_normalized: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SlackUser {
    pub id: String,
    pub name: String,
    pub is_bot: bool,
    pub deleted: bool,
    pub real_name: Option<String>,
    pub profile: SlackUserProfile,
}

#[derive(Debug, Deserialize)]
pub struct ResponseMetadata {
    pub next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SlackUsersResponse {
    pub ok: bool,
    pub members: Vec<SlackUser>,
    pub response_metadata: Option<ResponseMetadata>,
}
