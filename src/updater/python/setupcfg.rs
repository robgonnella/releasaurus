use log::*;
use regex::Regex;
use std::sync::LazyLock;

use crate::{
    forge::{
        request::{FileChange, FileUpdateType},
        traits::FileLoader,
    },
    result::Result,
    updater::framework::UpdaterPackage,
};

static VERSION_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?m)^(\s*version\s*=\s*)([\"']?)([\w\.\-\+]+)([\"']?)"#)
        .unwrap()
});

pub struct SetupCfg {}

impl SetupCfg {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn process_packages(
        &self,
        packages: &[UpdaterPackage],
        loader: &dyn FileLoader,
    ) -> Result<Option<Vec<FileChange>>> {
        let mut file_changes: Vec<FileChange> = vec![];

        for package in packages {
            let file_path = package.get_file_path("setup.cfg");

            let doc = self.load_doc(&file_path, loader).await?;

            if doc.is_none() {
                continue;
            }

            let doc = doc.unwrap();

            info!("found setup.cfg for package: {}", package.path);

            let content = VERSION_REGEX
                .replace(&doc, |caps: &regex::Captures| {
                    format!(
                        "{}{}{}{}",
                        &caps[1],
                        &caps[2],
                        package.next_version.semver,
                        &caps[4]
                    )
                })
                .to_string();

            file_changes.push(FileChange {
                path: file_path,
                content,
                update_type: FileUpdateType::Replace,
            });
        }

        if file_changes.is_empty() {
            return Ok(None);
        }

        Ok(Some(file_changes))
    }

    async fn load_doc(
        &self,
        file_path: &str,
        loader: &dyn FileLoader,
    ) -> Result<Option<String>> {
        let content = loader.get_file_content(file_path).await?;
        if content.is_none() {
            return Ok(None);
        }
        let content = content.unwrap();
        Ok(Some(content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_replacement_preserves_indentation() {
        let test_cases = vec![
            (
                r#"version = 1.0.0"#,
                r#"version = 2.0.0"#,
                "no quotes no spaces",
            ),
            (
                r#"version = "1.0.0""#,
                r#"version = "2.0.0""#,
                "with quotes",
            ),
            (
                r#"  version = 1.0.0"#,
                r#"  version = 2.0.0"#,
                "two space indent",
            ),
            (
                r#"    version = 1.0.0"#,
                r#"    version = 2.0.0"#,
                "four space indent",
            ),
            (
                r#"version=1.0.0"#,
                r#"version=2.0.0"#,
                "no spaces around equals",
            ),
            (
                r#"  version  =  1.0.0"#,
                r#"  version  =  2.0.0"#,
                "multiple spaces",
            ),
        ];

        for (input, expected, description) in test_cases {
            let result =
                VERSION_REGEX.replace(input, |caps: &regex::Captures| {
                    format!("{}{}{}{}", &caps[1], &caps[2], "2.0.0", &caps[4])
                });
            assert_eq!(
                result.as_ref(),
                expected,
                "Failed for: {}",
                description
            );
            println!("✓ {}: '{}' -> '{}'", description, input, result);
        }
    }

    #[test]
    fn test_regex_preserves_quote_style() {
        // Test that single quotes are preserved
        let input = r#"version = '1.0.0'"#;
        let result = VERSION_REGEX.replace(input, |caps: &regex::Captures| {
            format!("{}{}{}{}", &caps[1], &caps[2], "2.0.0", &caps[4])
        });
        assert_eq!(result.as_ref(), r#"version = '2.0.0'"#);

        // Test that double quotes are preserved
        let input = r#"version = "1.0.0""#;
        let result = VERSION_REGEX.replace(input, |caps: &regex::Captures| {
            format!("{}{}{}{}", &caps[1], &caps[2], "2.0.0", &caps[4])
        });
        assert_eq!(result.as_ref(), r#"version = "2.0.0""#);

        // Test that no quotes are preserved
        let input = r#"version = 1.0.0"#;
        let result = VERSION_REGEX.replace(input, |caps: &regex::Captures| {
            format!("{}{}{}{}", &caps[1], &caps[2], "2.0.0", &caps[4])
        });
        assert_eq!(result.as_ref(), r#"version = 2.0.0"#);
    }

    #[tokio::test]
    async fn test_process_packages_basic() {
        use crate::forge::traits::MockFileLoader;
        use crate::test_helpers::create_test_updater_package;
        use crate::updater::framework::Framework;

        let setupcfg = SetupCfg::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
            Framework::Python,
        );

        let setup_cfg = r#"[metadata]
name = test-package
version = 1.0.0
description = A test package

[options]
packages = find:
python_requires = >=3.8
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/setup.cfg"))
            .times(1)
            .returning(move |_| Ok(Some(setup_cfg.to_string())));

        let packages = vec![package];
        let result = setupcfg
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].path, "packages/test/setup.cfg");
        assert!(changes[0].content.contains("version = 2.0.0"));
        assert!(!changes[0].content.contains("version = 1.0.0"));
    }

    #[tokio::test]
    async fn test_process_packages_with_quotes() {
        use crate::forge::traits::MockFileLoader;
        use crate::test_helpers::create_test_updater_package;
        use crate::updater::framework::Framework;

        let setupcfg = SetupCfg::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "3.0.0",
            Framework::Python,
        );

        let setup_cfg = r#"[metadata]
name = test-package
version = "1.0.0"
description = A test package
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/setup.cfg"))
            .times(1)
            .returning(move |_| Ok(Some(setup_cfg.to_string())));

        let packages = vec![package];
        let result = setupcfg
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        assert_eq!(changes.len(), 1);
        assert!(changes[0].content.contains("version = \"3.0.0\""));
    }

    #[tokio::test]
    async fn test_process_packages_with_indentation() {
        use crate::forge::traits::MockFileLoader;
        use crate::test_helpers::create_test_updater_package;
        use crate::updater::framework::Framework;

        let setupcfg = SetupCfg::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.5.0",
            Framework::Python,
        );

        let setup_cfg = r#"[metadata]
  name = test-package
  version = 1.0.0
  description = Test
  author = John Doe
"#;

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/setup.cfg"))
            .times(1)
            .returning(move |_| Ok(Some(setup_cfg.to_string())));

        let packages = vec![package];
        let result = setupcfg
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_some());
        let changes = result.unwrap();
        let content = &changes[0].content;
        assert!(content.contains("version = 2.5.0"));
        assert!(content.contains("author = John Doe"));
    }

    #[tokio::test]
    async fn test_process_packages_no_file_found() {
        use crate::forge::traits::MockFileLoader;
        use crate::test_helpers::create_test_updater_package;
        use crate::updater::framework::Framework;

        let setupcfg = SetupCfg::new();
        let package = create_test_updater_package(
            "test-package",
            "packages/test",
            "2.0.0",
            Framework::Python,
        );

        let mut mock_loader = MockFileLoader::new();
        mock_loader
            .expect_get_file_content()
            .with(mockall::predicate::eq("packages/test/setup.cfg"))
            .times(1)
            .returning(|_| Ok(None));

        let packages = vec![package];
        let result = setupcfg
            .process_packages(&packages, &mock_loader)
            .await
            .unwrap();

        assert!(result.is_none());
    }
}
