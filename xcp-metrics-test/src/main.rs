use std::{os::unix::net::UnixListener, io::Read};

fn main() {
    let socket = UnixListener::bind("/var/lib/xcp/xcp-rrdd.forwarded").unwrap();
    
    while let Ok((mut socket, addr)) = socket.accept() {
        println!("{addr:?}");
        println!("Reponse:\n");
        let mut buffer = String::new();
        socket.read_to_string(&mut buffer).unwrap();
        println!("{buffer}");
    }
}
