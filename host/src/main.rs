use std::{
    io::Read,
    process::{Command, Stdio},
};
fn main() {
    let mut ffmpeg = Command::new("ffmpeg")
        .args([
            "-loglevel",
            "error",
            "-f",
            "kmsgrab",
            "-framerate",
            "30",
            "-i",
            "-",
            "-vf",
            "hwdownload, format=bgr0,format=yuv420p",
            "-vcodec",
            "h264_v412m2m",
            "-b:v",
            "4M",
            "bufsize",
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
