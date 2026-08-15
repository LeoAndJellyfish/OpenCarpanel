use std::{
    collections::BTreeMap,
    env,
    error::Error,
    fmt::{self, Display, Formatter},
    fs::{self, File},
    io::{self, BufReader, BufWriter},
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket},
    path::PathBuf,
    process::ExitCode,
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use opensimdash_adapter_api::AdapterId;
use opensimdash_telemetry_replay::{
    CaptureHeader, CaptureReader, CaptureRecord, CaptureWriter, DatagramSink, MAX_DATAGRAM_LEN,
    SystemReplayClock, replay_stream,
};

const DEFAULT_F1_ADDRESS: &str = "0.0.0.0:20777";
const DEFAULT_REPLAY_TARGET: &str = "127.0.0.1:20777";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    let Some(command) = arguments.first() else {
        print_help();
        return Ok(());
    };

    match command.as_str() {
        "--help" | "-h" | "help" => {
            print_help();
            Ok(())
        }
        "record" => record_command(&arguments[1..]),
        "replay" => replay_command(&arguments[1..]),
        _ => Err(CliError::new(format!("unknown command {command:?}; use --help")).into()),
    }
}

fn record_command(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    if wants_help(arguments) {
        print_record_help();
        return Ok(());
    }
    let options = Options::parse(arguments)?;
    options.reject_unknown(&["--adapter", "--bind", "--max-packets", "--output"])?;

    let output = PathBuf::from(options.required("--output")?);
    let bind_address = parse_socket(options.get_or("--bind", DEFAULT_F1_ADDRESS), "--bind")?;
    let adapter_id = AdapterId::new(options.get_or("--adapter", "f1-24"))?;
    let max_packets = options
        .get("--max-packets")
        .map(|value| parse_u64(value, "--max-packets"))
        .transpose()?;

    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    let file = File::options().write(true).create_new(true).open(&output)?;
    let header = CaptureHeader::new(adapter_id, unix_time_millis()?);
    let mut writer = CaptureWriter::new(BufWriter::new(file), &header)?;
    let socket = UdpSocket::bind(bind_address)?;
    socket.set_read_timeout(Some(Duration::from_millis(250)))?;

    let keep_running = Arc::new(AtomicBool::new(true));
    let signal_flag = Arc::clone(&keep_running);
    ctrlc::set_handler(move || signal_flag.store(false, Ordering::SeqCst))?;

    eprintln!(
        "Privacy warning: captures may contain session or player data. Review before sharing."
    );
    eprintln!(
        "Recording {} to {}",
        header.adapter_id().as_str(),
        output.display()
    );

    let started_at = Instant::now();
    let mut buffer = vec![0_u8; MAX_DATAGRAM_LEN].into_boxed_slice();
    let mut packet_count = 0_u64;
    while keep_running.load(Ordering::SeqCst)
        && max_packets.is_none_or(|maximum| packet_count < maximum)
    {
        match socket.recv_from(&mut buffer) {
            Ok((length, _source)) => {
                let delta_us = u64::try_from(started_at.elapsed().as_micros()).unwrap_or(u64::MAX);
                writer.write_record(&CaptureRecord::new(delta_us, buffer[..length].to_vec()))?;
                packet_count = packet_count.saturating_add(1);
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) => {}
            Err(error) => return Err(error.into()),
        }
    }
    writer.flush()?;
    println!("Recorded {packet_count} datagrams to {}", output.display());
    Ok(())
}

fn replay_command(arguments: &[String]) -> Result<(), Box<dyn Error>> {
    if wants_help(arguments) {
        print_replay_help();
        return Ok(());
    }
    let options = Options::parse(arguments)?;
    options.reject_unknown(&["--input", "--speed", "--target"])?;

    let input = PathBuf::from(options.required("--input")?);
    let target = parse_socket(
        options.get_or("--target", DEFAULT_REPLAY_TARGET),
        "--target",
    )?;
    let speed = options
        .get("--speed")
        .map_or(Ok(1.0), |value| parse_f64(value, "--speed"))?;
    let file = File::open(&input)?;
    let mut reader = CaptureReader::new(BufReader::new(file))?;
    let socket = UdpSocket::bind(unspecified_address(target.ip()))?;
    let mut sink = UdpSink { socket, target };
    let mut clock = SystemReplayClock::default();

    println!(
        "Replaying {} capture to {target} at {speed}x",
        reader.header().adapter_id().as_str()
    );
    let sent = replay_stream(&mut reader, speed, &mut clock, &mut sink)?;
    println!("Replayed {sent} datagrams");
    Ok(())
}

