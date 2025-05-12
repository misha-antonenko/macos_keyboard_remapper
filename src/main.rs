// A macOS keyboard remapper from QWERTY to Dvorak when Command or Control are not pressed.
use clap::{Parser, Subcommand};
use core_foundation::runloop::{CFRunLoop, kCFRunLoopCommonModes};
use core_foundation::string::CFStringRef;
use core_graphics::event::{
    CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
    CGEventType, EventField,
};
use std::error::Error;
use std::os::raw::c_void;
use std::{env, fs, process};
use tracing::{Level, debug, error, info, instrument, warn};
use tracing_subscriber::EnvFilter;

// Command-line interface
#[derive(Parser)]
#[command(
    name = "macos_keyboard_remapper",
    version,
    about = "Remap QWERTY to Dvorak on macOS"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Install as a LaunchAgent (auto-start at login)
    Install,
    /// Remove the LaunchAgent
    Uninstall,
}
// Key code constants (from HIToolbox/Events.h, kVK_*):
const VK_A: u64 = 0;
const VK_S: u64 = 1;
const VK_D: u64 = 2;
const VK_F: u64 = 3;
const VK_H: u64 = 4;
const VK_G: u64 = 5;
const VK_Z: u64 = 6;
const VK_X: u64 = 7;
const VK_C: u64 = 8;
const VK_V: u64 = 9;
const VK_B: u64 = 11;
const VK_Q: u64 = 12;
const VK_W: u64 = 13;
const VK_E: u64 = 14;
const VK_R: u64 = 15;
const VK_Y: u64 = 16;
const VK_T: u64 = 17;
const VK_ANSI_1: u64 = 18;
const VK_ANSI_2: u64 = 19;
const VK_ANSI_3: u64 = 20;
const VK_ANSI_4: u64 = 21;
const VK_ANSI_6: u64 = 22;
const VK_ANSI_5: u64 = 23;
const VK_ANSI_EQUALS: u64 = 24;
const VK_ANSI_9: u64 = 25;
const VK_ANSI_7: u64 = 26;
const VK_MINUS: u64 = 27;
const VK_ANSI_8: u64 = 28;
const VK_ANSI_0: u64 = 29;
const VK_RIGHTBRACKET: u64 = 30;
const VK_O: u64 = 31;
const VK_U: u64 = 32;
const VK_LEFTBRACKET: u64 = 33;
const VK_I: u64 = 34;
const VK_P: u64 = 35;
const VK_L: u64 = 37;
const VK_J: u64 = 38;
const VK_QUOTE: u64 = 39;
const VK_K: u64 = 40;
const VK_SEMICOLON: u64 = 41;
const VK_BACKSLASH: u64 = 42;
const VK_COMMA: u64 = 43;
const VK_SLASH: u64 = 44;
const VK_N: u64 = 45;
const VK_M: u64 = 46;
const VK_PERIOD: u64 = 47;

// Text input source detection (to only remap on US QWERTY)
type TISInputSourceRef = *mut c_void;
#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    fn TISCopyCurrentKeyboardLayoutInputSource() -> TISInputSourceRef;
    fn TISGetInputSourceProperty(
        source: TISInputSourceRef,
        propertyKey: CFStringRef,
    ) -> CFStringRef;
    static kTISPropertyInputSourceID: CFStringRef;
}
// CoreFoundation helpers
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFStringGetCStringPtr(theString: CFStringRef, encoding: u32) -> *const i8;
    fn CFStringGetCString(
        theString: CFStringRef,
        buffer: *mut i8,
        bufferSize: isize,
        encoding: u32,
    ) -> bool;
    fn CFRelease(cf: *const c_void);
}
use std::ffi::CStr;
const K_CFSTRING_ENCODING_UTF8: u32 = 0x08000100;

