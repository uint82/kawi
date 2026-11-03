use super::*;

fn temp_file(name: &str, text: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kawi_dap_config_{}_{}", std::process::id(), name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("debug.toml");
    fs::write(&path, text).unwrap();
    path
}

#[test]
fn loads_adapters_and_configurations() {
    let path = temp_file(
            "load",
            "[adapter.fake]\ncommand = \"fake-dap\"\nargs = [\"--stdio\"]\n\n\
             [configuration.rust]\nadapter = \"fake\"\nlanguages = [\"rust\"]\nrequest = \"launch\"\nprogram = \"${workspace}/target/app\"\n",
        );
    let config = DebugConfig::load(&[path]);
    assert!(config.warnings().is_empty());
    let adapter = config.adapters().next().expect("adapter");
    assert_eq!(adapter.0, "fake");
    assert_eq!(adapter.1.args, vec!["--stdio"]);
    let configuration = config.configuration("rust").expect("configuration");
    assert_eq!(configuration.languages, vec!["rust"]);
    assert_eq!(configuration.request, "launch");
    assert_eq!(
        configuration.arguments["program"],
        Value::String("${workspace}/target/app".to_string())
    );
}

#[test]
fn project_configuration_overrides_user_configuration() {
    let user = temp_file(
        "user",
        "[adapter.fake]\ncommand = \"user-dap\"\n\n\
             [configuration.rust]\nadapter = \"fake\"\nrequest = \"launch\"\nprogram = \"user\"\n",
    );
    let project_dir = user.parent().unwrap().join("project");
    fs::create_dir_all(&project_dir).unwrap();
    let project = project_dir.join("debug.toml");
    fs::write(
        &project,
        "[adapter.fake]\ncommand = \"project-dap\"\n\n\
             [configuration.rust]\nadapter = \"fake\"\nrequest = \"attach\"\nprocessId = 42\n",
    )
    .unwrap();
    let config = DebugConfig::load(&[user, project]);
    let configuration = config.configuration("rust").expect("configuration");
    assert_eq!(configuration.request, "attach");
    assert_eq!(configuration.arguments["processId"], Value::from(42));
    let adapter = config.adapters().next().expect("adapter");
    assert_eq!(adapter.1.command, "project-dap");
}

#[test]
fn expands_workspace_file_and_environment_variables() {
    let path = temp_file(
            "expand",
            "[adapter.fake]\ncommand = \"fake-dap\"\nargs = [\"--log\", \"${workspace}/dap.log\"]\n\n\
             [configuration.rust]\nadapter = \"fake\"\nrequest = \"launch\"\nprogram = \"${file}\"\ncwd = \"${fileDirname}\"\nargs = [\"${env:KAWI_DAP_TEST}\"]\n",
        );
    let config = DebugConfig::load(&[path]);
    let workspace = PathBuf::from("/tmp/workspace");
    let file = workspace.join("src/main.rs");
    std::env::set_var("KAWI_DAP_TEST", "value");
    let expanded = config
        .expand(
            "rust",
            &DebugContext {
                workspace: workspace.clone(),
                file: Some(file.clone()),
            },
        )
        .expect("expanded");
    assert_eq!(expanded.args, vec!["--log", "/tmp/workspace/dap.log"]);
    assert_eq!(
        expanded.arguments["program"],
        Value::String(file.display().to_string())
    );
    assert_eq!(
        expanded.arguments["cwd"],
        Value::String(workspace.join("src").display().to_string())
    );
    assert_eq!(
        expanded.arguments["args"][0],
        Value::String("value".to_string())
    );
}

#[test]
fn warns_about_invalid_configuration() {
    let path = temp_file(
        "invalid",
        "[adapter.fake]\ncommand = \"\"\n\n\
             [configuration.rust]\nadapter = \"fake\"\nrequest = \"invalid\"\n",
    );
    let config = DebugConfig::load(&[path]);
    assert_eq!(config.warnings().len(), 2);
    assert!(config.warnings()[0].format().contains("non-empty string"));
    assert!(config.warnings()[1].format().contains("launch or attach"));
}
