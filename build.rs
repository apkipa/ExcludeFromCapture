use cc::{Build, Tool};
use std::{env, fs};

fn get_compiler_from_target(target: &str) -> Tool {
    Build::new().target(target).get_compiler()
}

fn assert_is_msvc_compiler(compiler: &Tool) {
    assert!(
        compiler.is_like_msvc(),
        "Non-msvc compilers are currently unsupported"
    );
}

fn build_dll_from_src(target: &str, in_name: &str, out_name: &str) {
    let compiler = get_compiler_from_target(target);
    assert_is_msvc_compiler(&compiler);

    let out_dir = env::var("OUT_DIR").unwrap();
    let dll_path = format!("{}/{}", out_dir, out_name);
    let mut compiler_cmd = compiler.to_command();
    compiler_cmd
        .arg(format!("/Fo{}/", out_dir))
        .arg(in_name)
        .arg("/link")
        .arg("/DLL")
        .arg(format!("/OUT:{}", dll_path))
        .args(["user32.lib"]);
    let output = compiler_cmd.output().unwrap();
    if !output.status.success() {
        eprintln!(
            "* Compiler stdout: \n{}",
            String::from_utf8_lossy(&output.stdout)
        );
        eprintln!(
            "* Compiler stderr: \n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        panic!("Compiler invocation has failed");
    }

    fs::copy(dll_path, format!("{}/../../../{}", out_dir, out_name)).unwrap();

    println!("cargo:rerun-if-changed={}", in_name);
}

fn main() {
    // Generate DLLs for injection and manifest for linking
    build_dll_from_src("i686-pc-windows-msvc", "src/dllsub.c", "dllsub.x86.dll");
    build_dll_from_src("x86_64-pc-windows-msvc", "src/dllsub.c", "dllsub.x64.dll");
    embed_resource::compile("src/resource.rc");
}
