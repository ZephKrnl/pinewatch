//! Runs a .NET console program on the game's own 64-bit Mono runtime (a copy of
//! `MonoBleedingEdge/EmbedRuntime/mono-2.0-bdwgc.dll`), with the game's class libraries.
//!
//! Usage: gamehost <runtime dll> <Managed dir> <etc dir> <program.exe> [args...]
use std::ffi::{CString, c_char, c_int, c_void};
use std::os::windows::ffi::OsStrExt;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LoadLibraryW(name: *const u16) -> *mut c_void;
    fn GetProcAddress(module: *mut c_void, name: *const c_char) -> *mut c_void;
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert!(args.len() >= 5, "usage: gamehost <runtime dll> <Managed dir> <etc dir> <program.exe> [args...]");
    let dll: Vec<u16> = std::ffi::OsStr::new(&args[1]).encode_wide().chain([0]).collect();
    unsafe {
        let module = LoadLibraryW(dll.as_ptr());
        assert!(!module.is_null(), "LoadLibraryW failed for {}", args[1]);
        macro_rules! sym {
            ($name:literal, $ty:ty) => {{
                let p = GetProcAddress(module, concat!($name, "\0").as_ptr() as *const c_char);
                assert!(!p.is_null(), "missing export {}", $name);
                std::mem::transmute::<*mut c_void, $ty>(p)
            }};
        }
        let set_dirs = sym!("mono_set_dirs", unsafe extern "C" fn(*const c_char, *const c_char));
        let set_assemblies_path = sym!("mono_set_assemblies_path", unsafe extern "C" fn(*const c_char));
        let config_parse = sym!("mono_config_parse", unsafe extern "C" fn(*const c_char));
        let jit_init_version =
            sym!("mono_jit_init_version", unsafe extern "C" fn(*const c_char, *const c_char) -> *mut c_void);
        let assembly_open =
            sym!("mono_domain_assembly_open", unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void);
        let jit_exec =
            sym!("mono_jit_exec", unsafe extern "C" fn(*mut c_void, *mut c_void, c_int, *mut *mut c_char) -> c_int);

        let managed = CString::new(args[2].as_str()).unwrap();
        let etc = CString::new(args[3].as_str()).unwrap();
        set_dirs(managed.as_ptr(), etc.as_ptr());
        set_assemblies_path(managed.as_ptr());
        config_parse(std::ptr::null());
        let domain = jit_init_version(c"pinewatch-check".as_ptr(), c"v4.0.30319".as_ptr());
        assert!(!domain.is_null(), "mono_jit_init_version failed");
        let program = CString::new(args[4].as_str()).unwrap();
        let assembly = assembly_open(domain, program.as_ptr());
        assert!(!assembly.is_null(), "can't open {}", args[4]);
        let owned: Vec<CString> = args[4..].iter().map(|a| CString::new(a.as_str()).unwrap()).collect();
        let mut argv: Vec<*mut c_char> = owned.iter().map(|a| a.as_ptr() as *mut c_char).collect();
        let code = jit_exec(domain, assembly, argv.len() as c_int, argv.as_mut_ptr());
        std::process::exit(code);
    }
}
