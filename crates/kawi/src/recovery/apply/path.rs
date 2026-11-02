use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct JournalPath(PathBuf);

impl JournalPath {
    pub(crate) fn as_path(&self) -> &Path {
        &self.0
    }
}

impl From<PathBuf> for JournalPath {
    fn from(path: PathBuf) -> Self {
        Self(path)
    }
}

impl Serialize for JournalPath {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if let Some(text) = self.0.to_str() {
            return serializer.serialize_str(text);
        }
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            serializer.serialize_bytes(self.0.as_os_str().as_bytes())
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            let wide: Vec<u16> = self.0.as_os_str().encode_wide().collect();
            wide.serialize(serializer)
        }
        #[cfg(not(any(unix, windows)))]
        {
            self.0.to_string_lossy().serialize(serializer)
        }
    }
}

#[cfg(unix)]
#[derive(Deserialize)]
#[serde(untagged)]
enum PathRepresentation {
    Text(String),
    Bytes(Vec<u8>),
}

#[cfg(windows)]
#[derive(Deserialize)]
#[serde(untagged)]
enum PathRepresentation {
    Text(String),
    Bytes(Vec<u16>),
}

impl<'de> Deserialize<'de> for JournalPath {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            return match PathRepresentation::deserialize(deserializer)? {
                PathRepresentation::Text(text) => Ok(Self(PathBuf::from(text))),
                PathRepresentation::Bytes(bytes) => {
                    Ok(Self(PathBuf::from(std::ffi::OsString::from_vec(bytes))))
                }
            };
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStringExt;
            return match PathRepresentation::deserialize(deserializer)? {
                PathRepresentation::Text(text) => Ok(Self(PathBuf::from(text))),
                PathRepresentation::Bytes(wide) => {
                    Ok(Self(PathBuf::from(std::ffi::OsString::from_wide(&wide))))
                }
            };
        }
        #[cfg(not(any(unix, windows)))]
        {
            let text = String::deserialize(deserializer)?;
            Ok(Self(PathBuf::from(text)))
        }
    }
}
