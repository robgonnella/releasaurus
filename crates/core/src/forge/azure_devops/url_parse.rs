//! URL parsing for Azure DevOps repository URLs.
use secrecy::SecretString;
use url::Url;

use crate::{
    forge::config::{RepoUrl, Scheme},
    result::{ReleasaurusError, Result},
};

/// Parse an Azure DevOps Git URL of the form
/// `https://dev.azure.com/{org}/{project}/_git/{repo}` into a
/// [`RepoUrl`]. The `owner` field carries `"{org}/{project}"`.
/// Hosts other than `dev.azure.com` are rejected; on-prem Azure
/// DevOps Server is out of scope.
pub fn azure_git_url_to_repo_url(input: &str) -> Result<RepoUrl> {
    let url = Url::parse(input).map_err(|e| {
        ReleasaurusError::InvalidArgs(format!(
            "failed to parse azure devops repo url: {e}"
        ))
    })?;

    let scheme = match url.scheme() {
        "https" => Scheme::Https,
        "http" => Scheme::Http,
        s => {
            return Err(ReleasaurusError::InvalidArgs(format!(
                "azure devops repo url must start with http:// or https://, got: {s}"
            )));
        }
    };

    // Azure DevOps PAT URLs are of the form
    // `https://{anyusername}:{PAT}@dev.azure.com/...`
    let token = url.password().map(SecretString::from);

    let host = url
        .host_str()
        .ok_or_else(|| {
            ReleasaurusError::InvalidArgs(
                "azure devops repo url is missing a host".into(),
            )
        })?
        .to_string();

    if host != "dev.azure.com" {
        return Err(ReleasaurusError::InvalidArgs(format!(
            "only dev.azure.com is supported for azure devops urls \
             (on-prem Azure DevOps Server is not supported), got: {host}"
        )));
    }

    let segments: Vec<&str> =
        url.path_segments().map(|s| s.collect()).unwrap_or_default();

    let git_idx =
        segments.iter().position(|s| *s == "_git").ok_or_else(|| {
            ReleasaurusError::InvalidArgs(
                "azure devops repo url is missing the _git segment; \
                 expected https://dev.azure.com/{org}/{project}/_git/{repo}"
                    .into(),
            )
        })?;

    if git_idx < 2 || git_idx + 1 >= segments.len() {
        return Err(ReleasaurusError::InvalidArgs(
            "azure devops repo url must be of the form \
             https://dev.azure.com/{org}/{project}/_git/{repo}"
                .into(),
        ));
    }

    let org_project = segments[..git_idx].join("/");
    let name = segments[git_idx + 1].trim_end_matches(".git").to_string();
    let path = url.path().trim_end_matches(".git").to_string();
    let port = url.port();

    Ok(RepoUrl {
        host,
        owner: org_project,
        name,
        path,
        port,
        scheme,
        token,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;

    #[test]
    fn parses_basic_form() {
        let repo = azure_git_url_to_repo_url(
            "https://dev.azure.com/myorg/myproject/_git/myrepo",
        )
        .unwrap();
        assert_eq!(repo.host, "dev.azure.com");
        assert_eq!(repo.owner, "myorg/myproject");
        assert_eq!(repo.name, "myrepo");
        assert_eq!(repo.path, "/myorg/myproject/_git/myrepo");
        assert!(repo.token.is_none());
    }

    #[test]
    fn parses_nested_project_path() {
        let repo = azure_git_url_to_repo_url(
            "https://dev.azure.com/myorg/group/myproject/_git/myrepo",
        )
        .unwrap();
        assert_eq!(repo.owner, "myorg/group/myproject");
        assert_eq!(repo.name, "myrepo");
        assert_eq!(repo.path, "/myorg/group/myproject/_git/myrepo");
    }

    #[test]
    fn extracts_embedded_pat() {
        let repo = azure_git_url_to_repo_url(
            "https://user:mypat@dev.azure.com/myorg/myproject/_git/myrepo",
        )
        .unwrap();
        assert_eq!(repo.token.unwrap().expose_secret(), "mypat");
    }

    #[test]
    fn ignores_bare_username_as_token() {
        // ADO clone URLs commonly carry a placeholder username
        // (`https://user@dev.azure.com/...`) for credential helpers.
        // Treat it as no token so the env var fallback applies — the
        // PAT must be in the password component.
        let repo = azure_git_url_to_repo_url(
            "https://user@dev.azure.com/myorg/myproject/_git/myrepo",
        )
        .unwrap();
        assert!(repo.token.is_none());
    }

    #[test]
    fn rejects_non_dev_azure_host() {
        let result = azure_git_url_to_repo_url(
            "https://myorg.visualstudio.com/myproject/_git/myrepo",
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ReleasaurusError::InvalidArgs(_)
        ));
    }

    #[test]
    fn rejects_missing_git_segment() {
        let result = azure_git_url_to_repo_url(
            "https://dev.azure.com/myorg/myproject/myrepo",
        );
        assert!(result.is_err());
    }

    #[test]
    fn rejects_unsupported_scheme() {
        let result = azure_git_url_to_repo_url(
            "ssh://git@dev.azure.com/myorg/myproject/_git/myrepo",
        );
        assert!(result.is_err());
    }

    #[test]
    fn strips_dot_git_suffix() {
        let repo = azure_git_url_to_repo_url(
            "https://dev.azure.com/myorg/myproject/_git/myrepo.git",
        )
        .unwrap();
        assert_eq!(repo.name, "myrepo");
        assert_eq!(repo.path, "/myorg/myproject/_git/myrepo");
    }

    #[test]
    fn path_has_leading_slash() {
        let repo = azure_git_url_to_repo_url(
            "https://dev.azure.com/myorg/myproject/_git/myrepo",
        )
        .unwrap();
        assert!(
            repo.path.starts_with('/'),
            "azure path should start with '/' for consistency with other RepoUrl instances, got: {}",
            repo.path
        );
    }
}
