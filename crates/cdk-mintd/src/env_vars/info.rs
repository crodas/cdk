//! Info environment variables

use std::env;
use std::str::FromStr;

use cdk_common::common::QuoteTTL;

use super::common::*;
use crate::config::{Info, LoggingOutput};

impl Info {
    pub fn from_env(mut self) -> Self {
        // Required fields
        if let Ok(url) = env::var(ENV_URL) {
            self.url = url;
        }

        if let Ok(host) = env::var(ENV_LISTEN_HOST) {
            self.listen_host = host;
        }

        if let Ok(port_str) = env::var(ENV_LISTEN_PORT) {
            if let Ok(port) = port_str.parse() {
                self.listen_port = port;
            }
        }

        if let Ok(signatory_url) = env::var(ENV_SIGNATORY_URL) {
            self.signatory_url = Some(signatory_url);
        }

        if let Ok(signatory_certs) = env::var(ENV_SIGNATORY_CERTS) {
            self.signatory_certs = Some(signatory_certs);
        }

        if let Ok(seed) = env::var(ENV_SEED) {
            self.seed = Some(seed);
        }

        if let Ok(mnemonic) = env::var(ENV_MNEMONIC) {
            self.mnemonic = Some(mnemonic);
        }

        if let Ok(cache_seconds_str) = env::var(ENV_CACHE_SECONDS) {
            if let Ok(seconds) = cache_seconds_str.parse() {
                self.http_cache.ttl = Some(seconds);
            }
        }

        if let Ok(extend_cache_str) = env::var(ENV_EXTEND_CACHE_SECONDS) {
            if let Ok(seconds) = extend_cache_str.parse() {
                self.http_cache.tti = Some(seconds);
            }
        }

        if let Ok(fee_str) = env::var(ENV_INPUT_FEE_PPK) {
            if let Ok(fee) = fee_str.parse() {
                self.input_fee_ppk = Some(fee);
            }
        }

        if let Ok(info_page_str) = env::var(ENV_ENABLE_INFO_PAGE) {
            if let Ok(enable) = info_page_str.parse() {
                self.enable_info_page = Some(enable);
            }
        }

        if let Ok(use_keyset_v2_str) = env::var(ENV_USE_KEYSET_V2) {
            if let Ok(use_keyset_v2) = use_keyset_v2_str.parse() {
                self.use_keyset_v2 = Some(use_keyset_v2);
            }
        }

        if let Ok(rotation_enabled_str) = env::var(ENV_KEYSET_ROTATION_ENABLED) {
            if let Ok(enabled) = rotation_enabled_str.parse() {
                self.keyset_rotation_enabled = Some(enabled);
            }
        }

        if let Ok(rotation_interval_str) = env::var(ENV_KEYSET_ROTATION_INTERVAL_SECONDS) {
            match rotation_interval_str.parse::<u64>() {
                Ok(seconds) if seconds > 0 => {
                    self.keyset_rotation_interval_seconds = Some(seconds);
                }
                _ => tracing::warn!(
                    "Invalid {} '{}'; must be a positive integer. Ignoring.",
                    ENV_KEYSET_ROTATION_INTERVAL_SECONDS,
                    rotation_interval_str
                ),
            }
        }

        if let Ok(refresh_interval_str) = env::var(ENV_KEYSET_REFRESH_INTERVAL_SECONDS) {
            match refresh_interval_str.parse::<u64>() {
                Ok(seconds) if seconds > 0 => {
                    self.keyset_refresh_interval_seconds = Some(seconds);
                }
                _ => tracing::warn!(
                    "Invalid {} '{}'; must be a positive integer. Ignoring.",
                    ENV_KEYSET_REFRESH_INTERVAL_SECONDS,
                    refresh_interval_str
                ),
            }
        }

        // Logging configuration
        if let Ok(output_str) = env::var(ENV_LOGGING_OUTPUT) {
            if let Ok(output) = LoggingOutput::from_str(&output_str) {
                self.logging.output = output;
            } else {
                tracing::warn!(
                    "Invalid logging output '{}' in environment variable. Valid options: stdout, file, both",
                    output_str
                );
            }
        }

        if let Ok(console_level) = env::var(ENV_LOGGING_CONSOLE_LEVEL) {
            self.logging.console_level = Some(console_level);
        }

        if let Ok(file_level) = env::var(ENV_LOGGING_FILE_LEVEL) {
            self.logging.file_level = Some(file_level);
        }

        self.http_cache = self.http_cache.from_env();

        // Quote TTL from env
        let mut mint_ttl_env: Option<u64> = None;
        let mut melt_ttl_env: Option<u64> = None;
        if let Ok(mint_ttl_str) = env::var(ENV_QUOTE_TTL_MINT) {
            if let Ok(v) = mint_ttl_str.parse::<u64>() {
                mint_ttl_env = Some(v);
            }
        }
        if let Ok(melt_ttl_str) = env::var(ENV_QUOTE_TTL_MELT) {
            if let Ok(v) = melt_ttl_str.parse::<u64>() {
                melt_ttl_env = Some(v);
            }
        }
        if mint_ttl_env.is_some() || melt_ttl_env.is_some() {
            let current = self.quote_ttl.unwrap_or_default();
            self.quote_ttl = Some(QuoteTTL {
                mint_ttl: mint_ttl_env.unwrap_or(current.mint_ttl),
                melt_ttl: melt_ttl_env.unwrap_or(current.melt_ttl),
            });
        }

        self
    }
}
