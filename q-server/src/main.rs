use std::{
    net::{TcpListener, TcpStream, SocketAddr},
    io::{Read, Write},
    str::from_utf8_unchecked,
    fs::{OpenOptions, File},
};
use chrono::Timelike;
use threadpool::ThreadPool;

/**************************
 * CONSTANTS
***************************/

const LOG_DIRECTORY: &str = "../log/";

const MAX_THREADS: usize = 300;

const COMMAND_LENGTH: usize = 4; // commands are HELO, FILE, ABRA (from client) and SIZE, DATA, FAIL, DONE (from server)
const SERVER_SIDE_ERROR: &str = "internal server error";

// Expected string: HELO <ID>
const HELO_MSG_LENGTH: usize = COMMAND_LENGTH + 1 + 13;
const ABRA_MSG_LENGTH: usize = COMMAND_LENGTH + 1 + 1;

const TX: &str = "tx";
const RX: &str = "rx";

/**************************
 * DATA
***************************/

static FILE_BYTES: &[u8] = include_bytes!("../../csf303.png");

static mut SIZE_TO_SEND: Option<Vec<u8>> = None;
static mut DATA_TO_SEND: Option<Vec<u8>> = None;

/**************************
 * FUNCTIONS
***************************/

fn get_time() -> String {
    let time = chrono::Local::now().time();
    format!("{}:{}:{}", time.hour(), time.minute(), time.second())
}

fn write_log(log: &mut File, msg: &[u8], direction: &str) -> Result<(), ()> {
    if let Err(e) = writeln!(log, "{},{},{}", get_time(), unsafe { from_utf8_unchecked(msg) }, direction) {
        eprintln!("log failed: {}", e);
        return Err(());
    }
    log.flush().expect("flush failed");
    Ok(())
}

fn send_error(stream: &mut TcpStream, msg: &str, log: &mut File) {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"FAIL ");
    bytes.extend_from_slice(msg.as_bytes());

    stream.write_all(&bytes).expect("error sending failed");
    stream.flush().expect("flush failed");

    write_log(log, &bytes, TX).expect("log FAIL failed");
}

