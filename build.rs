use std::{
    env,
    fmt::Write,
    fs::{self, File},
    io::Write as _,
    process::Command,
};
macro_rules! kwin {
    () => {
        env::var("KWIN_INCLUDE")
            .as_deref()
            .unwrap_or(
                r#"
                /usr/include
                /usr/include/kwin
                /usr/include/KF6/KConfig
                /usr/include/KF6/KConfigCore
                /usr/include/KF6/KWindowSystem
                /usr/include/qt6
                /usr/include/qt6/QtCore
                /usr/include/qt6/QtDBus
                /usr/include/qt6/QtGui
                /usr/include/qt6/QtWidgets
            "#,
            )
            .split('\n')
            .map(str::trim)
            .filter(|x| x.len() > 0)
    };
}
// fn kwin<'a, E>(var: Result<&'a str, E>) -> impl Iterator<Item = &'a str> {
//     var
// }
fn main() {
    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    if cfg!(feature = "docgen-detect") && !kwin!().all(|x| fs::exists(x).unwrap_or(false)) {
        #[cfg(not(doc))]
        println!(
            "cargo:warning=Some folder located in KWIN_INCLUDE (or the default files) does not exists, generating dummy offset file instead. Please set KWIN_INCLUDE manually to prevent this warning occurs."
        );
        let mut file = File::create(format!("{}/consts.rs", env::var("OUT_DIR").unwrap())).unwrap();
        write!(
            &mut file,
            r#"/// for docgen only, should not rely on its value.
#[used]
#[unsafe(link_section = ".kwin.mouse.loc.pos")]
pub(crate) static mut POS_OFFSET: usize = 176usize; // seems stable
/// for docgen only, must not rely on its value since it will change each time when libkwin.so updated.
#[used]
#[unsafe(link_section = ".kwin.mouse.loc.kwin")]
pub(crate) static mut WORKSPACE_OFFSET: usize = 0x0usize;
"#
        )
        .unwrap();
        return;
    }
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .use_core()
        .header_contents("header.hpp", "#include<workspace.h>")
        .allowlist_type("^(.*Workspace.*)$")
        .clang_args(
            kwin!()
                .map(|x| format!("-I{}", x))
                .chain(
                    env::var("CUSTOM_ARGS")
                        .as_deref()
                        .unwrap_or("")
                        .split(' ')
                        .filter(|x| x.len() > 0)
                        .map(|x| x.to_owned()),
                )
                .chain(["-x", "c++", "-std=c++20"].map(ToString::to_string)),
        )
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the OUT_DIR file.
    let mut file = File::create(format!("{}/consts.rs", env::var("OUT_DIR").unwrap()))
        .expect("cannot save to $OUT_DIR");
    write!(&mut file, "#[used]\n#[unsafe(link_section = \".kwin.mouse.loc.pos\")]\npub(crate) static mut POS_OFFSET: usize = {};\n#[used]\n#[unsafe(link_section = \".kwin.mouse.loc.kwin\")]\npub(crate) static mut WORKSPACE_OFFSET: usize = 0x{:06x}usize;\n",
        bindings
            .to_string()
            .split_once(r#"offset_of!(KWin_Workspace, focusMousePos)"#)
            .expect("cannot calculate offset of focusMousePos.")
            .1
            .split_once("]")
            .expect(r#"do not find line [::core::mem::offset_of!(KWin_Workspace, focusMousePos) - $(SIZE)]."#)
            .0
            .split_once("-")
            .expect("grab offset failed")
            .1
            .trim(),
        usize::from_str_radix(String::from_utf8(Command::new("readelf").args(["-WCs", "/usr/lib/libkwin.so"]).output().expect("readelf execute failed").stdout).expect("failed to parse readelf").split_once(r#"KWin::Workspace::_self"#).expect("cannot find KWin::Workspace::_self").0.rsplit_once('\n').expect("cannot read offset of KWin::Workspace::_self").1.split_once(':').expect("parse `:` failed.").1.trim().split_once(' ').expect("cannot parse space").0,16).expect("cannot parse offset")
    ).expect("write failed");

    if cfg!(feature = "uinput") {
        let contents = &mut "#include<linux/input.h>\n#include<linux/uinput.h>\n".to_owned();
        let force_include = |s: &mut String, ty, val| {
            write!(s,"\nconst {ty} rust_bindgen_exclude_{val} = {val};\n#undef {val}\n const {ty} {val} = rust_bindgen_exclude_{val};").expect("write header failed.")
        };
        force_include(contents, "unsigned long", "UI_DEV_SETUP");
        force_include(contents, "unsigned long", "UI_DEV_CREATE");
        force_include(contents, "unsigned long", "UI_DEV_DESTROY");
        force_include(contents, "unsigned long", "UI_SET_EVBIT");
        force_include(contents, "unsigned long", "UI_SET_KEYBIT");
        force_include(contents, "unsigned long", "UI_SET_RELBIT");
        force_include(contents, "unsigned long", "UI_SET_ABSBIT");

        let bindings = bindgen::Builder::default()
            // The input header we would like to generate
            // bindings for.
            .use_core()
            .header_contents("header.h", contents)
            .allowlist_type("^((input_event|uinput_setup))$")
            .allowlist_var("^(.*)$")
            .blocklist_var("^(rust_bindgen_exclude_.*)$")
            .default_visibility(bindgen::FieldVisibilityKind::Public)
            .clang_args(
                env::var("UINPUT_INCLUDE")
                    .as_deref()
                    .unwrap_or("/usr/include")
                    .split('\n')
                    .map(str::trim)
                    .filter(|x| x.len() > 0)
                    .map(|x| format!("-I{}", x)),
            )
            // Tell cargo to invalidate the built crate whenever any of the
            // included header files changed.
            .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
            // Finish the builder and generate the bindings.
            .generate()
            // Unwrap the Result and panic on failure.
            .expect("Unable to generate bindings");
        bindings
            .write_to_file(format!("{}/uinput.rs", env::var("OUT_DIR").unwrap()))
            .expect("cannot deal with uinput")
    }
}
