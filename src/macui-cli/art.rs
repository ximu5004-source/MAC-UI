use macui_ipc::commands::{ArtCli, ArtVariant};

pub fn process(cmd: ArtCli) {
    match cmd.variant {
        ArtVariant::MacUiLogo => println!("{MACUI_LOGO_ASCII}"),
        ArtVariant::MacUiLogoSmall => println!("{MACUI_LOGO_ASCII_SMALL}"),
    }
}

static MACUI_LOGO_ASCII: &str = r#"
+--------------------------------------+
|                                      |
|    M   M    A    CCC    U   U  III    |
|    MM MM   A A   C      U   U   I     |
|    M M M  AAAAA  C      U   U   I     |
|    M   M  A   A  C      U   U   I     |
|    M   M  A   A  CCC     UUU   III    |
|                                      |
|             MAC UI / JONA            |
+--------------------------------------+
"#;

static MACUI_LOGO_ASCII_SMALL: &str = r#"
+----------------+
| MAC UI / JONA  |
+----------------+
"#;
