use std::collections::HashMap;
use std::io::BufReader;
use std::io::Read;

use tempfile::NamedTempFile;
use wasi_common::*;
use wasmtime::*;
use wasmtime_wasi::old::snapshot_0::Wasi as WasiUnstable;
use wasmtime_wasi::*;

fn main() {
    // Module data and output files
    let module_data = wat::parse_file("./printer.wasm").expect("module data");
    let stdout = NamedTempFile::new().expect("stdout tempfile");
    let stderr = NamedTempFile::new().expect("stderr tempfile");

    // fake args and env variables
    let mut env: HashMap<String, String> = HashMap::default();
    env.insert("FOO".into(), "bar".into());

    let args: Vec<String> = vec!["a", "lovely", "bunch", "of", "coconuts"]
        .iter()
        .map(|&s| s.to_owned())
        .collect();
    // Module setup
    let engine = Engine::default();
    let store = Store::new(&engine);
    let wasi_ctx_snapshot = WasiCtxBuilder::new()
        .args(&args)
        .envs(&env)
        .stdout(stdout.reopen().expect("reopen stdout"))
        .stderr(stderr.reopen().expect("reopen stderr"))
        .preopened_dir(preopen_dir(".").expect("preopen dir"), ".")
        .build()
        .expect("context build");
    let wasi_ctx_unstable = wasi_common::old::snapshot_0::WasiCtxBuilder::new()
        .args(&args)
        .envs(&env)
        .stdout(stdout.reopen().expect("reopen stdout"))
        .stderr(stderr.reopen().expect("reopen stderr"))
        .preopened_dir(preopen_dir(".").expect("preopen dir"), ".")
        .build()
        .expect("context build");
    let wasi_snapshot = Wasi::new(&store, wasi_ctx_snapshot);
    let wasi_unstable = WasiUnstable::new(&store, wasi_ctx_unstable);
    let module = Module::new(&store, &module_data).expect("new module");

    let imports = module
        .imports()
        .iter()
        .map(|i| {
            // This is super funky logic, but it matches what is in 0.11.0
            let export = match i.module() {
                "wasi_snapshot_preview1" => wasi_snapshot.get_export(i.name()),
                "wasi_unstable" => wasi_unstable.get_export(i.name()),
                other => panic!("import module `{}` was not found", other),
            };
            match export {
                Some(export) => export.clone().into(),
                None => panic!(
                    "import `{}` was not found in module `{}`",
                    i.name(),
                    i.module()
                ),
            }
        })
        .collect::<Vec<_>>();
    let instance = Instance::new(&module, &imports).expect("instance");

    println!("starting run of module");
    instance
        .get_export("_start")
        .expect("export")
        .func()
        .unwrap()
        .call(&[])
        .unwrap();

    println!("module run complete");

    let mut stdout_buf = BufReader::new(stdout.reopen().expect("stdout read"));
    let mut stderr_buf = BufReader::new(stderr.reopen().expect("stderr read"));

    let mut stdout_str = String::default();
    let mut stderr_str = String::default();

    stdout_buf.read_to_string(&mut stdout_str).unwrap();
    stderr_buf.read_to_string(&mut stderr_str).unwrap();

    println!("STDOUT is:\n{}", stdout_str);
    println!("STDERR is:\n{}", stderr_str);
}