/// Returns true if current keyboard layout is US QWERTY
#[instrument(ret, level = Level::DEBUG)]
fn is_us_qwerty() -> bool {
    debug!("entered `is_us_qwerty`");
    unsafe {
        let src = TISCopyCurrentKeyboardLayoutInputSource();
        if src.is_null() {
            warn!("No current keyboard layout");
            return false;
        }

        let id_cf = TISGetInputSourceProperty(src, kTISPropertyInputSourceID);
        let mut is_us = false;
        let ptr = CFStringGetCStringPtr(id_cf, K_CFSTRING_ENCODING_UTF8);
        if !ptr.is_null() {
            let s = str::from_utf8_unchecked(CStr::from_ptr(ptr).to_bytes());
            if s == "com.apple.keyboardlayout.US" || s == "com.apple.keylayout.US" {
                is_us = true;
            }
        } else {
            let mut buf = [0i8; 256];
            if CFStringGetCString(
                id_cf,
                buf.as_mut_ptr(),
                buf.len() as isize,
                K_CFSTRING_ENCODING_UTF8,
            ) {
                let s = str::from_utf8_unchecked(CStr::from_ptr(buf.as_ptr()).to_bytes());
                if s == "com.apple.keyboardlayout.US" || s == "com.apple.keylayout.US" {
                    is_us = true;
                }
            }
        }

        CFRelease(src as *const c_void);
        is_us
    }
}

// Remap QWERTY keycodes to Dvorak keycodes (only when on US QWERTY layout)
fn remap_key(key: u64) -> Option<u64> {
    if !is_us_qwerty() {
        return None;
    }
    match key {
        // Top row
        VK_Q => Some(VK_QUOTE),                  // Q -> '
        VK_W => Some(VK_COMMA),                  // W -> ,
        VK_E => Some(VK_PERIOD),                 // E -> .
        VK_R => Some(VK_P),                      // R -> P
        VK_T => Some(VK_Y),                      // T -> Y
        VK_Y => Some(VK_F),                      // Y -> F
        VK_U => Some(VK_G),                      // U -> G
        VK_I => Some(VK_C),                      // I -> C
        VK_O => Some(VK_R),                      // O -> R
        VK_P => Some(VK_L),                      // P -> L
        VK_LEFTBRACKET => Some(VK_SLASH),        // [ -> /
        VK_RIGHTBRACKET => Some(VK_ANSI_EQUALS), // ] -> =
        // Home row
        VK_A => Some(VK_A),         // A -> A
        VK_S => Some(VK_O),         // S -> O
        VK_D => Some(VK_E),         // D -> E
        VK_F => Some(VK_U),         // F -> U
        VK_G => Some(VK_I),         // G -> I
        VK_H => Some(VK_D),         // H -> D
        VK_J => Some(VK_H),         // J -> H
        VK_K => Some(VK_T),         // K -> T
        VK_L => Some(VK_N),         // L -> N
        VK_SEMICOLON => Some(VK_S), // ; -> S
        VK_QUOTE => Some(VK_MINUS), // ' -> -
        // Bottom row
        VK_Z => Some(VK_SEMICOLON),         // Z -> ;
        VK_X => Some(VK_Q),                 // X -> Q
        VK_C => Some(VK_J),                 // C -> J
        VK_V => Some(VK_K),                 // V -> K
        VK_B => Some(VK_X),                 // B -> X
        VK_N => Some(VK_B),                 // N -> B
        VK_M => Some(VK_M),                 // M -> M
        VK_COMMA => Some(VK_W),             // , -> W
        VK_PERIOD => Some(VK_V),            // . -> V
        VK_SLASH => Some(VK_Z),             // / -> Z
        VK_BACKSLASH => Some(VK_BACKSLASH), // \ stays \\
        // Number-row punctuation: map '-' and '=' to bracket keys so they produce Dvorak brackets
        VK_MINUS => Some(VK_LEFTBRACKET), // '-' -> '['  (and shift '{')
        VK_ANSI_EQUALS => Some(VK_RIGHTBRACKET), // '=' -> ']'  (and shift '}')
        // Digits (identity mapping)
        VK_ANSI_1 | VK_ANSI_2 | VK_ANSI_3 | VK_ANSI_4 | VK_ANSI_5 | VK_ANSI_6 | VK_ANSI_7
        | VK_ANSI_8 | VK_ANSI_9 | VK_ANSI_0 => Some(key),
        _ => None,
    }
}

