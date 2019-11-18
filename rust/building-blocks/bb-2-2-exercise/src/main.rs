use ron::{de, ser};
use std::str;

use bb_2_2_exercise::Move;

fn main() {
    let a = Move {
        x: String::from("b"),
        y: 3,
    };

    let ron_ser = ser::to_string(&a).unwrap();
    println!("********** Move direct RON serialization/deserialization ********** ");
    println!("ser::to_string: {:?}", ron_ser);
    let ron_de: Move = de::from_str(&ron_ser).unwrap();
    println!("de::from_str: {:?}", ron_de);

    println!("********** Move RON serialization/deserialization from buffer ********** ");
    // serialize String to a Vec<u8> buffer
    let ron_ser_bytes = ron_ser.as_bytes();
    let ron_ser_buf = ron_ser_bytes.to_vec();
    // deserialize
    let ron_de_bytes: Move = de::from_bytes(ron_ser_bytes).unwrap();
    println!("de::from_bytes: {:?}", ron_de_bytes);

    // Convert the Vec<u8> to String
    let ron_str = str::from_utf8(&ron_ser_buf).unwrap();
    // What Move looks like serialized to RON
    println!("ron str: {}", ron_str);
}
