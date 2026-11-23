use std::{
    io::{Cursor, Read, Seek, SeekFrom, Write},
    path::Path,
};

use base64::Engine;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{error::Result, utils::TsUnknown};

use super::yaml_ext;

use super::{Resource, ResourceId, ResourceKind};

/// A self-contained local bundle. No marketplace, account or synthetic cloud
/// metadata is required to share a widget, plugin or wallpaper.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MacUiLocalResourceFile {
    pub format: String,
    pub version: u32,
    pub kind: ResourceKind,
    pub data: TsUnknown,
}

#[derive(Debug, Clone)]
pub enum MacUiPackage {
    Local(MacUiLocalResourceFile),
    Legacy(MacUiResourceFile),
}

impl MacUiLocalResourceFile {
    pub fn new<T: Serialize>(kind: ResourceKind, data: &T) -> Result<Self> {
        let file = Self {
            format: "macui".into(),
            version: 3,
            kind,
            data: serde_json::to_value(data)?.into(),
        };
        file.resource_id()?;
        Ok(file)
    }

    pub fn resource_id(&self) -> Result<ResourceId> {
        let id = self
            .data
            .0
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or("Local MACUI package has no resource id")?;
        let id = ResourceId::from(id);
        id.validate().map_err(crate::error::MacUiLibError::from)?;
        Ok(id)
    }

    pub fn encode<W: Write>(&self, writer: W) -> Result<()> {
        self.resource_id()?;
        let mut file = self.clone();
        file.format = "macui".into();
        file.version = 3;
        encode_document(&file, writer)
    }

    pub async fn store(&self, path: &Path) -> Result<()> {
        let mut buf = Vec::new();
        self.encode(&mut buf)?;
        tokio::fs::write(path.with_extension("maui"), buf).await?;
        Ok(())
    }
}

impl MacUiPackage {
    pub fn decode<R: Read + Seek>(reader: R) -> Result<Self> {
        let mut value = decode_document(reader)?;
        yaml_ext::adopt_macui_resource_ids(&mut value);
        if value.get("format").and_then(serde_yaml::Value::as_str) == Some("macui") {
            if value.get("version").and_then(serde_yaml::Value::as_u64) != Some(3) {
                return Err("Unsupported local MACUI package version".into());
            }
            let self_id = value
                .get("data")
                .and_then(|data| data.get("id"))
                .and_then(serde_yaml::Value::as_str)
                .ok_or("Local MACUI package has no resource id")?
                .to_owned();
            let vars = std::collections::HashMap::from([("self.id".to_string(), self_id)]);
            let value = yaml_ext::resolve_vars_yaml(value, &vars);
            let file: MacUiLocalResourceFile = serde_yaml::from_value(value)?;
            file.resource_id()?;
            Ok(Self::Local(file))
        } else {
            let self_id = yaml_ext::extract_self_id_maui(&value)?;
            let vars = std::collections::HashMap::from([("self.id".to_string(), self_id)]);
            let value = yaml_ext::resolve_vars_yaml(value, &vars);
            Ok(Self::Legacy(serde_yaml::from_value(value)?))
        }
    }

    pub async fn load(path: &Path) -> Result<Self> {
        let bytes = tokio::fs::read(path).await?;
        let mut file = tokio::task::spawn_blocking(move || Self::decode(Cursor::new(bytes)))
            .await
            .map_err(|error| crate::error::MacUiLibError::from(error.to_string()))??;
        if let Self::Legacy(file) = &mut file {
            file.resource.sanitize();
            file.resource.verify()?;
        }
        Ok(file)
    }

    pub fn kind(&self) -> ResourceKind {
        match self {
            Self::Local(file) => file.kind,
            Self::Legacy(file) => file.resource.kind,
        }
    }

    pub fn resource_id(&self) -> Result<ResourceId> {
        match self {
            Self::Local(file) => file.resource_id(),
            Self::Legacy(file) => Ok(ResourceId::Remote(file.resource.id)),
        }
    }

    pub fn try_parse_into<T: DeserializeOwned>(&self) -> Result<T> {
        match self {
            Self::Local(file) => Ok(serde_json::from_value(file.data.0.clone())?),
            Self::Legacy(file) => file.try_parse_into(),
        }
    }

    pub fn replace_data<T: Serialize>(&mut self, value: &T) -> Result<()> {
        let data = serde_json::to_value(value)?.into();
        match self {
            Self::Local(file) => file.data = data,
            Self::Legacy(file) => file.data = data,
        }
        Ok(())
    }

    pub async fn store(&self, path: &Path) -> Result<()> {
        match self {
            Self::Local(file) => file.store(path).await,
            Self::Legacy(file) => file.store(path).await,
        }
    }
}

