use std::fs;
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use serde_json::Value;

use kawi_plugin::{
    load_catalog, load_permission_state, Capability, CapabilityRegistry, HostEnvironment,
    InvocationContext, NotifyOutcome, PluginCatalog, PluginError, PluginHost, PluginStatus,
    Position,
};

#[cfg(all(feature = "wasm-test-network", debug_assertions))]
use super::super::scenario::resolver_script;
#[cfg(feature = "wasm")]
use super::super::scenario::Behavior;
use super::super::scenario::{network_endpoint, NetworkTarget, Scenario};
use super::driver::Driver;
use super::fixtures::{temp_root, write_grants, write_manifest, ManifestSpec, NetworkSetup};
use super::model::{Finished, Observation, Policy, Runtime};
use super::{INVOCATION_TIMEOUT, PLUGIN_NAME, START_TIMEOUT, STATUS_POLL};

pub(crate) struct ConformanceHost {
    pub(crate) runtime: Runtime,
    pub(crate) scenario: &'static str,
    #[cfg(feature = "wasm")]
    pub(crate) behavior: Behavior,
    pub(crate) root: PathBuf,
    grants_path: PathBuf,
    catalog: PluginCatalog,
    pub(crate) host: Arc<PluginHost>,
    driver: Option<JoinHandle<()>>,
    stop: Arc<AtomicBool>,
    finished: Receiver<Finished>,
    pub(crate) observation: Arc<Mutex<Observation>>,
    pub(crate) registry: CapabilityRegistry,
    pub(crate) policy: Arc<Mutex<Policy>>,
    pub(crate) network_allowed: Option<SocketAddr>,
    pub(crate) network_denied: Option<SocketAddr>,
    network_hostname: Option<String>,
    network_script: Option<String>,
    network_listeners: Option<(TcpListener, TcpListener)>,
    _wake: Receiver<()>,
}

impl ConformanceHost {
    pub(crate) fn start(runtime: Runtime, scenario: &Scenario) -> Self {
        let root = temp_root();
        let config = root.join("config");
        let plugins = root.join("plugins");
        let grants_path = config.join("permissions.toml");
        fs::create_dir_all(&config).unwrap();
        fs::create_dir_all(&plugins).unwrap();
        let network = network_setup(&root, scenario);
        write_manifest(
            &plugins,
            ManifestSpec {
                runtime,
                scenario,
                network: network.setup.as_ref(),
            },
        );
        let catalog = load_catalog(&plugins);
        write_grants(
            &grants_path,
            scenario.grants,
            network.setup.as_ref(),
            &catalog,
        );
        if !scenario.behavior.expects_load_warning() {
            assert!(catalog.warnings.is_empty(), "{:?}", catalog.warnings);
        }
        let state = load_permission_state(&catalog, &grants_path);
        assert!(state.file_error.is_none(), "{:?}", state.file_error);
        let registry = CapabilityRegistry::default();
        registry.replace(state.grants);
        let (event_tx, events) = mpsc::channel();
        let (wake, wake_rx) = mpsc::channel();
        let environment = HostEnvironment::from_paths(Some(root.clone()), Some(config));
        let host = Arc::new(PluginHost::start(
            &catalog,
            registry.clone(),
            environment,
            event_tx,
            wake,
        ));
        let (finished_tx, finished) = mpsc::channel();
        let observation = Arc::new(Mutex::new(Observation::default()));
        let mut policy = Policy::new();
        policy.state_text = scenario.behavior.trim_text().map(str::to_string);
        let policy = Arc::new(Mutex::new(policy));
        let stop = Arc::new(AtomicBool::new(false));
        let driver = Driver {
            host: Arc::clone(&host),
            registry: registry.clone(),
            observation: Arc::clone(&observation),
            policy: Arc::clone(&policy),
            stop: Arc::clone(&stop),
            finished: finished_tx,
        };
        let driver = driver.spawn(events);
        Self {
            runtime,
            scenario: scenario.id,
            #[cfg(feature = "wasm")]
            behavior: scenario.behavior,
            root,
            grants_path,
            catalog,
            host,
            driver: Some(driver),
            stop,
            finished,
            observation,
            registry,
            policy,
            network_allowed: network.allowed,
            network_denied: network.denied,
            network_hostname: network.hostname,
            network_script: network.script,
            network_listeners: network.listeners,
            _wake: wake_rx,
        }
    }

    pub(crate) fn activate_ready(&self) {
        self.host
            .activate(PLUGIN_NAME)
            .expect("activate conformance plugin");
        self.wait_ready();
    }

    pub(crate) fn wait_ready(&self) {
        let deadline = Instant::now() + START_TIMEOUT;
        loop {
            match self.host.status(PLUGIN_NAME) {
                Some(PluginStatus::Ready) => return,
                Some(PluginStatus::Disabled) | Some(PluginStatus::Error(_)) => {
                    panic!(
                        "{}: plugin did not start: {}",
                        self.label(),
                        self.describe()
                    )
                }
                _ if Instant::now() >= deadline => {
                    panic!(
                        "{}: plugin did not become ready: {}",
                        self.label(),
                        self.describe()
                    )
                }
                _ => thread::sleep(STATUS_POLL),
            }
        }
    }

    pub(crate) fn revoke_all(&self) {
        self.set_grants(&[]);
    }

    pub(crate) fn set_grants(&self, capabilities: &[Capability]) {
        write_grants(&self.grants_path, capabilities, None, &self.catalog);
        let state = load_permission_state(&self.catalog, &self.grants_path);
        assert!(state.file_error.is_none(), "{:?}", state.file_error);
        self.registry.replace(state.grants);
    }

