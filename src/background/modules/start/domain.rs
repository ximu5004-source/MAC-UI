use std::{collections::HashSet, io::BufRead, path::Path};

use serde::Deserialize;

/// Only Windows Programs launch entries belong in Launchpad. Loose installers,
/// documents and metadata are not applications, even when their filename matches.
pub fn is_program_shortcut(path: &Path) -> bool {
    path.extension().is_some_and(|extension| {
        ["lnk", "url", "appref-ms"]
            .iter()
            .any(|candidate| extension.eq_ignore_ascii_case(candidate))
    })
}

// Parse only visibility data. Icon/display-resource fields are optional or
// version-specific and must never cause a valid All Apps entry to disappear.
#[derive(Default, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
struct VisibilityManifest {
    applications: VisibilityApplications,
}

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "PascalCase")]
struct VisibilityApplications {
    application: Vec<VisibilityApplication>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct VisibilityApplication {
    #[serde(rename = "@Id")]
    id: String,
    #[serde(default)]
    visual_elements: Option<VisibilityElements>,
}

#[derive(Deserialize)]
struct VisibilityElements {
    #[serde(rename = "@AppListEntry", default)]
    app_list_entry: Option<String>,
}

pub fn visible_packaged_app_ids(
    reader: impl BufRead,
) -> Result<HashSet<String>, quick_xml::DeError> {
    let manifest: VisibilityManifest = quick_xml::de::from_reader(reader)?;
    Ok(manifest
        .applications
        .application
        .into_iter()
        .filter_map(|app| {
            let visible = app.visual_elements.is_some_and(|visual| {
                !visual
                    .app_list_entry
                    .as_deref()
                    .is_some_and(|value| value.eq_ignore_ascii_case("none"))
            });
            visible.then_some(app.id)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_registered_launch_entries_without_guessing_from_app_names() {
        for filename in ["WeChat.lnk", "Web App.URL", "Company.appref-ms"] {
            assert!(is_program_shortcut(Path::new(filename)));
        }
        for filename in [
            "WeChat.exe",
            "WeChat.msi",
            "setup.msix",
            "desktop.ini",
            "Readme.pdf",
        ] {
            assert!(!is_program_shortcut(Path::new(filename)));
        }
    }

    #[test]
    fn only_visible_packaged_entries_belong_in_all_apps() {
        let xml = r#"<Package xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10">
          <Applications>
            <Application Id="Normal"><uap:VisualElements DisplayName="App" /></Application>
            <Application Id="Explicit"><uap:VisualElements AppListEntry="default" /></Application>
            <Application Id="Background"><uap:VisualElements AppListEntry="none" /></Application>
            <Application Id="Helper" />
          </Applications>
        </Package>"#;
        let ids = visible_packaged_app_ids(xml.as_bytes()).unwrap();
        assert_eq!(ids, HashSet::from(["Normal".into(), "Explicit".into()]));
    }

    #[test]
    fn resource_only_packages_have_no_launch_entries() {
        assert!(
            visible_packaged_app_ids("<Package><Properties /></Package>".as_bytes())
                .unwrap()
                .is_empty()
        );
        assert!(visible_packaged_app_ids("not xml".as_bytes()).is_err());
    }
}
