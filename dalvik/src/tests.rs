use super::*;
use decode::decode_all;

fn decode_and_display(ins: &[u16], expected: &[&str]) {
    let ins: Vec<String> = decode_all(ins, usize::MAX).unwrap().into_iter().map(|i| i.to_string()).collect();
    assert_eq!(ins.as_slice(), expected);
}

#[test]
fn move_object_from_16() {
    decode_and_display(&[0x0108, 0x001f], &["move-object/from16 v1, v31"]);
}

#[test]
fn const_string() {
    decode_and_display(&[0x031a, 0x1234], &["const-string v3, string@1234"]);
}

#[test]
fn const_string_jumbo() {
    decode_and_display(&[0x001b, 0x4ee5, 0x0021], &["const-string/jumbo v0, string@214ee5"]);
}

#[test]
fn invoke_static() {
    decode_and_display(&[0x2071, 0x4455, 0x0030], &["invoke-static {v0, v3}, method@4455"]);
}

#[test]
fn invoke_virtual() {
    decode_and_display(&[0x106e, 0xccdd, 0x0001], &["invoke-virtual {v1}, method@ccdd"]);
}

#[test]
fn move_result_object() {
    decode_and_display(&[0x040c], &["move-result-object v4"]);
}

#[test]
fn const4() {
    decode_and_display(&[0x7b12], &["const/4 v11, 0x7"]);
}

#[test]
fn if_nez() {
    decode_and_display(&[0x1039, 0x0401], &["if-nez v16, +1025"]);
}

#[test]
fn return_() {
    decode_and_display(&[0x030f], &["return v3"]);
}

#[test]
fn iget_object() {
    decode_and_display(&[0x2054, 0xbeef], &["iget-object v0, v2, field@beef"]);
}
