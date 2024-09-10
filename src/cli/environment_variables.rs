use crate::cli::constants;

pub const PASEJO_CONFIG: &str = const_str::convert_ascii_case!(
    upper,
    const_str::concat!(constants::APPLICATION_NAME, "_config")
);

pub const PASEJO_DEFAULT_STORE_NAME: &str = const_str::convert_ascii_case!(
    upper,
    const_str::concat!(constants::APPLICATION_NAME, "_default_store_name")
);
