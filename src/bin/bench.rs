use axioma::{compress, decompress};

fn main() {
    let data = b"Axioma streaming compression test data payload.";
    let compressed = compress(data);
    let decompressed = decompress(&compressed).expect("Failed to decompress");
    assert_eq!(decompressed, data.to_vec());
    println!("Axioma benchmark passed successfully!");
}
