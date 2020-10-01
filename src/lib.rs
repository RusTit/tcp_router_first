use std::error::Error;
use std::io::prelude::*;
use std::net::{SocketAddr, TcpStream};
use std::thread;
use std::time::Duration;

pub struct App {
    connect_thread: Option<thread::JoinHandle<()>>,
}

const BUFFER_SIZE: usize = 1024;

impl App {
    pub fn new() -> App {
        App {
            connect_thread: None,
        }
    }

    fn process_payload(v: &mut Vec<u8>) {
        if v.contains(&0x0D) || v.contains(&0x0A) {
            let last_index = v.len() - 1;
            if v[last_index] == 0 {
                unsafe {
                    v.set_len(last_index);
                }
            }
            v.clear();
        } else if v.len() > BUFFER_SIZE {
            v.clear();
        }
    }

    fn run_tcp_listner(&mut self) -> Result<(), Box<dyn Error>> {
        let join_handler = thread::spawn(move || {
            let addr = SocketAddr::from(([192, 168, 108, 26], 8005));
            let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(10));
            match stream {
                Ok(stream) => {
                    println!("Connection established.");
                    let mut buf: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
                    let mut general_buffer: Vec<u8> = Vec::with_capacity(BUFFER_SIZE * 2);
                    loop {
                        let read_result = (&stream).read(&mut buf);
                        match read_result {
                            Ok(0) => {
                                println!("Connection finished.");
                                break;
                            }
                            Ok(r) => {
                                println!("Data received: {}", r);
                                general_buffer.extend_from_slice(&buf[..r]);
                                App::process_payload(&mut general_buffer);
                            }
                            Err(e) => {
                                eprintln!("Err: {}", e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
        });
        self.connect_thread = Some(join_handler);
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), Box<dyn Error>> {
        match self.run_tcp_listner() {
            Ok(()) => {
                println!("Started well");
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}