#[derive(Debug)]
struct UdpSink {
    socket: UdpSocket,
    target: SocketAddr,
}

impl DatagramSink for UdpSink {
    fn send(&mut self, datagram: &[u8]) -> io::Result<()> {
        let sent = self.socket.send_to(datagram, self.target)?;
        if sent == datagram.len() {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::WriteZero,
                format!("sent {sent} of {} UDP bytes", datagram.len()),
            ))
        }
    }
}

#[derive(Debug, Default)]
struct Options {
    values: BTreeMap<String, String>,
}

impl Options {
    fn parse(arguments: &[String]) -> Result<Self, CliError> {
        let mut values = BTreeMap::new();
        let mut index = 0;
        while index < arguments.len() {
            let name = &arguments[index];
            if !name.starts_with("--") {
                return Err(CliError::new(format!("unexpected argument {name:?}")));
            }
            let value = arguments
                .get(index + 1)
                .ok_or_else(|| CliError::new(format!("missing value for {name}")))?;
            if value.starts_with("--") {
                return Err(CliError::new(format!("missing value for {name}")));
            }
            if values.insert(name.clone(), value.clone()).is_some() {
                return Err(CliError::new(format!("duplicate option {name}")));
            }
            index += 2;
        }
        Ok(Self { values })
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.values.get(name).map(String::as_str)
    }

    fn get_or<'a>(&'a self, name: &str, default: &'a str) -> &'a str {
        self.get(name).unwrap_or(default)
    }

    fn required(&self, name: &str) -> Result<&str, CliError> {
        self.get(name)
            .ok_or_else(|| CliError::new(format!("required option {name} is missing")))
    }

    fn reject_unknown(&self, allowed: &[&str]) -> Result<(), CliError> {
        if let Some(name) = self
            .values
            .keys()
            .find(|name| !allowed.contains(&name.as_str()))
        {
            Err(CliError::new(format!("unknown option {name}")))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug)]
struct CliError {
    message: String,
}

impl CliError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for CliError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for CliError {}

fn wants_help(arguments: &[String]) -> bool {
    arguments
        .iter()
        .any(|argument| matches!(argument.as_str(), "--help" | "-h"))
}

fn parse_socket(value: &str, option: &str) -> Result<SocketAddr, CliError> {
    SocketAddr::from_str(value)
        .map_err(|error| CliError::new(format!("invalid {option} address {value:?}: {error}")))
}

fn parse_u64(value: &str, option: &str) -> Result<u64, CliError> {
    u64::from_str(value)
        .map_err(|error| CliError::new(format!("invalid {option} value {value:?}: {error}")))
}

fn parse_f64(value: &str, option: &str) -> Result<f64, CliError> {
    f64::from_str(value)
        .map_err(|error| CliError::new(format!("invalid {option} value {value:?}: {error}")))
}

fn unix_time_millis() -> Result<u64, Box<dyn Error>> {
    let millis = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    u64::try_from(millis)
        .map_err(|_| CliError::new("system time is outside capture metadata range").into())
}

fn unspecified_address(target_ip: IpAddr) -> SocketAddr {
    match target_ip {
        IpAddr::V4(_) => SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
        IpAddr::V6(_) => SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), 0),
    }
}

fn print_help() {
    println!(
        "OpenSimDash telemetry capture/replay\n\n\
         Usage:\n  opensimdash-telemetry-replay record [OPTIONS]\n  \
         opensimdash-telemetry-replay replay [OPTIONS]\n\n\
         Commands:\n  record  Capture bounded UDP datagrams to a local file\n  \
         replay  Replay a capture to a configurable UDP target\n\n\
         Run a command with --help for its options."
    );
}

fn print_record_help() {
    println!(
        "Usage: opensimdash-telemetry-replay record --output PATH [OPTIONS]\n\n\
         Options:\n  --output PATH        New capture file; existing files are never overwritten\n  \
         --bind ADDRESS      UDP listen address (default: {DEFAULT_F1_ADDRESS})\n  \
         --adapter ID        Adapter slug (default: f1-24)\n  \
         --max-packets N     Stop after N datagrams; otherwise stop with Ctrl+C\n  \
         --help              Show this help"
    );
}

fn print_replay_help() {
    println!(
        "Usage: opensimdash-telemetry-replay replay --input PATH [OPTIONS]\n\n\
         Options:\n  --input PATH         Capture file to replay\n  \
         --target ADDRESS    UDP target (default: {DEFAULT_REPLAY_TARGET})\n  \
         --speed MULTIPLIER  Timing multiplier; 0 is immediate (default: 1)\n  \
         --help              Show this help"
    );
}