fn decode_document<R: Read + Seek>(mut reader: R) -> Result<serde_yaml::Value> {
    let mut prefix = [0u8; 1];
    reader.read_exact(&mut prefix)?;
    if prefix[0] == b'M' {
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if &magic != b"ACUI" {
            return Err("Invalid MACUI package header".into());
        }
        let mut version = [0u8; 1];
        reader.read_exact(&mut version)?;
        if version[0] != 3 {
            return Err("Unsupported MACUI package version".into());
        }
        reader.seek(SeekFrom::Current(4))?;
    } else if matches!(prefix[0], 1 | 2) {
        let mut magic = [0u8; 3];
        reader.read_exact(&mut magic)?;
        if &magic != b"SLU" {
            return Err("Invalid legacy resource package header".into());
        }
        if prefix[0] == 2 {
            reader.seek(SeekFrom::Current(4))?;
        }
    } else {
        return Err("Unsupported MACUI package header".into());
    }
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    let decoded = base64::engine::general_purpose::STANDARD.decode(&buffer)?;
    Ok(serde_yaml::from_slice(&decoded)?)
}

fn encode_document<T: Serialize, W: Write>(data: &T, mut writer: W) -> Result<()> {
    let encoded = base64::engine::general_purpose::STANDARD.encode(serde_yaml::to_string(data)?);
    writer.write_all(b"MACUI")?;
    writer.write_all(&[3])?;
    writer.write_all(&[0u8; 4])?;
    writer.write_all(encoded.as_bytes())?;
    Ok(())
}

/// A container for MAC UI resources.
///
/// This struct contains all the necessary data that a resource needs.
/// New packages use `.maui` with the MACUI magic header. Legacy packages can be
/// read for import compatibility, but are never written back in the old format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(all(feature = "gen-binds", not(feature = "salvo")), ts(export))]
pub struct MacUiResourceFile {
    pub version: u32,
    /// information about the downloaded resource
    pub resource: Resource,
    /// real resource data to be deserialized on load
    pub data: TsUnknown,
}

impl MacUiResourceFile {
    pub fn decode<R: Read + Seek>(reader: R) -> Result<Self> {
        match MacUiPackage::decode(reader)? {
            MacUiPackage::Legacy(file) => Ok(file),
            MacUiPackage::Local(_) => {
                Err("Expected legacy resource container, found local MACUI bundle".into())
            }
        }
    }

    pub fn encode<W: Write>(&self, writer: W) -> Result<()> {
        let mut package = self.clone();
        package.version = 3;
        encode_document(&package, writer)
    }

    pub async fn load(path: &Path) -> Result<Self> {
        let bytes = tokio::fs::read(path).await?;
        // base64 + yaml parsing is CPU-bound, offload it to the blocking pool so loading many
        // resources concurrently doesn't serialize on (or stall) the async worker threads.
        let mut decoded = tokio::task::spawn_blocking(move || Self::decode(Cursor::new(bytes)))
            .await
            .map_err(|e| crate::error::MacUiLibError::from(e.to_string()))??;
        decoded.resource.sanitize();
        decoded.resource.verify()?;
        Ok(decoded)
    }

    pub async fn store(&self, path: &Path) -> Result<()> {
        let mut buf = Vec::new();
        self.encode(&mut buf)?;
        tokio::fs::write(path.with_extension("maui"), buf).await?;
        Ok(())
    }

    pub fn try_parse_into<T>(&self) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let mut obj = serde_json::value::Map::new();

        obj.insert(
            "id".to_string(),
            serde_json::Value::String(self.resource.id.to_string()),
        );

        obj.insert(
            "metadata".to_string(),
            serde_json::to_value(&self.resource.metadata)?,
        );

        let data = self.data.0.as_object().ok_or("invalid data")?;
        obj.append(&mut data.clone());

        Ok(serde_json::from_value(obj.into())?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        resource::MacUiResource,
        state::{Plugin, Wallpaper, Widget},
    };

    fn fixture() -> MacUiResourceFile {
        serde_json::from_value(serde_json::json!({
            "version": 2,
            "resource": {
                "id": "00000000-0000-4000-8000-000000000001",
                "dataId": "00000000-0000-4000-8000-000000000002",
                "creatorId": "00000000-0000-4000-8000-000000000003",
                "kind": "Widget",
                "metadata": {"displayName": "Test"},
                "createdAt": "2026-10-11T00:00:00Z", "updatedAt": "2026-10-11T00:00:00Z",
                "status": "Published", "rejectedReason": null, "reviewedAt": null,
                "reviewedBy": null, "deletedAt": null, "version": 1, "stars": 0,
                "downloads": 0, "pendingUpdate": null, "rejectedPendingReason": null,
                "friendlyId": null
            },
            "data": {"value": "${self.id}"}
        }))
        .unwrap()
    }

    #[test]
    fn new_packages_roundtrip_with_macui_header_and_version_three() {
        let file = fixture();
        let mut encoded = Vec::new();
        file.encode(&mut encoded).unwrap();
        assert!(encoded.starts_with(b"MACUI\x03"));
        let decoded = MacUiResourceFile::decode(Cursor::new(encoded)).unwrap();
        assert_eq!(decoded.version, 3);
        assert_eq!(decoded.resource.id, file.resource.id);
        assert_eq!(decoded.data.0["value"], file.resource.id.to_string());
    }

