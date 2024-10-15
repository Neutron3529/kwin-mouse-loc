use std::{env, fs::OpenOptions, process::Command};
/// Update offset for WORKSPACE_OFFSET and POS_OFFSET using default path.
///
/// Since it modify static variable and executable, this function terminates the execution flow.
/// ```no_run
/// // The current execution will be terminated (by an invisible panic).
/// // This function also modify the binary executable itself.
/// #[cfg(feature = "update-offset")]
/// kwin_mouse_loc::consts::update_offset()
/// ```
/// Such function might be compined with some post-update hooks (e.g., with pacman's hook defined in, for example, `/etc/pacman.d/hooks/kwin.hook`)
/// ```text
/// [Trigger]
/// Operation = Install
/// Operation = Upgrade
/// Type = Package
/// Target = kwin
/// [Action]
/// Description = "Update outdated clicker."
/// When = PostTransaction
/// Exec = /bin/clicker update-offset
/// ```
pub fn update_offset() -> ! {
    #[cfg(feature = "update-pos")]
    let pos = Some(
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
    );
    #[cfg(not(feature = "update-pos"))]
    let pos = None;
    let kwin = Some("/usr/lib/libkwin.so");
    update_offset_custom(pos, kwin);
}

/// Update offset for WORKSPACE_OFFSET and POS_OFFSET using provided path.
///
/// Since it modify static variable and executable, this function terminates the execution flow.
pub fn update_offset_custom(pos: Option<&str>, kwin: Option<&str>) -> ! {
    unsafe {
        let wo = WORKSPACE_OFFSET;
        let po = POS_OFFSET;
        println!("Before update, WORKSPACE_OFFSET = 0x{wo:06x}, POS_OFFSET = 0x{po:04x}.");
    }
    unsafe {
        if let Some(pos) = pos {
            #[cfg(feature = "update-pos")]
            {
                POS_OFFSET = offset_pos(pos);
                save_offset(POS_OFFSET, Offset::Pos);
            }
            #[cfg(not(feature = "update-pos"))]
            {
                panic!("update-pos relies on bindgen and source code of kwin, which is not enabled by default, the provided {pos} cannot be dealt without bindgen, and thus the update failed.");
            }
        }
        if let Some(kwin) = kwin {
            WORKSPACE_OFFSET = offset_kwin(kwin);
            save_offset(WORKSPACE_OFFSET, Offset::Kwin);
        }
    }
    unsafe {
        let wo = WORKSPACE_OFFSET;
        let po = POS_OFFSET;
        println!("After update, WORKSPACE_OFFSET = 0x{wo:06x}, POS_OFFSET = 0x{po:04x}.");
    }
    std::panic::set_hook(Box::new(|_|{}));
    panic!();
}
/// parameter for `save_offset`, better not to use it.
pub enum Offset {
    Offset,
    Pos,
    Kwin,
}
#[unsafe(link_section = ".kwin.mouse.loc.offset")]
static mut OFFSET: [usize; 3] = [0, 0xfacefeedcafebabe, 0xdeadbeeffee1dead];

/// get section's offset from the output of `readelf -WCS`.
pub fn get_offset(data: &str, section: &str) -> usize {
    usize::from_str_radix(
        data.split_once(section)
            .expect("section information damaged")
            .1
            .trim()
            .split(' ')
            .filter(|x| !x.is_empty())
            .skip(2)
            .next()
            .unwrap_or("cannot got address"),
        16,
    )
    .expect("cannot parse address as usize")
}
/// Save offset into the program itself. Also modify static mut variables.
///
/// Do not use it unless you know what you're doing.
pub unsafe fn save_offset(val: usize, item: Offset) {
    use std::io::{Read, Seek, SeekFrom, Write};

    let exe = &env::current_exe().expect("cannot find current exe");
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(exe)
        .expect("cannot modify current exe");
    let offset = unsafe {
        if OFFSET[0] == 0 {
            println!(
                "Before offset init, offset is 0x{:016x} 0x{:016x} 0x{:016x}.",
                OFFSET[0], OFFSET[1], OFFSET[2]
            );
            let data = &String::from_utf8(
                Command::new("readelf")
                    .arg("-WCS")
                    .arg(exe)
                    .output()
                    .expect("readelf execute failed")
                    .stdout,
            )
            .expect("failed to parse readelf");
            let mut new = [0; 3];
            new[0] = get_offset(data, ".kwin.mouse.loc.offset");
            new[1] = get_offset(data, ".kwin.mouse.loc.pos");
            new[2] = get_offset(data, ".kwin.mouse.loc.kwin");
            file.seek(SeekFrom::Start(new[0] as u64))
                .expect("cannot seek file");
            let mut tmp = [0u8; 24];
            file.read(&mut tmp).expect("cannot read offset");
            assert!(
                tmp[0..8] == OFFSET[0].to_ne_bytes()
                    && tmp[8..16] == OFFSET[1].to_ne_bytes()
                    && tmp[16..24] == OFFSET[2].to_ne_bytes(),
                "value mismatch! The offset might be incorrect.",
            );
            file.seek(SeekFrom::Start(new[0] as u64))
                .expect("cannot seek file");
            file.write(&new[0].to_ne_bytes())
                .expect("cannot save file.");
            file.write(&new[1].to_ne_bytes())
                .expect("cannot save file.");
            file.write(&new[2].to_ne_bytes())
                .expect("cannot save file.");
            OFFSET = new;
            println!(
                "After offset init, offset is 0x{:06x} 0x{:06x} 0x{:06x}, which should not be changed in further executions.",
                OFFSET[0], OFFSET[1], OFFSET[2]
            );
            //            readelf -WCS
        }
        match item {
            Offset::Offset => OFFSET[0],
            Offset::Pos => OFFSET[1],
            Offset::Kwin => OFFSET[2],
        }
    };
    file.seek(SeekFrom::Start(offset as u64))
        .expect("cannot seek file");
    file.write(&val.to_ne_bytes()).expect("cannot save file.");
    file.sync_all().expect("cannot sync file.")
}
/// got offset of `KWin::Workspace::_self` from elf file
pub fn offset_kwin(elf_file: &str) -> usize {
    usize::from_str_radix(
        String::from_utf8(
            Command::new("readelf")
                .args(["-WCs", elf_file])
                .output()
                .expect("readelf execute failed")
                .stdout,
        )
        .expect("failed to parse readelf")
        .split_once(r#"KWin::Workspace::_self"#)
        .expect("cannot find KWin::Workspace::_self")
        .0
        .rsplit_once('\n')
        .expect("cannot read offset of KWin::Workspace::_self")
        .1
        .split_once(':')
        .expect("parse `:` failed.")
        .1
        .trim()
        .split_once(' ')
        .expect("cannot parse space")
        .0,
        16,
    )
    .expect("cannot be parsed as hex digits")
}
#[cfg(any(doc,feature = "update-pos"))]
#[cfg_attr(doc, doc(cfg(feature = "update-pos")))]
/// got the offset of `focusMousePos` in KWin_Workspace from the source code of KWin.
pub fn offset_pos(s: &str) -> usize {
    usize::from_str_radix(
        bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .use_core()
        .header_contents("header.hpp", "#include<workspace.h>")
        .allowlist_type("^(.*Workspace.*)$")
        .clang_args(
            s
                .split('\n')
                .map(str::trim)
                .filter(|x| x.len() > 0)
                .map(|x| format!("-I{}", x))
                .chain(["-x", "c++", "-std=c++20"].map(ToString::to_string)),
        )
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings")
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
            .trim(),16
    )
    .expect("cannot be parsed as hex digits")
}
