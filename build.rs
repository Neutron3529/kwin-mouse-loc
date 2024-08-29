use std::{env, fs::File, io::Write, process::Command};
fn main() {
    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .use_core()
        .header_contents("header.hpp", "#include<workspace.h>")
        .clang_args(
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
    write!(&mut file, "pub const POS_OFFSET: usize = {};\npub const WORKSPACE_OFFSET: usize = 0x{}usize;\n",
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
        String::from_utf8(Command::new("readelf").args(["-WCs", "/usr/lib/libkwin.so"]).output().expect("readelf execute failed").stdout).expect("failed to parse readelf").split_once(r#"KWin::Workspace::_self"#).expect("cannot find KWin::Workspace::_self").0.rsplit_once('\n').expect("cannot read offset of KWin::Workspace::_self").1.split_once(':').expect("parse `:` failed.").1.trim().split_once(' ').expect("cannot parse space").0
    ).expect("write failed");
}
