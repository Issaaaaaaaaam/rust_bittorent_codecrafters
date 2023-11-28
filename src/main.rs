//use hex::encode;
use serde_json;
use core::panic;
use std::env;
use std::fs;
use sha1::{Sha1, Digest};
use serde::{de::Visitor, Deserializer, Serializer};
use serde::Deserialize;
use serde::Serialize;
//use std::vec;
// Available if you need it!
// use serde_bencode

fn decode_string(encoded_value: &[u8]) ->  (serde_json::Value, &[u8]) {
    let colon_index = encoded_value.iter().position(|&b| b == b':').expect("SOMETHING WRONG WITH THE STRING WHERE TF IS THE '!'");
    let code = &encoded_value[colon_index+1..];
    let encoder =  String::from_utf8_lossy(&encoded_value[..colon_index])
                    .parse::<usize>()
                    .expect("wtf");
    return (serde_json::Value::String(String::from_utf8_lossy(&code[..encoder]).to_string()), &code[encoder..])
}


fn decode_number(encoded_value: &[u8]) ->  (serde_json::Value, &[u8]) {
    let e_index = encoded_value.iter().position(|&b| b == b'e').unwrap();
    let number = String::from_utf8_lossy(&encoded_value[1..e_index])
        .parse::<i64>()
        .expect("There should be a problem with a number");
    (serde_json::Value::Number(number.into()), &encoded_value[e_index+1..])
}


fn decode_list(encoded_value: &[u8]) -> (serde_json::Value, &[u8]){
    let mut values = Vec::new();
    let mut encoded_value  = encoded_value.split_at(1).1;
    while !encoded_value.is_empty() && !encoded_value.starts_with(b"e"){
        let (value, remainder) = decode_bencoded_value(encoded_value);
        values.push(value);
        encoded_value = remainder;
    }
    return (values.into(), &encoded_value[1..])    
}


fn decode_dictionary(encoded_value: &[u8]) -> (serde_json::Value, &[u8]){
    let mut dict = serde_json::Map::new(); 
    let mut encoded_value = encoded_value.strip_prefix(b"d").unwrap();
    while !encoded_value.is_empty() && !encoded_value.starts_with(b"e"){
        let (key, rest) = decode_bencoded_value(&encoded_value);
        let key = match key {
            serde_json::Value::String(key) => key,
            key => {
                panic!("key must be strings, not {key:?}");
            }
        };
        let (value, rest) = decode_bencoded_value(rest);
        dict.insert(key, value);
        encoded_value = rest; 
    }
    return (dict.into(), &encoded_value[1..]);
}


#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &[u8]) -> (serde_json::Value, &[u8]) {
    match encoded_value.first()  {
        Some(b'i') => {
            return decode_number(encoded_value)
        }
        Some(b'l') => {
            return decode_list(encoded_value)
        }
        Some(b'd') => {
            return decode_dictionary(encoded_value)
        }
        Some(b'0'..=b'9') => {
            return decode_string(encoded_value)
        }
        _ => {
            print!("{}\n", String::from_utf8_lossy(&encoded_value));
            panic!("WTF HAPPENED") 
        }
    }
}
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct TorrentFile {
    announce: String,
    info: Info,
}
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct Info {
    length: usize,
    name: String,
    #[serde(rename="piece length")]
    piece_length: usize,
    pieces: Hashes,
}
#[derive(Debug, PartialEq, Eq)]
struct Hashes(Vec<[u8; 20]>);

/////////////////////////////////////////////// Code copied on a solution found on internet 
#[derive(Debug)]
struct HashesVisitor;
impl Serialize for Hashes {

    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&self.0.concat())
    }

}
impl<'de> Visitor<'de> for HashesVisitor {
    type Value = Hashes;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {

        //println!("{:?}",self);

        formatter.write_str("a byte string whose length is multiple of 20")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Hashes(
            v.chunks_exact(20)
                .map(|chunk| chunk.try_into().expect("guaranteed to be length 20"))
                .collect(),
        ))
    }
}
impl<'de> Deserialize<'de> for Hashes {

    fn deserialize<D>(deserializer: D) -> Result<Hashes, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_bytes(HashesVisitor)
    }
}
//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
fn info_hash(decoded_value: &TorrentFile) -> (String, String){
    let serialized = serde_bencode::to_bytes(&decoded_value.info).expect("Could not serialize info");
    let mut hasher = Sha1::new();
    hasher.update(&serialized);
    let hash = hasher.finalize();
    let hash:String = hex::encode(&hash).to_string(); 
    let pieces = serde_bencode::to_bytes(&decoded_value.info.pieces).unwrap().as_slice()[3..] //serialize  info.pieces(Hashes class) in a byte vector then extract as slice and remove the first 3 bytes. 
                        .iter().map(|slice| format!("{:02x}", slice).to_string()) // make the slice into an iterator and map each element to hex string
                        .collect::<Vec<String>>().join(""); // collect the iterable and flatten with no separator
    println!("{}", hash);
    return (hash, pieces)
}       


// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];
    
    if command == "decode" {
        // You can use print statements as follows for debugging, they'll be visible when running tests.
        // Uncomment this block to pass the first stage
        let encoded_value = &args[2];
        let decoded_value = decode_bencoded_value(encoded_value.as_bytes());
        println!("{}", decoded_value.0);
    } else if  command == "info" {
        let file_name = &args[2];
        let encoded = fs::read(file_name).unwrap();
        let decoded_value: TorrentFile = serde_bencode::from_bytes(&encoded).unwrap();
        let hash_info = info_hash(&decoded_value);
        println!("Tracker URL: {}", &decoded_value.announce.as_str());
        println!("Length: {}", &decoded_value.info.length);
        println!("Info Hash: {}\nHash Pieces: {}\n", hash_info.0, hash_info.1);
    } else {
        println!("unknown command: {}", args[1])
    }
}



#[cfg(test)]
mod tests {
    use serde_json::Value;
    use serde_json::json;
    use super::*;
    #[test]
    fn test_decode_string(){
        let encoded_value = "5:hello";
        assert_eq!("hello",decode_bencoded_value(encoded_value.as_bytes()).0);
    }
    #[test]
    fn test_decode_number(){
        let encoded_value = "i52e";
        assert_eq!(Value::Number(52.into()),decode_bencoded_value(encoded_value.as_bytes()).0);
    }
    #[test]
    fn test_decode_list(){
        let encoded_value = "l5:helloi52ee";
        assert_eq!(json!(["hello", 52]),decode_bencoded_value(encoded_value.as_bytes()).0);
    }
    #[test]
    fn test_decode_dictionary(){
        let encoded_value = "d3:foo3:bar5:helloi52ee";
        assert_eq!(json!({"foo":"bar","hello":52}),decode_bencoded_value(encoded_value.as_bytes()).0);
    }
}