use crate::cli::constants;

pub const PASEJO_CONFIG: &str = const_str::convert_ascii_case!(
    upper,
    const_str::concat!(constants::APPLICATION_NAME, "_config")
);

pub const PASEJO_DEFAULT_STORE: &str = const_str::convert_ascii_case!(
    upper,
    const_str::concat!(constants::APPLICATION_NAME, "_default_store")
);

pub const CODEBERG_HOST: &str = "CODEBERG_HOST";
pub const GITHUB_HOST: &str = "GITHUB_HOST";
pub const GITLAB_HOST: &str = "GITLAB_HOST";
