use std::{
    io::Read,
    net::UdpSocket,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
};

use evdev::{AttributeSet, EventType, InputEvent as EvdevEvent, KeyCode, uinput::VirtualDevice};
use serde::{Deserialize, Serialize};

type Clients = Arc<Mutex<Vec<std::net::TcpStream>>>;
type Gamepads = Arc<Mutex<[evdev::uinput::VirtualDevice; 2]>>;

#[derive(Serialize, Deserialize, Debug)]
struct InputEvent {
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
    ])
}

fn build_virtual_gamepad(name: &str) -> evdev::uinput::VirtualDevice {
    VirtualDevice::builder()
        .expect("failed to open /dev/uinput")
        .name(name)
        .with_keys(&create_gamepad_keys())
        .expect("Failed to register keys")
        .build()
        .expect("failed to build virtual device")
}

fn sdl_name_to_key(name: &str) -> Option<KeyCode> {
    Some(match name {
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
        "b" => KeyCode::KEY_B,

        _ => return None,
    })
}

fn emit_key(device: &mut evdev::uinput::VirtualDevice, key: KeyCode, down: bool) {
    let value = if down { 1 } else { 0 };
    // let key_ev = EvdevEvent::new(EventType::KEY, code, value)
    // device.emit(events)
}

fn listen_input(gamepads: Gamepads) {
    let socket = UdpSocket::bind("0.0.0.0:9001").expect("Cannot bind to address 0.0.0.0:9001");
    println!("[host] input listener running on UDP 0.0.0.0:9001");

    let mut buf = [0u8; 4096];
    loop {
        let Ok((len, _)) = socket.recv_from(&mut buf) else {
            continue;
        };
        let Ok(event) = serde_json::from_slice::<InputEvent>(&buf[..len]) else {
            continue;
        };

        let idx = (event.player_id as usize).saturating_sub(1).min(1);
        let key_name = event.key.to_lowercase();

        if let Some(key) = sdl_name_to_key(&key_name) {
            let mut pads = gamepads.lock().unwrap();
        }
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
    // thread::spawn(move)

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
            "4M",
            "-bufsize",
            "2M",
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
                println!("ffmpeg process closed");
                break;
            }
            Ok(n) => {
                // Process the data in buffer[0..n]
                println!("Read {} bytes from ffmpeg", n);
            }
        }
    }

    let _ = ffmpeg.kill();
}
