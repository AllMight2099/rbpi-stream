use std::{
    io::{Read, Write},
    net::{TcpListener, UdpSocket},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    thread,
};

use evdev::{
    AbsInfo, AbsoluteAxisCode, AttributeSet, EventType, InputEvent as EvdevEvent, KeyCode,
    UinputAbsSetup, uinput::VirtualDevice,
};
use serde::{Deserialize, Serialize};

type Clients = Arc<Mutex<Vec<std::net::TcpStream>>>;
type Gamepads = Arc<Mutex<[evdev::uinput::VirtualDevice; 2]>>;

#[derive(Serialize, Deserialize, Debug)]
struct ClientInputEvent {
    player_id: u8,
    key: String,
    down: bool,
}

fn create_gamepad_keys() -> AttributeSet<KeyCode> {
    AttributeSet::from_iter([
        KeyCode::KEY_UP,
        KeyCode::KEY_DOWN,
        KeyCode::KEY_LEFT,
        KeyCode::KEY_RIGHT,
        KeyCode::KEY_ENTER,
        KeyCode::KEY_SPACE,
        KeyCode::KEY_BACKSPACE,
        KeyCode::KEY_TAB,
        KeyCode::KEY_ESC,
        KeyCode::KEY_LEFTSHIFT,
        KeyCode::KEY_RIGHTSHIFT,
        KeyCode::KEY_LEFTCTRL,
        KeyCode::KEY_RIGHTCTRL,
        KeyCode::KEY_LEFTALT,
        KeyCode::KEY_RIGHTALT,
        KeyCode::KEY_A,
        KeyCode::KEY_B,
        KeyCode::KEY_C,
        KeyCode::KEY_D,
        KeyCode::KEY_E,
        KeyCode::KEY_F,
        KeyCode::KEY_G,
        KeyCode::KEY_H,
        KeyCode::KEY_I,
        KeyCode::KEY_J,
        KeyCode::KEY_K,
        KeyCode::KEY_L,
        KeyCode::KEY_M,
        KeyCode::KEY_N,
        KeyCode::KEY_O,
        KeyCode::KEY_P,
        KeyCode::KEY_Q,
        KeyCode::KEY_R,
        KeyCode::KEY_S,
        KeyCode::KEY_T,
        KeyCode::KEY_U,
        KeyCode::KEY_V,
        KeyCode::KEY_W,
        KeyCode::KEY_X,
        KeyCode::KEY_Y,
        KeyCode::KEY_Z,
        KeyCode::KEY_1,
        KeyCode::KEY_2,
        KeyCode::KEY_3,
        KeyCode::KEY_4,
        KeyCode::KEY_5,
        KeyCode::KEY_6,
        KeyCode::KEY_7,
        KeyCode::KEY_8,
        KeyCode::KEY_9,
        KeyCode::KEY_0,
        KeyCode::KEY_F1,
        KeyCode::KEY_F2,
        KeyCode::KEY_F3,
        KeyCode::KEY_F4,
        KeyCode::KEY_F5,
        KeyCode::KEY_F6,
        KeyCode::KEY_F7,
        KeyCode::KEY_F8,
        KeyCode::KEY_F9,
        KeyCode::KEY_F10,
        KeyCode::KEY_F11,
        KeyCode::KEY_F12,
        KeyCode::BTN_SOUTH,
        KeyCode::BTN_EAST,
        KeyCode::BTN_NORTH,
        KeyCode::BTN_WEST,
        KeyCode::BTN_START,
        KeyCode::BTN_SELECT,
        KeyCode::BTN_DPAD_DOWN,
        KeyCode::BTN_DPAD_LEFT,
        KeyCode::BTN_DPAD_RIGHT,
        KeyCode::BTN_DPAD_UP,
        KeyCode::BTN_TL,
        KeyCode::BTN_TR,
    ])
}

fn build_virtual_gamepad(name: &str) -> evdev::uinput::VirtualDevice {
    VirtualDevice::builder()
        .expect("failed to open /dev/uinput")
        .name(name)
        .with_keys(&create_gamepad_keys())
        .expect("Failed to register keys")
        .with_absolute_axis(&UinputAbsSetup::new(
            AbsoluteAxisCode::ABS_X,
            AbsInfo::new(0, -32768, 32767, 16, 128, 0),
        ))
        .expect("Failed to setup axis")
        .build()
        .expect("failed to build virtual device")
}

