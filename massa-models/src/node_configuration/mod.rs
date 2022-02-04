///! Main node config and all that stuf is here
///
/// # Introduction
///
/// This module is mainelly used to define the default values used through
/// the project in all *Configuration* objects.
///
/// The name "constant" is a used for some hardcoded default values. It shouldn't
/// be used as it as a constant. When you need one of these values, you better
/// use the `cfg` parameter in your worker.
///
/// The only place where it's safe to use it is in files named `settings.rs`
/// or `config.rs`
///
/// # Testing or default
///
/// You have access to the constants (in test or in normal mode) with the
/// following imports. When you have a doubt for the import, use the auto
/// dispatcher `use massa_models::constants::*;`
///
/// ```ignore
/// // Classic import automatically root to `testings` or `default`
/// // depending on the compilation context.
/// use massa_models::constants::*;
///
/// // Force to import the nominal constants
/// use massa_models::constants::default::*;
///
/// // Force to import the testings constants
/// use massa_models::constants::default_testing::*;
/// ```
///

/**
 * We can force the access to one of defined value (test or not)
 * with `use massa_config::exported_constants::CONST_VALUE`
 *
 * Nevertheless the disign is more like using `massa_config::CONST_VALUE`
 * and defining in `Cargo.toml` if we are testing or not
 *
```ignore
[dependencies]
    massa_config = { path = "../massa-config" }
[dev-dependencies]
    massa_config = { path = "../massa-config", features = ["testing"] }
```
 */
pub mod default;
pub mod default_testing;

#[cfg(not(feature = "testing"))]
pub use default::*;
#[cfg(feature = "testing")]
pub use default_testing::*;

mod compact_config;
pub use compact_config::CompactConfig;

// Export tool to read user setting file
mod massa_settings;
pub use massa_settings::build_massa_settings;
