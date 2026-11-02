//! Lossless Windows path extraction for Lance's wrapped local-store scheme.
//!
//! Only extraction changes: the delegated store retains the original
//! `file-object-store` scheme, so Lance continues through ObjectStore wrappers.

use std::collections::HashMap;
use std::sync::Arc;

use lance_io::object_store::{ObjectStore, ObjectStoreParams, ObjectStoreProvider};
use object_store::path::Path as ObjectPath;
use reqwest::Url;

#[derive(Debug)]
struct WindowsFileObjectStoreProvider {
    original: Arc<dyn ObjectStoreProvider>,
}

#[async_trait::async_trait]
impl ObjectStoreProvider for WindowsFileObjectStoreProvider {
    async fn new_store(&self, url: Url, params: &ObjectStoreParams) -> lance::Result<ObjectStore> {
        self.original.new_store(url, params).await
    }

    fn extract_path(&self, url: &Url) -> lance::Result<ObjectPath> {
        if url.query().is_some() || url.fragment().is_some() || url.host_str().is_some() {
            return Err(lance::Error::invalid_input(
                "wrapped local URL must not contain an authority, query or fragment".to_string(),
            ));
        }
        let suffix = url.as_str().strip_prefix("file-object-store:")
            .ok_or_else(|| lance::Error::invalid_input("expected file-object-store URL".to_string()))?;
        // `file:///C:` names a drive-relative path to url 2.5.8 and its
        // `to_file_path` debug assertion panics. A bare drive in a URL denotes
        // the absolute volume root, which needs its trailing separator.
        let path = url.path().as_bytes();
        let is_drive_root = matches!(path,
            [b'/', drive, b':'] if drive.is_ascii_alphabetic())
            || matches!(path,
                [b'/', drive, b'%', b'3', colon] if drive.is_ascii_alphabetic() && (*colon == b'a' || *colon == b'A'));
        let file_uri = if is_drive_root {
            format!("file:{suffix}/")
        } else {
            format!("file:{suffix}")
        };
        let file_url = Url::parse(&file_uri)
            .map_err(|error| lance::Error::invalid_input(format!("invalid local file URL: {error}")))?;
        let path = file_url.to_file_path().map_err(|_| {
            lance::Error::invalid_input("wrapped URL is not an absolute local file path".to_string())
        })?;
        ObjectPath::from_absolute_path(&path).map_err(|error| {
            lance::Error::invalid_input(format!("invalid local object path: {error}"))
        })
    }

    fn calculate_object_store_prefix(
        &self,
        url: &Url,
        storage_options: Option<&HashMap<String, String>>,
    ) -> lance::Result<String> {
        self.original.calculate_object_store_prefix(url, storage_options)
    }
}

/// A session whose wrapped local provider decodes path escapes exactly once.
/// Every create, open, reopen and connection participating in the same dataset
/// must receive this session; an ordinary session uses the stock provider.
pub(crate) fn windows_wrapped_session() -> Arc<lance::session::Session> {
    let session = Arc::new(lance::session::Session::default());
    let registry = session.store_registry();
    let original = registry.get_provider("file-object-store")
        .expect("Lance default registry contains file-object-store");
    registry.insert("file-object-store", Arc::new(WindowsFileObjectStoreProvider { original }));
    session
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaped_unicode_and_reserved_names_decode_once() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("данные #100%.lance");
        std::fs::create_dir(&path).unwrap();
        let file_url = Url::from_file_path(&path).unwrap();
        let url = Url::parse(&file_url.as_str().replacen("file:", "file-object-store:", 1)).unwrap();
        let session = windows_wrapped_session();
        let provider = session.store_registry().get_provider("file-object-store").unwrap();
        let actual = provider.extract_path(&url).unwrap();
        assert_eq!(actual, ObjectPath::from_absolute_path(&path).unwrap());
        assert_eq!(provider.calculate_object_store_prefix(&url, None).unwrap(), "file-object-store");
        let question = Url::parse("file-object-store:///C:/x%3Fy%23z%25").unwrap();
        assert_eq!(provider.extract_path(&question).unwrap().as_ref(), "C:/x?y#z%");
        for uri in ["file-object-store:///C:", "file-object-store:///C%3A"] {
            let root = provider.extract_path(&Url::parse(uri).unwrap()).unwrap();
            assert_eq!(root, ObjectPath::from_absolute_path(r"C:\").unwrap());
        }
        for invalid in [
            "file-object-store://server/share/path",
            "file-object-store:///C:/path?query",
            "file-object-store:///C:/path#fragment",
        ] {
            assert!(provider.extract_path(&Url::parse(invalid).unwrap()).is_err());
        }
    }
}