fn sdl_name_to_key(name: &str) -> Option<KeyCode> {
    Some(match name {
        // "up" => KeyCode::BTN_DPAD_UP,
        // "down" => KeyCode::BTN_DPAD_DOWN,
        // "left" => KeyCode::BTN_DPAD_LEFT,
        // "right" => KeyCode::BTN_DPAD_RIGHT,

        // "z" => KeyCode::BTN_SOUTH,
        // "x" => KeyCode::BTN_EAST,
        // "a" => KeyCode::BTN_WEST,
        // "s" => KeyCode::BTN_NORTH,

        // "return" => KeyCode::BTN_START,
        // "right shift" => KeyCode::BTN_SELECT,
        // "q" => KeyCode::BTN_TL,
        // "w" => KeyCode::BTN_TR,
        "up" => KeyCode::KEY_UP,
        "down" => KeyCode::KEY_DOWN,
        "left" => KeyCode::KEY_LEFT,
        "right" => KeyCode::KEY_RIGHT,
        "return" => KeyCode::KEY_ENTER,
        "space" => KeyCode::KEY_SPACE,
        "backspace" => KeyCode::KEY_BACKSPACE,
        "tab" => KeyCode::KEY_TAB,
        "escape" => KeyCode::KEY_ESC,
        "left shift" => KeyCode::KEY_LEFTSHIFT,
        "right shift" => KeyCode::KEY_RIGHTSHIFT,
        "left ctrl" => KeyCode::KEY_LEFTCTRL,
        "right ctrl" => KeyCode::KEY_RIGHTCTRL,
        "left alt" => KeyCode::KEY_LEFTALT,
        "right alt" => KeyCode::KEY_RIGHTALT,
        "a" => KeyCode::KEY_A,
        "q" => KeyCode::KEY_Q,
        "s" => KeyCode::KEY_S,
        "w" => KeyCode::KEY_W,
        "x" => KeyCode::KEY_X,
        "z" => KeyCode::KEY_Z,
        "b" => KeyCode::KEY_B,
        "c" => KeyCode::KEY_C,
        "d" => KeyCode::KEY_D,
        "e" => KeyCode::KEY_E,
        "f" => KeyCode::KEY_F,
        "g" => KeyCode::KEY_G,
        "h" => KeyCode::KEY_H,
        "i" => KeyCode::KEY_I,
        "j" => KeyCode::KEY_J,
        "k" => KeyCode::KEY_K,
        "l" => KeyCode::KEY_L,
        "m" => KeyCode::KEY_M,
        "n" => KeyCode::KEY_N,
        "o" => KeyCode::KEY_O,
        "p" => KeyCode::KEY_P,

        "r" => KeyCode::KEY_R,

        "t" => KeyCode::KEY_T,
        "u" => KeyCode::KEY_U,
        "v" => KeyCode::KEY_V,

        "y" => KeyCode::KEY_Y,

        "1" => KeyCode::KEY_1,
        "2" => KeyCode::KEY_2,
        "3" => KeyCode::KEY_3,
        "4" => KeyCode::KEY_4,
        "5" => KeyCode::KEY_5,
        "6" => KeyCode::KEY_6,
        "7" => KeyCode::KEY_7,
        "8" => KeyCode::KEY_8,
        "9" => KeyCode::KEY_9,
        "0" => KeyCode::KEY_0,
        "f1" => KeyCode::KEY_F1,
        "f2" => KeyCode::KEY_F2,
        "f3" => KeyCode::KEY_F3,
        "f4" => KeyCode::KEY_F4,
        "f5" => KeyCode::KEY_F5,
        "f6" => KeyCode::KEY_F6,
        "f7" => KeyCode::KEY_F7,
        "f8" => KeyCode::KEY_F8,
        "f9" => KeyCode::KEY_F9,
        "f10" => KeyCode::KEY_F10,
        "f11" => KeyCode::KEY_F11,
        "f12" => KeyCode::KEY_F12,

        _ => return None,
    })
}

