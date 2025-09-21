use std::cmp::Ordering;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{Result, anyhow, bail};
use headers_accept::Accept;
use http::{HeaderMap, HeaderValue, header};
use mediatype::{MediaType, names::*};
use mime2ext::mime2ext;

use crate::storage::StorageBackend;

/// The storage key and content-type needed to satisfy an HTTP request.
/// All generated storage keys have a file extension to indicate the content type.
/// Rejects write operations if the *content-type* header contains an unknown media type.
/// Satisfies read operations based on file extension or *accept* header.
#[derive(Debug)]
pub struct NegotiatedPath<'a> {
    storage_key: PathBuf,
    media_type: MediaType<'a>,
}

impl<'a> NegotiatedPath<'a> {
    pub const GENERIC_MEDIA_TYPE: MediaType<'a> = MediaType::new(APPLICATION, OCTET_STREAM);
    pub const GENERIC_EXT: &'a str = "octet-stream";

    /// If negotiation fails, `Ok(None)` is returned to indicate
    /// that the value of the *content-type* header is not acceptable.
    pub fn for_write(path: &Path, headers: &'a HeaderMap) -> Result<Option<Self>> {
        match (path.extension(), headers.get(header::CONTENT_TYPE)) {
            // providing only an extension is acceptable, use generic content-type
            (Some(_path_ext), None) => Ok(Some(Self {
                storage_key: path.to_owned(),
                media_type: Self::GENERIC_MEDIA_TYPE,
            })),
            // guess extension from content-type, if recognized
            (None, Some(content_type)) => {
                let content_type = content_type.to_str()?;
                match MediaType::parse(content_type) {
                    Err(_) => Ok(None),
                    Ok(media_type) => {
                        let ext = mime2ext(content_type).unwrap_or(Self::GENERIC_EXT);
                        Ok(Some(Self {
                            storage_key: path.with_extension(ext),
                            media_type,
                        }))
                    }
                }
            }
            // use provided extension and content-type, if recognized
            (Some(_path_ext), Some(content_type)) => {
                let content_type = content_type.to_str()?;
                match MediaType::parse(content_type) {
                    Err(_) => Ok(None),
                    Ok(media_type) => Ok(Some(Self {
                        storage_key: path.to_owned(),
                        media_type,
                    })),
                }
            }
            // use defaults
            (None, None) => Ok(Some(Self {
                storage_key: path.with_extension(Self::GENERIC_EXT),
                media_type: Self::GENERIC_MEDIA_TYPE,
            })),
        }
    }

    /// Use `extensions` to find a specific representation
    /// or fall back to matching using the *accept* header.
    /// If negotiation fails, `Ok(None)` is returned to indicate that
    /// no acceptable content was found to serve in response.
    pub fn for_read(
        path: &Path,
        extensions: &'a PathExtensions,
        headers: &HeaderMap,
    ) -> Result<Option<Self>> {
        match (path.extension(), headers.get(header::ACCEPT)) {
            // search by file extension
            (Some(path_ext), _) if path_ext != Self::GENERIC_EXT => {
                let path_ext = path_ext.to_str().unwrap();
                match extensions.get_media_type(path_ext)? {
                    None => Ok(None),
                    Some(media_type) => Ok(Some(Self {
                        storage_key: path.to_owned(),
                        media_type,
                    })),
                }
            }
            // use the "accept" header to match against available types
            (None, Some(accept)) => {
                let accept = accept.to_str()?;
                let accept = Accept::from_str(accept)?;
                let available = extensions.get_all_media_types()?;
                match accept.negotiate(&available) {
                    None => Ok(None),
                    Some(media_type) => {
                        let ext = extensions.get_extension(media_type)?.unwrap();
                        Ok(Some(Self {
                            storage_key: path.with_extension(ext),
                            media_type: media_type.clone(),
                        }))
                    }
                }
            }
            // generic type available
            (None, None) if extensions.map.contains_key(Self::GENERIC_EXT) => Ok(Some(Self {
                storage_key: path.with_extension(Self::GENERIC_EXT),
                media_type: Self::GENERIC_MEDIA_TYPE,
            })),
            // otherwise not found
            _ => Ok(None),
        }
    }

    pub fn guess_media_type(&mut self) -> Result<()> {
        let ext = self.storage_extension();
        let guess = new_mime_guess::from_ext(&ext)
            .first_raw()
            .ok_or(anyhow!("no known media type for '.{ext}' extension"))?;
        self.media_type = MediaType::parse(guess)?;
        Ok(())
    }

