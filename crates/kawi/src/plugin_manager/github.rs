use std::path::Path;

use kawi_plugin::{validate_component_path, PluginManifest, PluginRuntime};
use serde::Deserialize;

use super::http::{encode_path_segment, HttpClient};
use super::source::PluginSpec;

const API_ROOT: &str = "https://api.github.com";
const RAW_ROOT: &str = "https://raw.githubusercontent.com";
const GITHUB_ROOT: &str = "https://github.com";
const MANIFEST_FILE: &str = "plugin.toml";
const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
const MAX_COMPONENT_BYTES: usize = 128 * 1024 * 1024;
const MAX_RELEASES: usize = 30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedRelease {
    pub(crate) tag: String,
    pub(crate) commit: String,
    pub(crate) author: String,
    pub(crate) manifest: String,
    pub(crate) component: String,
    pub(crate) asset_url: String,
}

pub(crate) trait ReleaseClient {
    fn resolve(&self, spec: &PluginSpec) -> Result<ResolvedRelease, String>;
    fn download(&self, release: &ResolvedRelease) -> Result<Vec<u8>, String>;
}

pub(crate) struct GithubClient {
    http: HttpClient,
}

impl GithubClient {
    pub(crate) fn new() -> Self {
        Self {
            http: HttpClient::new(),
        }
    }

    fn release(&self, spec: &PluginSpec) -> Result<ReleaseInfo, String> {
        let Some(reference) = spec.reference.as_deref() else {
            return self.latest_release(spec);
        };
        if let Some(release) = self.release_by_tag(spec, reference)? {
            return Ok(release);
        }
        let commit = self.resolve_commit(spec, reference)?;
        self.release_for_commit(spec, &commit)
    }

    fn latest_release(&self, spec: &PluginSpec) -> Result<ReleaseInfo, String> {
        let url = format!(
            "{API_ROOT}/repos/{}/{}/releases/latest",
            spec.owner, spec.repo
        );
        let release: ApiRelease = self.http.get_json(&url, MAX_MANIFEST_BYTES)?;
        self.finish_release(spec, release)
    }

    fn release_by_tag(&self, spec: &PluginSpec, tag: &str) -> Result<Option<ReleaseInfo>, String> {
        let url = format!(
            "{API_ROOT}/repos/{}/{}/releases/tags/{}",
            spec.owner,
            spec.repo,
            encode_path_segment(tag)
        );
        match self
            .http
            .get_optional_json::<ApiRelease>(&url, MAX_MANIFEST_BYTES)?
        {
            Some(release) => self.finish_release(spec, release).map(Some),
            None => Ok(None),
        }
    }

    fn release_for_commit(&self, spec: &PluginSpec, commit: &str) -> Result<ReleaseInfo, String> {
        let url = format!(
            "{API_ROOT}/repos/{}/{}/releases?per_page={MAX_RELEASES}",
            spec.owner, spec.repo
        );
        let releases: Vec<ApiRelease> = self.http.get_json(&url, MAX_MANIFEST_BYTES)?;
        for release in releases {
            if release.target_commitish == commit {
                return self.finish_release(spec, release);
            }
            let Ok(tag_commit) = self.resolve_commit(spec, &release.tag_name) else {
                continue;
            };
            if tag_commit == commit {
                return self.finish_release(spec, release);
            }
        }
        Err(format!(
            "no release found for {} at commit {commit}",
            spec.source()
        ))
    }

    fn finish_release(
        &self,
        spec: &PluginSpec,
        release: ApiRelease,
    ) -> Result<ReleaseInfo, String> {
        let commit = self.resolve_commit(spec, &release.tag_name)?;
        let author = release
            .author
            .map(|author| author.login)
            .filter(|login| !login.is_empty())
            .unwrap_or_else(|| spec.owner.clone());
        Ok(ReleaseInfo {
            tag: release.tag_name,
            commit,
            author,
            assets: release
                .assets
                .into_iter()
                .map(|asset| ReleaseAsset {
                    name: asset.name,
                    url: asset.browser_download_url,
                })
                .collect(),
        })
    }

    fn resolve_commit(&self, spec: &PluginSpec, reference: &str) -> Result<String, String> {
        let url = format!(
            "{API_ROOT}/repos/{}/{}/commits/{}",
            spec.owner,
            spec.repo,
            encode_path_segment(reference)
        );
        let commit: ApiCommit = self.http.get_json(&url, MAX_MANIFEST_BYTES)?;
        Ok(commit.sha)
    }

    fn manifest(&self, spec: &PluginSpec, commit: &str) -> Result<String, String> {
        let url = format!(
            "{RAW_ROOT}/{}/{}/{commit}/{MANIFEST_FILE}",
            spec.owner, spec.repo
        );
        self.http.get_text(&url, MAX_MANIFEST_BYTES)
    }

    fn asset_url(&self, release: &ReleaseInfo, component: &str) -> Result<String, String> {
        validate_component_path(component)?;
        let name = Path::new(component)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| "component must name a file".to_string())?;
        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == name)
            .ok_or_else(|| {
                let names = release
                    .assets
                    .iter()
                    .map(|asset| asset.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "release {} has no asset {name}; assets: {names}",
                    release.tag
                )
            })?;
        if !asset.url.starts_with(&format!("{GITHUB_ROOT}/")) {
            return Err("release asset URL is not on github.com".to_string());
        }
        Ok(asset.url.clone())
    }
}

impl ReleaseClient for GithubClient {
    fn resolve(&self, spec: &PluginSpec) -> Result<ResolvedRelease, String> {
        let release = self.release(spec)?;
        let manifest = self.manifest(spec, &release.commit)?;
        let parsed: PluginManifest = toml::from_str(&manifest)
            .map_err(|error| format!("invalid plugin manifest: {error}"))?;
        if parsed.runtime != PluginRuntime::Wasm {
            return Err("managed plugins must use the wasm runtime".to_string());
        }
        let component = parsed
            .component
            .as_deref()
            .ok_or_else(|| "wasm plugin manifest requires component".to_string())?;
        let asset_url = self.asset_url(&release, component)?;
        Ok(ResolvedRelease {
            tag: release.tag,
            commit: release.commit,
            author: release.author,
            manifest,
            component: component.to_string(),
            asset_url,
        })
    }

    fn download(&self, release: &ResolvedRelease) -> Result<Vec<u8>, String> {
        self.http.get_bytes(&release.asset_url, MAX_COMPONENT_BYTES)
    }
}

#[derive(Debug)]
struct ReleaseInfo {
    tag: String,
    commit: String,
    author: String,
    assets: Vec<ReleaseAsset>,
}

#[derive(Debug)]
struct ReleaseAsset {
    name: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct ApiRelease {
    tag_name: String,
    #[serde(default)]
    target_commitish: String,
    #[serde(default)]
    author: Option<ApiAuthor>,
    #[serde(default)]
    assets: Vec<ApiAsset>,
}

#[derive(Debug, Deserialize)]
struct ApiAuthor {
    login: String,
}

#[derive(Debug, Deserialize)]
struct ApiAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize)]
struct ApiCommit {
    sha: String,
}
