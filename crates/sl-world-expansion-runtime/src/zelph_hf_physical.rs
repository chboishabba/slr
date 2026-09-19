//! Zelph HF v2/v3 physical-object planning and bounded cached transport.
//!
//! This module deliberately works at the shard-object layer advertised by
//! Zelph's manifest/nodeRouteIndex contract.  It does not interpret ontology
//! edges or pay residuals; it only turns an already-resolved internal node into
//! the exact physical objects required to inspect that node.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;

use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::sprint1_acquisition_machine::{
    AcquisitionPath, BoundedPhysicalTransport, LogicalAcquisitionRequest, LogicalToPhysicalPlanner,
    PhysicalObjectFetch, PlannedPhysicalObject, ResolvedPhysicalObjects, Sprint1AcquisitionError,
    MAX_COLD_REMOTE_PHYSICAL_OBJECTS,
};

#[derive(Debug, Clone)]
pub struct ZelphHfPhysicalPlanner {
    manifest: Value,
    route_index: Value,
    resolved_nodes: BTreeMap<String, u64>,
    header_source_override: Option<String>,
    acquisition_path: AcquisitionPath,
}

impl ZelphHfPhysicalPlanner {
    pub fn from_json(
        manifest_json: &str,
        route_index_json: &str,
        resolved_nodes: BTreeMap<String, u64>,
        header_source_override: Option<String>,
        acquisition_path: AcquisitionPath,
    ) -> Result<Self, Sprint1AcquisitionError> {
        let manifest = serde_json::from_str(manifest_json)
            .map_err(|error| Sprint1AcquisitionError::Provider(format!("zelph-manifest-json:{error}")))?;
        let route_index = serde_json::from_str(route_index_json)
            .map_err(|error| Sprint1AcquisitionError::Provider(format!("zelph-route-json:{error}")))?;
        Ok(Self {
            manifest,
            route_index,
            resolved_nodes,
            header_source_override,
            acquisition_path,
        })
    }

    pub fn from_files(
        manifest_path: impl AsRef<Path>,
        route_index_path: impl AsRef<Path>,
        resolved_nodes: BTreeMap<String, u64>,
        header_source_override: Option<String>,
        acquisition_path: AcquisitionPath,
    ) -> Result<Self, Sprint1AcquisitionError> {
        let manifest_json = fs::read_to_string(manifest_path)
            .map_err(|error| Sprint1AcquisitionError::Provider(format!("zelph-manifest-read:{error}")))?;
        let route_index_json = fs::read_to_string(route_index_path)
            .map_err(|error| Sprint1AcquisitionError::Provider(format!("zelph-route-read:{error}")))?;
        Self::from_json(
            &manifest_json,
            &route_index_json,
            resolved_nodes,
            header_source_override,
            acquisition_path,
        )
    }

    fn section_chunks(&self, section: &str) -> Result<&Vec<Value>, Sprint1AcquisitionError> {
        self.manifest
            .get("sections")
            .and_then(|v| v.get(section))
            .and_then(|v| v.get("chunks"))
            .and_then(Value::as_array)
            .ok_or_else(|| {
                Sprint1AcquisitionError::Provider(format!(
                    "zelph manifest missing sections.{section}.chunks"
                ))
            })
    }

    fn routed_chunk_indexes(
        &self,
        section: &str,
        node_id: u64,
        route_name: &str,
    ) -> Result<BTreeSet<u64>, Sprint1AcquisitionError> {
        let entries = self
            .route_index
            .get("routing")
            .and_then(|v| v.get(section))
            .and_then(Value::as_array)
            .ok_or_else(|| {
                Sprint1AcquisitionError::Provider(format!(
                    "zelph nodeRouteIndex missing routing.{section}"
                ))
            })?;
        let mut indexes = BTreeSet::new();
        for entry in entries {
            let index = entry
                .get("chunkIndex")
                .and_then(Value::as_u64)
                .ok_or_else(|| {
                    Sprint1AcquisitionError::Provider(format!(
                        "zelph nodeRouteIndex {section} entry missing chunkIndex"
                    ))
                })?;
            let matched = if section == "nodeOfName" {
                entry.get("lang").and_then(Value::as_str) == Some("wikidata")
                    && entry
                        .get("names")
                        .and_then(Value::as_array)
                        .is_some_and(|names| names.iter().any(|name| name.as_str() == Some(route_name)))
            } else {
                entry
                    .get("nodes")
                    .and_then(Value::as_array)
                    .is_some_and(|nodes| nodes.iter().any(|node| node.as_u64() == Some(node_id)))
            };
            if matched {
                indexes.insert(index);
            }
        }
        Ok(indexes)
    }

