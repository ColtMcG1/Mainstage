use mainstage_core::vm::VM;

fn write_u32_le(buf: &mut Vec<u8>, v: u32) {
    buf.extend(&v.to_le_bytes());
}
fn write_string(buf: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    write_u32_le(buf, bytes.len() as u32);
    buf.extend(bytes);
}

#[test]
fn plugin_call_unknown_plugin_returns_error() {
    // Build minimal bytecode image with a PluginCall to a plugin not registered
    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend(b"MSBC");
    write_u32_le(&mut bytes, 1); // version
    write_u32_le(&mut bytes, 2); // op count

    // Op 0: PluginCall (0x72)
    bytes.push(0x72);
    write_string(&mut bytes, "no_such_plugin"); // plugin_name
    write_string(&mut bytes, "doit"); // func_name
    write_u32_le(&mut bytes, 0); // argc
    write_u32_le(&mut bytes, 0); // has_result = 0

    // Op 1: Halt (0x50)
    bytes.push(0x50);

    let vm = VM::new(bytes);
    let res = vm.run(false);
    assert!(res.is_err(), "Expected error when calling unknown plugin");
    let msg = res.err().unwrap();
    assert!(msg.contains("unknown plugin"), "Error message should mention unknown plugin, got: {}", msg);
}
