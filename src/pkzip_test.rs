use crate::pkzip::ByteArray;

#[test]
// Test that bytes can be bidirectionally cast with bits
fn byte_array_bidirectional_test()
{
    for i in 0u8..=255u8
    {

        let byte_array = ByteArray::new(i);
        println!("{:#?}", byte_array.bits);
        println!("{:#?}", i);
        let byte = byte_array.get_byte();
        println!("{:#?}", byte);
        assert_eq!(byte, i);
    }
}
