use std::{
    io::{Read, Write},
    net::{TcpStream, UdpSocket},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
};

const STREAM_WIDTH: u32 = 1280;
const STREAM_HEIGHT: u32 = 720;
const WINDOW_WIDTH: u32 = 1280;
const WINDOW_HEIGHT: u32 = 720;

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

fn main() {
    let rbpi_ip = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: client <raspberr pi IP>");
        std::process::exit(1);
    });

    print!("here");

    let mut tcp =
        TcpStream::connect(format!("{}:9000", rbpi_ip)).expect("Failed to connect to raspberry pi");
    let mut id_buf = [0u8; 1];

    print!("here2");

    // tcp.read_exact(&mut id_buf)
    //     .expect("Did not recieve player_id from host");

    let udp = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind to address");
    udp.connect(format!("{}:9001", rbpi_ip))
        .expect("Cannot establish UDP connection to rasberry pi");

    let frame_rx = start_video_recieve(tcp);

    let sdl = sdl2::init().expect("SDL2 init failed");
    let video = sdl.video().expect("SDL2 video init failed");
}
