extern crate getopts;
extern crate mp3_metadata;

use mp3_metadata::MP3Metadata;

use std::env;
use std::time::Duration;
use std::io::prelude::*;
use std::path::Path;
use std::fs;
use std::fs::{File, Metadata};

use getopts::Options;


static USAGE: &str = "
msplit.

Usage:
  msplit <filename>
  msplit <filename> [--seconds=number] [--output=filename] [--dir=dirname]
  msplit <filename> [-s number] [-o filename] [-d dirname]

Options:
  -h --help     Show this screen.
  -s --seconds=number  Duration of partition [default: 10].
  -o --output=filename Partition filename prefix [default: partition]
  -d --dir=dirname  Output directory (created if it does not already exist) [default: partitions].
";


fn main() {
    /////////////////////////////
    // Arguments configuration //
    /////////////////////////////
    let mut opts = Options::new();
    opts.optopt("s", "seconds", "partition duration (in seconds)", "sec");
    // TODO: partition by bytes instead of duration
    opts.optopt("d", "dir", "output directory", "dir");
    opts.optopt("o", "output", "output filename", "outputr");
    opts.optflag("h", "help", "print help menu");
    let args: Vec<_> = env::args().collect();
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(_) => {
            println!("{}", USAGE);
            return
        },
    };
    if matches.opt_present("h") {
        println!("{}", USAGE);
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
        None => 10,
    };
    let partition_size = Duration::new(seconds, 0);


    ////////////////////////////////
    // Get bytes iterator of File //
    ////////////////////////////////
    let filename: &str = if !matches.free.is_empty() {
        matches.free[0].as_str()
    } else {
        println!("{:}", USAGE);
        return;
    };
    let path: &Path = Path::new(filename);

    let fileinfo: Metadata = match fs::metadata(path) {
        Ok(result) => result,
        Err(_) => {
            println!("File does not exist:\n\n{}", USAGE);
            return
        },
    };

    let mut file_iter = match File::open(filename) {
        Ok(f) => f,
        Err(_) => {
            println!("Couldn't open file {}\n\n{}", filename, USAGE);
            return
        },
    }.bytes();

    //////////////////////
    // Get MP3 metadata //
    //////////////////////
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
    for _ in 0..header_bytes {
        let byte = match file_iter.next() {
            Some(x) => x,
            None => return,
        };
        header.push(byte.unwrap());
    }

    /////////////////////////////////////////////////
    // Loop over mp3 frames and write from file to //
    // smaller partitioned files                   //
    /////////////////////////////////////////////////
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
        for _ in 0..size {
            let byte = match file_iter.next() {
                Some(x) => x,
                None => break,
            };
            buffer.push(byte.unwrap());
        }

        let is_last_frame = match pos_size.peek() {
            Some(_) => false,
            None => true,
        };

        if is_last_frame || position > partition_size + seconds_written {
            let output_path = Path::new(&output_directory).join(
                format!("{}-{}.mp3", output_filename, counter)
            );

            let mut f: File = File::create(output_path).expect("Could not create file");

            let _ = f.write(header.as_slice()).expect("Unable to write to header");
            let _ = f.write(buffer.as_slice()).expect("Unable to write frame to file");

            bytes_written = bytes_written + buffer.len();
            seconds_written = position;
            buffer.clear();

            println!("Wrote partition {} up to {:} seconds", counter, position.as_secs() );
            counter += 1;
        }
    }
}
