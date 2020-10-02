use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct App {
    connect_thread: Option<thread::JoinHandle<()>>,
    server_thread: Option<thread::JoinHandle<()>>,
    worker_thread: Option<thread::JoinHandle<()>>,
    tx_event: Option<Arc<Mutex<mpsc::Sender<AppEvent>>>>,
}

const BUFFER_SIZE: usize = 1024;

impl Default for App {
    fn default() -> Self {
        App::new()
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.stop();
    }
}

impl App {
    pub fn new() -> Self {
        App {
            connect_thread: None,
            server_thread: None,
            worker_thread: None,
            tx_event: None,
        }
    }

    fn create_tcp_server_thread(
        tx_clients: Arc<Mutex<std::sync::mpsc::Sender<AppEvent>>>,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let tcp_listner = TcpListener::bind("127.0.0.1:10000");
            match tcp_listner {
                Ok(tcp_listner) => {
                    println!("Tcp server is started");
                    for socket in tcp_listner.incoming() {
                        match socket {
                            Ok(socket) => {
                                println!("New tcp client connected");
                                let socket = AppEvent::NewTcpClient(socket);
                                let tx_clients = tx_clients.lock().unwrap();
                                tx_clients.send(socket).unwrap();
                            }
                            Err(e) => {
                                eprintln!("Error with tcp client: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Tcp server error: {}", e);
                }
            }
        })
    }

    fn create_tcp_client_thread(
        tx_clients: Arc<Mutex<std::sync::mpsc::Sender<AppEvent>>>,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let socket = TcpStream::connect("127.0.0.1:8000");
            match socket {
                Ok(socket) => {
                    let mut buffer: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
                    let mut common_buffer = Vec::with_capacity(BUFFER_SIZE);
                    loop {
                        let read_count = (&socket).read(&mut buffer);
                        match read_count {
                            Ok(0) => {
                                println!("Socket closed");
                                break;
                            }
                            Ok(read_count) => {
                                println!("Received packet: {}", read_count);
                                common_buffer.extend_from_slice(&buffer[..read_count]);
                                if common_buffer.contains(&0x0A) || common_buffer.contains(&0x0D) {
                                    let last_index = common_buffer.len() - 1;
                                    if common_buffer[last_index] == 0 {
                                        println!("Remove parasyte byte");
                                        common_buffer.remove(last_index);
                                    }
                                    let ev = AppEvent::DataPacket(common_buffer);
                                    let tx_data = tx_clients.lock().unwrap();
                                    tx_data.send(ev).unwrap();
                                    common_buffer = Vec::with_capacity(BUFFER_SIZE);
                                } else if common_buffer.len() > BUFFER_SIZE {
                                    common_buffer.clear();
                                }
                            }
                            Err(e) => {
                                eprintln!("Socket read error: {}", e);
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Socket connect error: {}", e);
                }
            }
        })
    }

    fn create_worker_thread(
        rx_clients: std::sync::mpsc::Receiver<AppEvent>,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            let mut sockets: std::vec::Vec<TcpStream> = Vec::new();
            loop {
                let ev = rx_clients.recv().unwrap();
                match ev {
                    AppEvent::Exit => break,
                    AppEvent::NewTcpClient(socket) => sockets.push(socket),
                    AppEvent::DataPacket(packet) => {
                        for socket in &mut sockets {
                            let write_result = socket.write(&packet[..]);
                            match write_result {
                                Ok(0) => {
                                    println!("Socket closed");
                                }
                                Ok(w) => {
                                    println!("Data sent to client: {}", w);
                                }
                                Err(e) => {
                                    eprintln!("Write to client error: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        })
    }

    pub fn run(&mut self) {
        let (tx_event, rx_event) = mpsc::channel();
        let tx_event = Arc::new(Mutex::new(tx_event));
        let tcp_server_thread = App::create_tcp_server_thread(Arc::clone(&tx_event));
        self.server_thread = Some(tcp_server_thread);
        let tcp_client_thread = App::create_tcp_client_thread(Arc::clone(&tx_event));
        self.connect_thread = Some(tcp_client_thread);
        let worker_thread = App::create_worker_thread(rx_event);
        self.worker_thread = Some(worker_thread);
    }

    pub fn stop(&mut self) {
        if let Some(x) = self.tx_event.take() {
            let tx_event = x.lock().unwrap();
            tx_event.send(AppEvent::Exit).unwrap();
            if let Some(t) = self.worker_thread.take() {
                t.join().unwrap();
            }
        }
    }
}

enum AppEvent {
    NewTcpClient(TcpStream),
    DataPacket(Vec<u8>),
    Exit,
}