    fn source_bin(&self) -> Option<String> {
        self.header_source_override.clone().or_else(|| {
            self.manifest
                .get("source")
                .and_then(|v| v.get("binPath").or_else(|| v.get("path")))
                .and_then(Value::as_str)
                .map(str::to_owned)
                .or_else(|| {
                    self.manifest
                        .get("hfObjects")
                        .and_then(|v| v.get("bin"))
                        .and_then(|v| v.get("path"))
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
        })
    }

    fn header_length(&self) -> Option<u64> {
        self.manifest
            .get("source")
            .and_then(|v| {
                v.get("headerLengthBytes")
                    .or_else(|| v.get("headerLength"))
                    .or_else(|| v.get("header_length_bytes"))
            })
            .and_then(Value::as_u64)
    }

    fn header_object(&self) -> Option<PlannedPhysicalObject> {
        let source = self.source_bin()?;
        let length = self.header_length()?;
        if length == 0 {
            return None;
        }
        let end = length.saturating_sub(1);
        Some(PlannedPhysicalObject {
            physical_object_ref: format!("zelph-hf:header:{source}:0-{end}"),
            cache_key: format!("zelph-hf:header:{source}:0-{end}"),
            route_ref: "zelph-hf:header".into(),
            source_ref: source,
            source_range: Some((0, end)),
            expected_bytes: Some(length),
            acquisition_path: self.acquisition_path,
        })
    }

    fn chunk_object(
        &self,
        section: &str,
        chunk_index: u64,
    ) -> Result<PlannedPhysicalObject, Sprint1AcquisitionError> {
        let chunk = self
            .section_chunks(section)?
            .iter()
            .find(|chunk| chunk.get("chunkIndex").and_then(Value::as_u64) == Some(chunk_index))
            .ok_or_else(|| {
                Sprint1AcquisitionError::Provider(format!(
                    "zelph manifest missing {section} chunk {chunk_index}"
                ))
            })?;

        let length = chunk
            .get("length")
            .or_else(|| chunk.get("object").and_then(|v| v.get("sizeBytes")))
            .and_then(Value::as_u64);

        if let Some(object_path) = chunk.get("objectPath").and_then(Value::as_str) {
            let object_path = object_path.to_owned();
            return Ok(PlannedPhysicalObject {
                physical_object_ref: format!("zelph-hf:{section}:{chunk_index}:{object_path}"),
                cache_key: format!("zelph-hf:{section}:{chunk_index}:{object_path}"),
                route_ref: format!("zelph-hf:{section}:{chunk_index}"),
                source_ref: object_path,
                source_range: None,
                expected_bytes: length,
                acquisition_path: self.acquisition_path,
            });
        }

        let source = self.source_bin().ok_or_else(|| {
            Sprint1AcquisitionError::Provider(
                "zelph manifest chunk has no objectPath and no source bin".into(),
            )
        })?;
        let offset = chunk
            .get("sourceOffset")
            .and_then(Value::as_u64)
            .ok_or_else(|| {
                Sprint1AcquisitionError::Provider(format!(
                    "zelph manifest {section} chunk {chunk_index} missing sourceOffset"
                ))
            })?;
        let length = length.ok_or_else(|| {
            Sprint1AcquisitionError::Provider(format!(
                "zelph manifest {section} chunk {chunk_index} missing length"
            ))
        })?;
        let end = offset.saturating_add(length.saturating_sub(1));
        Ok(PlannedPhysicalObject {
            physical_object_ref: format!(
                "zelph-hf:{section}:{chunk_index}:{source}:{offset}-{end}"
            ),
            cache_key: format!("zelph-hf:{section}:{chunk_index}:{source}:{offset}-{end}"),
            route_ref: format!("zelph-hf:{section}:{chunk_index}"),
            source_ref: source,
            source_range: Some((offset, end)),
            expected_bytes: Some(length),
            acquisition_path: self.acquisition_path,
        })
    }

    fn objects_for(
        &self,
        target: &str,
        node_id: u64,
    ) -> Result<Vec<PlannedPhysicalObject>, Sprint1AcquisitionError> {
        let mut objects = BTreeMap::<String, PlannedPhysicalObject>::new();
        if let Some(header) = self.header_object() {
            objects.insert(header.physical_object_ref.clone(), header);
        }

        for section in ["left", "right", "nameOfNode", "nodeOfName"] {
            for index in self.routed_chunk_indexes(section, node_id, target)? {
                let object = self.chunk_object(section, index)?;
                objects.insert(object.physical_object_ref.clone(), object);
            }
        }

        if objects.is_empty() {
            return Err(Sprint1AcquisitionError::UnresolvedLogicalRequest(
                target.to_owned(),
            ));
        }
        Ok(objects.into_values().collect())
    }
}

impl LogicalToPhysicalPlanner for ZelphHfPhysicalPlanner {
    fn resolve(
        &mut self,
        request: &LogicalAcquisitionRequest,
    ) -> Result<ResolvedPhysicalObjects, Sprint1AcquisitionError> {
        let node_id = self
            .resolved_nodes
            .get(&request.semantic_target_ref)
            .copied()
            .ok_or_else(|| {
                Sprint1AcquisitionError::UnresolvedLogicalRequest(request.request_ref.clone())
            })?;
        Ok(ResolvedPhysicalObjects {
            resolved_node_ref: format!("zelph-node:{node_id}"),
            objects: self.objects_for(&request.semantic_target_ref, node_id)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct CurlCachedPhysicalTransport {
    cache_dir: PathBuf,
}

impl CurlCachedPhysicalTransport {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Result<Self, Sprint1AcquisitionError> {
        let cache_dir = cache_dir.into();
        fs::create_dir_all(&cache_dir)
            .map_err(|error| Sprint1AcquisitionError::Provider(format!("cache-create:{error}")))?;
        Ok(Self { cache_dir })
    }

    fn cache_path_for(cache_dir: &Path, object: &PlannedPhysicalObject) -> PathBuf {
        let mut hasher = Sha256::new();
        hasher.update(object.cache_key.as_bytes());
        let token = hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        cache_dir.join(format!("{token}.bin"))
    }

    fn digest_file(path: &Path) -> Result<String, Sprint1AcquisitionError> {
        let mut file = fs::File::open(path)
            .map_err(|error| Sprint1AcquisitionError::Provider(format!("cache-open:{error}")))?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 64 * 1024];
        loop {
            let read = file
                .read(&mut buffer)
                .map_err(|error| Sprint1AcquisitionError::Provider(format!("cache-read:{error}")))?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        Ok(format!(
            "sha256:{}",
            hasher
                .finalize()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        ))
    }

    fn hf_to_http(source: &str) -> String {
        if source.starts_with("http://") || source.starts_with("https://") {
            return source.to_owned();
        }
        let Some(relative) = source.strip_prefix("hf://") else {
            return source.to_owned();
        };
        let mut parts = relative.splitn(4, '/');
        let kind = parts.next().unwrap_or_default();
        let owner = parts.next().unwrap_or_default();
        let repo = parts.next().unwrap_or_default();
        let file = parts.next().unwrap_or_default();
        if owner.is_empty() || repo.is_empty() || file.is_empty() {
            return format!("https://huggingface.co/{relative}");
        }
        match kind {
            "datasets" | "spaces" => {
                format!("https://huggingface.co/{kind}/{owner}/{repo}/resolve/main/{file}")
            }
            "models" => {
                format!("https://huggingface.co/{owner}/{repo}/resolve/main/{file}")
            }
            _ => format!("https://huggingface.co/{relative}"),
        }
    }

    fn copy_local_range(
        source: &Path,
        destination: &Path,
        range: Option<(u64, u64)>,
    ) -> Result<(), Sprint1AcquisitionError> {
        let mut input = fs::File::open(source)
            .map_err(|error| Sprint1AcquisitionError::Provider(format!("source-open:{error}")))?;
        let mut output = fs::File::create(destination)
            .map_err(|error| Sprint1AcquisitionError::Provider(format!("cache-create:{error}")))?;

        let remaining = if let Some((start, end)) = range {
            input
                .seek(SeekFrom::Start(start))
                .map_err(|error| Sprint1AcquisitionError::Provider(format!("source-seek:{error}")))?;
            Some(end.saturating_sub(start).saturating_add(1))
        } else {
            None
        };

        match remaining {
            Some(mut remaining) => {
                let mut buffer = [0u8; 64 * 1024];
                while remaining > 0 {
                    let want = usize::try_from(remaining.min(buffer.len() as u64)).unwrap_or(buffer.len());
                    let read = input
                        .read(&mut buffer[..want])
                        .map_err(|error| Sprint1AcquisitionError::Provider(format!("source-read:{error}")))?;
                    if read == 0 {
                        break;
                    }
                    output
                        .write_all(&buffer[..read])
                        .map_err(|error| Sprint1AcquisitionError::Provider(format!("cache-write:{error}")))?;
                    remaining = remaining.saturating_sub(read as u64);
                }
                if remaining != 0 {
                    return Err(Sprint1AcquisitionError::Provider(
                        "source range ended before expected length".into(),
                    ));
                }
            }
            None => {
                std::io::copy(&mut input, &mut output)
                    .map_err(|error| Sprint1AcquisitionError::Provider(format!("source-copy:{error}")))?;
            }
        }
        Ok(())
    }

    fn fetch_one(
        cache_dir: PathBuf,
        object: PlannedPhysicalObject,
    ) -> Result<PhysicalObjectFetch, Sprint1AcquisitionError> {
        let final_path = Self::cache_path_for(&cache_dir, &object);
        let temp_path = final_path.with_extension(format!("{}.tmp", std::process::id()));
        let _ = fs::remove_file(&temp_path);

        let source = object
            .source_ref
            .strip_prefix("file://")
            .unwrap_or(&object.source_ref);
        let source_path = Path::new(source);
        if source_path.exists() {
            Self::copy_local_range(source_path, &temp_path, object.source_range)?;
        } else {
            let url = Self::hf_to_http(&object.source_ref);
            let mut command = Command::new("curl");
            command.arg("-fsSL");
            if let Some((start, end)) = object.source_range {
                command.arg("--range").arg(format!("{start}-{end}"));
            }
            let status = command
                .arg(&url)
                .arg("-o")
                .arg(&temp_path)
                .status()
                .map_err(|error| Sprint1AcquisitionError::Provider(format!("curl-spawn:{error}")))?;
            if !status.success() {
                let _ = fs::remove_file(&temp_path);
                return Err(Sprint1AcquisitionError::Provider(format!(
                    "curl failed for {} with {status}",
                    object.physical_object_ref
                )));
            }
        }

        let bytes = fs::metadata(&temp_path)
            .map_err(|error| Sprint1AcquisitionError::Provider(format!("cache-stat:{error}")))?
            .len();
        if object.expected_bytes.is_some_and(|expected| expected != bytes) {
            let _ = fs::remove_file(&temp_path);
            return Err(Sprint1AcquisitionError::Provider(format!(
                "physical object size mismatch for {}: expected {:?}, observed {bytes}",
                object.physical_object_ref, object.expected_bytes
            )));
        }
        fs::rename(&temp_path, &final_path)
            .map_err(|error| Sprint1AcquisitionError::Provider(format!("cache-rename:{error}")))?;
        let evidence_digest_ref = Self::digest_file(&final_path)?;
        Ok(PhysicalObjectFetch {
            physical_object_ref: object.physical_object_ref,
            bytes: usize::try_from(bytes).unwrap_or(usize::MAX),
            from_cache: false,
            live_fallback: object.acquisition_path == AcquisitionPath::GovernedLiveFallback,
            complete: true,
            source_revision_ref: object.route_ref,
            evidence_digest_ref,
        })
    }
}

impl BoundedPhysicalTransport for CurlCachedPhysicalTransport {
    fn cached(
        &mut self,
        object: &PlannedPhysicalObject,
    ) -> Result<Option<PhysicalObjectFetch>, Sprint1AcquisitionError> {
        let path = Self::cache_path_for(&self.cache_dir, object);
        if !path.exists() {
            return Ok(None);
        }
        let bytes = fs::metadata(&path)
            .map_err(|error| Sprint1AcquisitionError::Provider(format!("cache-stat:{error}")))?
            .len();
        if object.expected_bytes.is_some_and(|expected| expected != bytes) {
            fs::remove_file(&path)
                .map_err(|error| Sprint1AcquisitionError::Provider(format!("cache-remove:{error}")))?;
            return Ok(None);
        }
        Ok(Some(PhysicalObjectFetch {
            physical_object_ref: object.physical_object_ref.clone(),
            bytes: 0,
            from_cache: true,
            live_fallback: false,
            complete: true,
            source_revision_ref: object.route_ref.clone(),
            evidence_digest_ref: Self::digest_file(&path)?,
        }))
    }

    fn fetch_cold_batch(
        &mut self,
        objects: &[PlannedPhysicalObject],
    ) -> Result<Vec<PhysicalObjectFetch>, Sprint1AcquisitionError> {
        if objects.len() > MAX_COLD_REMOTE_PHYSICAL_OBJECTS {
            return Err(Sprint1AcquisitionError::ColdBatchTooWide(objects.len()));
        }
        let mut handles = Vec::with_capacity(objects.len());
        for object in objects.iter().cloned() {
            let cache_dir = self.cache_dir.clone();
            handles.push(thread::spawn(move || Self::fetch_one(cache_dir, object)));
        }
        let mut rows = Vec::with_capacity(handles.len());
        for handle in handles {
            let row = handle.join().map_err(|_| {
                Sprint1AcquisitionError::Provider("physical object fetch thread panicked".into())
            })??;
            rows.push(row);
        }
        rows.sort_by(|left, right| left.physical_object_ref.cmp(&right.physical_object_ref));
        Ok(rows)
    }
}