fn main() {
    // Initialize tracing subscriber (logs to stderr by default)
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_ansi(false)
        .init();
    // Parse command-line arguments
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Install) => {
            if let Err(e) = do_install() {
                error!(%e, "Install failed");
                process::exit(1);
            }
        }
        Some(Commands::Uninstall) => {
            if let Err(e) = do_uninstall() {
                error!(%e, "Uninstall failed");
                process::exit(1);
            }
        }
        None => {
            run_tap();
        }
    }
}

// Install as a LaunchAgent
fn do_install() -> Result<(), Box<dyn Error>> {
    let exe = env::current_exe()?;
    let home = env::var("HOME")?;
    let la_dir = format!("{}/Library/LaunchAgents", home);
    fs::create_dir_all(&la_dir)?;
    let plist_path = format!("{}/com.macos_keyboard_remapper.plist", la_dir);
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>com.macos_keyboard_remapper</string>
  <key>ProgramArguments</key>
  <array>
    <string>{}</string>
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>/tmp/macos_keyboard_remapper.log</string>
  <key>StandardErrorPath</key>
  <string>/tmp/macos_keyboard_remapper.log</string>
</dict>
</plist>
"#,
        exe.display()
    );
    // Write the LaunchAgent plist
    fs::write(&plist_path, plist)?;
    info!("Created LaunchAgent plist at {}", plist_path);
    // Reload agent
    // Unload any existing agent
    info!("Unloading existing LaunchAgent (if any)");
    let _ = process::Command::new("launchctl")
        .args(&["unload", &plist_path])
        .output();
    // Load the new agent
    info!("Loading LaunchAgent");
    let _ = process::Command::new("launchctl")
        .args(&["load", &plist_path])
        .output()?;
    info!("LaunchAgent installed and loaded");
    Ok(())
}

// Uninstall LaunchAgent
fn do_uninstall() -> Result<(), Box<dyn Error>> {
    let home = env::var("HOME")?;
    let plist_path = format!(
        "{}/Library/LaunchAgents/com.macos_keyboard_remapper.plist",
        home
    );
    // Unload the LaunchAgent
    info!("Unloading LaunchAgent");
    let _ = process::Command::new("launchctl")
        .args(&["unload", &plist_path])
        .output();
    // Remove the plist file
    fs::remove_file(&plist_path)?;
    info!("LaunchAgent removed (plist deleted)");
    Ok(())
}

/// Run the keyboard remapping event tap (never returns)
fn run_tap() -> ! {
    // Create a CGEventTap using the core-graphics wrapper
    let tap = CGEventTap::new(
        CGEventTapLocation::AnnotatedSession,
        CGEventTapPlacement::HeadInsertEventTap, // get the events before any other tap
        CGEventTapOptions::Default,              // block events and modify them
        vec![
            CGEventType::KeyDown,
            CGEventType::KeyUp,
            CGEventType::TapDisabledByTimeout,
            CGEventType::TapDisabledByUserInput,
        ],
        |_, event_type, event| {
            match event_type {
                CGEventType::KeyDown | CGEventType::KeyUp => {
                    let keycode =
                        event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE) as u64;
                    if (event.get_flags()
                        & (CGEventFlags::CGEventFlagCommand
                            | CGEventFlags::CGEventFlagControl
                            | CGEventFlags::CGEventFlagSecondaryFn))
                        .is_empty()
                    {
                        if let Some(mapped) = remap_key(keycode) {
                            debug!("Remapped {} to {}", keycode, mapped);
                            event.set_integer_value_field(
                                EventField::KEYBOARD_EVENT_KEYCODE,
                                mapped as i64,
                            );
                            return Some(event.clone());
                        }
                    } else {
                        debug!("Did not remap {}", keycode);
                    }
                }
                CGEventType::TapDisabledByTimeout | CGEventType::TapDisabledByUserInput => {
                    error!("Event tap disabled; cause: {:?}", event_type);
                }
                _ => {}
            }
            None
        },
    )
    .expect("Failed to create event tap. Make sure to grant accessibility permissions.");

    // Add the event tap to the current run loop
    let run_loop = CFRunLoop::get_current();
    let source = tap
        .mach_port
        .create_runloop_source(0)
        .expect("Failed to create run loop source");
    // Add the source to the run loop
    unsafe { run_loop.add_source(&source, kCFRunLoopCommonModes) };
    tap.enable();
    CFRunLoop::run_current();
    process::exit(0);
}
