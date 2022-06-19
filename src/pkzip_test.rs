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
#[test]
fn byte_array_creation_bidirectional_test()
{
    for i in 0u8..=255u8
    {
        let ba = ByteArray::new(i);
        let bits = ba.bits;
        let ba2 = ByteArray::new_from_bits(&bits);
        assert_eq!(ba.byte, ba2.byte);
    }
}
#[test]
fn byte_array_creation_test()
{
    let byte_vec = vec![false, false, false, false, true, false, false, true];
    let ba2 = ByteArray::new_from_bits(&byte_vec);
    assert_eq!(0b_0000_1001, ba2.byte);
}
