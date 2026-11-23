use std::path::PathBuf;

use macui_core::{
    constants::SUPPORTED_LANGUAGES,
    resource::{MacUiLocalResourceFile, MacUiResource, ResourceKind, ResourceText},
    state::{Plugin, Wallpaper, Widget},
};
use macui_ipc::commands::{ClapResourceKind, ResourceManagerCli, ResourceSubCommand};
use owo_colors::OwoColorize;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub async fn process(cmd: ResourceManagerCli) -> Result<()> {
    match cmd.subcommand {
        ResourceSubCommand::Bundle { kind, path } => bundle(kind, path).await?,
        ResourceSubCommand::Translate { path, source_lang } => translate(path, source_lang).await?,
        _ => return Err("This command needs MAC UI to be running".into()),
    }
    Ok(())
}

async fn bundle(kind: ClapResourceKind, path: PathBuf) -> Result<()> {
    let mut to_store_path = path.clone();

    let format = time::macros::format_description!("[year]-[month]-[day] [hour]-[minute]-[second]");
    let date = time::OffsetDateTime::now_local().map_err(time::Error::IndeterminateOffset)?;
    let date_str = date.format(&format).map_err(time::Error::Format)?;
    let filename = format!("macui-bundle {date_str}.maui");

    if to_store_path.is_dir() {
        to_store_path.push(filename);
    } else {
        to_store_path.set_file_name(filename);
    }

    let e = |e: macui_core::error::MacUiLibError| -> Box<dyn std::error::Error + Send + Sync> {
        e.to_string().into()
    };

    let file = match kind {
        ClapResourceKind::Theme | ClapResourceKind::IconPack => {
            return Err("MAC UI's Liquid Glass appearance and system icons are built in".into());
        }
        ClapResourceKind::Plugin => {
            let plugin = Plugin::load_ext(&path, false).await.map_err(e)?;
            MacUiLocalResourceFile::new(ResourceKind::Plugin, &plugin).map_err(e)?
        }
        ClapResourceKind::Widget => {
            let widget = Widget::load_ext(&path, false).await.map_err(e)?;
            MacUiLocalResourceFile::new(ResourceKind::Widget, &widget).map_err(e)?
        }
        ClapResourceKind::Wallpaper => {
            let wallpaper = Wallpaper::load_ext(&path, false).await.map_err(e)?;
            MacUiLocalResourceFile::new(ResourceKind::Wallpaper, &wallpaper).map_err(e)?
        }
        _ => return Err("Not implemented".into()),
    };
    file.store(&to_store_path).await.map_err(e)?;

    println!(
        "Bundle created successfully at: {}",
        to_store_path.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use macui_core::resource::MacUiPackage;

    #[tokio::test]
    async fn cli_bundle_outputs_native_maui_files_with_macui_headers() {
        let root =
            std::env::temp_dir().join(format!("macui-cli-bundle-test-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir(&root).await.unwrap();
        let input = root.join("metadata.yml");
        tokio::fs::write(
            &input,
            b"id: '@creator/my-widget'\nhtml: '<main>Local widget</main>'\n",
        )
        .await
        .unwrap();
        bundle(ClapResourceKind::Widget, input).await.unwrap();
        let mut entries = tokio::fs::read_dir(&root).await.unwrap();
        let mut bundles = Vec::new();
        while let Some(entry) = entries.next_entry().await.unwrap() {
            if entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "maui")
            {
                bundles.push(entry.path());
            }
        }
        assert_eq!(bundles.len(), 1);
        assert!(
            bundles[0]
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("macui-bundle ")
        );
        assert!(
            tokio::fs::read(&bundles[0])
                .await
                .unwrap()
                .starts_with(b"MACUI\x03")
        );
        let package = MacUiPackage::load(&bundles[0]).await.unwrap();
        assert!(matches!(package, MacUiPackage::Local(_)));
        assert_eq!(package.kind(), ResourceKind::Widget);
        assert_eq!(package.resource_id().unwrap(), "@creator/my-widget".into());
        tokio::fs::remove_dir_all(root).await.unwrap(); // fresh test-only directory
    }
}

async fn translate(path: PathBuf, source_lang: Option<String>) -> Result<()> {
    let file = std::fs::File::open(&path)?;
    let mut texts: ResourceText = serde_yaml::from_reader(file)?;

    let code = source_lang.unwrap_or_else(|| "en".to_string());

    if !texts.has(&code) {
        return Err(format!("Source Language ({code}) not found.").into());
    }

    let source = texts.get(&code).to_owned();
    let total = SUPPORTED_LANGUAGES.len();

    let longest_lang = SUPPORTED_LANGUAGES
        .iter()
        .map(|lang| lang.en_label.len())
        .max()
        .unwrap_or(0);

    for (idx, lang) in SUPPORTED_LANGUAGES.iter().enumerate() {
        let step = if idx < 9 {
            format!("0{}", idx + 1)
        } else {
            (idx + 1).to_string()
        };

        let label = format!(
            "{}{}",
            lang.en_label,
            " ".repeat(longest_lang - lang.en_label.len())
        );

        if texts.has(lang.value) {
            println!(
                "[{step}/{total}] {} => {}",
                label.bright_black(),
                "Skipped".bright_black()
            );
            continue;
        }

        match translate_text(&source, &code, lang.value).await {
            Ok(value) => {
                println!(
                    "[{step}/{total}] {} => \"{}\"",
                    label.bold().bright_green(),
                    value
                );
                texts.set(lang.value.to_string(), value);
            }
            Err(err) => {
                eprintln!(
                    "[{step}/{total}] {} => Error translating to {} ({}): {}",
                    label.bold().bright_red(),
                    lang.en_label,
                    lang.value,
                    err
                );
            }
        }
    }

    let file = std::fs::File::create(&path)?;
    serde_yaml::to_writer(file, &texts)?;
    Ok(())
}

async fn translate_text(source: &str, source_lang: &str, mut target_lang: &str) -> Result<String> {
    use translators::{GoogleTranslator, Translator};
    let translator = GoogleTranslator::default();

    if target_lang == "zh" {
        target_lang = "zh-CN";
    }
    if target_lang == "pt" {
        target_lang = "pt-BR";
    }

    let translated = translator
        .translate_async(source, source_lang, target_lang)
        .await?;
    Ok(translated)
}
