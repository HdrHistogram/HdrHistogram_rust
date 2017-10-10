/// Reads numbers from stdin, one per line, and writes them to a serialized histogram on stdout.

extern crate hdrsample;
extern crate clap;

use std::io::BufRead;

use clap::{App, Arg};

use hdrsample::Histogram;
use hdrsample::serialization::{V2Serializer, V2DeflateSerializer};

fn main() {
    let default_max = format!("{}", u64::max_value());
    let matches = App::new("hdrsample serialize example")
            .arg(Arg::with_name("min")
                    .long("min")
                    .help("Minimum discernible value")
                    .takes_value(true)
                    .default_value("1"))
            .arg(Arg::with_name("max")
                    .long("max")
                    .help("Maximum trackable value")
                    .takes_value(true)
                    .default_value(default_max.as_str()))
            .arg(Arg::with_name("sigfig")
                    .long("sigfig")
                    .help("Number of significant digits")
                    .takes_value(true)
                    .default_value("3"))
            .arg(Arg::with_name("compression")
                    .short("c")
                    .long("compression")
                    .help("Enable compression"))
            .get_matches();

    let min = matches.value_of("min").unwrap().parse().unwrap();
    let max = matches.value_of("max").unwrap().parse().unwrap();
    let sigfig = matches.value_of("sigfig").unwrap().parse().unwrap();

    let mut h: Histogram<u64> = Histogram::new_with_bounds(min, max, sigfig).unwrap();

    let stdin = std::io::stdin();
    let stdin_handle = stdin.lock();

    for num in stdin_handle.lines()
        .map(|l| l.expect("Should be able to read stdin"))
        .map(|s| s.parse().expect("Each line must be a u64")) {

        h.record(num).unwrap();
    }

    let stdout = std::io::stdout();
    let mut stdout_handle = stdout.lock();

    if matches.is_present("compression") {
        V2DeflateSerializer::new().serialize(&h, &mut stdout_handle).unwrap();
    } else {
        V2Serializer::new().serialize(&h, &mut stdout_handle).unwrap();
    }
}
