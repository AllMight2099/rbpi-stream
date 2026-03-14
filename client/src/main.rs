use std::{
    io::{Read, Write},
    net::{TcpStream, UdpSocket},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

use sdl2::{event::Event, keyboard::Keycode, pixels::PixelFormatEnum, rect::Rect};
use serde::{Deserialize, Serialize};

const STREAM_WIDTH: u32 = 1280;
const STREAM_HEIGHT: u32 = 720;
const WINDOW_WIDTH: u32 = 1280;
const WINDOW_HEIGHT: u32 = 720;

#[derive(Serialize, Deserialize, Debug)]
struct InputEvent {
    player_id: u8,
    key: String,
    down: bool,
}

fn start_video_recieve(tcp: TcpStream) -> mpsc::Receiver<Vec<u8>> {
    let mut ffmpeg = Command::new("ffmpeg")
        .args([
            "-loglevel",
            "error",
            "-f",
            "mpegts",
            "-i",
            "pipe:0",
            "-f",
            "rawvideo",
            "-pixel_format",
            "yuv420p",
            "-video_size",
            &format!("{}x{}", STREAM_WIDTH, STREAM_HEIGHT),
            "pipe:1",
        ])
        .stdin(Stdio::piped())
        .stderr(Stdio::inherit())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to start ffmpeg");

    println!("successully executed ffmpeg");

    let mut ffmpeg_input = ffmpeg.stdin.take().unwrap();
    let mut ffmpeg_output = ffmpeg.stdout.take().unwrap();

    let mut tcp_read = tcp.try_clone().unwrap();
    thread::spawn(move || {
        let mut buf = vec![0u8; 65536]; // why
        loop {
            match tcp_read.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if ffmpeg_input.write_all(&buf[..n]).is_err() {
                        break;
                    }
                }
            }
        }
    });

    let frame_size = (STREAM_WIDTH * STREAM_HEIGHT * 3 / 2) as usize;
    let (tx, rx) = mpsc::channel::<Vec<u8>>();

    thread::spawn(move || {
        let mut frame = vec![0u8; frame_size];
        loop {
            match ffmpeg_output.read_exact(&mut frame) {
                Ok(()) => {
                    let _ = tx.send(frame.clone());
                }
                Err(_) => break,
            }
        }
    });

    rx
}

fn send_input(socket: &UdpSocket, player_id: u8, key: &str, down: bool) {
    let event = InputEvent {
        player_id,
        key: key.to_string(),
        down,
    };
    if let Ok(bytes) = serde_json::to_vec(&event) {
        let _ = socket.send(&bytes);
    }
}

fn main() {
    let rbpi_ip = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: client <raspberr pi IP>");
        std::process::exit(1);
    });

    print!("here");

    let mut tcp =
        TcpStream::connect(format!("{}:9000", rbpi_ip)).expect("Failed to connect to raspberry pi");

    let mut id_buf = [0u8; 1];
    tcp.read_exact(&mut id_buf)
        .expect("Did not receive player_id from host");
    let player_id = id_buf[0];
    println!("[client] Connected as player {}", player_id);

    print!("here2");

    tcp.read_exact(&mut id_buf)
        .expect("Did not recieve player_id from host");

    let udp = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind to address");
    udp.connect(format!("{}:9001", rbpi_ip))
        .expect("Cannot establish UDP connection to rasberry pi");

    let frame_rx = start_video_recieve(tcp);

    let sdl = sdl2::init().expect("SDL2 init failed");
    let video = sdl.video().expect("SDL2 video init failed");

    let title = format!("rbpi-stream-player - player {}", player_id);
    let window = video
        .window(&title, WINDOW_WIDTH, WINDOW_HEIGHT)
        .resizable()
        .position_centered()
        .build()
        .expect("Window creation failed");

    let mut canvas = window
        .into_canvas()
        .accelerated()
        .present_vsync()
        .build()
        .expect("Canvas creation failed");

    let tc = canvas.texture_creator();
    let mut texture = tc
        .create_texture_streaming(PixelFormatEnum::IYUV, STREAM_WIDTH, STREAM_HEIGHT)
        .expect("Failed to generate texture");

    let mut events = sdl.event_pump().expect("event pump failed");
    let dst = Rect::new(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT);

    println!("[client] Running - press escape to quit");

    'main: loop {
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. } => break 'main,
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    repeat: false,
                    ..
                } => {
                    break 'main;
                }
                Event::KeyDown {
                    keycode: Some(k),
                    repeat: false,
                    ..
                } => {
                    send_input(&udp, player_id, &k.name().to_lowercase(), true);
                }

                Event::KeyUp {
                    keycode: Some(k), ..
                } => {
                    send_input(&udp, player_id, &k.name().to_lowercase(), false);
                }
                _ => {}
            }
        }

        let mut latest_frame: Option<Vec<u8>> = None;
        loop {
            match frame_rx.try_recv() {
                Ok(f) => latest_frame = Some(f),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => break 'main,
            }
        }

        if let Some(frame) = latest_frame {
            let y = (STREAM_WIDTH * STREAM_HEIGHT) as usize;
            let uv = y / 4;
            texture
                .update_yuv(
                    None,
                    &frame[..y],
                    STREAM_WIDTH as usize,
                    &frame[y..y + uv],
                    (STREAM_WIDTH / 2) as usize,
                    &frame[y + uv..],
                    (STREAM_WIDTH / 2) as usize,
                )
                .expect("Failed to update texture");
        }

        canvas.clear();
        canvas.copy(&texture, None, Some(dst)).unwrap();
        canvas.present();
        thread::sleep(Duration::from_millis(1));
    }
}