fn emit_key(device: &mut evdev::uinput::VirtualDevice, key: KeyCode, down: bool) {
    let value = if down { 1 } else { 0 };
    let key_ev = EvdevEvent::new(EventType::KEY.0, key.code(), value);
    let syn_ev = EvdevEvent::new(EventType::SYNCHRONIZATION.0, 0, 0);
    let _ = device.emit(&[key_ev, syn_ev]);
}

fn listen_input(gamepads: Gamepads) {
    let socket = UdpSocket::bind("0.0.0.0:9001").expect("Cannot bind to address 0.0.0.0:9001");
    println!("[host] input listener running on UDP 0.0.0.0:9001");

    let mut buf = [0u8; 4096];
    loop {
        let Ok((len, _)) = socket.recv_from(&mut buf) else {
            continue;
        };
        let Ok(event) = serde_json::from_slice::<ClientInputEvent>(&buf[..len]) else {
            continue;
        };

        let idx = (event.player_id as usize).saturating_sub(1).min(1);
        let key_name = event.key.to_lowercase();

        if let Some(key) = sdl_name_to_key(&key_name) {
            let mut pads = gamepads.lock().unwrap();
            emit_key(&mut pads[idx], key, event.down);
        } else {
            eprintln!(
                "[host] Unknown key '{}' from player {}",
                event.key, event.player_id
            );
        }
    }
}

fn broadcast_to_clients(clients: Clients) {
    println!("[host] Starting kmsgrab (card1) + h264_v4l2m2m encoder...");

    let mut ffmpeg = Command::new("ffmpeg")
        .args([
            "-loglevel",
            "error",
            "-f",
            "kmsgrab",
            "-device",
            "/dev/dri/card1",
            "-r",
            "30",
            "-i",
            "-",
            "-vf",
            "hwdownload,format=bgr0,format=yuv420p",
            "-vcodec",
            "h264_v4l2m2m",
            "-b:v",
            "2M",
            "-bufsize",
            "500k",
            "-fflags",
            "nobuffer",
            "-flags",
            "low_delay",
            "-strict",
            "experimental",
            "-avioflags",
            "direct",
            "-g",
            "24",
            "-f",
            "mpegts",
            "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to start ffmpeg");

    let mut output = ffmpeg.stdout.take().unwrap();
    let mut buffer = [0u8; 65536];

    loop {
        match output.read(&mut buffer) {
            Ok(0) | Err(_) => {
                println!("[host] ffmpeg process closed");
                break;
            }
            Ok(n) => {
                let mut clients = clients.lock().unwrap();
                clients.retain_mut(|s| s.write_all(&buffer[..n]).is_ok());
            }
        }
    }

    let _ = ffmpeg.kill();
}

fn accept_loop(clients: Clients) {
    let listener = TcpListener::bind("0.0.0.0:9000").expect("cannot bind TCP 0.0.0.0:9000");
    println!("[host] waiting for players on TCP :9000 .......");

    let mut next_id: u8 = 1;

    for stream in listener.incoming().flatten() {
        let mut s = stream;
        let addr = s.peer_addr().unwrap();

        if s.write_all(&[next_id]).is_err() {
            eprintln!("[host] Failed to send player_id to {}", addr);
            continue;
        }

        println!("[host] Player {} connected from {}", next_id, addr);
        clients.lock().unwrap().push(s);

        // Wrap back to 1 after player 2 — 3rd connection becomes player 1 again
        // (useful if a player disconnects and reconnects)
        next_id = if next_id >= 2 { 1 } else { next_id + 1 };
    }
}

fn main() {
    let gamepads: Gamepads = Arc::new(Mutex::new([
        build_virtual_gamepad("gamepad1"),
        build_virtual_gamepad("gamepad2"),
    ]));
    println!("[server] created virtual gamepads");

    let clients: Clients = Arc::new(Mutex::new(Vec::new()));

    let g = Arc::clone(&gamepads);
    thread::spawn(move || listen_input(g));

    // broadcast loop
    let c = Arc::clone(&clients);
    thread::spawn(move || broadcast_to_clients(c));

    // thread::spawn(move)
    accept_loop(clients);
}
