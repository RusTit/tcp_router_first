use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub struct App {
    server_thread: Option<thread::JoinHandle<()>>,
    worker_thread: Option<thread::JoinHandle<()>>,
    tx_event: Option<mpsc::Sender<AppEvent>>,
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
            server_thread: None,
            worker_thread: None,
            tx_event: None,
        }
    }

    fn create_tcp_server_thread(tx: mpsc::Sender<AppEvent>) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let tcp_listner = TcpListener::bind("127.0.0.1:3000");
            match tcp_listner {
                Ok(tcp_listner) => {
                    println!("Tcp server is started");
                    for socket in tcp_listner.incoming() {
                        match socket {
                            Ok(socket) => {
                                println!("New tcp client connected");
                                let socket = AppEvent::NewTcpClient(socket);
                                tx.send(socket).unwrap();
                            }
                            Err(e) => eprintln!("Error with tcp client: {}", e),
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Tcp server error: {}", e);
                }
            }
        })
    }

    fn process_common_buffer(mut common_buffer: Vec<u8>, tx: &mpsc::Sender<AppEvent>) -> Vec<u8> {
        if common_buffer.contains(&0x0A) || common_buffer.contains(&0x0D) {
            let last_index = common_buffer.len() - 1;
            if common_buffer[last_index] == 0 {
                println!("Remove parasyte byte");
                common_buffer.remove(last_index);
            }
            let ev = AppEvent::DataPacket(common_buffer);
            tx.send(ev).unwrap();
            common_buffer = Vec::with_capacity(BUFFER_SIZE);
        } else if common_buffer.len() > BUFFER_SIZE {
            common_buffer.clear();
        }
        common_buffer
    }

    fn process_client_socket(socket: &mut TcpStream, tx: &mpsc::Sender<AppEvent>) {
        let mut buffer: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
        let mut common_buffer = Vec::with_capacity(BUFFER_SIZE);
        loop {
            let read_count = socket.read(&mut buffer);
            match read_count {
                Ok(0) => {
                    println!("Socket closed");
                    break;
                }
                Ok(read_count) => {
                    println!("Received packet: {}", read_count);
                    common_buffer.extend_from_slice(&buffer[..read_count]);
                    common_buffer = App::process_common_buffer(common_buffer, &tx);
                }
                Err(e) => {
                    eprintln!("Socket read error: {}", e);
                    break;
                }
            }
        }
    }

    fn create_tcp_client_thread(tx: mpsc::Sender<AppEvent>) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let socket = TcpStream::connect("127.0.0.1:8000");
            match socket {
                Ok(mut socket) => App::process_client_socket(&mut socket, &tx),
                Err(e) => eprintln!("Socket connect error: {}", e),
            }
            tx.send(AppEvent::TcpConnectionLost).unwrap();
        })
    }

    fn create_worker_thread(
        tx_clients: mpsc::Sender<AppEvent>,
        rx_clients: mpsc::Receiver<AppEvent>,
    ) -> thread::JoinHandle<()> {
        App::create_tcp_client_thread(tx_clients.clone());
        thread::spawn(move || {
            let mut sockets: Vec<TcpStream> = Vec::new();
            loop {
                let ev = rx_clients.recv().unwrap();
                match ev {
                    AppEvent::Exit => break,
                    AppEvent::NewTcpClient(socket) => sockets.push(socket),
                    AppEvent::DataPacket(packet) => {
                        let mut temp_sockets: Vec<TcpStream> = Vec::with_capacity(sockets.len());
                        while let Some(mut socket) = sockets.pop() {
                            let write_result = socket.write(&packet[..]);
                            match write_result {
                                Ok(0) => println!("Client disconnected"),
                                Ok(w) => {
                                    println!("Data sent to client: {}", w);
                                    temp_sockets.push(socket);
                                }
                                Err(e) => eprintln!("Write to client error: {}", e),
                            }
                        }
                        sockets = temp_sockets;
                    }
                    AppEvent::TcpConnectionLost => {
                        println!("Waiting 5 seconds for new connection.");
                        thread::sleep(Duration::from_secs(5));
                        App::create_tcp_client_thread(tx_clients.clone());
                    }
                }
            }
        })
    }

    pub fn run(&mut self) {
        let (tx_event, rx_event) = mpsc::channel();
        let tcp_server_thread = App::create_tcp_server_thread(tx_event.clone());
        self.server_thread = Some(tcp_server_thread);
        let worker_thread = App::create_worker_thread(tx_event.clone(), rx_event);
        self.worker_thread = Some(worker_thread);
        self.tx_event = Some(tx_event);
    }

    pub fn stop(&mut self) {
        if let Some(tx_event) = self.tx_event.take() {
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
    TcpConnectionLost,
}
