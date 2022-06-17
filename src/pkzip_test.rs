use crate::pkzip::ByteArray;

#[test]
// Test that bytes can be bidirectionally cast with bits
fn byte_array_bidirectional_test()
{
    let byte_array = ByteArray::new(15u8);
    println!("{:#?}", byte_array.bits);
    println!("{:#?}", 15u8);
    let byte = byte_array.get_byte();
    println!("{:#?}", byte);
    assert_eq!(byte, 15u8);
}
