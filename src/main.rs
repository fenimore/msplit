#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(dead_code)]
extern crate getopts;
extern crate mp3_metadata;

use mp3_metadata::MP3Metadata;

use std::error;
use std::io::Bytes;
use std::env;
use std::time::Duration;
use std::io::Result;
use std::io::prelude::*;
use std::path::Path;
use std::fs;
use std::fs::{File, Metadata};

use getopts::Options;


const MINBUF: usize = 2889;
const SOFTLIMIT: usize = 960;


static USAGE: &str = "program FILENAME";


// so FYI the nix::fcntl module _has_ a `tee` function
fn main() {
    // Arguments configuration
    let args: Vec<_> = env::args().collect();
    let mut opts = Options::new();
    opts.optopt("s", "seconds", "partition duration (in seconds)", "sec");
    opts.optopt("d", "dir", "output directory", "dir");
    opts.optopt("o", "output", "output filename", "outputr");
    opts.optflag("h", "help", "print help menu");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => panic!(e.to_string()),
    };
    if matches.opt_present("h") {
        println!("{:}", USAGE);
        return;
    };

    let output_filename = match matches.opt_str("o") {
        Some(s) => s,
        None => String::from("partition"),
    };
    let output_directory = match matches.opt_str("d") {
        Some(s) => s,
        None => String::from("partitions"),
    };
    let seconds: u64 = match matches.opt_str("s") {
        Some(s) => s.parse().expect("Couldn't parse Duration"),
        None => 5,
    };
    let partition_size = Duration::new(seconds, 0);

    //////////////////////
    // Get MP3 metadata //
    //////////////////////
    let filename: &str = if !matches.free.is_empty() {
        matches.free[0].as_str()
    } else {
        println!("{:}", USAGE);
        return;
    };
    let path: &Path = Path::new(filename);

    let fileinfo: Metadata = match fs::metadata(path) {
        Err(e) => panic!("No such file {}", e),
        Ok(result) => result,
    };

    let mut file_iter = match File::open(filename) {
        Err(e) => panic!("Couldn't open file"),
        Ok(f) => f,
    }.bytes();

    let metadata: MP3Metadata = mp3_metadata::read_from_file(path).unwrap();

    let frame_bytes_count = metadata.frames.iter().map(
        |frame| frame.size
    ).fold(0, |acc, x| acc + x);
    let mut pos_size = metadata.frames.iter().map(
        |frame| (frame.position, frame.size)
    ).peekable();

    /////////////////////////////////////////////////
    // Put Header aside for each partition write   //
    /////////////////////////////////////////////////
    let header_bytes: u32 = fileinfo.len() as u32 - frame_bytes_count + 3;
    let mut header: Vec<u8> = Vec::new();
    for i in 0..header_bytes {
        let byte = match file_iter.next() {
            Some(x) => x,
            None => return,
        };
        header.push(byte.unwrap());
    }

    let mut bytes_written = 0  + header_bytes as usize;
    let mut seconds_written = Duration::new(0, 0);
    let mut buffer: Vec<u8> = Vec::new();
    let mut counter = 1;
    // create output directory
    fs::create_dir(Path::new(&output_directory)).ok();
    loop {
        // pos_size are pairs corresponding to an mpeg frame
        let (position, size) = match pos_size.next() {
            Some(pair) => (pair.0, pair.1),
            None => {break},
        };
        // load buffer with next frame
        for i in 0..size {
            let byte = match file_iter.next() {
                Some(x) => x,
                None => break,
            };
            buffer.push(byte.unwrap());
        }

        let is_last_frame = match pos_size.peek() {
            Some(pos) => false,
            None => true,
        };

        if is_last_frame || position > partition_size + seconds_written {
            let output_path = Path::new(&output_directory).join(
                format!("{}-{}.mp3", output_filename, counter)
            );
            let mut f: File = File::create(output_path).expect("Could not create file");
            let n = f.write(header.as_slice()).expect("Unable to write to header");
            if n != header_bytes as usize {
                panic!("Wrote too few bytes to file");
            }
            let n = f.write(buffer.as_slice()).expect("Unable to write frame to file");
            if n != buffer.len() as usize {
                panic!("Wrote too few bytes to file");
            }
            bytes_written = bytes_written + buffer.len();
            seconds_written = position;
            buffer.clear();
            println!("Wrote partition {} up to {:} seconds", counter, position.as_secs() );
            counter += 1;
        }
    }
}
