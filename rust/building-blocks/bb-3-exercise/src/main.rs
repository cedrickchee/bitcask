use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

/// Exercise: Write a Redis ping-pong client and server using std::io.
///
/// Write a simple client and server that speaks the Redis protocol, with the client issuing
/// PING commands and the server responding appropriately. Use the std::io APIs to read and
/// write bytes directly.
fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    for stream in listener.incoming() {
        println!("Connection established");
        handle_connection(stream);
        // _process_stream(stream.unwrap());
    }
}

fn handle_connection(stream: Result<TcpStream, std::io::Error>) {
    match stream {
        Ok(mut stream) => {
            // ********** Reading request from the stream **********
            let mut buffer = Vec::new();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            while let Ok(read_bytes) = reader.read_until(b'\n', &mut buffer) {
                if read_bytes == 0 {
                    break;
                }

                let message = String::from_utf8_lossy(&buffer); // convert buffer to String
                println!("Message received: {}", message);

                // ********** Writing response to the stream **********
                let response = b"+PONG\r\n";
                stream.write(response).expect("Response failed");
            }
        }
        Err(e) => println!("Unable to connect: {}", e),
    }
}

fn _process_stream(mut stream: TcpStream) {
    // ********** Reading request from the stream **********
    let mut buffer = [0; 32]; // byte array (or formally known as slice of bytes)
    let _r_size = stream.read(&mut buffer).unwrap(); // unbuffered reader
    let ping = String::from_utf8_lossy(&buffer); // convert buffer to String
    println!("Message received: {}.", ping);
    assert_ne!("ping", ping);

    // ********** Writing response to the stream **********
    let pong = "pong";
    let w_size = stream.write(pong.as_bytes()).unwrap(); // convert &str to &[u8]
    println!("wrote {} bytes.", w_size);
}