    #[test]
    fn legacy_containers_are_readable_but_reencode_only_as_macui() {
        let file = fixture();
        let payload =
            base64::engine::general_purpose::STANDARD.encode(serde_yaml::to_string(&file).unwrap());
        for version in [1, 2] {
            let mut bytes = vec![version];
            bytes.extend_from_slice(b"SLU");
            if version == 2 {
                bytes.extend_from_slice(&[0; 4]);
            }
            bytes.extend_from_slice(payload.as_bytes());
            let decoded = MacUiResourceFile::decode(Cursor::new(bytes)).unwrap();
            let mut encoded = Vec::new();
            decoded.encode(&mut encoded).unwrap();
            assert!(encoded.starts_with(b"MACUI\x03"));
        }
    }

    #[test]
    fn wrong_magic_and_unknown_versions_are_rejected() {
        for bytes in [
            b"NOTUI\x03\0\0\0\0".as_slice(),
            b"MACUI\x04\0\0\0\0",
            b"\x02BAD\0\0\0\0",
        ] {
            assert!(MacUiResourceFile::decode(Cursor::new(bytes)).is_err());
        }
    }

    #[test]
    fn local_bundles_roundtrip_without_cloud_or_account_metadata() {
        for kind in [
            ResourceKind::Widget,
            ResourceKind::Plugin,
            ResourceKind::Wallpaper,
        ] {
            let file = MacUiLocalResourceFile::new(
                kind,
                &serde_json::json!({
                    "id": "@creator/my-resource", "metadata": {"displayName": "Local test"},
                    "html": "<main>${self.id}</main>", "target": "@seelen/weg"
                }),
            )
            .unwrap();
            let mut bytes = Vec::new();
            file.encode(&mut bytes).unwrap();
            assert!(bytes.starts_with(b"MACUI\x03"));
            let package = MacUiPackage::decode(Cursor::new(bytes)).unwrap();
            assert_eq!(package.kind(), kind);
            assert_eq!(
                package.resource_id().unwrap(),
                "@creator/my-resource".into()
            );
            let MacUiPackage::Local(file) = package else {
                panic!("Expected local package");
            };
            assert_eq!(file.data.0["target"], "@macui/weg");
            assert_eq!(file.data.0["html"], "<main>@creator/my-resource</main>");
            assert!(file.data.0.get("creatorId").is_none());
        }
    }

    #[test]
    fn local_resource_ids_reject_paths_and_missing_ids() {
        for value in [
            serde_json::json!({}),
            serde_json::json!({"id": "../../outside"}),
        ] {
            assert!(MacUiLocalResourceFile::new(ResourceKind::Widget, &value).is_err());
        }
    }

    #[tokio::test]
    async fn local_packages_load_and_save_through_all_supported_resource_types() {
        let root =
            std::env::temp_dir().join(format!("macui-package-test-{}", uuid::Uuid::new_v4()));
        tokio::fs::create_dir(&root).await.unwrap();
        let data = serde_json::json!({
            "id": "@creator/my-resource", "html": "<main>Local</main>", "type": "Layered",
            "target": "@creator/widget-ui", "plugin": {"value": "Local"}
        });
        for kind in [
            ResourceKind::Widget,
            ResourceKind::Plugin,
            ResourceKind::Wallpaper,
        ] {
            let requested = root.join(format!("{kind:?}.slu"));
            let path = requested.with_extension("maui");
            MacUiLocalResourceFile::new(kind, &data)
                .unwrap()
                .store(&requested)
                .await
                .unwrap();
            assert!(!requested.exists());
            match kind {
                ResourceKind::Widget => {
                    let mut resource = Widget::load(&path).await.unwrap();
                    assert!(resource.metadata.internal.remote.is_none());
                    resource.html = Some("<main>Updated</main>".into());
                    resource.save().await.unwrap();
                    assert_eq!(
                        Widget::load(&path).await.unwrap().html.as_deref(),
                        Some("<main>Updated</main>")
                    );
                }
                ResourceKind::Plugin => {
                    assert!(
                        Plugin::load(&path)
                            .await
                            .unwrap()
                            .metadata
                            .internal
                            .remote
                            .is_none()
                    );
                }
                ResourceKind::Wallpaper => {
                    assert!(
                        Wallpaper::load(&path)
                            .await
                            .unwrap()
                            .metadata
                            .internal
                            .remote
                            .is_none()
                    );
                }
                _ => unreachable!(),
            }
            assert!(
                tokio::fs::read(&path)
                    .await
                    .unwrap()
                    .starts_with(b"MACUI\x03")
            );
        }
        tokio::fs::remove_dir_all(root).await.unwrap(); // fresh test-only directory
    }
}
