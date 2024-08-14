use std::fmt;

#[derive(Debug)]
pub enum ScriptErrors {
    #[allow(dead_code)]
    ConfigError(String),
    #[allow(dead_code)]
    ParseError(String),
}

impl fmt::Display for ScriptErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for ScriptErrors {}
