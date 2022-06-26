use crate::pkzip::{get_fixed_huffman_trees, BitArray, ByteStream, HuffmanTree};

#[test]
// Test that bytes can be bidirectionally cast with bits
fn byte_array_bidirectional_test()
{
    for i in 0u8..=255u8
    {
        let byte_array = BitArray::new(i);
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
        let ba = BitArray::new(i);
        let bits = ba.bits.to_vec();
        let ba2 = BitArray::new_from_bits(&bits);
        assert_eq!(ba.get_byte(), ba2.get_byte());
    }
}
#[test]
fn byte_array_creation_test()
{
    let byte_vec = vec![false, false, false, false, true, false, false, true];
    let ba2 = BitArray::new_from_bits(&byte_vec);
    assert_eq!(0b_0000_1001, ba2.get_byte());
}
#[test]
fn byte_array_reverse_test()
{
    let ba = BitArray::new(15);
    eprintln!("ba.bits = {:?}", ba.bits);
    let ba2 = ba.get_flipped();
    eprintln!("ba.get_flipped().bits = {:?}", ba2.bits);
    assert_eq!(
        ba2.bits,
        vec![true, true, true, true, false, false, false, false]
    );
}
#[test]
fn byte_stream_first_byte_test()
{
    let ba = BitArray::new(15);
    let bas = vec![ba];
    let mut bs = ByteStream::new(bas);
    assert_eq!(15, bs.read_byte().unwrap());
}
#[test]
fn huffman_tree_creation_test()
{
    let mut huffmanTree = HuffmanTree::new();
    huffmanTree.insert(0b10, 2, 1);
    huffmanTree.insert(0b0, 1, 2);
    huffmanTree.insert(0b110, 3, 3);
    huffmanTree.insert(0b111, 3, 4);

    println!("{:#?}", huffmanTree);
    assert_eq!(4u16, huffmanTree.get_value(0b111, 3).unwrap());
}
#[test]
fn huffman_tree_creation_from_bit_lengths_test()
{
    let mut huffman_tree = HuffmanTree::new();
    huffman_tree.insert(0b10, 2, 1);
    huffman_tree.insert(0b0, 1, 2);
    huffman_tree.insert(0b110, 3, 3);
    huffman_tree.insert(0b111, 3, 4);

    assert_eq!(4u16, huffman_tree.get_value(0b111, 3).unwrap());

    let huffman_tree2 = HuffmanTree::construct_from_bitlengths(
        &[1u16, 2u16, 3u16, 4u16],
        &[2u16, 1u16, 3u16, 3u16],
    );
    eprintln!("huffmanTree2 = {:#?}", huffman_tree2);
    assert_eq!(4u16, huffman_tree2.get_value(0b111, 3).unwrap());
}
#[test]
fn huffman_tree_fixed_test()
{
    let (literal_and_length, distance, _length_values, _dist_values) = get_fixed_huffman_trees();
    assert_eq!(255, literal_and_length.get_value(0b111111111, 9).unwrap());
}
//#[test]
fn huffman_tree_bit_decode_test()
{
    let mut huffman_tree = HuffmanTree::new();

    huffman_tree.insert(0b01, 2, 1);
    huffman_tree.insert(0b1, 1, 2);
    huffman_tree.insert(0b000, 3, 3);
    huffman_tree.insert(0b001, 3, 4);

    let k = vec![
        true, false, false, false, false, false, true, false, true, false, true, false, false,
        true, false, false,
    ]; // 2, 3, 4, 1, 1, 4 == 130, 164

    let b1 = BitArray::new(130);
    let b2 = BitArray::new(164);

    let mut bs = ByteStream::new(vec![b1, b2]);
    loop
    {
        let s = bs.read_next_symbol(&huffman_tree);
        println!("{}", s);
    }
}
