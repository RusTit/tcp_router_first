use std::io::stdin;
use tcp_router_first::App;

fn main() {
    let mut app = App::new();
    app.run();
    let console = stdin();
    let mut input_buffer = String::new();
    loop {
        console.read_line(&mut input_buffer).unwrap();
        if input_buffer.contains("exit") {
            break;
        }
    }
}
