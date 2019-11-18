use std::fs::File;
use std::io::prelude::*;

use bb_2_1_exercise::Move;

fn main() {
    let a = Move {
        x: String::from("b"),
        y: 3,
    };

    // ********************************************************************************************
    // Serialize to a file
    // ********************************************************************************************

    // Convert the Move to a JSON string.
    let serialized = serde_json::to_string(&a).unwrap();
    println!("serialized = {}", serialized); // represented as `{"x":"b","y":3}`

    // Creates a new file and write bytes to it
    let mut file = File::create("game_moves.txt").unwrap();
    file.write_all(&serialized.as_bytes()).unwrap();

    // ********************************************************************************************
    // Deserialize back again to a variable
    // ********************************************************************************************

    // Read the contents of the file into a String
    let mut file = File::open("game_moves.txt").unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    assert_eq!(contents, "{\"x\":\"b\",\"y\":3}");

    // Deserialize the JSON string back to a Move.
    let c: Move = serde_json::from_str(&contents).unwrap();

    // Prints deserialized = Move { x: "b", y: 3 }
    println!("deserialized = {:?}", c);
}
