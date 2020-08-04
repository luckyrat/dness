use log::LevelFilter;
use serde::Deserialize;
use std::error;
use std::fmt;
use std::fs::File;
use std::io::Error as IoError;
use std::io::Read;
use std::path::Path;

#[derive(Debug)]
pub struct ConfigError {
    kind: ConfigErrorKind,
}

#[derive(Debug)]
pub enum ConfigErrorKind {
    FileNotFound(IoError),
    Misread(IoError),
    Parse(toml::de::Error),
}

impl error::Error for ConfigError {
    fn cause(&self) -> Option<&dyn error::Error> {
        match self.kind {
            ConfigErrorKind::FileNotFound(ref e) => Some(e),
            ConfigErrorKind::Misread(ref e) => Some(e),
            ConfigErrorKind::Parse(ref e) => Some(e),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "config issue: ")?;
        match self.kind {
            ConfigErrorKind::FileNotFound(ref _e) => write!(f, "file not found"),
            ConfigErrorKind::Misread(ref _e) => write!(f, "unable to read file"),
            ConfigErrorKind::Parse(ref _e) => write!(f, "a parsing error"),
        }
    }
}

#[derive(Deserialize, Clone, PartialEq, Debug)]
#[serde(deny_unknown_fields)]
pub struct DnsConfig {
    #[serde(default = "default_resolver")]
    pub ip_resolver: String,

    #[serde(default)]
    pub log: LogConfig,

    #[serde(default)]
    pub domains: Vec<DomainConfig>,
}

fn default_resolver() -> String {
    String::from("opendns")
}

impl Default for DnsConfig {
    fn default() -> Self {
        DnsConfig {
            ip_resolver: default_resolver(),
            log: Default::default(),
            domains: Default::default(),
        }
    }
}

#[derive(Deserialize, Clone, PartialEq, Debug)]
#[serde(deny_unknown_fields)]
pub struct LogConfig {
    #[serde(default = "default_log_level")]
    pub level: LevelFilter,
}

fn default_log_level() -> LevelFilter {
    LevelFilter::Info
}

impl Default for LogConfig {
    fn default() -> LogConfig {
        LogConfig {
            level: default_log_level(),
        }
    }
}

#[derive(Deserialize, Clone, PartialEq, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum DomainConfig {
    Cloudflare(CloudflareConfig),
    GoDaddy(GoDaddyConfig),
    Namecheap(NamecheapConfig),
}

impl DomainConfig {
    pub fn display_name(&self) -> String {
        match self {
            DomainConfig::Cloudflare(c) => format!("{} ({})", c.zone, "cloudflare"),
            DomainConfig::GoDaddy(c) => format!("{} ({})", c.domain, "godaddy"),
            DomainConfig::Namecheap(c) => format!("{} ({})", c.domain, "namecheap"),
        }
    }
}

#[derive(Deserialize, Clone, PartialEq, Debug)]
#[serde(deny_unknown_fields)]
pub struct CloudflareConfig {
    pub email: String,
    pub key: String,
    pub token: String,
    pub zone: String,
    pub records: Vec<String>,
}

#[derive(Deserialize, Clone, PartialEq, Debug)]
#[serde(deny_unknown_fields)]
pub struct GoDaddyConfig {
    #[serde(default = "godaddy_base_url")]
    pub base_url: String,
    pub key: String,
    pub secret: String,
    pub domain: String,
    pub records: Vec<String>,
}

#[derive(Deserialize, Clone, PartialEq, Debug)]
#[serde(deny_unknown_fields)]
pub struct NamecheapConfig {
    #[serde(default = "namecheap_base_url")]
    pub base_url: String,
    pub domain: String,
    pub ddns_password: String,
    pub records: Vec<String>,
}

fn godaddy_base_url() -> String {
    String::from("https://api.godaddy.com")
}

fn namecheap_base_url() -> String {
    String::from("https://dynamicdns.park-your-domain.com")
}