    pub(crate) fn invoke(&self, command: &str, args: Value) -> Result<Value, PluginError> {
        match self.start_invocation(command, args) {
            Ok(request) => self.wait_for_finished(request),
            Err(error) => Err(error),
        }
    }

    pub(crate) fn start_invocation(&self, command: &str, args: Value) -> Result<u64, PluginError> {
        self.host.invoke(PLUGIN_NAME, command, args, context())
    }

    pub(crate) fn wait_for_finished(&self, request: u64) -> Result<Value, PluginError> {
        let deadline = Instant::now() + INVOCATION_TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            assert!(
                !remaining.is_zero(),
                "{}: invocation {request} timed out",
                self.label()
            );
            match self.finished.recv_timeout(remaining) {
                Ok(finished) if finished.request == request => return finished.result,
                Ok(_) => {}
                Err(error) => panic!(
                    "{}: invocation {request} did not finish: {error}",
                    self.label()
                ),
            }
        }
    }

    pub(crate) fn restart(&self) {
        self.host
            .restart(PLUGIN_NAME)
            .expect("restart conformance plugin");
    }

    pub(crate) fn notify(&self, event: &str, payload: Value) -> NotifyOutcome {
        self.host
            .notify(PLUGIN_NAME, event, payload)
            .expect("notify conformance plugin")
    }

    pub(crate) fn accept_allowed(&self) -> std::io::Result<TcpStream> {
        let (listener, _) = self
            .network_listeners
            .as_ref()
            .expect("network scenario has no listener");
        listener.accept().map(|(stream, _)| stream)
    }

    pub(crate) fn network_target(&self, target: NetworkTarget) -> String {
        let allowed = self
            .network_allowed
            .expect("network scenario has no allowed address");
        let denied = self
            .network_denied
            .expect("network scenario has no denied address");
        match target {
            NetworkTarget::Allowed => allowed.to_string(),
            NetworkTarget::Denied => denied.to_string(),
            NetworkTarget::Hostname => self
                .network_hostname
                .clone()
                .expect("network scenario has no hostname"),
            NetworkTarget::OtherHostname => "example.com:443".to_string(),
            NetworkTarget::PublicIp => "8.8.8.8:443".to_string(),
            NetworkTarget::Mapped => format!("[::ffff:127.0.0.1]:{}", allowed.port()),
            NetworkTarget::LinkLocal => format!("169.254.169.254:{}", allowed.port()),
            NetworkTarget::Loopback => format!("[::1]:{}", allowed.port()),
        }
    }

    pub(crate) fn label(&self) -> String {
        format!("{:?} {}", self.runtime, self.scenario)
    }

    fn describe(&self) -> String {
        let observation = self.observation();
        format!(
            "status {:?}, logs {:?}",
            self.host.status(PLUGIN_NAME),
            observation.logs
        )
    }
}

impl Drop for ConformanceHost {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.release_host_responses();
        remove_network_script(self.network_script.as_ref());
        self.host.shutdown();
        if let Some(driver) = self.driver.take() {
            let _ = driver.join();
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}

struct NetworkHarness {
    setup: Option<NetworkSetup>,
    hostname: Option<String>,
    script: Option<String>,
    listeners: Option<(TcpListener, TcpListener)>,
    allowed: Option<SocketAddr>,
    denied: Option<SocketAddr>,
}

fn network_setup(root: &std::path::Path, scenario: &Scenario) -> NetworkHarness {
    if !scenario.behavior.is_network() {
        return NetworkHarness {
            setup: None,
            hostname: None,
            script: None,
            listeners: None,
            allowed: None,
            denied: None,
        };
    }
    let allowed = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let denied = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let allowed_address = allowed.local_addr().unwrap();
    let denied_address = denied.local_addr().unwrap();
    let hostname = network_endpoint(scenario.id, allowed_address.port());
    let setup = NetworkSetup::new(
        root.to_path_buf(),
        scenario,
        allowed_address,
        hostname.clone(),
    );
    let script = install_network_script(scenario);
    NetworkHarness {
        setup: Some(setup),
        hostname: Some(hostname),
        script,
        listeners: Some((allowed, denied)),
        allowed: Some(allowed_address),
        denied: Some(denied_address),
    }
}

#[cfg(all(feature = "wasm-test-network", debug_assertions))]
fn install_network_script(scenario: &Scenario) -> Option<String> {
    let script = resolver_script(scenario.id)?;
    let answers = script
        .answers
        .iter()
        .map(|answer| {
            answer
                .iter()
                .map(|address| {
                    address
                        .parse::<SocketAddr>()
                        .expect("resolver script address")
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    kawi_plugin::install_resolver_script(script.host, answers);
    Some(script.host.to_string())
}

#[cfg(not(all(feature = "wasm-test-network", debug_assertions)))]
fn install_network_script(_scenario: &Scenario) -> Option<String> {
    None
}

fn remove_network_script(host: Option<&String>) {
    #[cfg(all(feature = "wasm-test-network", debug_assertions))]
    if let Some(host) = host {
        kawi_plugin::remove_resolver_script(host);
    }
    #[cfg(not(all(feature = "wasm-test-network", debug_assertions)))]
    let _ = host;
}

fn context() -> InvocationContext {
    InvocationContext {
        buffer: 1,
        revision: 7,
        path: Some("/tmp/conformance.rs".to_string()),
        language: Some("rust".to_string()),
        mode: "normal".to_string(),
        cursor: Position {
            line: 0,
            byte_column: 0,
        },
        selection: None,
    }
}
