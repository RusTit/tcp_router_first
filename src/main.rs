use std::io::stdin;
use std::process;
use tcp_router_first::App;

fn main() {
    let mut app = App::new();
    match app.run() {
        Ok(()) => println!("All good"),
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
    let iio = stdin();
    let mut b_string = String::new();
    loop {
        println!("Type \"exit\" to stop the app");
        iio.read_line(&mut b_string).unwrap();
        if b_string == "exit" {
            break;
        }
    }
}
