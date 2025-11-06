use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
struct Test {
    #[serde(with = "serde_bytes")]
    data: Vec<u8>,
}

fn main() {
    let test = Test {
        data: vec![101, 118, 101, 110, 116],
    };
    let json = serde_json::to_string(&test).unwrap();
    println!("Serialized: {}", json);
}
