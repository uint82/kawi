use std::path::Path;

use sha2::{Digest, Sha256};

use super::github::{ReleaseClient, ResolvedRelease};
use super::source::PluginSpec;

pub(crate) struct FakeClient {
    manifest: String,
    component: Vec<u8>,
    tag: String,
    commit: String,
}

impl ReleaseClient for FakeClient {
    fn resolve(&self, _spec: &PluginSpec) -> Result<ResolvedRelease, String> {
        Ok(ResolvedRelease {
            tag: self.tag.clone(),
            commit: self.commit.clone(),
            author: "owner".to_string(),
            manifest: self.manifest.clone(),
            component: "plugin.wasm".to_string(),
            asset_url: "https://github.com/owner/repo/releases/download/v1/plugin.wasm".to_string(),
        })
    }

    fn download(&self, _release: &ResolvedRelease) -> Result<Vec<u8>, String> {
        Ok(self.component.clone())
    }
}

pub(crate) fn client(extra: &str) -> FakeClient {
    let component = fixture();
    FakeClient {
        manifest: manifest(&component, extra),
        component,
        tag: "v1.0.0".to_string(),
        commit: "0123456789abcdef0123456789abcdef01234567".to_string(),
    }
}

pub(crate) fn client_at(extra: &str, commit: &str) -> FakeClient {
    let mut client = client(extra);
    client.commit = commit.to_string();
    client
}

fn fixture() -> Vec<u8> {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../kawi-plugin/tests/fixtures/wasm/echo.wasm");
    std::fs::read(path).unwrap()
}

fn manifest(component: &[u8], extra: &str) -> String {
    let hash = format!("sha256:{:x}", Sha256::digest(component));
    format!(
        "name = \"echo\"\nversion = \"0.1.0\"\nprotocol = 1\nruntime = \"wasm\"\ncomponent = \"plugin.wasm\"\nsha256 = \"{hash}\"\ntimeout_ms = 5000\n\n[capabilities]\n\"ui.message\" = true\n{extra}\n\n[[commands]]\nid = \"ping\"\ntitle = \"Ping\"\n"
    )
}
