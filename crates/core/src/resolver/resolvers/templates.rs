//! Resolution and validation of the Tera templates used for release
//! commit messages and PR titles.
//!
//! Templates are validated here rather than at render time so a typo
//! fails before any forge call is made. See
//! [`validate`] for why that matters.

use crate::{
    config::{
        defaults::{
            DEFAULT_COMMIT_AND_PR_TITLE_TEMPLATE,
            DEFAULT_MONOREPO_COMMIT_AND_PR_TITLE_TEMPLATE, DefaultsConfig,
        },
        package::PackageConfig,
    },
    result::{ReleasaurusError, Result},
};

/// Templates for a PR covering a single package. Used when
/// `separate_pull_requests` is enabled, and for a combined PR that ends
/// up carrying only one package.
#[derive(Debug)]
pub struct PackageTemplates {
    pub commit_message: String,
    pub pr_title: String,
}

/// Templates for a combined PR carrying more than one package.
#[derive(Debug)]
pub struct MonorepoTemplates {
    pub commit_message: String,
    pub pr_title: String,
}

/// Variables a per-package template may reference, paired with the probe
/// values used to validate it.
///
/// The probes are shaped like real values — a `v`-prefixed tag, a bare
/// semver — so a template that pipes them through a filter validates the
/// same way it will render.
const PACKAGE_VARIABLES: &[(&str, &str)] = &[
    ("branch", "main"),
    ("repo_name", "repo"),
    ("package_name", "package"),
    ("tag", "v0.0.0"),
    ("semver", "0.0.0"),
];

/// Variables a monorepo template may reference. Deliberately a subset of
/// [`PACKAGE_VARIABLES`]: a combined PR spans several packages, so there
/// is no single package name, tag, or version to offer.
const MONOREPO_VARIABLES: &[(&str, &str)] =
    &[("branch", "main"), ("repo_name", "repo")];

/// Resolves the commit message and PR title templates for one package.
///
/// Precedence is the package's own value, then `[defaults]`, then the
/// built-in [`DEFAULT_COMMIT_AND_PR_TITLE_TEMPLATE`].
pub fn resolve_package_templates(
    package_name: &str,
    package_config: &PackageConfig,
    defaults: &DefaultsConfig,
) -> Result<PackageTemplates> {
    let commit_message = resolve_package_template_with_defaults(
        package_config.commit_message_template.as_ref(),
        defaults.commit_message_template.as_ref(),
    );

    let pr_title = resolve_package_template_with_defaults(
        package_config.pr_title_template.as_ref(),
        defaults.pr_title_template.as_ref(),
    );

    let scope = format!("package \"{package_name}\"");

    validate(
        &scope,
        "commit_message_template",
        &commit_message,
        PACKAGE_VARIABLES,
    )?;
    validate(&scope, "pr_title_template", &pr_title, PACKAGE_VARIABLES)?;

    Ok(PackageTemplates {
        commit_message,
        pr_title,
    })
}

/// Resolves the templates used for a combined PR carrying more than one
/// package. These have no per-package override — a single PR spanning
/// several packages has no one package to take them from.
pub fn resolve_monorepo_templates(
    defaults: &DefaultsConfig,
) -> Result<MonorepoTemplates> {
    let commit_message = defaults
        .monorepo_commit_message_template
        .clone()
        .unwrap_or_else(|| {
            DEFAULT_MONOREPO_COMMIT_AND_PR_TITLE_TEMPLATE.into()
        });

    let pr_title =
        defaults
            .monorepo_pr_title_template
            .clone()
            .unwrap_or_else(|| {
                DEFAULT_MONOREPO_COMMIT_AND_PR_TITLE_TEMPLATE.into()
            });

    validate(
        "[defaults]",
        "monorepo_commit_message_template",
        &commit_message,
        MONOREPO_VARIABLES,
    )?;
    validate(
        "[defaults]",
        "monorepo_pr_title_template",
        &pr_title,
        MONOREPO_VARIABLES,
    )?;

    Ok(MonorepoTemplates {
        commit_message,
        pr_title,
    })
}

/// First template set wins, falling back to the built-in default.
fn resolve_package_template_with_defaults(
    package: Option<&String>,
    default: Option<&String>,
) -> String {
    package
        .or(default)
        .cloned()
        .unwrap_or_else(|| DEFAULT_COMMIT_AND_PR_TITLE_TEMPLATE.into())
}

