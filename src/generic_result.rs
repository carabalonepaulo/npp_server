pub type GenericResult<T> = core::result::Result<T, Box<dyn std::error::Error + Sync + Send>>;
