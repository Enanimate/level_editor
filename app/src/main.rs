#[allow(unused_imports)]
use std::{error::Error, ffi::{c_char, CStr}, io::Read, path::PathBuf};
use gfx::gui::interface::Color;
#[allow(unused_imports)]
use libloading::{Library, Symbol};
#[allow(unused_imports)]
use serde::Deserialize;

use crate::window::gui::EditorApp;

mod window;

fn main() {
    //load_lib().unwrap();
    //let mut config_buf: String = String::new();
    //let file = std::fs::File::open("config.toml").expect("Failed to open config file...").read_to_string(&mut config_buf);
    //let config = toml::from_str::<Config>(&config_buf).unwrap();

    //println!("{:?}", config.keys.github);
    Color::from_hex("#ff1212");
    EditorApp::new().unwrap();
    //run(gui_interface).unwrap();
}

/*
fn load_lib() -> Result<(), Box<dyn Error>> {
    println!("Starting editor...");
    let lib_path = {
        let mut path = PathBuf::from("../game_engine_core/target/debug/");
        if cfg!(target_os = "windows") {
            path.push("game_engine_core.dll");
        } else {
            todo!()
        }
        path
    };

    println!("Attemting to load plugin from {:?}", lib_path);

    let library = unsafe { Library::new(&lib_path)? };

    let get_message: Symbol<unsafe extern "C" fn() -> *mut c_char> = unsafe {
        library.get(b"get_plugin_message\0")? };
    println!("'get_plugin_message' symbol found.");

    let free_string: Symbol<unsafe extern "C" fn(*mut c_char)> = unsafe {
        library.get(b"free_plugin_string\0")? };
    println!("'free_plugin_string' symbol found.");
    
    let message_ptr = unsafe { get_message() };

    if message_ptr.is_null() {
        eprintln!("Plugin returned a null pointer for the message!");
        return Err("Plugin message was null".into());
    }

    let c_str_message = unsafe {
        CStr::from_ptr(message_ptr) };
    
    let rust_message = c_str_message.to_string_lossy().into_owned();

    println!("Message from plugin: {}", rust_message);

    unsafe {
        free_string(message_ptr);
    }

    println!("Plugin string memory freed.");

    println!("Editor finished.");

    Ok(())
}

#[derive(Deserialize)]
struct Config {
   ip: String,
   port: Option<u16>,
   keys: Keys,
}

#[derive(Deserialize)]
struct Keys {
   github: String,
   travis: Option<String>,
}
   */