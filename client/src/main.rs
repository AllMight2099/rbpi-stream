use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpStream, UdpSocket},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

use sdl2::{
    event::Event,
    keyboard::Keycode,
    pixels::{Color, PixelFormatEnum},
    rect::Rect,
    render::Canvas,
    video::Window,
};
use serde::{Deserialize, Serialize};

const STREAM_WIDTH: u32 = 1024;
const STREAM_HEIGHT: u32 = 768;
const WINDOW_WIDTH: u32 = 1024;
const WINDOW_HEIGHT: u32 = 768;

const BG: Color = Color::RGB(15, 15, 25);
const ACCENT: Color = Color::RGB(99, 179, 237);
const TEXT: Color = Color::RGB(220, 220, 220);
const MUTED: Color = Color::RGB(100, 100, 120);
const SELECTED: Color = Color::RGB(30, 50, 80);
const BORDER: Color = Color::RGB(50, 50, 80);

#[derive(Serialize, Deserialize, Debug)]
struct InputEvent {
    player_id: u8,
    key: String,
    down: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "cmd")]
enum ControlCommand {
    ListGames,
    LaunchGame { path: String },
    StopGame,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum ControlResponse {
    GameList { games: Vec<GameEntry> },
    Launched { name: String },
    Stopped,
    Error { message: String },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct GameEntry {
    name: String,
    path: String,
}

struct ControlChannel {
    writer: TcpStream,
    reader: BufReader<TcpStream>,
}

impl ControlChannel {
    fn connect(pi_ip: &str) -> Self {
        let stream = TcpStream::connect(format!("{}:9002", pi_ip))
            .expect("cannot connect to control channel :9002");
        let reader = BufReader::new(stream.try_clone().unwrap());
        Self {
            writer: stream,
            reader,
        }
    }

    fn send(&mut self, cmd: &ControlCommand) -> ControlResponse {
        let mut json = serde_json::to_string(cmd).unwrap();
        json.push('\n');
        self.writer
            .write_all(json.as_bytes())
            .expect("control write failed");

        let mut line = String::new();
        self.reader
            .read_line(&mut line)
            .expect("Control read failed");
        serde_json::from_str(line.trim()).unwrap_or(ControlResponse::Error {
            message: "Bad Response".to_string(),
        })
    }

    fn list_games(&mut self) -> Vec<GameEntry> {
        match self.send(&ControlCommand::ListGames) {
            ControlResponse::GameList { games } => games,
            _ => vec![],
        }
    }

    fn launch(&mut self, path: String) -> bool {
        matches!(
            self.send(&ControlCommand::LaunchGame { path }),
            ControlResponse::Launched { .. }
        )
    }
}
fn draw_picker(
    canvas: &mut Canvas<Window>,
    games: &[GameEntry],
    selected: usize,
    scroll: usize,
    status: &str,
    _player_id: u8,
) {
    canvas.set_draw_color(BG);
    canvas.clear();

    let item_h: i32 = 36;
    let list_top: i32 = 60;
    let visible = ((WINDOW_HEIGHT as i32 - list_top - 50) / item_h) as usize;

    // Header bar
    canvas.set_draw_color(ACCENT);
    canvas.fill_rect(Rect::new(0, 0, WINDOW_WIDTH, 50)).ok();

    // Selected item highlight + accent bar
    if !games.is_empty() {
        let sel_y = list_top + (selected - scroll) as i32 * item_h;
        canvas.set_draw_color(SELECTED);
        canvas
            .fill_rect(Rect::new(0, sel_y, WINDOW_WIDTH, item_h as u32))
            .ok();
        canvas.set_draw_color(ACCENT);
        canvas.fill_rect(Rect::new(0, sel_y, 4, item_h as u32)).ok();
    }

    // Draw a row indicator for each visible game (colored bar as placeholder)
    for (i, _game) in games.iter().enumerate().skip(scroll).take(visible) {
        let y = list_top + (i - scroll) as i32 * item_h;
        let is_selected = i == selected;
        let color = if is_selected { ACCENT } else { MUTED };
        canvas.set_draw_color(color);
        // Draw a thin underline per row
        canvas
            .draw_line(
                (20, y + item_h - 1),
                (WINDOW_WIDTH as i32 - 20, y + item_h - 1),
            )
            .ok();
    }

    // Scrollbar
    if games.len() > visible && !games.is_empty() {
        let bar_h = WINDOW_HEIGHT as i32 - list_top - 50;
        let thumb_h = (bar_h * visible as i32 / games.len() as i32).max(20);
        let thumb_y = list_top + bar_h * scroll as i32 / games.len() as i32;
        canvas.set_draw_color(BORDER);
        canvas
            .fill_rect(Rect::new(
                WINDOW_WIDTH as i32 - 6,
                list_top,
                6,
                bar_h as u32,
            ))
            .ok();
        canvas.set_draw_color(MUTED);
        canvas
            .fill_rect(Rect::new(
                WINDOW_WIDTH as i32 - 6,
                thumb_y,
                6,
                thumb_h as u32,
            ))
            .ok();
    }

    // Status bar at bottom
    canvas.set_draw_color(BORDER);
    canvas
        .fill_rect(Rect::new(0, WINDOW_HEIGHT as i32 - 40, WINDOW_WIDTH, 40))
        .ok();

    // Print game list to stdout so user can see it in terminal
    println!(
        "[picker] {} games | selected: {} | {}",
        games.len(),
        games
            .get(selected)
            .map(|g| g.name.as_str())
            .unwrap_or("none"),
        status
    );

    canvas.present();
}

fn start_video_recieve(tcp: TcpStream) -> mpsc::Receiver<Vec<u8>> {
    let mut ffmpeg = Command::new("ffmpeg")
        .args([
            "-loglevel",
            "error",
            "-fflags",
            "nobuffer",
            "-flags",
            "low_delay",
            "-strict",
            "experimental",
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
    let (tx, rx) = mpsc::sync_channel::<Vec<u8>>(1);

    thread::spawn(move || {
        let mut frame = vec![0u8; frame_size];
        loop {
            match ffmpeg_output.read_exact(&mut frame) {
                Ok(()) => {
                    let _ = tx.try_send(frame.clone());
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

fn run_game(
    canvas: &mut Canvas<Window>,
    events: &mut sdl2::EventPump,
    frame_rx: mpsc::Receiver<Vec<u8>>,
    udp: &UdpSocket,
    player_id: u8,
) {
    let tc = canvas.texture_creator();
    let mut texture = tc
        .create_texture_streaming(PixelFormatEnum::IYUV, STREAM_WIDTH, STREAM_HEIGHT)
        .expect("Texture failed");
    let dst = Rect::new(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT);

    println!("[client] Streaming — press Escape to return to game picker");

    loop {
        for event in events.poll_iter() {
            match event {
                Event::Quit { .. } => return,
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    repeat: false,
                    ..
                } => return,
                Event::KeyDown {
                    keycode: Some(k),
                    repeat: false,
                    ..
                } => {
                    send_input(udp, player_id, &k.name().to_lowercase(), true);
                }
                Event::KeyUp {
                    keycode: Some(k), ..
                } => {
                    send_input(udp, player_id, &k.name().to_lowercase(), false);
                }
                _ => {}
            }
        }

        let mut latest: Option<Vec<u8>> = None;
        loop {
            match frame_rx.try_recv() {
                Ok(f) => latest = Some(f),
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => return,
            }
        }

        if let Some(frame) = latest {
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
                .ok();
        }

        canvas.clear();
        canvas.copy(&texture, None, Some(dst)).unwrap();
        canvas.present();
        thread::sleep(Duration::from_millis(1));
    }
}

fn main() {
    let rbpi_ip = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: client <raspberr pi IP>");
        std::process::exit(1);
    });

    let mut tcp =
        TcpStream::connect(format!("{}:9000", rbpi_ip)).expect("Failed to connect to raspberry pi");

    let mut id_buf = [0u8; 1];
    tcp.read_exact(&mut id_buf)
        .expect("Did not receive player_id from host");
    let player_id = id_buf[0];
    println!("[client] Connected as player {}", player_id);

    let mut ctrl = ControlChannel::connect(&rbpi_ip);

    print!("here2");

    // tcp.read_exact(&mut id_buf)
    //     .expect("Did not recieve player_id from host");

    let udp = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind to address");
    udp.connect(format!("{}:9001", rbpi_ip))
        .expect("Cannot establish UDP connection to rasberry pi");

    // let frame_rx = start_video_recieve(tcp);

    let sdl = sdl2::init().expect("SDL2 init failed");
    let video = sdl.video().expect("SDL2 video init failed");
    // let ttf = sdl2::ttf::init().expect("SDL2 TTF init failed");

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

    // let tc = canvas.texture_creator();
    // let mut texture = tc
    //     .create_texture_streaming(PixelFormatEnum::IYUV, STREAM_WIDTH, STREAM_HEIGHT)
    //     .expect("Failed to generate texture");

    let mut events = sdl.event_pump().expect("event pump failed");

    if player_id != 1 {
        println!("[client] Player 2 — waiting for player 1 to launch a game...");
        println!("[client] Press Enter when player 1 has selected a game...");
        // Wait for user to press enter before connecting stream
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
        let frame_rx = start_video_recieve(tcp);
        run_game(&mut canvas, &mut events, frame_rx, &udp, player_id);
        return;
    }

    let mut games = vec![];
    let mut selected: usize = 0;
    let mut scroll: usize = 0;
    let mut status = String::from("Loading game list...");
    let visible_items = ((WINDOW_HEIGHT as i32 - 130) / 40) as usize;

    draw_picker(&mut canvas, &games, selected, scroll, &status, player_id);

    // Fetch game list
    games = ctrl.list_games();
    status = if games.is_empty() {
        "No ROMs found on the Pi".to_string()
    } else {
        format!("{} games found", games.len())
    };

    'picker: loop {
        draw_picker(&mut canvas, &games, selected, scroll, &status, player_id);

        // Wait for next event
        let event = events.wait_event_timeout(100);
        let Some(event) = event else { continue };

        match event {
            Event::Quit { .. } => break 'picker,

            Event::KeyDown {
                keycode: Some(key),
                repeat: false,
                ..
            } => match key {
                Keycode::Escape => break 'picker,

                Keycode::Up => {
                    if selected > 0 {
                        selected -= 1;
                        if selected < scroll {
                            scroll = selected;
                        }
                    }
                }

                Keycode::Down => {
                    if selected + 1 < games.len() {
                        selected += 1;
                        if selected >= scroll + visible_items {
                            scroll = selected - visible_items + 1;
                        }
                    }
                }

                Keycode::Return => {
                    if let Some(game) = games.get(selected) {
                        status = format!("Launching {}...", game.name);
                        draw_picker(&mut canvas, &games, selected, scroll, &status, player_id);

                        let path = game.path.clone();
                        let name = game.name.clone();
                        if ctrl.launch(path) {
                            status = format!("Launching {}... connecting stream", name);
                            draw_picker(&mut canvas, &games, selected, scroll, &status, player_id);
                            // Wait for RetroArch + broadcaster to start
                            thread::sleep(Duration::from_secs(3));
                            // Now start the video pipeline
                            let frame_rx = start_video_recieve(tcp);
                            run_game(&mut canvas, &mut events, frame_rx, &udp, player_id);
                            break 'picker;
                        } else {
                            status = "Failed to launch game".to_string();
                        }
                    }
                }

                _ => {}
            },

            _ => {}
        }
    }

    // let dst = Rect::new(0, 0, WINDOW_WIDTH, WINDOW_HEIGHT);

    // println!("[client] Running - press escape to quit");

    // 'main: loop {
    //     for event in events.poll_iter() {
    //         match event {
    //             Event::Quit { .. } => break 'main,
    //             Event::KeyDown {
    //                 keycode: Some(Keycode::Escape),
    //                 repeat: false,
    //                 ..
    //             } => {
    //                 break 'main;
    //             }
    //             Event::KeyDown {
    //                 keycode: Some(k),
    //                 repeat: false,
    //                 ..
    //             } => {
    //                 send_input(&udp, player_id, &k.name().to_lowercase(), true);
    //             }

    //             Event::KeyUp {
    //                 keycode: Some(k), ..
    //             } => {
    //                 send_input(&udp, player_id, &k.name().to_lowercase(), false);
    //             }
    //             _ => {}
    //         }
    //     }

    //     let mut latest_frame: Option<Vec<u8>> = None;
    //     loop {
    //         match frame_rx.try_recv() {
    //             Ok(f) => latest_frame = Some(f),
    //             Err(mpsc::TryRecvError::Empty) => break,
    //             Err(mpsc::TryRecvError::Disconnected) => break 'main,
    //         }
    //     }

    //     if let Some(frame) = latest_frame {
    //         let y = (STREAM_WIDTH * STREAM_HEIGHT) as usize;
    //         let uv = y / 4;
    //         texture
    //             .update_yuv(
    //                 None,
    //                 &frame[..y],
    //                 STREAM_WIDTH as usize,
    //                 &frame[y..y + uv],
    //                 (STREAM_WIDTH / 2) as usize,
    //                 &frame[y + uv..],
    //                 (STREAM_WIDTH / 2) as usize,
    //             )
    //             .expect("Failed to update texture");
    //     }

    //     canvas.clear();
    //     canvas.copy(&texture, None, Some(dst)).unwrap();
    //     canvas.present();
    //     thread::sleep(Duration::from_millis(1));
    // }
}