/// Renders `template` against the probe values in `variables`, discarding
/// the output.
///
/// This catches both malformed syntax and references to variables that
/// will not be in the render context — notably a package-only variable
/// such as `package_name` in a `monorepo_*` template, which the two-tier
/// design invites. Tera treats an undefined variable as an error, so
/// without this the mistake surfaces mid-release instead of at config
/// load.
fn validate(
    scope: &str,
    key: &str,
    template: &str,
    variables: &[(&str, &str)],
) -> Result<()> {
    let mut context = tera::Context::new();

    for (name, probe) in variables {
        context.insert(*name, probe);
    }

    tera::Tera::one_off(template, &context, false)
        .map(|_| ())
        .map_err(|e| {
            ReleasaurusError::invalid_config(format!(
                "{scope}: {key}: {}",
                flatten(&e)
            ))
        })
}

/// Name Tera gives a [`tera::Tera::one_off`] template. It shows up in
/// error messages, where it means nothing to someone editing a config
/// file.
const ONE_OFF_TEMPLATE_NAME: &str = "__tera_one_off";

/// Joins a [`tera::Error`] with its source chain.
///
/// Tera's outermost message is only ever `Failed to parse/render
/// '<template>'`; the actionable detail — the missing variable, the
/// syntax error and its position — sits further down the chain, so
/// reporting just the outermost error would say nothing useful.
fn flatten(err: &tera::Error) -> String {
    let mut messages = vec![];
    let mut next: Option<&(dyn std::error::Error + 'static)> = Some(err);

    while let Some(err) = next {
        messages.push(err.to_string().trim().to_string());
        next = err.source();
    }

    messages
        .join(": ")
        .replace(&format!(" while rendering '{ONE_OFF_TEMPLATE_NAME}'"), "")
        .replace(&format!("'{ONE_OFF_TEMPLATE_NAME}'"), "template")
}

#[cfg(test)]
mod tests {
    use crate::resolver::resolvers::test_helper::create_test_package;

    use super::*;

    /// Every variable the book documents for a per-package template must
    /// be in the probe context, or a valid template would be rejected at
    /// config load.
    const ALL_PACKAGE_VARIABLES: &str = "{{ branch }} {{ repo_name }} \
         {{ package_name }} {{ tag }} {{ semver }}";

    const ALL_MONOREPO_VARIABLES: &str = "{{ branch }} {{ repo_name }}";

    fn defaults_with(
        commit_message: Option<&str>,
        pr_title: Option<&str>,
    ) -> DefaultsConfig {
        DefaultsConfig {
            commit_message_template: commit_message.map(Into::into),
            pr_title_template: pr_title.map(Into::into),
            ..DefaultsConfig::default()
        }
    }

    #[test]
    fn package_template_precedence() {
        let mut pkg = create_test_package("test");
        pkg.commit_message_template = Some("pkg commit".into());
        pkg.pr_title_template = Some("pkg title".into());

        let defaults =
            defaults_with(Some("default commit"), Some("default title"));

        // Package wins over defaults
        let templates =
            resolve_package_templates("test", &pkg, &defaults).unwrap();
        assert_eq!(templates.commit_message, "pkg commit");
        assert_eq!(templates.pr_title, "pkg title");

        // Package wins with no defaults set
        let templates =
            resolve_package_templates("test", &pkg, &DefaultsConfig::default())
                .unwrap();
        assert_eq!(templates.commit_message, "pkg commit");
        assert_eq!(templates.pr_title, "pkg title");

        // Defaults win when the package is silent
        let templates = resolve_package_templates(
            "test",
            &create_test_package("test"),
            &defaults,
        )
        .unwrap();
        assert_eq!(templates.commit_message, "default commit");
        assert_eq!(templates.pr_title, "default title");

        // Built-in when neither is set
        let templates = resolve_package_templates(
            "test",
            &create_test_package("test"),
            &DefaultsConfig::default(),
        )
        .unwrap();
        assert_eq!(
            templates.commit_message,
            DEFAULT_COMMIT_AND_PR_TITLE_TEMPLATE
        );
        assert_eq!(templates.pr_title, DEFAULT_COMMIT_AND_PR_TITLE_TEMPLATE);
    }

    /// The two templates resolve through separate but identically-shaped
    /// chains, so a swapped field would otherwise go unnoticed.
    #[test]
    fn resolves_each_template_independently() {
        let mut pkg = create_test_package("test");
        pkg.commit_message_template = Some("only commit".into());

        let templates =
            resolve_package_templates("test", &pkg, &DefaultsConfig::default())
                .unwrap();

        assert_eq!(templates.commit_message, "only commit");
        assert_eq!(templates.pr_title, DEFAULT_COMMIT_AND_PR_TITLE_TEMPLATE);

        let mut pkg = create_test_package("test");
        pkg.pr_title_template = Some("only title".into());

        let templates =
            resolve_package_templates("test", &pkg, &DefaultsConfig::default())
                .unwrap();

        assert_eq!(
            templates.commit_message,
            DEFAULT_COMMIT_AND_PR_TITLE_TEMPLATE
        );
        assert_eq!(templates.pr_title, "only title");

        // Same independence one tier down, in [defaults]
        let templates = resolve_package_templates(
            "test",
            &create_test_package("test"),
            &defaults_with(Some("only default commit"), None),
        )
        .unwrap();

        assert_eq!(templates.commit_message, "only default commit");
        assert_eq!(templates.pr_title, DEFAULT_COMMIT_AND_PR_TITLE_TEMPLATE);
    }

    #[test]
    fn monorepo_template_precedence() {
        let defaults = DefaultsConfig {
            monorepo_commit_message_template: Some("mono commit".into()),
            monorepo_pr_title_template: Some("mono title".into()),
            ..DefaultsConfig::default()
        };

        let templates = resolve_monorepo_templates(&defaults).unwrap();
        assert_eq!(templates.commit_message, "mono commit");
        assert_eq!(templates.pr_title, "mono title");

        let templates =
            resolve_monorepo_templates(&DefaultsConfig::default()).unwrap();
        assert_eq!(
            templates.commit_message,
            DEFAULT_MONOREPO_COMMIT_AND_PR_TITLE_TEMPLATE
        );
        assert_eq!(
            templates.pr_title,
            DEFAULT_MONOREPO_COMMIT_AND_PR_TITLE_TEMPLATE
        );
    }

    #[test]
    fn accepts_all_documented_package_variables() {
        let mut pkg = create_test_package("test");
        pkg.commit_message_template = Some(ALL_PACKAGE_VARIABLES.into());
        pkg.pr_title_template = Some(ALL_PACKAGE_VARIABLES.into());

        let result =
            resolve_package_templates("test", &pkg, &DefaultsConfig::default());

        assert!(result.is_ok(), "{:?}", result.err());
    }

    #[test]
    fn accepts_all_documented_monorepo_variables() {
        let defaults = DefaultsConfig {
            monorepo_commit_message_template: Some(
                ALL_MONOREPO_VARIABLES.into(),
            ),
            monorepo_pr_title_template: Some(ALL_MONOREPO_VARIABLES.into()),
            ..DefaultsConfig::default()
        };

        let result = resolve_monorepo_templates(&defaults);

        assert!(result.is_ok(), "{:?}", result.err());
    }

    /// The built-in defaults must survive their own validation.
    #[test]
    fn accepts_the_built_in_defaults() {
        assert!(
            resolve_package_templates(
                "test",
                &create_test_package("test"),
                &DefaultsConfig::default(),
            )
            .is_ok()
        );

        assert!(resolve_monorepo_templates(&DefaultsConfig::default()).is_ok());
    }

    #[test]
    fn rejects_unknown_variable_in_package_template() {
        let mut pkg = create_test_package("test");
        pkg.pr_title_template = Some("release {{ pkg_name }}".into());

        let err =
            resolve_package_templates("test", &pkg, &DefaultsConfig::default())
                .unwrap_err();

        assert!(matches!(err, ReleasaurusError::InvalidConfig(_)));
    }

    /// A monorepo template covers several packages at once, so the
    /// package-scoped variables are absent. This is the mistake the
    /// two-tier design invites, and it must fail at config load.
    #[test]
    fn rejects_package_only_variable_in_monorepo_template() {
        for template in [
            "release {{ package_name }}",
            "release {{ tag }}",
            "release {{ semver }}",
        ] {
            let defaults = DefaultsConfig {
                monorepo_pr_title_template: Some(template.into()),
                ..DefaultsConfig::default()
            };

            let err = resolve_monorepo_templates(&defaults).unwrap_err();

            assert!(
                matches!(err, ReleasaurusError::InvalidConfig(_)),
                "expected {template} to be rejected"
            );
        }
    }

    #[test]
    fn rejects_malformed_template_syntax() {
        let mut pkg = create_test_package("test");
        pkg.commit_message_template = Some("release {{ branch".into());

        let err =
            resolve_package_templates("test", &pkg, &DefaultsConfig::default())
                .unwrap_err();

        assert!(matches!(err, ReleasaurusError::InvalidConfig(_)));

        let defaults = DefaultsConfig {
            monorepo_commit_message_template: Some(
                "release {% if branch %}".into(),
            ),
            ..DefaultsConfig::default()
        };

        let err = resolve_monorepo_templates(&defaults).unwrap_err();

        assert!(matches!(err, ReleasaurusError::InvalidConfig(_)));
    }

    /// Filters are a common reason to reach for a template, so the probe
    /// values have to survive being piped through one.
    #[test]
    fn accepts_filters_applied_to_probe_values() {
        let mut pkg = create_test_package("test");
        pkg.pr_title_template = Some(
            r#"{{ package_name | upper }} {{ tag | replace(from="v", to="") }}"#
                .into(),
        );

        let result =
            resolve_package_templates("test", &pkg, &DefaultsConfig::default());

        assert!(result.is_ok(), "{:?}", result.err());
    }
}