fn client_handler(mut stream: TcpStream, addr: SocketAddr, data_checksum: u8) {
    eprintln!("Handling client at {}", addr);

    let ip = match addr {
        SocketAddr::V4(addr) => addr.ip().to_string(),
        SocketAddr::V6(addr) => addr.ip().to_string(),
    };

    let mut path = String::from(LOG_DIRECTORY);
    path.extend(ip.chars());
    path.extend(".csv".chars());
    let mut log = match OpenOptions::new().append(true).create(true).open(path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("{} log file creation failed: {}", addr, e);
            let mut bytes = Vec::new();
            bytes.extend_from_slice(b"FAIL ");
            bytes.extend_from_slice(SERVER_SIDE_ERROR.as_bytes());

            stream.write_all(&bytes).expect("error sending failed");
            stream.flush().expect("flush failed");
            return;
        },
    };
    writeln!(log, "\n{} connected", addr.to_string()).expect("addr log failed");
    log.flush().expect("flush failed");

    // Wait for HELO <ID>

    let mut buf = [0u8; 32];

    let bytes_read = match stream.read(&mut buf) {
        Err(e) => {
            eprintln!("{} HELO read failed: {}", addr, e);
            send_error(&mut stream, SERVER_SIDE_ERROR, &mut log);
            return;
        },
        Ok(n) => n,
    };

    if let Err(()) = write_log(&mut log, &buf[..bytes_read], RX) {
        send_error(&mut stream, SERVER_SIDE_ERROR, &mut log);
        return;
    }

    if bytes_read != HELO_MSG_LENGTH || &buf[..COMMAND_LENGTH + 1] != b"HELO " {
        const MSG: &str = "invalid HELO format";
        eprintln!("{}", MSG);
        send_error(&mut stream, MSG, &mut log);
        return;
    }

    let id = unsafe { from_utf8_unchecked(&buf[5..bytes_read]) };
    eprintln!("ID: {}", id);

    // Send SIZE <length as string>

    let msg = unsafe { SIZE_TO_SEND.as_ref().unwrap() };

    if let Err(e) = stream.write_all(msg) {
        eprintln!("{} SIZE write failed: {}", addr, e);
        send_error(&mut stream, SERVER_SIDE_ERROR, &mut log);
        return;
    }
    stream.flush().expect("flush failed");

    if let Err(()) = write_log(&mut log, msg, TX) {
        send_error(&mut stream, SERVER_SIDE_ERROR, &mut log);
        return;
    }

    // Wait for "FILE "

    let mut buf = [0u8; 32];

    let bytes_read = match stream.read(&mut buf) {
        Err(e) => {
            eprintln!("{} FILE read failed: {}", addr, e);
            send_error(&mut stream, SERVER_SIDE_ERROR, &mut log);
            return;
        },
        Ok(n) => n,
    };

    if let Err(()) = write_log(&mut log, &buf[..bytes_read], RX) {
        send_error(&mut stream, SERVER_SIDE_ERROR, &mut log);
        return;
    }

    if bytes_read != COMMAND_LENGTH + 1 || &buf[..COMMAND_LENGTH + 1] != b"FILE " {
        const MSG: &str = "invalid FILE format";
        eprintln!("{}", MSG);
        send_error(&mut stream, MSG, &mut log);
        return;
    }

    // Send DATA <file bytes>

    let msg = unsafe { DATA_TO_SEND.as_ref().unwrap() };

    if let Err(e) = stream.write_all(msg) {
        eprintln!("{} DATA write failed: {}", addr, e);
        send_error(&mut stream, SERVER_SIDE_ERROR, &mut log);
        return;
    }
    stream.flush().expect("flush failed");

    if let Err(()) = write_log(&mut log, b"DATA <data>", TX) {
        send_error(&mut stream, SERVER_SIDE_ERROR, &mut log);
        return;
    }

    // Wait for ABRA <8-bit 2's complement checksum of data>

    let mut buf = [0u8; 32];

    let bytes_read = match stream.read(&mut buf) {
        Err(e) => {
            eprintln!("{} ABRA read failed: {}", addr, e);
            send_error(&mut stream, SERVER_SIDE_ERROR, &mut log);
            return;
        },
        Ok(n) => n,
    };

    if let Err(()) = write_log(&mut log, &buf[..bytes_read], RX) {
        send_error(&mut stream, SERVER_SIDE_ERROR, &mut log);
        return;
    }

    if bytes_read < ABRA_MSG_LENGTH || &buf[..COMMAND_LENGTH + 1] != b"ABRA " {
        const MSG: &str = "invalid ABRA format";
        eprintln!("{}", MSG);
        send_error(&mut stream, MSG, &mut log);
        return;
    }

    // Correct ABRA format received

    if &buf[COMMAND_LENGTH + 1..bytes_read] != data_checksum.to_string().as_bytes() {
        const MSG: &str = "invalid ABRA checksum";
        eprintln!("{}", MSG);
        send_error(&mut stream, MSG, &mut log);
        return;
    }

    // Correct ABRA checksum received
    // Send "DONE "

    eprintln!("DONE ID: {}", id);

    let msg = b"DONE ";

    if let Err(e) = stream.write_all(msg) {
        eprintln!("{} DONE write failed: {}", addr, e);
        send_error(&mut stream, SERVER_SIDE_ERROR, &mut log);
        return;
    }
    stream.flush().expect("flush failed");

    if let Err(()) = write_log(&mut log, msg, TX) {
        send_error(&mut stream, SERVER_SIDE_ERROR, &mut log);
        return;
    }
}

fn main() {
    let mut args = std::env::args();
    args.next();
    let addr = args.next().expect("expecting socket address to bind to as argument");

    let mut size_to_send = Vec::new();
    size_to_send.extend_from_slice(format!("SIZE {}", FILE_BYTES.len().to_string()).as_bytes());
    unsafe { SIZE_TO_SEND = Some(size_to_send); }

    let mut data_to_send = Vec::new();
    data_to_send.extend_from_slice(b"DATA ");
    data_to_send.extend_from_slice(FILE_BYTES);
    unsafe { DATA_TO_SEND = Some(data_to_send); }

    let data_checksum = FILE_BYTES.iter()
        .fold(0u8, |sum, e| sum.wrapping_add(*e))
        .wrapping_neg();

    eprintln!("checksum {}", data_checksum);

    let listener = TcpListener::bind(addr).expect("bind failed");

    std::fs::create_dir_all(LOG_DIRECTORY).expect("log directory creation failed");

    let thread_pool = ThreadPool::new(MAX_THREADS);

    loop {
        let (stream, addr) = match listener.accept() {
            Ok(result) => result,
            Err(e) => {
                eprintln!("accept failed: {}", e);
                continue;
            },
        };

        // execute only if first client from that IP?

        thread_pool.execute(move || client_handler(stream, addr, data_checksum));
    }
}
