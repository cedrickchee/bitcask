use bson::{self, decode_document, encode_document, Bson, Document};
use std::io::Cursor;

use bb_2_3_exercise::Person;

/// Serialize and deserialize 1000 data structures with serde (BSON).
///
/// This one is slightly different. Where the previous exercises serialized and deserialized a
/// single value to a buffer, in this one serialize 1000 different Move values to a single file,
/// back-to-back, then deserialize them again. This time use the BSON format.
///
/// Things to discover here are whether serde automatically maintains the correct file offsets (the
/// "cursor") to deserialize multiple values in sequence, or if you need to parse your own "frames"
/// around each value to define their size, and how to detect that there are no more values to
/// parse at the end of the file.
///
/// After you've succeeded at serializing and deserializing multiple values to a file, try it again
/// with a Vec<u8>. Serializing and deserializing generally requires the destination implement the
/// Write and Read traits. Does Vec<u8> implement either or both? What is the behavior of those
/// implementations? You may need to wrap your buffer in wrapper types that implement these traits
/// in order to get the correct behavior — the API docs for the traits list all their implementors
/// in the standard library, and whatever you need will be in there somewhere.
fn main() {
    let person = Person {
        id: bson::oid::ObjectId::new().unwrap(),
        name: "John".to_string(),
        age: 30,
    };

    // ********************************************************************************************
    // Serialize the struct
    // ********************************************************************************************

    println!("********** Basic - serialize/deserialize the struct **********");

    // Encode a T Serializable into a BSON Value.
    let serialized_person = bson::to_bson(&person).unwrap();

    if let Bson::Document(document) = &serialized_person {
        // mongoCollection.insert_one(document, None)?;  // Insert into a MongoDB collection
        println!(
            "BSON object converted into a MongoDB document: {}",
            &document
        );
    } else {
        println!("Error converting the BSON object into a MongoDB document");
    }

    // ********************************************************************************************
    // Deserialize the struct
    // ********************************************************************************************

    // Decode a BSON Value into a T Deserializable.

    // Deserialize the BSON into a Person instance
    let person: Person = bson::from_bson(serialized_person).unwrap();
    println!("bson::from_bson: {:?}", person);

    // ********************************************************************************************
    // Serialize 2 different Person values to a single file, then deserialize them again
    // ********************************************************************************************

    println!("********** Serialize/Deserialize 2 values **********");

    // BSON is a binary format in which zero or more key/value pairs are stored as a single entity.
    // We call this entity a document.
    let mut doc = Document::new();

    // ********** Serialize value 1 **********
    let person1 = Person {
        id: bson::oid::ObjectId::new().unwrap(),
        name: "David".to_string(),
        age: 28,
    };
    let ser_person1 = bson::to_bson(&person1).unwrap();
    doc.insert_bson("person1".to_owned(), ser_person1);

    // ********** Serialize value 2 **********
    let person2 = Person {
        name: "James".to_string(),
        ..person1
    };
    let ser_person2 = bson::to_bson(&person2).unwrap();
    doc.insert_bson("person2".to_owned(), ser_person2);

    println!("New document: {}", doc);

    // ********** Store document in-memory instead of file **********
    let mut buf = Vec::new();
    // Attempt to encode a `Document` into a byte stream.
    encode_document(&mut buf, &doc).unwrap();

    // ********** Deserialize document from memory **********
    // Attempt to decode a `Document` from a byte stream.
    let doc = decode_document(&mut Cursor::new(&buf[..])).unwrap();
    println!("Document decoded from buffer: {}", doc);
}