    pub fn storage_extension(&self) -> std::borrow::Cow<'_, str> {
        self.storage_key.extension().unwrap().to_string_lossy()
    }

    pub fn content_type_header(&self) -> HeaderValue {
        HeaderValue::from_str(self.media_type.essence().to_string().as_str()).unwrap()
    }

    pub fn content_location_header(&self) -> HeaderValue {
        let file_name = match self.storage_extension().deref() {
            ext if ext == Self::GENERIC_EXT => self.storage_key.file_stem(),
            _ => self.storage_key.file_name(),
        };
        // `self.storage_key` is relative to the root of the storage tree,
        // but content-location needs to be relative to the request URL
        // https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Content-Location
        let relative_location = Path::new("/").join(file_name.unwrap());
        HeaderValue::from_str(relative_location.to_str().unwrap()).unwrap()
    }
}

impl<'a> std::fmt::Display for NegotiatedPath<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "\"{}\" {}",
            self.storage_key.to_str().unwrap(),
            self.media_type
        )
    }
}

impl<'a> AsRef<Path> for NegotiatedPath<'a> {
    fn as_ref(&self) -> &Path {
        &self.storage_key
    }
}

pub struct PathExtensions {
    pub path: PathBuf,
    map: serde_json::Map<String, serde_json::Value>,
}

impl PathExtensions {
    pub const META_EXT: &str = "ext";

    /// Instantiate from storage backend.
    pub fn get_for_path(path: &Path, db: Arc<impl StorageBackend>) -> Self {
        let path = Path::new("/")
            .join(crate::util::path_stem(path))
            .with_extension(Self::META_EXT);
        let map = db
            .get(&path)
            .ok()
            .flatten()
            .and_then(|s| serde_json::from_slice(s.as_slice()).ok())
            .unwrap_or_default();
        Self { path, map }
    }

    /// Returns a description of the storage operation to perform in a batch update.
    pub fn insert(&mut self, negotiated: &NegotiatedPath) -> Result<(&Path, Option<Vec<u8>>)> {
        self.map.insert(
            negotiated
                .storage_key
                .extension()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            serde_json::Value::String(negotiated.media_type.to_string()),
        );
        let map_string = serde_json::to_string(&self.map)?;
        Ok((&self.path, Some(map_string.into_bytes())))
    }

    /// Returns a description of the storage operation to perform in a batch update.
    pub fn remove(&mut self, extension: &str) -> Result<(&Path, Option<Vec<u8>>)> {
        self.map.remove(extension).unwrap();
        if self.map.is_empty() {
            // remove the path from storage
            Ok((&self.path, None))
        } else {
            // update the stored extensions
            let map_string = serde_json::to_string(&self.map)?;
            Ok((&self.path, Some(map_string.into_bytes())))
        }
    }

    fn get_media_type(&self, extension: &str) -> Result<Option<MediaType<'_>>> {
        match self.map.get(extension) {
            Some(v) => match v {
                serde_json::Value::String(mt) => MediaTypeString(mt).try_into().map(Some),
                other => bail!("{extension}: {other:?} (should be string)"),
            },
            None => Ok(None),
        }
    }

    fn get_all_media_types(&self) -> Result<Vec<MediaType<'_>>> {
        let mut mt_strings: Vec<MediaTypeString> = self
            .map
            .iter()
            .filter_map(|(_, v)| match v {
                serde_json::Value::String(mt) => Some(MediaTypeString(mt)),
                _ => None,
            })
            .collect();
        mt_strings.sort_by(|a, b| {
            let first = "application/json";
            if a.0.starts_with(first) {
                Ordering::Less
            } else if b.0.starts_with(first) {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        });
        mt_strings.into_iter().map(|mt| mt.try_into()).collect()
    }

    fn get_extension(&self, media_type: &MediaType<'_>) -> Result<Option<&str>> {
        for (k, v) in self.map.iter() {
            if let serde_json::Value::String(mt) = v {
                let mt: MediaType<'_> = MediaTypeString(mt).try_into()?;
                if mt == *media_type {
                    return Ok(Some(k.as_str()));
                }
            }
        }
        Ok(None)
    }
}

struct MediaTypeString<'a>(&'a String);

impl<'a> TryInto<MediaType<'a>> for MediaTypeString<'a> {
    type Error = anyhow::Error;

    fn try_into(self) -> std::result::Result<MediaType<'a>, Self::Error> {
        match MediaType::parse(self.0.as_str()) {
            Ok(mt) => Ok(mt),
            Err(e) => bail!("should be able to parse \"{}\" as media type: {e}", self.0),
        }
    }
}
