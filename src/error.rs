#[derive(Debug)]
pub enum Error {
    #[cfg(all(feature = "lua", not(target_arch = "wasm32")))]
    LuaError(String),
    NoActiveBuffer,
    EditError(String),
}

#[derive(Debug)]
pub enum EditError {
    Unchanged,
    InvalidData,
}

impl From<EditError> for Error {
    fn from(value: EditError) -> Self {
        match value {
            EditError::Unchanged => Self::EditError("unchanged".into()),
            EditError::InvalidData => Self::EditError("invalid data".into()),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(all(feature = "lua", not(target_arch = "wasm32")))]
            Error::LuaError(msg) => {
                f.write_str("Failed to execute script\n")?;
                f.write_str(msg)?;
            }
            Error::EditError(msg) => {
                f.write_str("Error: ")?;
                f.write_str(msg)?;
            }
            Error::NoActiveBuffer => {
                f.write_str("No active Buffer")?;
            }
        }
        Ok(())
    }
}
impl std::error::Error for Error {}