pub fn parse_config<P: AsRef<Path>>(path: P) -> Result<DnsConfig, ConfigError> {
    let mut f = File::open(path).map_err(|e| ConfigError {
        kind: ConfigErrorKind::FileNotFound(e),
    })?;

    let mut contents = String::new();
    f.read_to_string(&mut contents).map_err(|e| ConfigError {
        kind: ConfigErrorKind::Misread(e),
    })?;

    toml::from_str(&contents).map_err(|e| ConfigError {
        kind: ConfigErrorKind::Parse(e),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_config_empty() {
        let config: DnsConfig = toml::from_str("").unwrap();
        assert_eq!(
            config,
            DnsConfig {
                ip_resolver: String::from("opendns"),
                log: LogConfig {
                    level: LevelFilter::Info,
                },
                domains: vec![]
            }
        )
    }

    #[test]
    fn deserialize_config_deny_unknown() {
        let err = toml::from_str::<DnsConfig>(r#"log_info = "DEBUG""#).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("unknown field `log_info`"));
    }

    #[test]
    fn deserialize_config_simple() {
        let toml_str = &include_str!("../assets/base-config.toml");
        let config: DnsConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config,
            DnsConfig {
                ip_resolver: String::from("opendns"),
                log: LogConfig {
                    level: LevelFilter::Info,
                },
                domains: vec![DomainConfig::Cloudflare(CloudflareConfig {
                    email: String::from("a@b.com"),
                    key: String::from("deadbeef"),
                    token: String::from("deadbeef"),
                    zone: String::from("example.com"),
                    records: vec![String::from("n.example.com")]
                })]
            }
        );
    }

    #[test]
    fn deserialize_config_godaddy() {
        let toml_str = &include_str!("../assets/godaddy-config.toml");
        let config: DomainConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config,
            DomainConfig::GoDaddy(GoDaddyConfig {
                base_url: String::from("https://api.godaddy.com"),
                domain: String::from("example.com"),
                key: String::from("abc123"),
                secret: String::from("ef"),
                records: vec![String::from("@")]
            })
        );
    }

    #[test]
    fn deserialize_config_namecheap() {
        let toml_str = &include_str!("../assets/namecheap-config.toml");
        let config: DomainConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config,
            DomainConfig::Namecheap(NamecheapConfig {
                base_url: String::from("https://dynamicdns.park-your-domain.com"),
                domain: String::from("test-dness-1.xyz"),
                ddns_password: String::from("super_secret_password"),
                records: vec![String::from("@"), String::from("*"), String::from("sub")]
            })
        );
    }

    #[test]
    fn deserialize_config_readme() {
        let toml_str = &include_str!("../assets/readme-config.toml");
        let config: DnsConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config,
            DnsConfig {
                ip_resolver: String::from("opendns"),
                log: LogConfig {
                    level: LevelFilter::Debug,
                },
                domains: vec![
                    DomainConfig::Cloudflare(CloudflareConfig {
                        email: String::from("admin@example.com"),
                        key: String::from("deadbeef"),
                        token: String::from("deadbeef"),
                        zone: String::from("example.com"),
                        records: vec![String::from("n.example.com")]
                    }),
                    DomainConfig::Cloudflare(CloudflareConfig {
                        email: String::from("admin@example.com"),
                        key: String::from("deadbeef"),
                        token: String::from("deadbeef"),
                        zone: String::from("example2.com"),
                        records: vec![
                            String::from("n.example2.com"),
                            String::from("n2.example2.com")
                        ]
                    })
                ]
            }
        );
    }

    #[test]
    fn deserialize_ipify_config() {
        let toml_str = &include_str!("../assets/ipify-config.toml");
        let config: DnsConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config,
            DnsConfig {
                ip_resolver: String::from("ipify"),
                log: LogConfig {
                    level: LevelFilter::Info,
                },
                domains: vec![]
            }
        );
    }
}
