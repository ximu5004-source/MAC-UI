macro_rules! define_app_errors {
    ($(
        $variant:ident($error_type:ty);
    )*) => {
        #[derive(Debug)]
        pub enum MacUiLibError {
            $(
                $variant($error_type),
            )*
        }

        $(
            impl From<$error_type> for MacUiLibError {
                fn from(err: $error_type) -> Self {
                    MacUiLibError::$variant(err)
                }
            }
        )*
    };
}

define_app_errors!(
    Custom(String);
    Io(std::io::Error);
    SerdeJson(serde_json::Error);
    SerdeYaml(serde_yaml::Error);
    Base64Decode(base64::DecodeError);
    Grass(Box<grass::Error>);
);

impl From<&str> for MacUiLibError {
    fn from(err: &str) -> Self {
        err.to_owned().into()
    }
}

impl std::fmt::Display for MacUiLibError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

pub type Result<T, E = MacUiLibError> = std::result::Result<T, E>;
