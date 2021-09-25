use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use serde::{de::DeserializeOwned, Serialize};
use snafu::{ResultExt, Snafu};

use polyglot::Format;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Unknown config directory"))]
    UnknownConfigDirectory,

    FailedToCreateConfigDir {
        source: io::Error,
    },

    FailedToCreateDefaultConfigFile {
        source: io::Error,
    },

    FailedToSerializeDefaultConfig {
        source: polyglot::Error,
    },

    #[snafu(display("Failed to find config file"))]
    FailedToFindConfigFile,

    FailedToOpenConfigFile {
        source: io::Error,
    },

    FailedToDeserializeConfigFile {
        source: polyglot::Error,
    },
}

type Result<T> = std::result::Result<T, Error>;

fn deser<T: DeserializeOwned>(file: &Path, format: Format) -> Result<T> {
    let f = fs::File::open(file).context(FailedToOpenConfigFile)?;
    polyglot::from_reader(f, format).context(FailedToDeserializeConfigFile)
}

fn find_config_file(namespace: &str, name: &str) -> Result<Option<(PathBuf, Format)>> {
    let config_dir = dirs::config_dir().ok_or(Error::UnknownConfigDirectory)?;
    let file_path = config_dir.join(namespace).join(name);

    let (ext, format) = if file_path.with_extension("toml").exists() {
        ("toml", Format::TOML)
    } else if file_path.with_extension("json").exists() {
        ("json", Format::JSON)
    } else if file_path.with_extension("yml").exists() {
        ("yml", Format::YAML)
    } else {
        return Ok(None);
    };

    Ok(Some((file_path.with_extension(ext), format)))
}

pub fn load<T: DeserializeOwned>(namespace: &str, name: &str) -> Result<T> {
    let (file_path, format) =
        find_config_file(namespace, name)?.ok_or(Error::FailedToFindConfigFile)?;
    deser(&file_path, format)
}

pub fn load_or_default<T: DeserializeOwned + Serialize>(
    namespace: &str,
    name: &str,
    default: T,
) -> Result<T> {
    if let Some((file, format)) = find_config_file(namespace, name)? {
        deser(&file, format)
    } else {
        // Create the default config.
        let config_dir = dirs::config_dir().ok_or(Error::UnknownConfigDirectory)?;
        let namespace_dir = config_dir.join(namespace);
        fs::create_dir_all(&namespace_dir).context(FailedToCreateConfigDir)?;
        let file_path = namespace_dir.join(name).with_extension("toml");

        let f = fs::File::create(&file_path).context(FailedToCreateDefaultConfigFile)?;
        polyglot::to_writer(f, &default, Format::TOML).context(FailedToSerializeDefaultConfig)?;

        Ok(default)
    }
}
